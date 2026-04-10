use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "mauns", version, about = "Mauns - autonomous LLM agent system")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[arg(
        long,
        global = true,
        default_value = "anthropic",
        env = "MAUNS_PROVIDER"
    )]
    pub provider: String,

    #[arg(long, global = true, default_value = "info", env = "MAUNS_LOG")]
    pub log_level: String,

    /// Show execution details (step-by-step progress).
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    /// Show full debug logs (tracing).
    #[arg(long, global = true)]
    pub debug: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the autonomous agent pipeline for the given task.
    Run {
        task: String,

        /// Simulate all operations without touching disk or git.
        #[arg(long, default_value_t = false)]
        dry_run: bool,

        /// Skip the confirmation prompt before committing.
        #[arg(long, default_value_t = false)]
        no_confirm: bool,

        /// Skip GitHub PR creation.
        #[arg(long, default_value_t = false)]
        no_pr: bool,

        /// Use temperature=0 for fully reproducible outputs.
        #[arg(long, default_value_t = false)]
        deterministic: bool,

        /// Faster execution: single iteration per step, skip confirmation.
        #[arg(long, default_value_t = false)]
        vibe: bool,

        /// Test mode: dry-run + no-git + no-confirm.
        #[arg(long, default_value_t = false)]
        test: bool,

        /// Override max agent loop iterations (0 = use config value).
        #[arg(long, default_value = "0")]
        max_iterations: usize,

        /// Hard token budget per run (0 = no limit).
        #[arg(long, default_value = "0")]
        max_tokens: usize,
    },

    /// Write a default mauns.toml to the current directory.
    ConfigInit,

    /// Validate and display the current effective configuration.
    ConfigEdit,
}
