// Copyright SM6WJM 2026

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "civlink", about = "WebRTC remote control for Icom radios")]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "civlink.toml")]
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
    /// Test audio capture (prints frame info and detects silence)
    TestAudio,
    /// Test radio event stream (prints all RadioEvents as JSON)
    TestEvents,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("civlink=info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Serve => {
            let config = civlink::Config::load(&cli.config)?;
            civlink::commands::serve::run(config).await?;
        }
        Command::UserAdd { username } => {
            civlink::commands::user_add::run(&cli.config, &username)?;
        }
        Command::UserRemove { username } => {
            civlink::commands::user_remove::run(&cli.config, &username)?;
        }
        Command::UserList => {
            civlink::commands::user_list::run(&cli.config)?;
        }
        Command::TestAudio => {
            let config = civlink::Config::load(&cli.config)?;
            civlink::commands::test_audio::run(config).await?;
        }
        Command::TestEvents => {
            let config = civlink::Config::load(&cli.config)?;
            civlink::commands::test_events::run(config).await?;
        }
    }

    Ok(())
}
