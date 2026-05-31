//! Runtime configuration: where data lives and how we authenticate to GitHub.
//!
//! Token resolution order (zero-config for anyone who already uses `gh`):
//!   1. `GITHUB_TOKEN` environment variable
//!   2. `gh auth token` (the GitHub CLI)
//!   3. otherwise a friendly error explaining how to authenticate.

use std::path::PathBuf;
use std::process::Command;

use crate::error::{AppError, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub token: String,
    pub db_path: PathBuf,
    pub archive_dir: PathBuf,
}

impl Config {
    /// Load configuration, resolving the GitHub token and data paths.
    pub fn load() -> Result<Self> {
        Ok(Self {
            token: load_token()?,
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

pub fn load_token() -> Result<String> {
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        let t = t.trim().to_string();
        if !t.is_empty() {
            return Ok(t);
        }
    }

    // Fall back to the GitHub CLI if it's installed and logged in.
    if let Ok(out) = Command::new("gh").args(["auth", "token"]).output()
        && out.status.success()
    {
        let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !t.is_empty() {
            return Ok(t);
        }
    }

    Err(AppError::msg(
        "No GitHub token found. Set GITHUB_TOKEN, or install the GitHub CLI and run `gh auth login`.",
    ))
}
