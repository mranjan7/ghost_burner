use std::fmt::format;
use clap::{Parser, Subcommand};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_account_decoder::UiAccountData;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::sol_str_to_lamports,
    native_token::LAMPORTS_PER_SOL,
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
use serde_json::Value;
use spl_token::instruction as token_instruction;
use spl_associated_token_account::get_associated_token_address;
use serde::Deserialize;

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
    Fund{ amount_sol: f64},
    Swap{ mint:String,
          amount_in_sol: f64},
    Burn,
    Sweep,
    List,
}

struct GhostWallet {
    keypair: Keypair,
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

    fn load(name: &str) -> Option<Self>{
        let path = format!("{}/{}.json", GHOST_DIR, name);
        if Path::new(&path).exists() {
            let bytes = fs::read(path).unwrap();
            let keypair = keypair_from_seed(&bytes).unwrap();
            let pubkey = keypair.pubkey();
            Some(Self{
                keypair,pubkey,
            })
        } else {
            None
        }
    }


}
#[derive(Debug, Deserialize)]
struct JupiterQuoteResponse{
    data: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct JupiterSwapResponse{
    #[serde(rename = "swapTransaction")]
    swap_transaction: String,

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
            let lamports = sol_str_to_lamports("amount_sol");

            let ix = system_instruction::transfer(&main_kp.pubkey(), &ghost.pubkey, lamports.unwrap());
            let recent_blockhash = client.get_latest_blockhash().unwrap();
            let msg = Message::new(&[ix], Some(&main_kp.pubkey()));
            let mut tx = Transaction::new_unsigned(msg);
            tx.sign(&[&main_kp], recent_blockhash);
            let sig = client.send_and_confirm_transaction(&tx).unwrap();
            println!("Funded {} SOL -> {} | tx: {}",amount_sol,ghost.pubkey,sig);
        }
       Commands::Swap {mint, amount_in_sol} => {
           let ghost = select_ghost_wallet();
           let mint = Pubkey::from_str(mint).expect("invalid mint");
           let jupiter_quote:JupiterQuoteResponse = reqwest::blocking::get(&format!("https://quoate-api.iup.ag/v6/quoate?inputMint=So111111111111111112\
                &outputMint = {}\
                 &amount={}\
                 &slippageBps=50",
           mint,
           sol_str_to_lamports(&format!("{}",*amount_in_sol)).unwrap()))
               .unwrap()
               .json()
               .unwrap();
           let url = format!("https://quote-api.jup.ag/v6/quote?...{}",mint);
            let jupiter_quote: JupiterQuoteResponse =reqwest::blocking::get(&url)
                .unwrap()
                .json()
                .unwrap();
           let route = &jupiter_quote.data[0];
           let swap_tx: String = reqwest::blocking::Client::new().post("https://quote-api.iup.ag/v6/swap")
               .json(&serde_json::json!({
                   "route": route,
                   "userPublicKey": ghost.pubkey.to_string(),
                   "wrapAndUnwrapSol":true,
               }))
               .send()
               .unwrap()
               .json()
               .unwrap()["swapTransaction"]
               .as_str()
               .unwrap()
               .to_string();

           let mut swap_tx: Transaction = bincode::deserialize(&base64::decode(swap_tx).unwrap()).unwrap();
           swap_tx.sign(&[&ghost.keypair],
           client.get_latest_blockhash().unwrap());

           let sig = client.send_and_confirm_transaction(&swap_tx).unwrap();
           println!("Swap executed -> {} | tx: {}",mint,sig);

       }
        Commands::Burn => {
            let ghost = select_ghost_wallet();
            let accounts = client.get_token_accounts_by_owner(&ghost.pubkey,
            TokenAccountsFilter::ProgramId(spl_token::id())).unwrap();
            for rcp_keyed_account in accounts {
                let token_account_pubkey = rcp_keyed_account.pubkey;
                let account = rcp_keyed_account.account;
                let token_acc = Pubkey::from_str(&token_account_pubkey).unwrap();
                let parsed = match &account.data{
                    UiAccountData::Json(parsed) => parsed,
                    _ => continue,
                };
                let mint = Pubkey::from_str(&parsed.parsed["info"]["mint"].as_str().unwrap()).unwrap();
                let ix = token_instruction::burn(
                    &spl_token::id(),
                    &token_acc,
                    &mint,
                    &ghost.pubkey,
                    &[],
                    parsed.parsed["info"]["tokenAmount"]["amount"].as_str().unwrap().parse::<u64>().unwrap())
                .unwrap();

                send_and_confirm(&client, &[ix], &ghost.keypair);
            }
            println!("All tokens burned for {}",ghost.pubkey);
        }
        Commands::Sweep => {
            let ghost = select_ghost_wallet();
            let main_pubkey = load_main_keypair().pubkey();
            let balance = client.get_balance(&ghost.pubkey).unwrap();
            if balance > 5000{
                let ix = system_instruction::transfer(&ghost.pubkey, &main_pubkey, balance-5000);
                send_and_confirm(&client, &[ix], &ghost.keypair);
            }
            fs::remove_file(format!("{}/{}.json", GHOST_DIR, "active")).ok();
            println!("Ghost wallet swept and deleted: {}",ghost.pubkey);
        }
        Commands::List => {
            if Path::new(GHOST_DIR).exists() {
                for entry in fs::read_dir(GHOST_DIR).unwrap() {
                    let name = entry.unwrap().file_name().into_string().unwrap();
                    println!("->{}", name);
                }
            }
        }
    }
}
fn select_ghost_wallet() -> GhostWallet {
    let path = format!("{}/active.json",GHOST_DIR);
    GhostWallet::load("active").expect("No active ghost wallet. Run 'ghost new' first.")
}
fn load_main_keypair() -> Keypair{
    let bytes = fs::read("main.json").expect("Put your main wallet as main.json (base58 or raw)");
    Keypair::from_bytes(&bytes).unwrap()
}

fn send_and_confirm(client: &RpcClient, ixs: &[Instruction], signer: &Keypair){
    let blockhash = client.get_latest_blockhash().unwrap();
    let msg = Message::new(ixs, Some(&signer.pubkey()));
    let mut tx = Transaction::new_unsigned(msg);
    tx.sign(&[signer], blockhash);
    client.send_and_confirm_transaction(&tx).unwrap();
}


