//! Route handlers. Rendering handlers take a `lang: Lang` extractor and return
//! `Result<Html<String>>`; `AppError` renders as a 500 (see [`super`]).

use axum::extract::{Form, Path, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE, LOCATION, REFERER, SET_COOKIE};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use chrono::Utc;
use serde::Deserialize;

use crate::ai;
use crate::error::{AppError, Result};
use crate::github::models::Repo;
use crate::i18n::Lang;
use crate::recommend::{self, Profile};
use crate::runner;
use crate::web::{AppState, views};

/// `GET /` — the Stars dashboard.
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

/// `POST /stars/refresh` — sync, then return the updated stars list partial.
pub async fn refresh_stars(lang: Lang, State(st): State<AppState>) -> Result<Html<String>> {
    let report =
        runner::run_sync(&st.client, &st.db, &st.config.archive_dir, "manual", false).await?;
    let stars = st.db.get_stars_sorted().await?;
    let banner = lang.refreshed_banner(report.added, report.updated, report.removed, stars.len());
    Ok(Html(
        views::stars_list_inner(lang, &stars, Some(&banner)).into_string(),
    ))
}

/// `GET /trending` — trending + personalized "For You".
pub async fn trending(lang: Lang, State(st): State<AppState>) -> Result<Html<String>> {
    let trending = st.get_trending().await?;
    let (for_you, note) = rank_for_you(lang, &st, &trending).await?;
    let body = views::trending_page(lang, &trending, &for_you, &note);
    Ok(Html(views::layout(
        lang,
        lang.nav_trending(),
        "trending",
        body,
    )))
}

/// `POST /trending/refresh` — refetch trending (bypassing cache), return partial.
pub async fn trending_refresh(lang: Lang, State(st): State<AppState>) -> Result<Html<String>> {
    let trending = st.refresh_trending().await?;
    let (for_you, note) = rank_for_you(lang, &st, &trending).await?;
    Ok(Html(
        views::trending_inner(lang, &trending, &for_you, &note).into_string(),
    ))
}

async fn rank_for_you(
    lang: Lang,
    st: &AppState,
    trending: &[Repo],
) -> Result<(Vec<recommend::Scored>, String)> {
    let active = st.db.get_active_repos().await?;
    let profile = recommend::build_profile(&active);
    let for_you = recommend::score_trending(&profile, trending);
    Ok((for_you, profile_note(lang, &profile)))
}

fn profile_note(lang: Lang, profile: &Profile) -> String {
    if profile.total == 0 {
        lang.profile_note_empty().to_string()
    } else {
        let langs: Vec<String> = profile
            .top_languages(3)
            .into_iter()
            .map(|(l, _)| l)
            .collect();
        lang.profile_note(profile.total, &langs.join(", "))
    }
}

/// `GET /discover` — project-based discovery form.
pub async fn discover_page(lang: Lang, State(st): State<AppState>) -> Result<Html<String>> {
    let body = views::discover_page(lang, st.llm.is_some());
    Ok(Html(views::layout(
        lang,
        lang.nav_discover(),
        "discover",
        body,
    )))
}

#[derive(Deserialize)]
pub struct DiscoverForm {
    description: String,
}

/// `POST /discover` — run LLM discovery, return result cards.
pub async fn discover_run(
    lang: Lang,
    State(st): State<AppState>,
    Form(form): Form<DiscoverForm>,
) -> Result<Html<String>> {
    let Some(llm) = &st.llm else {
        return Ok(Html(views::discover_results(lang, &[]).into_string()));
    };
    let desc = form.description.trim();
    if desc.is_empty() {
        return Ok(Html(views::discover_results(lang, &[]).into_string()));
    }
    let stars = st.db.get_active_repos().await?;
    match ai::discover(llm, &st.client, &stars, desc, lang).await {
        Ok(recs) => Ok(Html(views::discover_results(lang, &recs).into_string())),
        Err(e) => Ok(Html(error_fragment(&e.to_string()))),
    }
}

/// `GET /archive/{owner}/{name}` — rendered markdown archive + license + AI summary slot.
pub async fn archive_view(
    lang: Lang,
    State(st): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
) -> Result<Html<String>> {
    let md = read_archive(&st, &owner, &name).await?;
    let body_html = render_markdown_html(&md);

    let license = match st.db.repo_id_by_full_name(&owner, &name).await? {
        Some(id) => st
            .db
            .get_repo(id)
            .await?
            .and_then(|r| r.license_spdx().and_then(crate::license::explain)),
        None => None,
    };

    let title = format!("{owner}/{name}");
    let body = views::archive_page(
        lang,
        &owner,
        &name,
        &body_html,
        &md,
        license.as_ref(),
        st.llm.is_some(),
    );
    Ok(Html(views::layout(lang, &title, "stars", body)))
}

