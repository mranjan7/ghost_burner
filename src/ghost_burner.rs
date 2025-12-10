use std::fmt::format;
use clap::{Parser, Subcommand};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::{
        so_to_lamports,
        LAMPORTS_PER_SOL,
    },
    pubkey::Pubkey,
    signature::{ Signer, Keypair, },
    signer::keypair::keypair_from_seed,
    transaction::Transaction,
    instruction::{AccountMeta, Instruction},
    system_instruction,
    message::Message,
};
use std::str::FromStr;
use std::fs;
use std::path::Path;
use std::process::Command;
use spl_token::instruction as token_instruction;
use spl_associated_token_account::get_associated_token_address;

const RPC_URL: &str = "https://api.devnet.solana.com";
const GHOST_DIR: &str = "ghost_wallets";

#[derive(Parser)]
#[command(name = "ghost", about = "ghost")]

struct Cli{
   #[command(subcommand)]
   command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    New,
    Fund(amount_sol: f64),
    Swap(mint: String, amount_in_sol: f64),
    Burn,
    Sweep,
    List,
}

struct GhostWallet {
    keypair: KeyPair,
    pubkey: Pubkey,
}

impl GhostWallet {
    fn new() -> Self {
        let keypair = Keypair::new();
        let pubkey = keypair.pubkey();
        Self { keypair, pubkey }
    }

    fn save(&self, name: &str){
        let path = format!("{}/{}.json", GHOST_DIR, name);
        fs::create_dir_all(GHOST_DIR).ok();
        fs::write(path,self.keypair.to_bytes()).expect("Failed to save ghost wallet");
        println!("Ghost wallet create : {} -> {}",name,self.pubkey);
    }

    fn load(name: &str) -> Option{
        let path = format!("{}/{}.json", GHOST_DIR, name);
        if Path::new(&path).exists() {
            let keypair = keypair_from_seed(&bytes).unwrap();
            Some(Self{
                keypair,pubkey:keypair.pubkey()
            })
        } else {
            None
        }
    }


}

fn main() {
    let cli = Cli::parse();
    let client = RpcClient::new_with_commitment(RPC_URL.into(), CommitmentConfig::confirmed());
    match &cli.command {
        Commands::New => {
            let ghost = GhostWallet::new();
            let name = format!("ghost_{}", chrono::Utc::now().timestamp());
            ghost.save(&name);
        }
        Commands::Fund{ amount_sol} => {
            let ghost = select_ghost_wallet();
            let main_kp = load_main_keypair();
            let lamports = sol_to_lamports("amount_sol");

            let ix = system_instruction::transfer(&main_kp.pubkey(), &ghost.pubkey, lamports);
            let recent_blockhash = client.get_latest_blockhash(0.unwrap();
            let msg = Message::new(&[ix], Some(&main_kp.pubkey()));
            let mut tx = Transaction::new_unsigned(msg);
            tx.sign(&[&main_kp], recent_blockhash);
            let sig = client.send_and_confirm_transaction(&tx).unwrap();
            println!("Funded {} SOL -> {} | tx: {}",amount_sol,ghost.pubkey,sig);
        }
       Commands::Swap {}
    }
}


