//! Web dashboard: axum router + shared state. Templates live in [`views`] and
//! handlers in [`handlers`]. The error type's `IntoResponse` lives here so the
//! rest of the crate stays axum-free.

mod handlers;
pub mod views;

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use chrono::{DateTime, Duration, Utc};

use crate::config::Config;
use crate::db::Db;
use crate::error::{AppError, Result};
use crate::github::{self, GithubClient, SearchParams, trending_query};

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
    pub client: GithubClient,
    pub config: Arc<Config>,
    trending: Arc<Mutex<TrendingCache>>,
}

impl AppState {
    fn new(db: Db, client: GithubClient, config: Config) -> Self {
        Self {
            db,
            client,
            config: Arc::new(config),
            trending: Arc::new(Mutex::new(TrendingCache::default())),
        }
    }

    /// Trending repos, served from a 1-hour in-memory cache to stay well within
    /// the Search API's 30 req/min limit.
    async fn get_trending(&self) -> Result<Vec<github::Repo>> {
        {
            let cache = self.trending.lock().unwrap();
            if let Some(at) = cache.fetched_at {
                if Utc::now() - at < Duration::minutes(TRENDING_TTL_MINUTES) && !cache.repos.is_empty() {
                    return Ok(cache.repos.clone());
                }
            }
        }

        let since = (Utc::now() - Duration::days(TRENDING_WINDOW_DAYS))
            .format("%Y-%m-%d")
            .to_string();
        let params = SearchParams {
            query: trending_query(&since, TRENDING_MIN_STARS, None),
            per_page: TRENDING_PER_PAGE,
        };
        let repos = self.client.search_trending(&params).await?;

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
        .route("/trending", get(handlers::trending))
        .route("/archive/{owner}/{name}", get(handlers::archive_view))
        .with_state(state)
}

pub async fn serve(port: u16) -> Result<()> {
    let config = Config::load(port)?;
    let db = Db::open(&config.db_path).await?;
    let client = GithubClient::new(&config.token)?;
    let state = AppState::new(db, client, config);

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
