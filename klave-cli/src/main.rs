mod deploy;
mod init;
mod start;
mod utils;

use clap::{Parser, Subcommand};

/// KLAVE — Agentic wallet infrastructure for Solana.
#[derive(Parser)]
#[command(name = "klave", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate .env with random API key and encryption key.
    Init,

    /// Build and start all services.
    Start {
        /// Also start the Kora gasless transaction server.
        #[arg(long)]
        with_kora: bool,

        /// Serve the monitoring dashboard on port 8888.
        #[arg(long)]
        dashboard: bool,

        /// Build in release mode.
        #[arg(long)]
        release: bool,
    },

    /// Build and deploy the Anchor program.
    Deploy {
        /// Target Solana cluster.
        #[arg(long, default_value = "devnet")]
        cluster: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => init::run(),
        Commands::Start {
            with_kora,
            dashboard,
            release,
        } => start::run(with_kora, dashboard, release).await,
        Commands::Deploy { cluster } => deploy::run(&cluster),
    };

    if let Err(e) = result {
        eprintln!("\x1b[31merror:\x1b[0m {e}");
        std::process::exit(1);
    }
}
