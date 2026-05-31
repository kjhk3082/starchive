//! Route handlers. Each returns `Result<Html<String>>`; `AppError` renders as a
//! 500 (see [`super`]).

use std::path::PathBuf;

use axum::extract::{Path, State};
use axum::response::Html;

use crate::error::{AppError, Result};
use crate::recommend::{self, Profile};
use crate::runner;
use crate::web::{AppState, views};

/// `GET /` — the Stars dashboard. Highlights are acknowledged (cleared) after
/// the page is rendered, so newly-synced repos glow once.
pub async fn dashboard(State(st): State<AppState>) -> Result<Html<String>> {
    let stars = st.db.get_stars_sorted().await?;
    let html = views::layout("Stars", "stars", views::stars_page(&stars));
    st.db.clear_is_new().await?;
    Ok(Html(html))
}

/// `POST /stars/refresh` — run the sync pipeline and return the updated stars
/// list as an htmx partial (new repos pinned to the top).
pub async fn refresh_stars(State(st): State<AppState>) -> Result<Html<String>> {
    let report =
        runner::run_sync(&st.client, &st.db, &st.config.archive_dir, "manual", false).await?;
    let stars = st.db.get_stars_sorted().await?;
    let banner = format!("Refreshed — {} (★ {} total)", report.summary(), stars.len());
    Ok(Html(
        views::stars_list_inner(&stars, Some(&banner)).into_string(),
    ))
}

/// `GET /trending` — trending repos plus a personalized "For You" ranking.
pub async fn trending(State(st): State<AppState>) -> Result<Html<String>> {
    let trending = st.get_trending().await?;
    let active = st.db.get_active_repos().await?;
    let profile = recommend::build_profile(&active);
    let for_you = recommend::score_trending(&profile, &trending);
    let note = profile_note(&profile);
    let html = views::layout(
        "Trending",
        "trending",
        views::trending_page(&trending, &for_you, &note),
    );
    Ok(Html(html))
}

/// `GET /archive/{owner}/{name}` — render a repo's stored markdown archive.
pub async fn archive_view(
    State(st): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
) -> Result<Html<String>> {
    let path: PathBuf = st
        .config
        .archive_dir
        .join(&owner)
        .join(format!("{name}.md"));
    let md = tokio::fs::read_to_string(&path).await.map_err(|_| {
        AppError::msg(format!(
            "No archive for {owner}/{name} yet — run a sync first."
        ))
    })?;
    let full = format!("{owner}/{name}");
    let html = views::layout(&full, "stars", views::archive_page(&full, &md));
    Ok(Html(html))
}

fn profile_note(profile: &Profile) -> String {
    let top = profile.top_languages(3);
    if top.is_empty() {
        "Sync your stars to personalize these picks.".to_string()
    } else {
        let langs: Vec<String> = top.into_iter().map(|(l, _)| l).collect();
        format!(
            "Ranked from your {} stars · top languages: {}",
            profile.total,
            langs.join(", ")
        )
    }
}
