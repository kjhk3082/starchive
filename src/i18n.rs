//! Minimal internationalization: English and Korean.
//!
//! `Lang` is an axum extractor — handlers just add a `lang: Lang` parameter and
//! it's resolved from (1) the `lang` cookie, (2) the `Accept-Language` header,
//! (3) English as the default. Every user-facing string is a method on `Lang`,
//! so translations live in one place and the compiler enforces coverage.

use std::convert::Infallible;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Ko,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ko => "ko",
        }
    }

    pub fn parse(code: &str) -> Option<Lang> {
        match code {
            "en" => Some(Lang::En),
            "ko" => Some(Lang::Ko),
            _ => None,
        }
    }

    // --- Navigation / chrome -------------------------------------------------
    pub fn nav_stars(self) -> &'static str {
        match self {
            Lang::En => "Stars",
            Lang::Ko => "스타",
        }
    }
    pub fn nav_trending(self) -> &'static str {
        match self {
            Lang::En => "Trending",
            Lang::Ko => "트렌딩",
        }
    }
    pub fn footer(self) -> &'static str {
        match self {
            Lang::En => "starchive · your GitHub stars, archived as AI-readable markdown",
            Lang::Ko => "starchive · 내 GitHub 스타를 AI가 읽기 좋은 마크다운으로 아카이브",
        }
    }

    // --- Stars page ----------------------------------------------------------
    pub fn stars_title(self) -> &'static str {
        match self {
            Lang::En => "Your Stars",
            Lang::Ko => "내 스타",
        }
    }
    pub fn stars_count(self, n: usize) -> String {
        match self {
            Lang::En => format!("{n} repositories tracked"),
            Lang::Ko => format!("저장소 {n}개 추적 중"),
        }
    }
    pub fn refresh(self) -> &'static str {
        match self {
            Lang::En => "↻ Refresh",
            Lang::Ko => "↻ 새로고침",
        }
    }
    pub fn stars_empty_title(self) -> &'static str {
        match self {
            Lang::En => "No stars recorded yet.",
            Lang::Ko => "아직 기록된 스타가 없어요.",
        }
    }
    pub fn stars_empty_hint(self) -> &'static str {
        match self {
            Lang::En => "Hit Refresh to pull your GitHub stars and archive them.",
            Lang::Ko => "새로고침을 누르면 GitHub 스타를 가져와 아카이브합니다.",
        }
    }
    pub fn refreshed_banner(self, added: u32, updated: u32, removed: u32, total: usize) -> String {
        match self {
            Lang::En => format!(
                "Refreshed — {added} added, {updated} updated, {removed} removed (★ {total} total)"
            ),
            Lang::Ko => {
                format!(
                    "새로고침 완료 — 추가 {added} · 갱신 {updated} · 제거 {removed} (★ 총 {total}개)"
                )
            }
        }
    }
    pub fn new_badge(self) -> &'static str {
        match self {
            Lang::En => "NEW",
            Lang::Ko => "새 항목",
        }
    }
    pub fn archive_link(self) -> &'static str {
        match self {
            Lang::En => "📄 archive",
            Lang::Ko => "📄 아카이브",
        }
    }

    // --- Trending page -------------------------------------------------------
    pub fn trending_title(self) -> &'static str {
        match self {
            Lang::En => "Trending",
            Lang::Ko => "트렌딩",
        }
    }
    pub fn trending_sub(self) -> &'static str {
        match self {
            Lang::En => "newly popular repositories on GitHub",
            Lang::Ko => "GitHub에서 새로 떠오르는 저장소",
        }
    }
    pub fn for_you(self) -> &'static str {
        match self {
            Lang::En => "★ For You",
            Lang::Ko => "★ 추천",
        }
    }
    pub fn trending_now(self) -> &'static str {
        match self {
            Lang::En => "🔥 Trending now",
            Lang::Ko => "🔥 지금 트렌딩",
        }
    }
    pub fn for_you_empty(self) -> &'static str {
        match self {
            Lang::En => "Sync your stars first to get personalized picks.",
            Lang::Ko => "먼저 스타를 동기화하면 맞춤 추천이 나와요.",
        }
    }
    pub fn trending_empty(self) -> &'static str {
        match self {
            Lang::En => "No trending results right now.",
            Lang::Ko => "지금은 트렌딩 결과가 없어요.",
        }
    }
    pub fn profile_note(self, n: usize, langs: &str) -> String {
        match self {
            Lang::En => format!("Ranked from your {n} stars · top languages: {langs}"),
            Lang::Ko => format!("내 스타 {n}개 기반 · 주요 언어: {langs}"),
        }
    }
    pub fn profile_note_empty(self) -> &'static str {
        match self {
            Lang::En => "Sync your stars to personalize these picks.",
            Lang::Ko => "스타를 동기화하면 맞춤 추천이 켜집니다.",
        }
    }

    // --- Archive viewer ------------------------------------------------------
    pub fn archive_subtitle(self) -> &'static str {
        match self {
            Lang::En => "AI-readable archive",
            Lang::Ko => "AI가 읽기 좋은 아카이브",
        }
    }
    pub fn back(self) -> &'static str {
        match self {
            Lang::En => "← Back",
            Lang::Ko => "← 뒤로",
        }
    }

    // --- Recommendation reasons (rendered from structured data) ---------------
    pub fn reason_language(self, language: &str) -> String {
        match self {
            Lang::En => format!("{language} matches your stars"),
            Lang::Ko => format!("{language} — 내 스타와 일치"),
        }
    }
    pub fn reason_topics(self, topics: &str) -> String {
        match self {
            Lang::En => format!("topics: {topics}"),
            Lang::Ko => format!("토픽: {topics}"),
        }
    }
}

/// Resolve from cookie → `Accept-Language` → default.
impl<S> FromRequestParts<S> for Lang
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(lang) = cookie_lang(parts) {
            return Ok(lang);
        }
        if let Some(lang) = accept_language(parts) {
            return Ok(lang);
        }
        Ok(Lang::default())
    }
}

fn cookie_lang(parts: &Parts) -> Option<Lang> {
    let cookies = parts.headers.get("cookie")?.to_str().ok()?;
    cookies
        .split(';')
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, _)| *k == "lang")
        .and_then(|(_, v)| Lang::parse(v.trim()))
}

fn accept_language(parts: &Parts) -> Option<Lang> {
    let header = parts.headers.get("accept-language")?.to_str().ok()?;
    // The first language tag wins; we only distinguish Korean from the rest.
    let first = header.split(',').next()?.split(';').next()?.trim();
    if first.to_ascii_lowercase().starts_with("ko") {
        Some(Lang::Ko)
    } else {
        Some(Lang::En)
    }
}
