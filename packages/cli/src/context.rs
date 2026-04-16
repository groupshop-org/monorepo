use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signer::{keypair::Keypair, Signer},
};

use crate::command::Command;

pub struct CliCtx {
    pub args: CliArgs,
    _rpc_client: Option<RpcClient>,
    _keypair: Option<Keypair>,
}

impl CliCtx {
    pub fn new() -> Self {
        let args = CliArgs::parse();
        Self {
            args,
            _rpc_client: None,
            _keypair: None,
        }
    }

    pub fn rpc_client(&mut self) -> Result<&RpcClient> {
        if self._rpc_client.is_none() {
            let url = self
                .args
                .rpc_url
                .as_ref()
                .context("--rpc-url is required for Solana commands")?;
            self._rpc_client = Some(RpcClient::new_with_commitment(
                url,
                CommitmentConfig::confirmed(),
            ));
        }
        Ok(self._rpc_client.as_ref().unwrap())
    }

    pub fn keypair(&mut self) -> Result<&Keypair> {
        if self._keypair.is_none() {
            let path = &self.args.keypair;
            let data = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read keypair file: {}", path.display()))?;
            let bytes: Vec<u8> = serde_json::from_str(&data)
                .with_context(|| format!("failed to parse keypair file: {}", path.display()))?;
            let kp = Keypair::try_from(bytes.as_slice()).context("invalid keypair bytes")?;
            println!("Using payer: {}", kp.pubkey());
            self._keypair = Some(kp);
        }
        Ok(self._keypair.as_ref().unwrap())
    }
}

fn default_keypair_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/solana/id.json")
    } else {
        PathBuf::from(".config/solana/id.json")
    }
}

#[derive(Parser)]
#[command(name = "groupshop", about = "GROUPSHOP CLI")]
pub struct CliArgs {
    /// Solana RPC URL (required for Solana commands)
    #[arg(long)]
    pub rpc_url: Option<String>,

    /// Path to payer keypair JSON file
    #[arg(long, default_value_os_t = default_keypair_path())]
    pub keypair: PathBuf,

    #[arg(long, env = "GROUPSHOP_CLI_API_AUTH_EMAIL", hide_env_values = true)]
    pub api_auth_email: String,

    #[arg(long, env = "GROUPSHOP_CLI_API_AUTH_PASSWORD", hide_env_values = true)]
    pub api_auth_password: String,

    #[command(subcommand)]
    pub command: Command,
}
