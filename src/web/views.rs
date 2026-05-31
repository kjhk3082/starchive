//! HTML views, rendered with maud to `Markup`/`String` (no maud↔axum coupling).
//! All user-facing text comes from [`Lang`], so the UI is bilingual.

use maud::{DOCTYPE, Markup, PreEscaped, html};

use crate::ai::Recommendation;
use crate::db::StarView;
use crate::github::models::Repo;
use crate::i18n::Lang;
use crate::license::LicenseInfo;
use crate::recommend::{Reason, Scored};

const CSS: &str = r#"
:root{--bg:#0d1117;--card:#161b22;--border:#30363d;--fg:#e6edf3;--muted:#8b949e;
--accent:#2f81f7;--accent2:#3fb950;--new:#d29922;--chip:#21262d;--purple:#a371f7}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);
font:15px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif}
a{color:var(--accent);text-decoration:none}a:hover{text-decoration:underline}
header{position:sticky;top:0;background:rgba(13,17,23,.85);backdrop-filter:blur(8px);
border-bottom:1px solid var(--border);padding:14px 24px;display:flex;align-items:center;gap:18px;z-index:10}
header .logo{font-weight:800;font-size:19px;letter-spacing:-.4px;
background:linear-gradient(90deg,#e6edf3,#d29922);-webkit-background-clip:text;background-clip:text;color:transparent}
nav{display:flex;gap:4px;margin-left:6px}
nav a{padding:6px 13px;border-radius:8px;color:var(--muted);font-weight:500}
nav a.active{background:var(--chip);color:var(--fg)}
.langtoggle{margin-left:auto;display:flex;gap:2px;font-size:12.5px}
.langtoggle a{padding:5px 10px;border-radius:7px;color:var(--muted)}
.langtoggle a.active{background:var(--chip);color:var(--fg);font-weight:600}
.main{max-width:1120px;margin:0 auto;padding:24px}
.row{display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:16px;flex-wrap:wrap}
h1{font-size:23px;margin:0}h2{font-size:15px;color:var(--muted);font-weight:600;margin:26px 0 12px;
text-transform:uppercase;letter-spacing:.5px}
.sub{color:var(--muted);font-size:13px}
.btn{background:var(--accent);color:#fff;border:0;border-radius:8px;padding:9px 16px;
font-size:14px;font-weight:600;cursor:pointer;display:inline-block}
.btn:hover{filter:brightness(1.1)}.btn:disabled{opacity:.6;cursor:wait}
.btn.ghost{background:var(--chip);color:var(--fg)}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(330px,1fr));gap:14px}
.card{background:var(--card);border:1px solid var(--border);border-radius:12px;padding:16px;
display:flex;flex-direction:column;gap:10px;transition:border-color .15s,transform .15s}
.card:hover{border-color:#444c56;transform:translateY(-2px)}
.card.new{border-color:var(--new);box-shadow:0 0 0 1px var(--new) inset}
.card .title{display:flex;align-items:center;gap:8px;justify-content:space-between}
.card .title b{font-size:15px}
.badge{font-size:11px;font-weight:700;padding:2px 7px;border-radius:20px;background:var(--new);color:#000}
.badge.src{background:var(--chip);color:var(--muted);font-weight:600}
.badge.star{background:rgba(210,153,34,.18);color:var(--new)}
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
.stats{display:flex;gap:18px;flex-wrap:wrap;background:var(--card);border:1px solid var(--border);
border-radius:12px;padding:14px 18px;margin-bottom:18px;align-items:center}
.stat b{font-size:20px}.stat span{color:var(--muted);font-size:12px;margin-left:5px}
.langbar{display:flex;height:8px;border-radius:6px;overflow:hidden;min-width:160px;flex:1}
.search{background:var(--card);border:1px solid var(--border);color:var(--fg);border-radius:8px;
padding:9px 14px;font-size:14px;width:240px}
.search:focus{outline:none;border-color:var(--accent)}
.lic{display:inline-flex;align-items:center;gap:8px;background:var(--card);border:1px solid var(--border);
border-radius:10px;padding:10px 14px;margin:4px 0 16px;font-size:13.5px}
.lic .k{font-size:11px;font-weight:700;padding:2px 8px;border-radius:20px}
.k.permissive{background:rgba(63,185,80,.18);color:var(--accent2)}
.k.weakcopyleft{background:rgba(210,153,34,.18);color:var(--new)}
.k.copyleft{background:rgba(248,81,73,.18);color:#f85149}
.k.publicdomain{background:rgba(163,113,247,.18);color:var(--purple)}
.ai{background:linear-gradient(180deg,rgba(163,113,247,.10),transparent);
border:1px solid rgba(163,113,247,.35);border-radius:12px;padding:14px 16px;margin:6px 0 18px}
.ai h3{margin:0 0 6px;font-size:13px;color:var(--purple);text-transform:uppercase;letter-spacing:.4px}
textarea{background:var(--card);border:1px solid var(--border);color:var(--fg);border-radius:10px;
padding:14px;font:inherit;width:100%;min-height:120px;resize:vertical}
textarea:focus{outline:none;border-color:var(--accent)}
.prose{background:var(--card);border:1px solid var(--border);border-radius:12px;padding:6px 22px}
.prose img{max-width:100%}.prose pre{background:#0d1117;padding:14px;border-radius:8px;overflow:auto}
.prose code{background:#0d1117;padding:1px 5px;border-radius:4px;font-size:90%}
.prose pre code{padding:0}.prose h1,.prose h2{border:0;text-transform:none;letter-spacing:0;color:var(--fg)}
.prose table{border-collapse:collapse}.prose td,.prose th{border:1px solid var(--border);padding:6px 10px}
.spin{display:none}.htmx-request .spin{display:inline}.htmx-request.btn{opacity:.7}
footer{color:var(--muted);font-size:12px;text-align:center;padding:30px;line-height:1.8}
footer b{color:var(--fg)}
"#;

const FILTER_JS: &str = r#"
function starFilter(q){q=q.toLowerCase();document.querySelectorAll('#stars-list .card').forEach(function(c){c.style.display=(c.dataset.search||'').includes(q)?'':'none'})}
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

fn license_badge(lang: Lang, info: &LicenseInfo) -> Markup {
    let kind_class = format!("k {:?}", info.kind)
        .to_lowercase()
        .replace("::", "");
    html! {
        div.lic {
            span class=(kind_class) { (info.kind.label(lang)) }
            div { b { (info.name) } " — " (info.summary(lang)) }
        }
    }
}

/// Full HTML page shell: nav, language toggle, favicon, footer credit.
pub fn layout(lang: Lang, title: &str, active: &str, content: Markup) -> String {
    let nav_item = |href: &str, key: &str, label: &str| -> Markup {
        let class = if active == key { "active" } else { "" };
        html! { a href=(href) class=(class) { (label) } }
    };
    let page = html! {
        (DOCTYPE)
        html lang=(lang.code()) {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · starchive" }
                link rel="icon" type="image/svg+xml" href="/favicon.svg";
                style { (PreEscaped(CSS)) }
                script src="https://unpkg.com/htmx.org@2.0.4" {}
            }
            body {
                header {
                    div.logo { "starchive" }
                    nav {
                        (nav_item("/", "stars", lang.nav_stars()))
                        (nav_item("/trending", "trending", lang.nav_trending()))
                        (nav_item("/discover", "discover", lang.nav_discover()))
                    }
                    div.langtoggle {
                        a href="/lang/en" class=(if lang == Lang::En { "active" } else { "" }) { "EN" }
                        a href="/lang/ko" class=(if lang == Lang::Ko { "active" } else { "" }) { "한국어" }
                    }
                }
                div.main { (content) }
                footer {
                    div { (PreEscaped(lang.footer())) }
                    div { (lang.credit()) " · " b { "김재형" } " — Onyultea" }
                }
            }
        }
    };
    page.into_string()
}

fn top_langs(stars: &[StarView]) -> Vec<(String, usize)> {
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for s in stars {
        if let Some(l) = s.language.as_deref() {
            *counts.entry(l).or_insert(0) += 1;
        }
    }
    let mut v: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(k, c)| (k.to_string(), c))
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v.truncate(5);
    v
}

fn stats_strip(lang: Lang, stars: &[StarView]) -> Markup {
    let total = stars.len();
    let langs = top_langs(stars);
    let lang_total: usize = langs.iter().map(|(_, c)| c).sum::<usize>().max(1);
    html! {
        div.stats {
            div.stat { b { (total) } span { (lang.stat_stars()) } }
            @if !langs.is_empty() {
                div.langbar {
                    @for (l, c) in &langs {
                        span title=(format!("{l} · {c}"))
                            style=(format!("width:{}%;background:{}", c * 100 / lang_total, lang_color(l))) {}
                    }
                }
                div.sub { @for (i, (l, _)) in langs.iter().take(3).enumerate() {
                    @if i > 0 { " · " } (l)
                } }
            }
        }
    }
}

fn star_card(lang: Lang, s: &StarView) -> Markup {
    let search = format!(
        "{} {} {}",
        s.full_name,
        s.description.as_deref().unwrap_or(""),
        s.topics_json
    )
    .to_lowercase();
    html! {
        div.card.new[s.is_new()] data-search=(search) {
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

pub fn stars_page(lang: Lang, stars: &[StarView]) -> Markup {
    html! {
        script { (PreEscaped(FILTER_JS)) }
        div.row {
            div {
                h1 { (lang.stars_title()) }
                div.sub { (lang.stars_count(stars.len())) }
            }
            div style="display:flex;gap:10px;align-items:center" {
                input.search type="search" placeholder=(lang.search_placeholder())
                    oninput="starFilter(this.value)";
                button.btn hx-post="/stars/refresh" hx-target="#stars-list" hx-swap="innerHTML" {
                    (lang.refresh()) span.spin { " …" }
                }
            }
        }
        (stats_strip(lang, stars))
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

/// The two trending sections, swappable by the refresh button.
pub fn trending_inner(lang: Lang, trending: &[Repo], for_you: &[Scored], note: &str) -> Markup {
    html! {
        h2 { (lang.for_you()) }
        div.sub style="margin:-6px 0 12px" { (note) }
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

pub fn trending_page(lang: Lang, trending: &[Repo], for_you: &[Scored], note: &str) -> Markup {
    html! {
        div.row {
            div { h1 { (lang.trending_title()) } div.sub { (lang.trending_sub()) } }
            button.btn hx-post="/trending/refresh" hx-target="#trending-list" hx-swap="innerHTML" {
                (lang.refresh()) span.spin { " …" }
            }
        }
        div #trending-list { (trending_inner(lang, trending, for_you, note)) }
    }
}

fn rec_card(lang: Lang, r: &Recommendation) -> Markup {
    html! {
        div.card {
            div.title {
                b { a href=(r.html_url) target="_blank" rel="noopener" { (r.full_name) } }
                @if r.from_stars {
                    span.badge.star { (lang.your_star()) }
                } @else {
                    span.badge.src { (lang.via_github()) }
                }
            }
            @if let Some(d) = &r.description { div.desc { (d) } }
            div.reasons { span.reason { (r.reason.clone()) } }
            div.meta {
                (lang_dot(r.language.as_deref()))
                span { "★ " (fmt_count(r.stars)) }
                @if let Some(spdx) = &r.license {
                    @if let Some(info) = crate::license::explain(spdx) {
                        span { (info.name) " · " (info.kind.label(lang)) }
                    } @else {
                        span { (spdx) }
                    }
                }
            }
        }
    }
}

pub fn discover_results(lang: Lang, recs: &[Recommendation]) -> Markup {
    html! {
        @if recs.is_empty() {
            div.empty { p.sub { (lang.discover_none()) } }
        } @else {
            div.grid { @for r in recs { (rec_card(lang, r)) } }
        }
    }
}

pub fn discover_page(lang: Lang, llm_available: bool) -> Markup {
    html! {
        div.row { div { h1 { (lang.discover_title()) } div.sub { (lang.discover_sub()) } } }
        @if !llm_available {
            div.empty {
                p { (lang.discover_nokey_title()) }
                p.sub { (lang.discover_nokey_hint()) }
            }
        } @else {
            form hx-post="/discover" hx-target="#discover-results" hx-swap="innerHTML" {
                textarea name="description" placeholder=(lang.discover_placeholder()) {}
                div style="margin-top:10px;display:flex;align-items:center;gap:14px;flex-wrap:wrap" {
                    button.btn #disc-btn type="submit" {
                        (lang.discover_btn()) span.spin { " …" }
                    }
                    p.sub.htmx-indicator style="margin:0" { (lang.discover_waiting()) }
                }
            }
            div #discover-results style="margin-top:20px" {}
        }
    }
}

/// AI summary fragment (htmx response on the archive page).
pub fn ai_summary(lang: Lang, summary: &str) -> Markup {
    html! {
        div.ai {
            h3 { (lang.ai_summary_title()) }
            div { (summary) }
        }
    }
}

/// Markdown archive viewer: license box, optional AI summary, rendered README.
pub fn archive_page(
    lang: Lang,
    owner: &str,
    name: &str,
    body_html: &str,
    license: Option<&LicenseInfo>,
    llm_available: bool,
) -> Markup {
    let full = format!("{owner}/{name}");
    html! {
        div.row {
            div { h1 { (full) } div.sub { (lang.archive_subtitle()) } }
            div style="display:flex;gap:8px" {
                a.btn href=(format!("/archive/{owner}/{name}/raw")) { "⬇ .md" }
                a.btn.ghost href="/" { (lang.back()) }
            }
        }
        @if let Some(info) = license { (license_badge(lang, info)) }
        @if llm_available {
            div hx-post=(format!("/archive/{owner}/{name}/summary"))
                hx-trigger="load" hx-swap="outerHTML" {
                div.ai { h3 { (lang.ai_summary_title()) } div.sub { (lang.ai_generating()) } }
            }
        }
        div.prose { (PreEscaped(body_html)) }
    }
}
