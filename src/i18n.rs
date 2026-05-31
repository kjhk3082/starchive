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
    pub fn view_reading(self) -> &'static str {
        match self {
            Lang::En => "👁 Reading",
            Lang::Ko => "👁 읽기용",
        }
    }
    pub fn view_markdown(self) -> &'static str {
        match self {
            Lang::En => "🤖 Markdown",
            Lang::Ko => "🤖 마크다운",
        }
    }
    pub fn export_all(self) -> &'static str {
        match self {
            Lang::En => "⬇ Export all",
            Lang::Ko => "⬇ 전체 내보내기",
        }
    }
    pub fn export_intro(self, n: usize) -> String {
        match self {
            Lang::En => {
                format!("Combined archive of {n} starred repositories, generated by starchive.")
            }
            Lang::Ko => format!("starchive가 생성한 스타 저장소 {n}개의 통합 아카이브입니다."),
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

    // --- Chrome additions ----------------------------------------------------
    pub fn nav_discover(self) -> &'static str {
        match self {
            Lang::En => "Discover",
            Lang::Ko => "발견",
        }
    }
    pub fn credit(self) -> &'static str {
        match self {
            Lang::En => "Developer",
            Lang::Ko => "개발자",
        }
    }
    pub fn stat_stars(self) -> &'static str {
        match self {
            Lang::En => "stars",
            Lang::Ko => "스타",
        }
    }
    pub fn search_placeholder(self) -> &'static str {
        match self {
            Lang::En => "Search your stars…",
            Lang::Ko => "스타 검색…",
        }
    }

    // --- Discover page -------------------------------------------------------
    pub fn discover_title(self) -> &'static str {
        match self {
            Lang::En => "Discover repos for your project",
            Lang::Ko => "내 프로젝트에 맞는 레포 발견",
        }
    }
    pub fn discover_sub(self) -> &'static str {
        match self {
            Lang::En => {
                "Describe what you're building — get matching repos from your stars and GitHub."
            }
            Lang::Ko => "만들고 있는 걸 설명하면, 내 스타와 GitHub에서 어울리는 레포를 찾아줍니다.",
        }
    }
    pub fn discover_placeholder(self) -> &'static str {
        match self {
            Lang::En => {
                "e.g. A Rust CLI that syncs Notion pages to local markdown with offline full-text search…"
            }
            Lang::Ko => {
                "예: 노션 페이지를 로컬 마크다운으로 동기화하고 오프라인 전문검색되는 Rust CLI…"
            }
        }
    }
    pub fn discover_btn(self) -> &'static str {
        match self {
            Lang::En => "✨ Find repos",
            Lang::Ko => "✨ 레포 찾기",
        }
    }
    pub fn discover_waiting(self) -> &'static str {
        match self {
            Lang::En => "⏳ The AI is searching and ranking repos — this can take up to a minute.",
            Lang::Ko => "⏳ AI가 레포를 검색하고 평가하는 중 — 최대 1분 정도 걸릴 수 있어요.",
        }
    }
    pub fn discover_none(self) -> &'static str {
        match self {
            Lang::En => "No matches — try describing your project differently.",
            Lang::Ko => "맞는 결과가 없어요 — 프로젝트 설명을 바꿔보세요.",
        }
    }
    pub fn discover_nokey_title(self) -> &'static str {
        match self {
            Lang::En => "An LLM API key is required.",
            Lang::Ko => "LLM API 키가 필요해요.",
        }
    }
    pub fn discover_nokey_hint(self) -> &'static str {
        match self {
            Lang::En => "Open Settings (⚙) and add an API key to enable AI discovery.",
            Lang::Ko => "설정(⚙)에서 API 키를 추가하면 AI 발견 기능이 켜집니다.",
        }
    }
    pub fn your_star(self) -> &'static str {
        match self {
            Lang::En => "★ your star",
            Lang::Ko => "★ 내 스타",
        }
    }
    pub fn via_github(self) -> &'static str {
        match self {
            Lang::En => "🔎 GitHub",
            Lang::Ko => "🔎 GitHub",
        }
    }

    // --- AI summary ----------------------------------------------------------
    pub fn ai_summary_title(self) -> &'static str {
        match self {
            Lang::En => "AI Summary",
            Lang::Ko => "AI 요약",
        }
    }
    pub fn ai_generating(self) -> &'static str {
        match self {
            Lang::En => "Generating…",
            Lang::Ko => "생성 중…",
        }
    }

    // --- Settings page -------------------------------------------------------
    pub fn nav_settings(self) -> &'static str {
        match self {
            Lang::En => "Settings",
            Lang::Ko => "설정",
        }
    }
    pub fn settings_title(self) -> &'static str {
        match self {
            Lang::En => "Settings",
            Lang::Ko => "설정",
        }
    }
    pub fn settings_sub(self) -> &'static str {
        match self {
            Lang::En => "Connect an AI provider to enable summaries, license help, and discovery.",
            Lang::Ko => "AI 프로바이더를 연결하면 요약·라이선스 설명·발견 기능이 켜집니다.",
        }
    }
    pub fn settings_status_on(self, provider: &str, model: &str) -> String {
        match self {
            Lang::En => format!("✅ Connected — {provider} · {model}"),
            Lang::Ko => format!("✅ 연결됨 — {provider} · {model}"),
        }
    }
    pub fn settings_status_off(self) -> &'static str {
        match self {
            Lang::En => "⚪ Not connected yet — add a key below.",
            Lang::Ko => "⚪ 아직 연결 안 됨 — 아래에 키를 입력하세요.",
        }
    }
    pub fn settings_provider(self) -> &'static str {
        match self {
            Lang::En => "Provider",
            Lang::Ko => "프로바이더",
        }
    }
    pub fn settings_key(self) -> &'static str {
        match self {
            Lang::En => "API key",
            Lang::Ko => "API 키",
        }
    }
    pub fn settings_key_ph(self) -> &'static str {
        match self {
            Lang::En => "Paste your key — leave blank to keep the current one",
            Lang::Ko => "키 붙여넣기 — 비우면 기존 키 유지",
        }
    }
    pub fn settings_model(self) -> &'static str {
        match self {
            Lang::En => "Model (optional)",
            Lang::Ko => "모델 (선택)",
        }
    }
    pub fn settings_save_btn(self) -> &'static str {
        match self {
            Lang::En => "Save",
            Lang::Ko => "저장",
        }
    }
    pub fn settings_saved(self) -> &'static str {
        match self {
            Lang::En => "Saved ✓",
            Lang::Ko => "저장됨 ✓",
        }
    }
    pub fn settings_note(self) -> &'static str {
        match self {
            Lang::En => {
                "Stored only in your local database (git-ignored). The key is sent only to your chosen provider, never anywhere else."
            }
            Lang::Ko => {
                "키는 로컬 DB(git 제외)에만 저장되고, 선택한 프로바이더에만 전송됩니다. 그 외 어디에도 안 보냅니다."
            }
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
