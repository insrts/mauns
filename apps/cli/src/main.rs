//! Mauns v0.1.1 — Agent Session Mode
//!
//! Running `mauns` with no arguments enters the interactive agent session.
//! All slash commands (/help, /models, /config, /plan, etc.) are handled
//! inside the session.

use mauns_config::load_config;
use mauns_session::{SessionRunner, SessionState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[mauns] Configuration error: {e}");
            eprintln!("[mauns] Run: mauns config-init  to create a default mauns.toml");
            std::process::exit(1);
        }
    };

    // Check if a simple subcommand was given.
    let args: Vec<String> = std::env::args().skip(1).collect();

    if !args.is_empty() {
        match args[0].as_str() {
            "config-init" => {
                let path = std::path::Path::new("mauns.toml");
                if path.exists() {
                    eprintln!("[mauns] mauns.toml already exists.");
                    std::process::exit(1);
                }
                let toml = mauns_config::schema::MaunsConfig::default_toml();
                std::fs::write(path, toml).expect("failed to write mauns.toml");
                println!("[mauns] mauns.toml created. Edit it to set your API keys.");
                return;
            }
            "--version" | "-V" | "version" => {
                println!("mauns {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--help" | "-h" | "help" => {
                print_help();
                return;
            }
            unknown => {
                eprintln!("[mauns] Unknown argument '{unknown}'.");
                eprintln!("Run `mauns` with no arguments to enter the agent session.");
                eprintln!("Run `mauns --help` for usage information.");
                std::process::exit(1);
            }
        }
    }

    // Logging is off by default in session mode to keep the UI clean.
    init_logging(&config.logging.level);

    // No arguments → enter agent session mode.
    let state = SessionState::new(config);
    let runner = SessionRunner::new(state);
    runner.run().await;
}

fn print_help() {
    println!(
        "Mauns v{} — autonomous AI agent session",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("USAGE:");
    println!("  mauns              Enter the interactive agent session");
    println!("  mauns config-init  Create a default mauns.toml in the current directory");
    println!("  mauns --version    Print the version");
    println!("  mauns --help       Show this help message");
    println!();
    println!("INSIDE THE SESSION:");
    println!("  Type a task and press Enter to run it.");
    println!("  Use /help inside the session for all slash commands.");
}

fn init_logging(level: &str) {
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("off"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_ansi(false)
        .without_time()
        .try_init();
}
