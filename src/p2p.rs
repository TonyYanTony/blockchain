use futures::StreamExt;
use libp2p::{
    core::upgrade,
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{Mdns, MdnsEvent},
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    NetworkBehaviour, PeerId, Transport,
};
use log::info;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc, sync::Mutex};
use tokio::sync::mpsc;

use crate::blockchain::{Block, Blockchain, Transaction};

// Define topics for different types of messages
static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));
static TRANSACTION_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("transactions"));

// Message types for blockchain communication
#[derive(Debug, Serialize, Deserialize)]
enum BlockchainMessage {
    NewBlock(Block),
    NewTransaction(Transaction),
    ChainRequest,
    ChainResponse(Vec<Block>),
}

// Define behavior for our P2P network
#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
struct BlockchainBehaviour {
    floodsub: Floodsub,
    mdns: Mdns,
    #[behaviour(ignore)]
    response_sender: mpsc::UnboundedSender<BlockchainResponse>,
}

// Define response types from the network
#[derive(Debug)]
enum BlockchainResponse {
    Blocks(Vec<Block>),
    Transactions(Vec<Transaction>),
    PeerDiscovered(PeerId),
    PeerExpired(PeerId),
}

// Handle FloodSub events
impl NetworkBehaviourEventProcess<FloodsubEvent> for BlockchainBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(message) = event {
            if let Ok(blockchain_message) = serde_json::from_slice::<BlockchainMessage>(&message.data) {
                match blockchain_message {
                    BlockchainMessage::NewBlock(block) => {
                        info!("Received new block from {:?}: {:?}", message.source, block);
                        // TODO: Validate and add block to chain
                    }
                    BlockchainMessage::NewTransaction(transaction) => {
                        info!("Received new transaction from {:?}: {:?}", message.source, transaction);
                        // TODO: Validate and add transaction to mempool
                    }
                    BlockchainMessage::ChainRequest => {
                        info!("Received chain request from {:?}", message.source);
                        // TODO: Send chain response
                    }
                    BlockchainMessage::ChainResponse(blocks) => {
                        info!("Received chain response from {:?} with {} blocks", message.source, blocks.len());
                        let _ = self.response_sender.send(BlockchainResponse::Blocks(blocks));
                    }
                }
            }
        }
    }
}

// Handle MDNS events for peer discovery
impl NetworkBehaviourEventProcess<MdnsEvent> for BlockchainBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(peers) => {
                for (peer_id, _) in peers {
                    info!("mDNS discovered peer: {:?}", peer_id);
                    self.floodsub.add_node_to_partial_view(peer_id);
                    let _ = self.response_sender.send(BlockchainResponse::PeerDiscovered(peer_id));
                }
            }
            MdnsEvent::Expired(peers) => {
                for (peer_id, _) in peers {
                    info!("mDNS expired peer: {:?}", peer_id);
                    if !self.mdns.has_node(&peer_id) {
                        self.floodsub.remove_node_from_partial_view(&peer_id);
                    }
                    let _ = self.response_sender.send(BlockchainResponse::PeerExpired(peer_id));
                }
            }
        }
    }
}

// P2P network manager
pub struct P2P {
    swarm: Swarm<BlockchainBehaviour>,
    response_receiver: mpsc::UnboundedReceiver<BlockchainResponse>,
    known_peers: HashSet<PeerId>,
}

