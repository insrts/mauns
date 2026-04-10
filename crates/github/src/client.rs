//! Authenticated GitHub API client.
//!
//! The token is held in memory only and is never written to logs, config,
//! or any persistent storage.

use mauns_core::error::{MaunsError, Result};
use reqwest::{header, Client};
use tracing::debug;

const GITHUB_API: &str = "https://api.github.com";
const USER_AGENT: &str = concat!("mauns/", env!("CARGO_PKG_VERSION"));

/// Thin wrapper around `reqwest::Client` that injects authentication headers.
#[derive(Clone)]
pub struct GitHubClient {
    http:  Client,
    token: String, // held in memory; never logged
}

impl std::fmt::Debug for GitHubClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately omit the token from Debug output.
        f.debug_struct("GitHubClient")
            .field("token", &"[redacted]")
            .finish()
    }
}

impl GitHubClient {
    /// Construct a client from `GITHUB_TOKEN` environment variable.
    pub fn from_env() -> Result<Self> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            MaunsError::GitHub(
                "GITHUB_TOKEN environment variable is not set; \
                 it is required for GitHub operations"
                    .to_string(),
            )
        })?;

        if token.trim().is_empty() {
            return Err(MaunsError::GitHub(
                "GITHUB_TOKEN is set but empty".to_string(),
            ));
        }

        Self::new(token)
    }

    /// Construct a client from a token string.
    pub fn new(token: impl Into<String>) -> Result<Self> {
        let http = Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| MaunsError::GitHub(format!("failed to build HTTP client: {e}")))?;

        Ok(Self { http, token: token.into() })
    }

    /// Perform an authenticated POST to `path` (relative to the GitHub API
    /// base URL) with `body` serialised as JSON.
    pub async fn post<B, R>(&self, path: &str, body: &B) -> Result<R>
    where
        B: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let url = format!("{GITHUB_API}{path}");
        debug!(github = "post", path = %path);

        let response = self
            .http
            .post(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(body)
            .send()
            .await
            .map_err(|e| MaunsError::GitHub(format!("POST {path} failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(MaunsError::GitHub(format!(
                "GitHub API returned {status} for POST {path}: {body}"
            )));
        }

        response
            .json::<R>()
            .await
            .map_err(|e| MaunsError::GitHub(format!("failed to parse GitHub response: {e}")))
    }

    /// Perform an authenticated GET.
    pub async fn get<R>(&self, path: &str) -> Result<R>
    where
        R: serde::de::DeserializeOwned,
    {
        let url = format!("{GITHUB_API}{path}");
        debug!(github = "get", path = %path);

        let response = self
            .http
            .get(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| MaunsError::GitHub(format!("GET {path} failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(MaunsError::GitHub(format!(
                "GitHub API returned {status} for GET {path}: {body}"
            )));
        }

        response
            .json::<R>()
            .await
            .map_err(|e| MaunsError::GitHub(format!("failed to parse GitHub response: {e}")))
    }
}
