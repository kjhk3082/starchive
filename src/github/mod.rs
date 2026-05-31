//! GitHub API access: typed models + a thin async HTTP client.
//!
//! The client is deliberately thin — it performs requests and delegates all
//! parsing to [`models`], whose pure functions are unit-tested. Auth, User-Agent
//! and the API version are set once as default headers; only `Accept` varies
//! per endpoint.

pub mod models;

pub use models::{Repo, StarredRepo};

use reqwest::StatusCode;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue};

use crate::error::{AppError, Result};
use models::{parse_search, parse_starred};

const API: &str = "https://api.github.com";
const UA: &str = concat!("starchive/", env!("CARGO_PKG_VERSION"));
const API_VERSION: &str = "2022-11-28";
const PER_PAGE: u32 = 100;
const MAX_PAGES: u32 = 50; // safety cap: up to 5000 stars

/// Parameters for a trending search.
#[derive(Debug, Clone)]
pub struct SearchParams {
    pub query: String,
    pub per_page: u32,
}

/// Build the `q=` value approximating "trending": repositories created within
/// the recent window, with enough stars to be notable, optionally filtered by
/// language. (The Search API cannot sort by *star velocity*; see spec §5.2.)
pub fn trending_query(created_since: &str, min_stars: u32, language: Option<&str>) -> String {
    let mut q = format!("created:>{created_since} stars:>{min_stars}");
    if let Some(lang) = language {
        if !lang.trim().is_empty() {
            q.push_str(&format!(" language:{lang}"));
        }
    }
    q
}

#[derive(Clone)]
pub struct GithubClient {
    http: reqwest::Client,
}

impl GithubClient {
    pub fn new(token: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        let mut auth = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| AppError::msg("GITHUB_TOKEN contains invalid characters"))?;
        auth.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth);
        headers.insert("X-GitHub-Api-Version", HeaderValue::from_static(API_VERSION));

        let http = reqwest::Client::builder()
            .user_agent(UA)
            .default_headers(headers)
            .build()?;
        Ok(Self { http })
    }

    /// Fetch every starred repo, paginated, newest-starred first. The
    /// `star+json` media type yields the `starred_at` timestamp per item.
    pub async fn fetch_starred(&self) -> Result<Vec<StarredRepo>> {
        let mut all = Vec::new();
        for page in 1..=MAX_PAGES {
            let url = format!(
                "{API}/user/starred?per_page={PER_PAGE}&page={page}&sort=created&direction=desc"
            );
            let resp = self
                .http
                .get(&url)
                .header(ACCEPT, "application/vnd.github.star+json")
                .send()
                .await?;
            let resp = check(resp).await?;
            let body = resp.text().await?;
            let mut batch = parse_starred(&body)?;
            let len = batch.len();
            all.append(&mut batch);
            if len < PER_PAGE as usize {
                break;
            }
        }
        Ok(all)
    }

    /// Search repositories (trending approximation), sorted by stars desc.
    pub async fn search_trending(&self, params: &SearchParams) -> Result<Vec<Repo>> {
        let per_page = params.per_page.to_string();
        let resp = self
            .http
            .get(format!("{API}/search/repositories"))
            .header(ACCEPT, "application/vnd.github+json")
            .query(&[
                ("q", params.query.as_str()),
                ("sort", "stars"),
                ("order", "desc"),
                ("per_page", per_page.as_str()),
            ])
            .send()
            .await?;
        let resp = check(resp).await?;
        let body = resp.text().await?;
        parse_search(&body)
    }

    /// Fetch a repo's README as raw markdown. Missing README → `Ok(None)`.
    pub async fn fetch_readme(&self, owner: &str, name: &str) -> Result<Option<String>> {
        let url = format!("{API}/repos/{owner}/{name}/readme");
        let resp = self
            .http
            .get(&url)
            .header(ACCEPT, "application/vnd.github.raw")
            .send()
            .await?;
        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let resp = check(resp).await?;
        Ok(Some(resp.text().await?))
    }
}

/// Turn a non-2xx response into a helpful error, detecting rate limits.
async fn check(resp: reqwest::Response) -> Result<reqwest::Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }

    let rate_limited = status == StatusCode::FORBIDDEN
        && resp
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            == Some("0");
    let reset = resp
        .headers()
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if rate_limited {
        let when = reset
            .map(|r| format!(" (resets at unix {r})"))
            .unwrap_or_default();
        return Err(AppError::msg(format!(
            "GitHub rate limit exceeded{when}. Wait a bit and try again."
        )));
    }

    let body = resp.text().await.unwrap_or_default();
    let snippet: String = body.chars().take(200).collect();
    Err(AppError::msg(format!("GitHub API {status}: {snippet}")))
}
