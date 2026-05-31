//! Route handlers. Rendering handlers take a `lang: Lang` extractor and return
//! `Result<Html<String>>`; `AppError` renders as a 500 (see [`super`]).

use axum::extract::{Path, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE, LOCATION, REFERER, SET_COOKIE};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

use crate::error::{AppError, Result};
use crate::i18n::Lang;
use crate::recommend;
use crate::runner;
use crate::web::{AppState, views};

/// `GET /` — the Stars dashboard. Highlights are acknowledged (cleared) after
/// the page is rendered, so newly-synced repos glow once.
pub async fn dashboard(lang: Lang, State(st): State<AppState>) -> Result<Html<String>> {
    let stars = st.db.get_stars_sorted().await?;
    let html = views::layout(
        lang,
        lang.nav_stars(),
        "stars",
        views::stars_page(lang, &stars),
    );
    st.db.clear_is_new().await?;
    Ok(Html(html))
}

/// `POST /stars/refresh` — run the sync pipeline and return the updated stars
/// list as an htmx partial (new repos pinned to the top).
pub async fn refresh_stars(lang: Lang, State(st): State<AppState>) -> Result<Html<String>> {
    let report =
        runner::run_sync(&st.client, &st.db, &st.config.archive_dir, "manual", false).await?;
    let stars = st.db.get_stars_sorted().await?;
    let banner = lang.refreshed_banner(report.added, report.updated, report.removed, stars.len());
    Ok(Html(
        views::stars_list_inner(lang, &stars, Some(&banner)).into_string(),
    ))
}

/// `GET /trending` — trending repos plus a personalized "For You" ranking.
pub async fn trending(lang: Lang, State(st): State<AppState>) -> Result<Html<String>> {
    let trending = st.get_trending().await?;
    let active = st.db.get_active_repos().await?;
    let profile = recommend::build_profile(&active);
    let for_you = recommend::score_trending(&profile, &trending);

    let note = if profile.total == 0 {
        lang.profile_note_empty().to_string()
    } else {
        let langs: Vec<String> = profile
            .top_languages(3)
            .into_iter()
            .map(|(l, _)| l)
            .collect();
        lang.profile_note(profile.total, &langs.join(", "))
    };

    let html = views::layout(
        lang,
        lang.nav_trending(),
        "trending",
        views::trending_page(lang, &trending, &for_you, &note),
    );
    Ok(Html(html))
}

/// `GET /archive/{owner}/{name}` — render a repo's stored markdown archive.
pub async fn archive_view(
    lang: Lang,
    State(st): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
) -> Result<Html<String>> {
    let md = read_archive(&st, &owner, &name).await?;
    let title = format!("{owner}/{name}");
    let html = views::layout(
        lang,
        &title,
        "stars",
        views::archive_page(lang, &owner, &name, &md),
    );
    Ok(Html(html))
}

/// `GET /archive/{owner}/{name}/raw` — download the raw markdown file.
pub async fn archive_raw(
    State(st): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
) -> Result<Response> {
    let md = read_archive(&st, &owner, &name).await?;
    // Sanitize the filename so it can't inject into the header.
    let safe: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect();
    let disposition = format!("attachment; filename=\"{safe}.md\"");
    Ok((
        [
            (CONTENT_TYPE, "text/markdown; charset=utf-8".to_string()),
            (CONTENT_DISPOSITION, disposition),
        ],
        md,
    )
        .into_response())
}

/// `GET /lang/{code}` — set the language cookie and return to the previous page.
pub async fn set_lang(Path(code): Path<String>, headers: HeaderMap) -> Response {
    let lang = Lang::parse(&code).unwrap_or_default();
    let cookie = format!(
        "lang={}; Path=/; Max-Age=31536000; SameSite=Lax",
        lang.code()
    );
    let back = headers
        .get(REFERER)
        .and_then(|v| v.to_str().ok())
        .and_then(referer_path)
        .unwrap_or_else(|| "/".to_string());
    (
        StatusCode::SEE_OTHER,
        [(SET_COOKIE, cookie), (LOCATION, back)],
    )
        .into_response()
}

async fn read_archive(st: &AppState, owner: &str, name: &str) -> Result<String> {
    let path = st.config.archive_dir.join(owner).join(format!("{name}.md"));
    tokio::fs::read_to_string(&path).await.map_err(|_| {
        AppError::msg(format!(
            "No archive for {owner}/{name} yet — run a sync first."
        ))
    })
}

/// Extract just the path+query of a Referer so any redirect stays same-origin.
fn referer_path(referer: &str) -> Option<String> {
    let path = if let Some(idx) = referer.find("://") {
        let after = &referer[idx + 3..];
        after.find('/').map(|p| &after[p..]).unwrap_or("/")
    } else if referer.starts_with('/') {
        referer
    } else {
        return None;
    };
    // Reject protocol-relative ("//host") targets.
    if path.starts_with("//") {
        Some("/".to_string())
    } else {
        Some(path.to_string())
    }
}
