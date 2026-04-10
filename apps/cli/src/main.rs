use std::sync::Arc;

use clap::Parser;
use mauns_agents::{context_loader::load_run_context, git_orchestrator::GitConfig, Pipeline};
use mauns_cli::{args::{Cli, Command}, output::print_report};
use mauns_config::{load_config, schema::MaunsConfig};
use mauns_llm::{build_provider, deterministic::DeterministicProvider, registry::ProviderKind};
use tracing::error;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let config = match load_config() {
        Ok(c)  => c,
        Err(e) => { eprintln!("[error] config: {e}"); std::process::exit(1); }
    };

    let log_level = if cli.log_level != "info" {
        cli.log_level.clone()
    } else {
        config.logging.level.clone()
    };
    init_logging(&log_level);

    if let Err(e) = run(cli, config).await {
        error!("{e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli, config: MaunsConfig) -> anyhow::Result<()> {
    match cli.command {
        Command::ConfigInit => {
            let path = std::path::Path::new("mauns.toml");
            if path.exists() {
                eprintln!("[error] mauns.toml already exists");
                std::process::exit(1);
            }
            std::fs::write(path, MaunsConfig::default_toml())?;
            println!("mauns.toml created.");
            return Ok(());
        }

        Command::ConfigEdit => {
            match config.validate() {
                Ok(())  => println!("[ok] configuration is valid"),
                Err(e)  => { eprintln!("[error] invalid config: {e}"); std::process::exit(1); }
            }
            println!("provider        = {}", config.provider);
            println!("dry_run         = {}", config.safety.dry_run);
            println!("confirm_writes  = {}", config.safety.confirm_before_write);
            println!("create_pr       = {}", config.git.create_pr);
            println!("max_iterations  = {}", config.execution.max_iterations);
            println!("max_retries     = {}", config.execution.max_retries);
            println!("context_window  = {}", config.execution.context_window);
            println!("log_level       = {}", config.logging.level);
            return Ok(());
        }

        Command::Run {
            task, dry_run, no_confirm, no_pr,
            deterministic, vibe, test,
            max_iterations, max_tokens,
        } => {
            let effective_dry_run   = test || dry_run  || config.safety.dry_run;
            let effective_confirm   = !test && !no_confirm && config.safety.confirm_before_write;
            let effective_no_pr     = test || no_pr;
            let effective_max_iter  = if max_iterations > 0 {
                max_iterations
            } else {
                config.execution.max_iterations
            };

            let provider_name = if cli.provider != "anthropic" {
                cli.provider.clone()
            } else {
                config.provider.clone()
            };

            let kind: ProviderKind = provider_name.parse()?;

            if kind == ProviderKind::Anthropic && std::env::var("CLAUDE_API_KEY").is_err() {
                if !config.claude.api_key.is_empty() {
                    std::env::set_var("CLAUDE_API_KEY", &config.claude.api_key);
                }
            }
            if kind == ProviderKind::OpenAi && std::env::var("OPENAI_API_KEY").is_err() {
                if !config.openai.api_key.is_empty() {
                    std::env::set_var("OPENAI_API_KEY", &config.openai.api_key);
                }
            }

            let base = build_provider(&kind)?;
            let provider: Arc<dyn mauns_llm::LlmProvider> = if deterministic {
                Arc::new(DeterministicProvider::new(base))
            } else {
                base
            };

            let git_cfg = GitConfig::new(config.git.create_pr, effective_no_pr);

            let ctx = load_run_context(
                effective_dry_run,
                effective_confirm,
                deterministic,
                vibe,
                effective_max_iter,
                config.execution.max_retries,
                config.execution.context_window,
                max_tokens,
            );

            let pipeline = Pipeline::new(provider, git_cfg, vec![]);
            let report   = pipeline.run(&task, &ctx).await?;
            print_report(&report);
        }
    }
    Ok(())
}

fn init_logging(level: &str) {
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_ansi(false)
        .init();
}
