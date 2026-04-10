use std::sync::Arc;

use clap::Parser;
use mauns_agents::{context_loader::load_run_context, git_orchestrator::GitConfig, Pipeline};
use mauns_cli::{
    args::{Cli, Command},
    error_handler::handle_error,
    output::{print_report, Ui, Verbosity},
};
use mauns_config::{load_config, schema::MaunsConfig};
use mauns_llm::{build_provider, deterministic::DeterministicProvider, registry::ProviderKind};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!();
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    let log_level = if cli.debug {
        "debug".to_string()
    } else if cli.verbose {
        "info".to_string()
    } else {
        "off".to_string()
    };

    init_logging(&log_level, cli.debug);

    if let Err(e) = run(cli, config).await {
        if let Some(mauns_err) = e.downcast_ref::<mauns_core::error::MaunsError>() {
            handle_error(mauns_err);
        } else {
            eprintln!();
            eprintln!("Error: {}", e);
            eprintln!();
        }
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
                Ok(()) => println!("[ok] configuration is valid"),
                Err(e) => {
                    eprintln!("[error] invalid config: {e}");
                    std::process::exit(1);
                }
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
            task,
            dry_run,
            no_confirm,
            no_pr,
            deterministic,
            vibe,
            test,
            max_iterations,
            max_tokens,
        } => {
            let effective_dry_run = test || dry_run || config.safety.dry_run;
            let effective_confirm = !test && !no_confirm && config.safety.confirm_before_write;
            let effective_no_pr = test || no_pr;
            let effective_max_iter = if max_iterations > 0 {
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

            if kind == ProviderKind::Anthropic
                && std::env::var("CLAUDE_API_KEY").is_err()
                && !config.claude.api_key.is_empty()
            {
                std::env::set_var("CLAUDE_API_KEY", &config.claude.api_key);
            }
            if kind == ProviderKind::OpenAi
                && std::env::var("OPENAI_API_KEY").is_err()
                && !config.openai.api_key.is_empty()
            {
                std::env::set_var("OPENAI_API_KEY", &config.openai.api_key);
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

            let verbosity = if cli.debug {
                Verbosity::Debug
            } else if cli.verbose {
                Verbosity::Verbose
            } else {
                Verbosity::Normal
            };
            let ui = Ui::new(verbosity);

            ui.print_task(&task);

            let pipeline = Pipeline::new(provider, git_cfg, vec![]);
            let report = pipeline.run(&task, &ctx, Some(&ui)).await?;
            print_report(&report);
        }
    }
    Ok(())
}

fn init_logging(level: &str, is_debug: bool) {
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(is_debug)
        .with_thread_ids(false)
        .with_ansi(true);

    if !is_debug {
        subscriber
            .with_level(false)
            .with_target(false)
            .without_time()
            .init();
    } else {
        subscriber.init();
    }
}
