//! HTML views, rendered with maud to `Markup`/`String`. We render to string and
//! wrap in `axum::response::Html` in handlers, so there's no maud↔axum version
//! coupling. All user-facing text comes from [`Lang`], so the UI is bilingual.

use maud::{DOCTYPE, Markup, PreEscaped, html};

use crate::db::StarView;
use crate::github::models::Repo;
use crate::i18n::Lang;
use crate::recommend::{Reason, Scored};

const CSS: &str = r#"
:root{--bg:#0d1117;--card:#161b22;--border:#30363d;--fg:#e6edf3;--muted:#8b949e;
--accent:#2f81f7;--accent2:#3fb950;--new:#d29922;--chip:#21262d}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);
font:15px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif}
a{color:var(--accent);text-decoration:none}a:hover{text-decoration:underline}
header{position:sticky;top:0;background:rgba(13,17,23,.85);backdrop-filter:blur(8px);
border-bottom:1px solid var(--border);padding:14px 24px;display:flex;align-items:center;gap:20px;z-index:10}
header .logo{font-weight:700;font-size:18px;letter-spacing:-.3px}
header .logo span{color:var(--new)}
nav{display:flex;gap:4px;margin-left:8px}
nav a{padding:6px 14px;border-radius:8px;color:var(--muted);font-weight:500}
nav a.active{background:var(--chip);color:var(--fg)}
.langtoggle{margin-left:auto;display:flex;gap:2px;font-size:12.5px}
.langtoggle a{padding:5px 10px;border-radius:7px;color:var(--muted)}
.langtoggle a.active{background:var(--chip);color:var(--fg);font-weight:600}
.main{max-width:1100px;margin:0 auto;padding:24px}
.row{display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:18px;flex-wrap:wrap}
h1{font-size:22px;margin:0}h2{font-size:16px;color:var(--muted);font-weight:600;margin:28px 0 12px;
text-transform:uppercase;letter-spacing:.5px}
.sub{color:var(--muted);font-size:13px}
.btn{background:var(--accent);color:#fff;border:0;border-radius:8px;padding:9px 16px;
font-size:14px;font-weight:600;cursor:pointer;display:inline-block}
.btn:hover{filter:brightness(1.1)}.btn:disabled{opacity:.6;cursor:wait}
.btn.ghost{background:var(--chip);color:var(--fg)}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(330px,1fr));gap:14px}
.card{background:var(--card);border:1px solid var(--border);border-radius:12px;padding:16px;
display:flex;flex-direction:column;gap:10px;transition:border-color .15s}
.card:hover{border-color:#444c56}
.card.new{border-color:var(--new);box-shadow:0 0 0 1px var(--new) inset}
.card .title{display:flex;align-items:center;gap:8px;justify-content:space-between}
.card .title b{font-size:15px}
.badge{font-size:11px;font-weight:700;padding:2px 7px;border-radius:20px;background:var(--new);color:#000}
.desc{color:var(--muted);font-size:13.5px;min-height:2.6em}
.meta{display:flex;gap:12px;align-items:center;flex-wrap:wrap;font-size:12.5px;color:var(--muted)}
.dot{width:9px;height:9px;border-radius:50%;display:inline-block;margin-right:5px;vertical-align:-1px}
.chips{display:flex;gap:6px;flex-wrap:wrap}
.chip{background:var(--chip);color:#adbac7;font-size:11.5px;padding:2px 8px;border-radius:20px}
.reasons{font-size:12px;color:var(--accent2);display:flex;gap:6px;flex-wrap:wrap}
.reason{background:rgba(63,185,80,.12);padding:2px 8px;border-radius:20px}
.banner{background:rgba(63,185,80,.15);border:1px solid var(--accent2);color:var(--accent2);
padding:10px 14px;border-radius:10px;margin-bottom:14px;font-weight:600}
.empty{color:var(--muted);text-align:center;padding:60px 20px}
.htmx-request .btn{opacity:.6}
.spin{display:none}.htmx-request .spin{display:inline}
pre{background:var(--card);border:1px solid var(--border);border-radius:12px;padding:20px;
overflow:auto;font-size:13px;white-space:pre-wrap;word-wrap:break-word}
footer{color:var(--muted);font-size:12px;text-align:center;padding:30px}
"#;

fn lang_color(lang: &str) -> &'static str {
    match lang {
        "Rust" => "#dea584",
        "Python" => "#3572A5",
        "TypeScript" => "#3178c6",
        "JavaScript" => "#f1e05a",
        "Go" => "#00ADD8",
        "Shell" => "#89e051",
        "C" | "C++" => "#f34b7d",
        "Java" => "#b07219",
        "Ruby" => "#701516",
        "HTML" => "#e34c26",
        "Jupyter Notebook" => "#DA5B0B",
        _ => "#8b949e",
    }
}

/// Compact star/fork counts: 1234 -> 1.2k, 213534 -> 213.5k.
fn fmt_count(n: i64) -> String {
    if n >= 1000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}

fn lang_dot(lang: Option<&str>) -> Markup {
    html! {
        @if let Some(l) = lang {
            span { span.dot style=(format!("background:{}", lang_color(l))) {} (l) }
        }
    }
}

fn render_reason(lang: Lang, reason: &Reason) -> String {
    match reason {
        Reason::Language(l) => lang.reason_language(l),
        Reason::Topics(ts) => lang.reason_topics(&ts.join(", ")),
    }
}

/// Full HTML page shell, including nav and the EN/한국어 toggle.
pub fn layout(lang: Lang, title: &str, active: &str, content: Markup) -> String {
    let page = html! {
        (DOCTYPE)
        html lang=(lang.code()) {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · starchive" }
                style { (PreEscaped(CSS)) }
                script src="https://unpkg.com/htmx.org@2.0.4" {}
            }
            body {
                header {
                    div.logo { "star" span { "chive" } }
                    nav {
                        a href="/" class=(if active == "stars" { "active" } else { "" }) { (lang.nav_stars()) }
                        a href="/trending" class=(if active == "trending" { "active" } else { "" }) { (lang.nav_trending()) }
                    }
                    div.langtoggle {
                        a href="/lang/en" class=(if lang == Lang::En { "active" } else { "" }) { "EN" }
                        a href="/lang/ko" class=(if lang == Lang::Ko { "active" } else { "" }) { "한국어" }
                    }
                }
                div.main { (content) }
                footer { (lang.footer()) }
            }
        }
    };
    page.into_string()
}

fn star_card(lang: Lang, s: &StarView) -> Markup {
    html! {
        div.card.new[s.is_new()] {
            div.title {
                b { a href=(s.html_url) target="_blank" rel="noopener" { (s.full_name) } }
                @if s.is_new() { span.badge { (lang.new_badge()) } }
            }
            div.desc { (s.description.as_deref().unwrap_or("")) }
            div.meta {
                (lang_dot(s.language.as_deref()))
                span { "★ " (fmt_count(s.stargazers_count)) }
                @if s.archived_path.is_some() {
                    a href=(format!("/archive/{}/{}", s.owner, s.name)) { (lang.archive_link()) }
                }
            }
            @let topics = s.topics();
            @if !topics.is_empty() {
                div.chips { @for t in topics.iter().take(4) { span.chip { (t) } } }
            }
        }
    }
}

/// The inner content of the stars list (returned by the refresh endpoint).
pub fn stars_list_inner(lang: Lang, stars: &[StarView], banner: Option<&str>) -> Markup {
    html! {
        @if let Some(b) = banner { div.banner { (b) } }
        @if stars.is_empty() {
            div.empty {
                p { (lang.stars_empty_title()) }
                p.sub { (lang.stars_empty_hint()) }
            }
        } @else {
            div.grid { @for s in stars { (star_card(lang, s)) } }
        }
    }
}

/// Full Stars page.
pub fn stars_page(lang: Lang, stars: &[StarView]) -> Markup {
    html! {
        div.row {
            div {
                h1 { (lang.stars_title()) }
                div.sub { (lang.stars_count(stars.len())) }
            }
            button.btn hx-post="/stars/refresh" hx-target="#stars-list" hx-swap="innerHTML" {
                (lang.refresh()) span.spin { " …" }
            }
        }
        div #stars-list { (stars_list_inner(lang, stars, None)) }
    }
}

fn repo_card(lang: Lang, r: &Repo, reasons: Option<&[Reason]>) -> Markup {
    html! {
        div.card {
            div.title { b { a href=(r.html_url) target="_blank" rel="noopener" { (r.full_name) } } }
            div.desc { (r.description.as_deref().unwrap_or("")) }
            div.meta {
                (lang_dot(r.language.as_deref()))
                span { "★ " (fmt_count(r.stargazers_count)) }
            }
            @if let Some(rs) = reasons {
                @if !rs.is_empty() {
                    div.reasons { @for reason in rs { span.reason { (render_reason(lang, reason)) } } }
                }
            }
            @if !r.topics.is_empty() {
                div.chips { @for t in r.topics.iter().take(4) { span.chip { (t) } } }
            }
        }
    }
}

/// Trending + personalized "For You" page.
pub fn trending_page(
    lang: Lang,
    trending: &[Repo],
    for_you: &[Scored],
    profile_note: &str,
) -> Markup {
    html! {
        div.row { div { h1 { (lang.trending_title()) } div.sub { (lang.trending_sub()) } } }

        h2 { (lang.for_you()) }
        div.sub style="margin:-6px 0 12px" { (profile_note) }
        @if for_you.is_empty() {
            div.empty { p.sub { (lang.for_you_empty()) } }
        } @else {
            div.grid { @for s in for_you.iter().take(12) { (repo_card(lang, &s.repo, Some(&s.reasons))) } }
        }

        h2 { (lang.trending_now()) }
        @if trending.is_empty() {
            div.empty { p.sub { (lang.trending_empty()) } }
        } @else {
            div.grid { @for r in trending { (repo_card(lang, r, None)) } }
        }
    }
}

/// Markdown archive viewer, with a raw-download button.
pub fn archive_page(lang: Lang, owner: &str, name: &str, md: &str) -> Markup {
    let full = format!("{owner}/{name}");
    html! {
        div.row {
            div { h1 { (full) } div.sub { (lang.archive_subtitle()) } }
            div style="display:flex;gap:8px" {
                a.btn href=(format!("/archive/{owner}/{name}/raw")) { "⬇ .md" }
                a.btn.ghost href="/" { (lang.back()) }
            }
        }
        pre { (md) }
    }
}
