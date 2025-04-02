mod blockchain;
mod p2p;

use blockchain::{Blockchain, Transaction};
use clap::{App, Arg, SubCommand};
use log::{error, info};
use p2p::start_p2p_node;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Debug)]
enum BlockchainResponse {
    Success(String),
    Error(String),
}

async fn run_interactive_mode(blockchain: Arc<Mutex<Blockchain>>, tx: mpsc::Sender<BlockchainCommand>) {
    use std::io::{self, BufRead, Write};
    
    println!("Interactive mode started. Type 'help' for commands.");
    
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    loop {
        print!("blockchain> ");
        stdout.flush().unwrap();
        
        let mut line = String::new();
        stdin.lock().read_line(&mut line).unwrap();
        let input = line.trim();
        
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        
        match parts[0] {
            "help" => {
                println!("Available commands:");
                println!("  save <path>            - Save blockchain to disk");
                println!("  load <path>            - Load blockchain from disk");
                println!("  transaction <from> <to> <amount> - Create transaction");
                println!("  mine <address>         - Mine pending transactions");
                println!("  balance <address>      - Check balance");
                println!("  validate               - Validate blockchain");
                println!("  exit                   - Exit interactive mode");
                println!("  help                   - Show this help message");
            },
            "save" => {
                if parts.len() < 2 {
                    println!("Usage: save <path>");
                    continue;
                }
                
                // Create response channel
                let (resp_tx, mut resp_rx) = mpsc::channel::<BlockchainResponse>(1);
                
                let _ = tx.send(BlockchainCommand::SaveChain(parts[1].to_string(), resp_tx)).await;
                
                // Wait for response
                if let Some(response) = resp_rx.recv().await {
                    match response {
                        BlockchainResponse::Success(msg) => println!("{}", msg),
                        BlockchainResponse::Error(err) => println!("Error: {}", err),
                    }
                }
            },
            "load" => {
                if parts.len() < 2 {
                    println!("Usage: load <path>");
                    continue;
                }
                
                let (resp_tx, mut resp_rx) = mpsc::channel::<BlockchainResponse>(1);
                let _ = tx.send(BlockchainCommand::LoadChain(parts[1].to_string(), resp_tx)).await;
                
                // Wait for response
                if let Some(response) = resp_rx.recv().await {
                    match response {
                        BlockchainResponse::Success(msg) => println!("{}", msg),
                        BlockchainResponse::Error(err) => println!("Error: {}", err),
                    }
                }
            },
            "transaction" => {
                if parts.len() < 4 {
                    println!("Usage: transaction <from> <to> <amount>");
                    continue;
                }
                let amount = match parts[3].parse::<f32>() {
                    Ok(val) => val,
                    Err(_) => {
                        println!("Invalid amount");
                        continue;
                    }
                };
                
                let transaction = Transaction {
                    sender: parts[1].to_string(),
                    receiver: parts[2].to_string(),
                    amount,
                };
                
                let (resp_tx, mut resp_rx) = mpsc::channel::<BlockchainResponse>(1);
                let _ = tx.send(BlockchainCommand::AddTransaction(transaction, resp_tx)).await;
                
                // Wait for response
                if let Some(response) = resp_rx.recv().await {
                    match response {
                        BlockchainResponse::Success(msg) => println!("{}", msg),
                        BlockchainResponse::Error(err) => println!("Error: {}", err),
                    }
                }
            },
            "mine" => {
                if parts.len() < 2 {
                    println!("Usage: mine <address>");
                    continue;
                }
                
                let (resp_tx, mut resp_rx) = mpsc::channel::<BlockchainResponse>(1);
                let _ = tx.send(BlockchainCommand::MineBlock(parts[1].to_string(), resp_tx)).await;
                
                // Wait for response
                if let Some(response) = resp_rx.recv().await {
                    match response {
                        BlockchainResponse::Success(msg) => println!("{}", msg),
                        BlockchainResponse::Error(err) => println!("Error: {}", err),
                    }
                }
            },
            "balance" => {
                if parts.len() < 2 {
                    println!("Usage: balance <address>");
                    continue;
                }
                
                let (resp_tx, mut resp_rx) = mpsc::channel::<BlockchainResponse>(1);
                let _ = tx.send(BlockchainCommand::GetBalance(parts[1].to_string(), resp_tx)).await;
                
                // Wait for response
                if let Some(response) = resp_rx.recv().await {
                    match response {
                        BlockchainResponse::Success(msg) => println!("{}", msg),
                        BlockchainResponse::Error(err) => println!("Error: {}", err),
                    }
                }
            },
            "validate" => {
                let (resp_tx, mut resp_rx) = mpsc::channel::<BlockchainResponse>(1);
                let _ = tx.send(BlockchainCommand::ValidateChain(resp_tx)).await;
                
                // Wait for response
                if let Some(response) = resp_rx.recv().await {
                    match response {
                        BlockchainResponse::Success(msg) => println!("{}", msg),
                        BlockchainResponse::Error(err) => println!("Error: {}", err),
                    }
                }
            },
            "exit" => {
                println!("Exiting interactive mode");
                break;
            },
            _ => {
                println!("Unknown command. Type 'help' for available commands.");
            }
        }
    }
}

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
        .arg(
            Arg::with_name("interactive")
                .short("i")
                .long("interactive")
                .help("Start in interactive mode")
                .takes_value(false),
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
        .subcommand(
            SubCommand::with_name("save")
                .about("Save blockchain to disk")
                .arg(
                    Arg::with_name("path")
                        .help("Path to save blockchain")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("load")
                .about("Load blockchain from disk")
                .arg(
                    Arg::with_name("path")
                        .help("Path to load blockchain from")
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
    // Replace the existing tokio::spawn block with this:

    tokio::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                BlockchainCommand::AddTransaction(tx, resp_tx) => {
                    let response = {
                        let mut chain = blockchain_clone.lock().unwrap();
                        chain.add_transaction(tx.sender, tx.receiver, tx.amount);
                        BlockchainResponse::Success("Transaction added to pending pool".to_string())
                    };
                    let _ = resp_tx.send(response).await;
                }
                BlockchainCommand::MineBlock(address, resp_tx) => {
                    let response = {
                        let mut chain = blockchain_clone.lock().unwrap();
                        chain.mine_pending_transactions(&address);
                        BlockchainResponse::Success("Block mined successfully".to_string())
                    };
                    let _ = resp_tx.send(response).await;
                }
                BlockchainCommand::GetBalance(address, resp_tx) => {
                    let response = {
                        let chain = blockchain_clone.lock().unwrap();
                        let balance = chain.get_balance(&address);
                        BlockchainResponse::Success(format!("Balance for {}: {}", address, balance))
                    };
                    let _ = resp_tx.send(response).await;
                }
                BlockchainCommand::ValidateChain(resp_tx) => {
                    let response = {
                        let chain = blockchain_clone.lock().unwrap();
                        let valid = chain.is_chain_valid();
                        let status = if valid { "Valid" } else { "Invalid" };
                        BlockchainResponse::Success(format!("Blockchain validation: {}", status))
                    };
                    let _ = resp_tx.send(response).await;
                }
                BlockchainCommand::SaveChain(path, resp_tx) => {
                    let response = {
                        let chain = blockchain_clone.lock().unwrap();
                        match chain.save_to_disk(&path) {
                            Ok(_) => BlockchainResponse::Success(format!("Blockchain saved to {}", path)),
                            Err(e) => BlockchainResponse::Error(format!("Failed to save blockchain: {}", e)),
                        }
                    };
                    let _ = resp_tx.send(response).await;
                }
                BlockchainCommand::LoadChain(path, resp_tx) => {
                    let (difficulty, mining_reward) = {
                        let current = blockchain_clone.lock().unwrap();
                        (current.difficulty, current.mining_reward)
                    };
                    
                    let response = match Blockchain::load_from_disk(&path, difficulty, mining_reward) {
                        Ok(loaded_chain) => {
                            let mut chain = blockchain_clone.lock().unwrap();
                            *chain = loaded_chain;
                            BlockchainResponse::Success(format!("Blockchain loaded from {}", path))
                        }
                        Err(e) => BlockchainResponse::Error(format!("Failed to load blockchain: {}", e)),
                    };
                    let _ = resp_tx.send(response).await;
                }
            }
        }
    });

    // Handle CLI commands
    if let Some(matches) = matches.subcommand_matches("mine") {
        let address = matches.value_of("address").unwrap().to_string();
        // Create a channel to receive the response
        let (resp_tx, mut resp_rx) = mpsc::channel::<BlockchainResponse>(1);
        let _ = tx.send(BlockchainCommand::MineBlock(address, resp_tx)).await;
        // Wait for the response
        if let Some(response) = resp_rx.recv().await {
            match response {
                BlockchainResponse::Success(msg) => info!("{}", msg),
                BlockchainResponse::Error(err) => error!("{}", err),
            }
        }
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
        
        // Create a channel to receive the response
        let (resp_tx, mut resp_rx) = mpsc::channel::<BlockchainResponse>(1);
        let _ = tx.send(BlockchainCommand::AddTransaction(transaction, resp_tx)).await;
        // Wait for the response
        if let Some(response) = resp_rx.recv().await {
            match response {
                BlockchainResponse::Success(msg) => info!("{}", msg),
                BlockchainResponse::Error(err) => error!("{}", err),
            }
        }
    } else if let Some(matches) = matches.subcommand_matches("balance") {
        let address = matches.value_of("address").unwrap().to_string();
        
        // Create a channel to receive the response
        let (resp_tx, mut resp_rx) = mpsc::channel::<BlockchainResponse>(1);
        
        // Send the command with the response channel
        let _ = tx.send(BlockchainCommand::GetBalance(address, resp_tx)).await;
        
        // Wait for the response
        if let Some(response) = resp_rx.recv().await {
            match response {
                BlockchainResponse::Success(msg) => info!("{}", msg),
                BlockchainResponse::Error(err) => error!("{}", err),
            }
        }
    } else if matches.is_present("interactive") {
        let listen_addr = matches.value_of("listen_addr").unwrap().to_string();
        let peer = matches.value_of("peer").map(|s| s.to_string());
        
        // Start P2P node in the background if needed
        if peer.is_some() {
            let blockchain_clone = blockchain.clone();
            let listen_addr_clone = listen_addr.clone();
            let peer_clone = peer.clone();
            
            tokio::spawn(async move {
                info!("Starting P2P node in background...");
                let node_result = start_p2p_node(
                    blockchain_clone, 
                    &listen_addr_clone, 
                    peer_clone.as_deref()
                ).await;
                
                if let Err(e) = node_result {
                    error!("P2P node error: {}", e);
                }
            });
        }
        
        // Run interactive mode
        run_interactive_mode(blockchain, tx_clone).await;
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
    AddTransaction(Transaction, mpsc::Sender<BlockchainResponse>),
    MineBlock(String, mpsc::Sender<BlockchainResponse>),
    GetBalance(String, mpsc::Sender<BlockchainResponse>),
    ValidateChain(mpsc::Sender<BlockchainResponse>),
    SaveChain(String, mpsc::Sender<BlockchainResponse>),
    LoadChain(String, mpsc::Sender<BlockchainResponse>),
}