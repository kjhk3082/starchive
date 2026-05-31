//! Web dashboard: axum router + shared state. Templates live in [`views`] and
//! handlers in [`handlers`]. The error type's `IntoResponse` lives here so the
//! rest of the crate stays axum-free.

mod handlers;
pub mod views;

use std::sync::{Arc, Mutex, RwLock};

use axum::Router;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use chrono::{DateTime, Duration, Utc};

use crate::config::Config;
use crate::db::Db;
use crate::error::{AppError, Result};
use crate::github::{self, GithubClient, SearchParams, trending_query};
use crate::llm::{Llm, Provider};

const TRENDING_TTL_MINUTES: i64 = 60;
const TRENDING_WINDOW_DAYS: i64 = 14;
const TRENDING_MIN_STARS: u32 = 20;
const TRENDING_PER_PAGE: u32 = 30;

#[derive(Default)]
struct TrendingCache {
    fetched_at: Option<DateTime<Utc>>,
    repos: Vec<github::Repo>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<Config>,
    /// Both are runtime-swappable so the settings page can connect GitHub / change
    /// the LLM key without a restart.
    client: Arc<RwLock<Option<GithubClient>>>,
    llm: Arc<RwLock<Option<Llm>>>,
    trending: Arc<Mutex<TrendingCache>>,
}

impl AppState {
    fn new(db: Db, client: Option<GithubClient>, config: Config, llm: Option<Llm>) -> Self {
        Self {
            db,
            config: Arc::new(config),
            client: Arc::new(RwLock::new(client)),
            llm: Arc::new(RwLock::new(llm)),
            trending: Arc::new(Mutex::new(TrendingCache::default())),
        }
    }

    /// Clone out the current GitHub client (never held across an await).
    pub fn current_client(&self) -> Option<GithubClient> {
        self.client.read().unwrap().clone()
    }

    /// Clone out the current LLM client (never held across an await).
    pub fn current_llm(&self) -> Option<Llm> {
        self.llm.read().unwrap().clone()
    }

    /// Re-resolve the GitHub client from DB token (then env/gh) and hot-swap it.
    async fn reload_github(&self) -> Result<()> {
        let resolved = resolve_github(&self.db).await?;
        *self.client.write().unwrap() = resolved;
        Ok(())
    }

    /// Re-resolve the LLM from DB settings (then env) and hot-swap it in.
    async fn reload_llm(&self) -> Result<()> {
        let resolved = resolve_llm(&self.db).await?;
        *self.llm.write().unwrap() = resolved;
        Ok(())
    }

    /// Trending repos from a 1-hour in-memory cache (Search API is 30 req/min).
    async fn get_trending(&self) -> Result<Vec<github::Repo>> {
        {
            let cache = self.trending.lock().unwrap();
            if let Some(at) = cache.fetched_at
                && Utc::now() - at < Duration::minutes(TRENDING_TTL_MINUTES)
                && !cache.repos.is_empty()
            {
                return Ok(cache.repos.clone());
            }
        }
        self.refresh_trending().await
    }

    /// Force a fresh trending fetch and update the cache. No GitHub connection
    /// yet → empty (the page then prompts the user to connect).
    async fn refresh_trending(&self) -> Result<Vec<github::Repo>> {
        let Some(client) = self.current_client() else {
            return Ok(Vec::new());
        };
        let since = (Utc::now() - Duration::days(TRENDING_WINDOW_DAYS))
            .format("%Y-%m-%d")
            .to_string();
        let params = SearchParams {
            query: trending_query(&since, TRENDING_MIN_STARS, None),
            per_page: TRENDING_PER_PAGE,
        };
        let repos = client.search_trending(&params).await?;
        {
            let mut cache = self.trending.lock().unwrap();
            cache.fetched_at = Some(Utc::now());
            cache.repos = repos.clone();
        }
        Ok(repos)
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::dashboard))
        .route("/stars/refresh", post(handlers::refresh_stars))
        .route("/export.md", get(handlers::export_all))
        .route("/trending", get(handlers::trending))
        .route("/trending/refresh", post(handlers::trending_refresh))
        .route(
            "/discover",
            get(handlers::discover_page).post(handlers::discover_run),
        )
        .route("/archive/{owner}/{name}", get(handlers::archive_view))
        .route("/archive/{owner}/{name}/raw", get(handlers::archive_raw))
        .route(
            "/archive/{owner}/{name}/summary",
            post(handlers::archive_summary),
        )
        .route(
            "/settings",
            get(handlers::settings_page).post(handlers::settings_save),
        )
        .route("/lang/{code}", get(handlers::set_lang))
        .route("/favicon.svg", get(handlers::favicon))
        .with_state(state)
}

/// Resolve the active LLM: dashboard-entered settings (DB) first, else env.
async fn resolve_llm(db: &Db) -> Result<Option<Llm>> {
    if let Some(key) = db.get_setting("llm_api_key").await?
        && !key.trim().is_empty()
    {
        let provider = db
            .get_setting("llm_provider")
            .await?
            .as_deref()
            .and_then(Provider::parse)
            .unwrap_or(Provider::OpenRouter);
        let model = db.get_setting("llm_model").await?;
        return Ok(Some(Llm::build(provider, key, model)));
    }
    Ok(Llm::from_env())
}

/// Resolve the GitHub client: dashboard token (DB) → env → `gh auth token`.
async fn resolve_github(db: &Db) -> Result<Option<GithubClient>> {
    match crate::config::resolve_github_token(db).await? {
        Some(token) => Ok(Some(GithubClient::new(&token)?)),
        None => Ok(None),
    }
}

pub async fn serve(port: u16) -> Result<()> {
    let config = Config::load()?;
    let db = Db::open(&config.db_path).await?;
    let client = resolve_github(&db).await?;
    let llm = resolve_llm(&db).await?;
    match &client {
        Some(_) => println!("GitHub connected."),
        None => println!(
            "GitHub not connected — add a token in Settings (⚙), GITHUB_TOKEN, or `gh auth login`"
        ),
    }
    match &llm {
        Some(l) => println!("LLM enabled: {} ({})", l.provider().label(), l.model()),
        None => println!("LLM disabled — add a key in Settings (⚙), or via .env"),
    }
    let state = AppState::new(db, client, config, llm);

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("starchive dashboard → http://{addr}");
    axum::serve(listener, router(state)).await?;
    Ok(())
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("request error: {self}");
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}
