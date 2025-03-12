mod blockchain;
mod p2p;

use blockchain::{Blockchain, Transaction};
use clap::{App, Arg, SubCommand};
use log::{error, info};
use p2p::start_p2p_node;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    // Parse command line arguments
    let matches = App::new("RustChain")
        .version("0.1.0")
        .author("Your Name")
        .about("A blockchain implementation in Rust")
        .arg(
            Arg::with_name("listen_addr")
                .short("l")
                .long("listen")
                .value_name("ADDRESS")
                .help("Sets the listen address for p2p communication (e.g., /ip4/0.0.0.0/tcp/8000)")
                .takes_value(true)
                .default_value("/ip4/0.0.0.0/tcp/0"),
        )
        .arg(
            Arg::with_name("peer")
                .short("p")
                .long("peer")
                .value_name("PEER_ADDR")
                .help("Specifies a peer to connect to (e.g., /ip4/127.0.0.1/tcp/8001)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("difficulty")
                .short("d")
                .long("difficulty")
                .value_name("DIFFICULTY")
                .help("Sets the mining difficulty")
                .takes_value(true)
                .default_value("4"),
        )
        .arg(
            Arg::with_name("reward")
                .short("r")
                .long("reward")
                .value_name("REWARD")
                .help("Sets the mining reward")
                .takes_value(true)
                .default_value("100.0"),
        )
        .subcommand(
            SubCommand::with_name("mine")
                .about("Mine a new block with pending transactions")
                .arg(
                    Arg::with_name("address")
                        .help("Miner's reward address")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("transaction")
                .about("Create a new transaction")
                .arg(
                    Arg::with_name("sender")
                        .help("Sender's address")
                        .required(true),
                )
                .arg(
                    Arg::with_name("receiver")
                        .help("Receiver's address")
                        .required(true),
                )
                .arg(
                    Arg::with_name("amount")
                        .help("Amount to transfer")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("balance")
                .about("Check balance for an address")
                .arg(
                    Arg::with_name("address")
                        .help("Address to check")
                        .required(true),
                ),
        )
        .get_matches();

    // Parse difficulty and mining reward
    let difficulty = matches
        .value_of("difficulty")
        .unwrap()
        .parse::<usize>()
        .expect("Difficulty must be a number");
    
    let mining_reward = matches
        .value_of("reward")
        .unwrap()
        .parse::<f32>()
        .expect("Mining reward must be a number");

    // Create a new blockchain
    let blockchain = Arc::new(Mutex::new(Blockchain::new(difficulty, mining_reward)));
    
    // Create channels for communication with the P2P layer
    let (tx, mut rx) = mpsc::channel::<BlockchainCommand>(100);
    let tx_clone = tx.clone();

    // Handle blockchain commands in a separate task
    let blockchain_clone = blockchain.clone();
    tokio::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                BlockchainCommand::AddTransaction(tx) => {
                    let mut chain = blockchain_clone.lock().unwrap();
                    chain.add_transaction(tx.sender, tx.receiver, tx.amount);
                    info!("Transaction added to pending pool");
                }
                BlockchainCommand::MineBlock(address) => {
                    let mut chain = blockchain_clone.lock().unwrap();
                    chain.mine_pending_transactions(address);
                    info!("Block mined successfully");
                }
                BlockchainCommand::GetBalance(address) => {
                    let chain = blockchain_clone.lock().unwrap();
                    let balance = chain.get_balance(&address);
                    info!("Balance for {}: {}", address, balance);
                }
                BlockchainCommand::ValidateChain => {
                    let chain = blockchain_clone.lock().unwrap();
                    let valid = chain.is_chain_valid();
                    info!("Blockchain validation: {}", if valid { "Valid" } else { "Invalid" });
                }
            }
        }
    });

    // Handle CLI commands
    if let Some(matches) = matches.subcommand_matches("mine") {
        let address = matches.value_of("address").unwrap().to_string();
        let _ = tx.send(BlockchainCommand::MineBlock(address)).await;
    } else if let Some(matches) = matches.subcommand_matches("transaction") {
        let sender = matches.value_of("sender").unwrap().to_string();
        let receiver = matches.value_of("receiver").unwrap().to_string();
        let amount = matches
            .value_of("amount")
            .unwrap()
            .parse::<f32>()
            .expect("Amount must be a number");
        
        let transaction = Transaction {
            sender,
            receiver,
            amount,
        };
        
        let _ = tx.send(BlockchainCommand::AddTransaction(transaction)).await;
    } else if let Some(matches) = matches.subcommand_matches("balance") {
        let address = matches.value_of("address").unwrap().to_string();
        let _ = tx.send(BlockchainCommand::GetBalance(address)).await;
    } else {
        // No subcommand, start P2P node
        let listen_addr = matches.value_of("listen_addr").unwrap();
        let peer = matches.value_of("peer");
        
        info!("Starting P2P node...");
        info!("Listening on: {}", listen_addr);
        if let Some(peer_addr) = peer {
            info!("Connecting to peer: {}", peer_addr);
        }
        
        // Start P2P Node
        let node_result = start_p2p_node(blockchain, listen_addr, peer).await;
        
        if let Err(e) = node_result {
            error!("P2P node error: {}", e);
        }
    }

    Ok(())
}

// Commands for interacting with the blockchain
enum BlockchainCommand {
    AddTransaction(Transaction),
    MineBlock(String),
    GetBalance(String),
    ValidateChain,
}