use clap::Subcommand;

pub mod invoke;

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    /// Invoke a deployed Solana program with an empty instruction
    Invoke {
        /// Program ID (base58 public key)
        program_id: String,
    },
}
