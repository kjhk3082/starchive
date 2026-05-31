//! starchive — track your GitHub stars, archive them as AI-readable markdown,
//! and discover trending repos from an htmx dashboard.

mod archive;
mod config;
mod db;
mod error;
mod github;
mod recommend;
mod runner;
mod sync;
mod web;

use clap::{Parser, Subcommand};

use config::Config;
use db::Db;
use github::GithubClient;

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
    Sync {
        /// Commit the archive directory after syncing (best-effort).
        #[arg(long)]
        git: bool,
    },
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
        Command::Sync { git } => {
            let cfg = Config::load(0)?;
            let client = GithubClient::new(&cfg.token)?;
            let db = Db::open(&cfg.db_path).await?;
            println!("Syncing stars from GitHub…");
            let report = runner::run_sync(&client, &db, &cfg.archive_dir, "manual", git).await?;
            println!("✓ {}", report.summary());
            println!("  archive → {}", cfg.archive_dir.display());
            println!("  db      → {}", cfg.db_path.display());
        }
        Command::Archive => {
            let cfg = Config::load(0)?;
            let client = GithubClient::new(&cfg.token)?;
            let db = Db::open(&cfg.db_path).await?;
            let n = runner::run_archive_all(&client, &db, &cfg.archive_dir).await?;
            println!("✓ archived {n} repositories → {}", cfg.archive_dir.display());
        }
        Command::Serve { port } => {
            web::serve(port).await?;
        }
    }
    Ok(())
}
