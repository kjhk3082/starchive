//! Crate-wide error type. Kept free of any `axum` dependency so the CLI paths
//! (`sync`, `archive`) can use it without pulling in web concerns. The
//! `IntoResponse` impl lives in `web` instead.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("GitHub/HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// A human-facing message (e.g. missing token, GitHub API non-2xx).
    #[error("{0}")]
    Msg(String),
}

impl AppError {
    pub fn msg(s: impl Into<String>) -> Self {
        AppError::Msg(s.into())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
