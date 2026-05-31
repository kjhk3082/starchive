//! Runtime configuration: where data lives and how we authenticate to GitHub.
//!
//! GitHub token resolution order (so anyone can "clone and run"): the
//! `github_token` saved via the dashboard Settings page (local DB), else the
//! `GITHUB_TOKEN` environment variable, else `gh auth token` (the GitHub CLI).
//! The token is optional at startup — the dashboard prompts you to connect.

use std::path::PathBuf;
use std::process::Command;

use crate::db::Db;
use crate::error::Result;

#[derive(Clone, Debug)]
pub struct Config {
    pub db_path: PathBuf,
    pub archive_dir: PathBuf,
}

impl Config {
    /// Load data paths. The GitHub token is resolved separately (and lazily) so
    /// the server can start before a token exists.
    pub fn load() -> Result<Self> {
        Ok(Self {
            db_path: db_path(),
            archive_dir: archive_dir(),
        })
    }
}

pub fn db_path() -> PathBuf {
    std::env::var_os("STARCHIVE_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/starchive.db"))
}

pub fn archive_dir() -> PathBuf {
    std::env::var_os("STARCHIVE_ARCHIVE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("archive"))
}

/// Resolve a GitHub token: dashboard setting (DB) → env → `gh auth token`.
pub async fn resolve_github_token(db: &Db) -> Result<Option<String>> {
    if let Some(t) = db.get_setting("github_token").await? {
        let t = t.trim().to_string();
        if !t.is_empty() {
            return Ok(Some(t));
        }
    }
    Ok(token_from_env_or_gh())
}

/// Token from `GITHUB_TOKEN`, else the GitHub CLI. `None` if neither is present.
pub fn token_from_env_or_gh() -> Option<String> {
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        let t = t.trim().to_string();
        if !t.is_empty() {
            return Some(t);
        }
    }
    if let Ok(out) = Command::new("gh").args(["auth", "token"]).output()
        && out.status.success()
    {
        let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !t.is_empty() {
            return Some(t);
        }
    }
    None
}
