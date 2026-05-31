//! starchive — track your GitHub stars, archive them as AI-readable markdown,
//! and discover trending repos from an htmx dashboard.

mod config;
mod db;
mod error;
mod github;
mod sync;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "starchive",
    version,
    about = "Track GitHub stars, archive them as AI-readable markdown, and discover trending repos."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Sync starred repos into the local DB and archive new ones as markdown.
    Sync,
    /// (Re)generate the markdown archive for all currently-starred repos.
    Archive,
    /// Run the web dashboard.
    Serve {
        #[arg(short, long, default_value_t = 7878)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "starchive=info".into()),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Sync => println!("sync: not yet implemented"),
        Command::Archive => println!("archive: not yet implemented"),
        Command::Serve { port } => println!("serve on :{port}: not yet implemented"),
    }
    Ok(())
}
