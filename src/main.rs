#![allow(unused)]

mod block;
mod chain;
mod crypto;
mod storage;

use clap::Parser;
use owo_colors::OwoColorize;

use crate::{
    chain::ClaimChain,
    storage::{BlockStorage, MemoryStorage, SqliteStorage},
};

const DEFAULT_STORAGE: &str = "lendlink.db";

#[derive(Debug, Parser)]
#[command(version, about, long_about = "Leding Blockchain")]
struct CLI {
    #[arg(long)]
    storage: Option<String>,

    #[arg(long)]
    test: bool,
}

fn main() {
    let cli = CLI::parse();
    println!("{}", "A new Lend-Link node appears!".green());

    let block_storage: Box<dyn BlockStorage> = if cli.test {
        println!("{}", "Running in test mode".yellow());
        Box::new(MemoryStorage::init().map_err(|e| e.to_string()).unwrap())
    } else {
        match cli.storage {
            Some(path) => Box::new(
                SqliteStorage::init_with_path(&path)
                    .map_err(|e| e.to_string())
                    .unwrap(),
            ),
            None => Box::new(
                SqliteStorage::init_with_path(DEFAULT_STORAGE)
                    .map_err(|e| e.to_string())
                    .unwrap(),
            ),
        }
    };

    let mut blockchain = ClaimChain::new(block_storage);
    match blockchain.index_from_storage() {
        Err(e) => println!("{}", e.to_string()),
        _ => {}
    }

    match blockchain.get_tip() {
        Ok(_) => println!("{}", "Recovered save chain successfully!".green()),
        Err(e) => {
            println!("{}", "Did NOT find any saved chain");
            println!("{}", e.to_string());
        }
    }
}
