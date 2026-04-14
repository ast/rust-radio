use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sdrlink", about = "WebRTC web SDR streaming server")]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "sdrlink.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the server
    Serve,
    /// Add a user
    UserAdd { username: String },
    /// Remove a user
    UserRemove { username: String },
    /// List configured users
    UserList,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("sdrlink=info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Serve => {
            let config = sdrlink::Config::load(&cli.config)?;
            sdrlink::commands::serve::run(config).await?;
        }
        Command::UserAdd { username } => {
            sdrlink::commands::user_add::run(&cli.config, &username)?;
        }
        Command::UserRemove { username } => {
            sdrlink::commands::user_remove::run(&cli.config, &username)?;
        }
        Command::UserList => {
            sdrlink::commands::user_list::run(&cli.config)?;
        }
    }

    Ok(())
}
