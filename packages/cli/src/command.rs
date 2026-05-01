use clap::Subcommand;

pub mod import;
pub mod invoke;
pub mod market;
pub mod wipe_catalog;

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    /// Invoke a deployed Solana program with an empty instruction
    Invoke {
        /// Program ID (base58 public key)
        program_id: String,
    },

    /// Import product catalog from a Qogita CSV export
    Import {
        /// Path to the CSV file
        #[arg(long)]
        csv: String,

        /// API base URL (e.g. http://localhost:8787)
        #[arg(long)]
        api_url: String,

        /// Maximum products per category (0 = unlimited)
        #[arg(long, default_value = "0")]
        max_per_category: usize,

        /// Minimum MOQ — skip products with MOQ below this value
        #[arg(long, default_value = "5")]
        min_moq: u32,

        /// Dry run — parse and report without creating anything
        #[arg(long, default_value = "false")]
        dry_run: bool,
    },

    /// Wipe all products, brands, and categories from the catalog
    WipeCatalog {
        /// API base URL (e.g. http://localhost:8787)
        #[arg(long)]
        api_url: String,
    },

    /// Interact with the Groupshop market program (group-buying escrow).
    Market {
        #[command(subcommand)]
        sub: market::MarketCmd,
    },
}
