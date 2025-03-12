use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub sender: String,
    pub receiver: String,
    pub amount: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    pub previous_hash: String,
    pub nonce: u64,
    pub hash: String,
}

#[derive(Debug)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub pending_transactions: Vec<Transaction>,
    pub difficulty: usize,
    pub mining_reward: f32,
}

impl Block {
    pub fn new(index: u64, timestamp: u64, transactions: Vec<Transaction>, previous_hash: String) -> Self {
        let mut block = Block {
            index,
            timestamp,
            transactions,
            previous_hash,
            nonce: 0,
            hash: String::new(),
        };
        
        block.hash = block.calculate_hash();
        block
    }
    
    pub fn calculate_hash(&self) -> String {
        let block_data = serde_json::json!({
            "index": self.index,
            "timestamp": self.timestamp,
            "transactions": self.transactions,
            "previous_hash": self.previous_hash,
            "nonce": self.nonce,
        });
        
        let mut hasher = Sha256::new();
        hasher.update(block_data.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    pub fn mine_block(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);
        
        while &self.hash[0..difficulty] != target {
            self.nonce += 1;
            self.hash = self.calculate_hash();
        }
        
        println!("Block mined: {}", self.hash);
    }
}

impl Blockchain {
    pub fn new(difficulty: usize, mining_reward: f32) -> Self {
        let mut blockchain = Blockchain {
            chain: Vec::new(),
            pending_transactions: Vec::new(),
            difficulty,
            mining_reward,
        };
        
        blockchain.create_genesis_block();
        blockchain
    }
    
    pub fn create_genesis_block(&mut self) {
        let genesis_block = Block::new(
            0,
            Self::get_timestamp(),
            Vec::new(),
            String::from("0"),
        );
        
        self.chain.push(genesis_block);
    }
    
    pub fn get_latest_block(&self) -> &Block {
        self.chain.last().unwrap()
    }
    
    pub fn add_transaction(&mut self, sender: String, receiver: String, amount: f32) {
        let transaction = Transaction {
            sender,
            receiver,
            amount,
        };
        
        self.pending_transactions.push(transaction);
    }
    
    pub fn mine_pending_transactions(&mut self, mining_reward_address: String) {
        // Add mining reward transaction
        self.add_transaction(
            String::from("BLOCKCHAIN"),
            mining_reward_address.clone(),
            self.mining_reward,
        );
        
        let block = {
            let latest_block = self.get_latest_block();
            let mut new_block = Block::new(
                latest_block.index + 1,
                Self::get_timestamp(),
                self.pending_transactions.clone(),
                latest_block.hash.clone(),
            );
            
            new_block.mine_block(self.difficulty);
            new_block
        };
        
        self.chain.push(block);
        self.pending_transactions = Vec::new();
    }
    
    pub fn is_chain_valid(&self) -> bool {
        for i in 1..self.chain.len() {
            let current_block = &self.chain[i];
            let previous_block = &self.chain[i - 1];
            
            // Verify current hash
            if current_block.hash != current_block.calculate_hash() {
                println!("Current hash is invalid");
                return false;
            }
            
            // Verify link to previous hash
            if current_block.previous_hash != previous_block.hash {
                println!("Link to previous hash is broken");
                return false;
            }
        }
        
        true
    }
    
    pub fn get_balance(&self, address: &str) -> f32 {
        let mut balance = 0.0;
        
        for block in &self.chain {
            for transaction in &block.transactions {
                if transaction.sender == address {
                    balance -= transaction.amount;
                }
                
                if transaction.receiver == address {
                    balance += transaction.amount;
                }
            }
        }
        
        balance
    }
    
    pub fn get_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
    }
}

// fn main() {
//     // Create a new blockchain with difficulty 4 and 100 coins as mining reward
//     let mut my_blockchain = Blockchain::new(4, 100.0);
    
//     println!("Mining first block...");
//     my_blockchain.add_transaction(String::from("Alice"), String::from("Bob"), 50.0);
//     my_blockchain.mine_pending_transactions(String::from("Miner1"));
    
//     println!("Mining second block...");
//     my_blockchain.add_transaction(String::from("Bob"), String::from("Charlie"), 25.0);
//     my_blockchain.add_transaction(String::from("Alice"), String::from("Charlie"), 35.0);
//     my_blockchain.mine_pending_transactions(String::from("Miner1"));
    
//     println!("Miner1's balance: {}", my_blockchain.get_balance("Miner1"));
    
//     println!("Mining one more block for the miner to get the reward...");
//     my_blockchain.mine_pending_transactions(String::from("Miner1"));
    
//     println!("Miner1's balance: {}", my_blockchain.get_balance("Miner1"));
    
//     println!("Is blockchain valid? {}", my_blockchain.is_chain_valid());
// }