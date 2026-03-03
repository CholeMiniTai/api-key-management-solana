use std::{rc::Rc, str::FromStr};

use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Signer},
    },
    Client, Cluster,
};
use anyhow::Result;
use api_key_management::{
    accounts as program_accounts, instruction as program_ix, ApiKeyAccount, READ, WRITE,
};
use clap::{Parser, Subcommand};
use comfy_table::{presets::UTF8_FULL, Table};
use sha2::{Digest, Sha256};

#[derive(Parser)]
#[command(name = "api-key-cli")]
struct Cli {
    #[arg(long, default_value = "devnet")]
    cluster: String,
    #[arg(long, default_value = "~/.config/solana/id.json")]
    wallet: String,
    #[arg(long, default_value = "BUxg7dR7avAMivxrXizTgj64LQEiYrk8Qjt3xSN4JRDc")]
    program_id: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    InitRegistry,
    CreateKey { key_id: u64, name: String, raw_key: String, #[arg(long, default_value_t = READ | WRITE)] permissions: u64, #[arg(long)] expires_at: Option<i64>, #[arg(long, default_value = "") ] metadata: String },
    RevokeKey { key_id: u64 },
    RotateKey { key_id: u64, new_raw_key: String },
    UpdatePermissions { key_id: u64, permissions: u64 },
    RecordUsage { owner: String, key_id: u64, #[arg(long, default_value_t = READ)] required_flag: u64 },
    VerifyKey { owner: String, key_id: u64, raw_key: String },
    CloseKey { key_id: u64 },
    ListKeys { owner: String },
    ShowKey { owner: String, key_id: u64 },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let wallet_path = shellexpand::tilde(&cli.wallet).to_string();
    let payer = Rc::new(read_keypair_file(wallet_path).map_err(|e| anyhow::anyhow!(e.to_string()))?);
    let program_id = Pubkey::from_str(&cli.program_id)?;
    let owner = payer.pubkey();

    let cluster = match cli.cluster.as_str() {
        "localnet" => Cluster::Localnet,
        "mainnet" | "mainnet-beta" => Cluster::Mainnet,
        _ => Cluster::Devnet,
    };

    let client = Client::new_with_options(cluster, payer, CommitmentConfig::confirmed());
    let program = client.program(program_id)?;

    match cli.command {
        Commands::InitRegistry => {
            let (registry, _) = registry_pda(&owner, &program_id);
            let sig = program
                .request()
                .accounts(program_accounts::InitializeRegistry {
                    authority: owner,
                    registry,
                    system_program: anchor_client::solana_sdk::system_program::id(),
                })
                .args(program_ix::InitializeRegistry {})
                .send()?;
            print_tx_ok(sig.to_string());
        }
        Commands::CreateKey { key_id, name, raw_key, permissions, expires_at, metadata } => {
            let (registry, _) = registry_pda(&owner, &program_id);
            let (api_key, _) = api_key_pda(&owner, key_id, &program_id);
            let hash = sha256(raw_key.as_bytes());
            let sig = program
                .request()
                .accounts(program_accounts::CreateApiKey {
                    owner,
                    registry,
                    authority: owner,
                    api_key,
                    system_program: anchor_client::solana_sdk::system_program::id(),
                })
                .args(program_ix::CreateApiKey {
                    key_id,
                    name,
                    key_hash: hash,
                    permissions,
                    expires_at,
                    metadata,
                })
                .send()?;
            print_tx_ok(sig.to_string());
        }
        Commands::RevokeKey { key_id } => {
            let (registry, _) = registry_pda(&owner, &program_id);
            let (api_key, _) = api_key_pda(&owner, key_id, &program_id);
            let sig = program
                .request()
                .accounts(program_accounts::OwnerMutateApiKey {
                    owner,
                    registry,
                    api_key,
                })
                .args(program_ix::RevokeApiKey { key_id })
                .send()?;
            print_tx_ok(sig.to_string());
        }
        Commands::RotateKey { key_id, new_raw_key } => {
            let (registry, _) = registry_pda(&owner, &program_id);
            let (api_key, _) = api_key_pda(&owner, key_id, &program_id);
            let sig = program
                .request()
                .accounts(program_accounts::OwnerMutateApiKey {
                    owner,
                    registry,
                    api_key,
                })
                .args(program_ix::RotateApiKey {
                    key_id,
                    new_key_hash: sha256(new_raw_key.as_bytes()),
                })
                .send()?;
            print_tx_ok(sig.to_string());
        }
        Commands::UpdatePermissions { key_id, permissions } => {
            let (registry, _) = registry_pda(&owner, &program_id);
            let (api_key, _) = api_key_pda(&owner, key_id, &program_id);
            let sig = program
                .request()
                .accounts(program_accounts::OwnerMutateApiKey {
                    owner,
                    registry,
                    api_key,
                })
                .args(program_ix::UpdatePermissions {
                    key_id,
                    new_permissions: permissions,
                })
                .send()?;
            print_tx_ok(sig.to_string());
        }
        Commands::RecordUsage { owner, key_id, required_flag } => {
            let owner = Pubkey::from_str(&owner)?;
            let (api_key, _) = api_key_pda(&owner, key_id, &program_id);
            let sig = program
                .request()
                .accounts(program_accounts::RecordUsage { caller: owner, api_key })
                .args(program_ix::RecordUsage { key_id, required_flag })
                .send()?;
            print_tx_ok(sig.to_string());
        }
        Commands::VerifyKey { owner, key_id, raw_key } => {
            let owner = Pubkey::from_str(&owner)?;
            let (api_key, _) = api_key_pda(&owner, key_id, &program_id);
            let sig = program
                .request()
                .accounts(program_accounts::VerifyApiKey { caller: owner, api_key })
                .args(program_ix::VerifyApiKey {
                    key_id,
                    provided_hash: sha256(raw_key.as_bytes()),
                })
                .send()?;
            print_tx_ok(sig.to_string());
        }
        Commands::CloseKey { key_id } => {
            let (api_key, _) = api_key_pda(&owner, key_id, &program_id);
            let sig = program
                .request()
                .accounts(program_accounts::CloseApiKey { owner, api_key })
                .args(program_ix::CloseApiKey { key_id })
                .send()?;
            print_tx_ok(sig.to_string());
        }
        Commands::ListKeys { owner } => {
            let owner = Pubkey::from_str(&owner)?;
            let keys = program.accounts::<ApiKeyAccount>(vec![])?;
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["PDA", "Key ID", "Name", "Active", "Usage", "Expires At"]);
            for (pk, acct) in keys.into_iter().filter(|(_, a)| a.owner == owner) {
                table.add_row(vec![
                    pk.to_string(),
                    acct.key_id.to_string(),
                    acct.name,
                    acct.is_active.to_string(),
                    acct.usage_count.to_string(),
                    acct.expires_at.map(|v| v.to_string()).unwrap_or_else(|| "none".to_string()),
                ]);
            }
            println!("{}", table);
        }
        Commands::ShowKey { owner, key_id } => {
            let owner = Pubkey::from_str(&owner)?;
            let (api_key, _) = api_key_pda(&owner, key_id, &program_id);
            let acct: ApiKeyAccount = program.account(api_key)?;
            println!("key_id: {}", acct.key_id);
            println!("name: {}", acct.name);
            println!("permissions: {}", acct.permissions);
            println!("active: {}", acct.is_active);
            println!("usage_count: {}", acct.usage_count);
            println!("expires_at: {:?}", acct.expires_at);
            println!("metadata: {}", acct.metadata);
        }
    }

    Ok(())
}

fn registry_pda(owner: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"registry", owner.as_ref()], program_id)
}

fn api_key_pda(owner: &Pubkey, key_id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"api_key", owner.as_ref(), &key_id.to_le_bytes()], program_id)
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

fn print_tx_ok(sig: String) {
    println!("✅ Tx: {}", sig);
    println!("Explorer: https://explorer.solana.com/tx/{}?cluster=devnet", sig);
}
