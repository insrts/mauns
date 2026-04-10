use mauns_core::error::MaunsError;

pub fn handle_error(err: &MaunsError) {
    let title: &str;
    let message: String;
    let fix: Option<&str>;

    match err {
        MaunsError::LlmProvider(msg) => {
            if msg.contains("429") || msg.contains("quota") || msg.contains("insufficient_quota") {
                title = "LLM API error: quota exceeded.";
                message = "Your API key has no remaining credits or has hit its rate limit.".to_string();
                fix = Some("Fix:\n  - Add billing at the provider's platform (OpenAI/Anthropic)\n  - Or switch provider: mauns --provider <name> run \"...\"");
            } else if msg.contains("401") || msg.contains("invalid_api_key") || msg.contains("authentication") {
                title = "LLM API error: authentication failed.";
                message = "The provided API key is invalid or expired.".to_string();
                fix = Some("Fix:\n  - Check your OPENAI_API_KEY or CLAUDE_API_KEY environment variables.\n  - Or update your mauns.toml configuration.");
            } else {
                title = "LLM API error";
                message = msg.clone();
                fix = None;
            }
        }

        MaunsError::Config(msg) if msg.contains("API_KEY") => {
            title = "No API key found.";
            message = "Set one of the following environment variables:".to_string();
            fix = Some("  - OPENAI_API_KEY\n  - CLAUDE_API_KEY\n\nOr run in test mode:\n  mauns --test run \"...\"");
        }

        MaunsError::Config(msg) => {
            title = "Configuration error";
            message = msg.clone();
            fix = None;
        }

        MaunsError::OutsideWorkspace { path } => {
            title = "Access denied: file is outside the allowed workspace.";
            message = format!("Path: {}", path);
            fix = Some("Mauns prevents modifying files outside your project directory for safety.");
        }

        MaunsError::PathTraversal(msg) | MaunsError::RestrictedPath(msg) => {
            title = "Access denied: restricted path.";
            message = msg.clone();
            fix = Some("Mauns prevents modifying sensitive or hidden system files.");
        }

        MaunsError::Aborted => {
            title = "Operation aborted.";
            message = "The task was cancelled by the user.".to_string();
            fix = None;
        }

        MaunsError::LimitExceeded(msg) => {
            title = "Execution limit exceeded.";
            message = msg.clone();
            fix = Some("You can increase limits using --max-iterations or --max-tokens.");
        }

        MaunsError::Git(msg) => {
            title = "Git error.";
            message = msg.clone();
            fix = Some("Ensure you are in a git repository and have necessary permissions.");
        }

        _ => {
            title = "Error";
            message = format!("{}", err);
            fix = None;
        }
    };

    eprintln!();
    eprintln!("{}", title);
    if !message.is_empty() {
        eprintln!();
        eprintln!("{}", message);
    }
    if let Some(f) = fix {
        eprintln!();
        eprintln!("{}", f);
    }
    eprintln!();
}