impl P2P {
    pub async fn new() -> Self {
        // Create a random key for our identity
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        info!("Local peer id: {:?}", local_peer_id);

        // Set up noise for authenticated encryption
        let noise_keys = Keypair::<X25519Spec>::new()
            .into_authentic(&local_key)
            .expect("Failed to create noise keys");

        // Set up TCP transport with noise and mplex for multiplexing
        let transport = TokioTcpConfig::new()
            .nodelay(true)
            .upgrade(upgrade::Version::V1)
            .authenticate(NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(mplex::MplexConfig::new())
            .boxed();

        // Create a channel for handling responses
        let (response_sender, response_receiver) = mpsc::unbounded_channel();

        // Create MDNS service for peer discovery on local network
        let mdns = Mdns::new(Default::default())
            .await
            .expect("Failed to create MDNS service");

        // Set up FloodSub for message broadcasting
        let mut floodsub = Floodsub::new(local_peer_id);
        floodsub.subscribe(BLOCK_TOPIC.clone());
        floodsub.subscribe(TRANSACTION_TOPIC.clone());

        // Create the network behavior
        let behaviour = BlockchainBehaviour {
            floodsub,
            mdns,
            response_sender,
        };

        // Build the swarm
        let swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

        Self {
            swarm,
            response_receiver,
            known_peers: HashSet::new(),
        }
    }

    // Start listening on the given address
    pub async fn start(&mut self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let multiaddr = addr.parse()?;
        Swarm::listen_on(&mut self.swarm, multiaddr)
            .expect("Failed to listen on multiaddr");
        
        Ok(())
    }

    // Connect to a peer
    pub async fn connect(&mut self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let multiaddr = addr.parse()?;
        Swarm::dial_addr(&mut self.swarm, multiaddr)
            .expect("Failed to dial address");
        
        Ok(())
    }

    // Broadcast a new block to the network
    pub fn broadcast_block(&mut self, block: Block) {
        let message = BlockchainMessage::NewBlock(block);
        let json = serde_json::to_string(&message).expect("Failed to serialize message");
        self.swarm.behaviour_mut().floodsub.publish(BLOCK_TOPIC.clone(), json.as_bytes());
    }

    // Broadcast a new transaction to the network
    pub fn broadcast_transaction(&mut self, transaction: Transaction) {
        let message = BlockchainMessage::NewTransaction(transaction);
        let json = serde_json::to_string(&message).expect("Failed to serialize message");
        self.swarm.behaviour_mut().floodsub.publish(TRANSACTION_TOPIC.clone(), json.as_bytes());
    }

    // Request the blockchain from peers
    pub fn request_blockchain(&mut self) {
        let message = BlockchainMessage::ChainRequest;
        let json = serde_json::to_string(&message).expect("Failed to serialize message");
        self.swarm.behaviour_mut().floodsub.publish(BLOCK_TOPIC.clone(), json.as_bytes());
    }

    // Main event loop to process network events
    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    info!("Swarm event: {:?}", event);
                },
                response = self.response_receiver.recv() => {
                    if let Some(response) = response {
                        match response {
                            BlockchainResponse::Blocks(blocks) => {
                                // Process received blocks
                                // TODO: Validate chain and update if needed
                                info!("Received blocks: {}", blocks.len());
                            },
                            BlockchainResponse::Transactions(transactions) => {
                                // Process received transactions
                                // TODO: Add to mempool
                                info!("Received transactions: {}", transactions.len());
                            },
                            BlockchainResponse::PeerDiscovered(peer) => {
                                self.known_peers.insert(peer);
                            },
                            BlockchainResponse::PeerExpired(peer) => {
                                self.known_peers.remove(&peer);
                            },
                        }
                    }
                }
            }
        }
    }

    // Get the list of currently connected peers
    pub fn peers(&self) -> Vec<PeerId> {
        self.known_peers.iter().cloned().collect()
    }
}

// Example of how to use the P2P module with a blockchain
pub async fn start_p2p_node(
    blockchain: Arc<Mutex<Blockchain>>,
    listen_address: &str,
    known_peer: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create and start a new P2P node
    let mut p2p = P2P::new().await;
    p2p.start(listen_address).await?;
    
    // Connect to a known peer if specified
    if let Some(peer) = known_peer {
        p2p.connect(peer).await?;
    }
    
    // Request the current blockchain from peers
    p2p.request_blockchain();
    
    // Start the main event loop
    p2p.run().await;
    
    Ok(())
}