/// `GET /export.md` — one combined markdown file of every starred repo's archive.
pub async fn export_all(lang: Lang, State(st): State<AppState>) -> Result<Response> {
    let mut repos = st.db.get_active_repos().await?;
    repos.sort_by(|a, b| a.full_name.to_lowercase().cmp(&b.full_name.to_lowercase()));

    let mut out = String::new();
    out.push_str("# starchive — Starred Repositories\n\n");
    out.push_str(&lang.export_intro(repos.len()));
    out.push_str("\n\n## Contents\n\n");
    for r in &repos {
        out.push_str(&format!("- {}\n", r.full_name));
    }
    out.push_str("\n---\n\n");

    for r in &repos {
        let path = st
            .config
            .archive_dir
            .join(r.owner())
            .join(format!("{}.md", r.name));
        match tokio::fs::read_to_string(&path).await {
            Ok(md) => {
                out.push_str(md.trim_end());
                out.push_str("\n\n---\n\n");
            }
            // Repo not archived yet → a minimal stub so the export stays complete.
            Err(_) => {
                out.push_str(&format!(
                    "# {}\n\n> {}\n\n{}\n\n---\n\n",
                    r.full_name,
                    r.description.as_deref().unwrap_or(""),
                    r.html_url
                ));
            }
        }
    }

    Ok((
        [
            (CONTENT_TYPE, "text/markdown; charset=utf-8".to_string()),
            (
                CONTENT_DISPOSITION,
                "attachment; filename=\"starchive-stars.md\"".to_string(),
            ),
        ],
        out,
    )
        .into_response())
}

/// `POST /archive/{owner}/{name}/summary` — AI summary (cached per repo+lang).
pub async fn archive_summary(
    lang: Lang,
    State(st): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
) -> Result<Html<String>> {
    let Some(llm) = &st.llm else {
        return Ok(Html(String::new()));
    };
    let Some(repo_id) = st.db.repo_id_by_full_name(&owner, &name).await? else {
        return Ok(Html(String::new()));
    };

    if let Some(cached) = st.db.get_ai_cache(repo_id, lang.code(), "summary").await? {
        return Ok(Html(render_summary(lang, &cached)));
    }
    let Some(repo) = st.db.get_repo(repo_id).await? else {
        return Ok(Html(String::new()));
    };

    let readme = st
        .client
        .fetch_readme(repo.owner(), &repo.name)
        .await
        .unwrap_or(None);
    match ai::repo_summary(llm, &repo, readme.as_deref(), lang).await {
        Ok(summary) => {
            let now = Utc::now().to_rfc3339();
            st.db
                .set_ai_cache(repo_id, lang.code(), "summary", &summary, llm.model(), &now)
                .await
                .ok();
            Ok(Html(render_summary(lang, &summary)))
        }
        Err(e) => Ok(Html(render_summary(lang, &format!("⚠ {e}")))),
    }
}

/// `GET /archive/{owner}/{name}/raw` — download the raw markdown file.
pub async fn archive_raw(
    State(st): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
) -> Result<Response> {
    let md = read_archive(&st, &owner, &name).await?;
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

/// `GET /favicon.svg` — a gold star, so browsers stop 404-ing on the favicon.
pub async fn favicon() -> Response {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#d29922"><path d="M12 .6l3.1 7.3 7.9.6-6 5.1 1.9 7.7L12 18.3 5.1 22.4 7 14.7l-6-5.1 7.9-.6z"/></svg>"##;
    ([(CONTENT_TYPE, "image/svg+xml")], svg).into_response()
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

fn render_summary(lang: Lang, text: &str) -> String {
    views::ai_summary(lang, text).into_string()
}

fn error_fragment(msg: &str) -> String {
    let safe = msg.replace('<', "&lt;");
    format!("<div class=\"empty\"><p class=\"sub\">⚠ {safe}</p></div>")
}

fn render_markdown_html(md: &str) -> String {
    let mut opts = comrak::Options::default();
    opts.extension.table = true;
    opts.extension.strikethrough = true;
    opts.extension.autolink = true;
    opts.extension.tasklist = true;
    comrak::markdown_to_html(md, &opts)
}

async fn read_archive(st: &AppState, owner: &str, name: &str) -> Result<String> {
    let path = st.config.archive_dir.join(owner).join(format!("{name}.md"));
    tokio::fs::read_to_string(&path).await.map_err(|_| {
        AppError::msg(format!(
            "No archive for {owner}/{name} yet — run a sync first."
        ))
    })
}

/// Extract just the path of a Referer so any redirect stays same-origin.
fn referer_path(referer: &str) -> Option<String> {
    let path = if let Some(idx) = referer.find("://") {
        let after = &referer[idx + 3..];
        after.find('/').map(|p| &after[p..]).unwrap_or("/")
    } else if referer.starts_with('/') {
        referer
    } else {
        return None;
    };
    if path.starts_with("//") {
        Some("/".to_string())
    } else {
        Some(path.to_string())
    }
}
