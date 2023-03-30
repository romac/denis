use std::path::PathBuf;

use clap::Parser;
use color_eyre::Report;

pub mod data;
pub mod db;
pub mod record;
pub mod server;
pub mod trie;

#[derive(Debug, Parser)]
struct Args {
    #[clap(short, long)]
    db: PathBuf,

    #[clap(short, long, default_value = "7777")]
    port: u16,
}

impl Args {
    fn listen_addr(&self) -> (&str, u16) {
        ("127.0.0.1", self.port)
    }
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    setup()?;

    let args = Args::parse();
    server::run(&args.db, args.listen_addr()).await?;

    Ok(())
}

fn setup() -> Result<(), Report> {
    use tracing_subscriber::EnvFilter;

    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "0")
    }

    color_eyre::install()?;

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }

    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    Ok(())
}
