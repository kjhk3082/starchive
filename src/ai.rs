//! LLM-powered features: per-repo summaries and project-based discovery. These
//! orchestrate the [`Llm`] client with GitHub data and always answer in the
//! caller's UI language. Everything here is only reachable when a key is set.

use std::collections::HashMap;

use serde::Deserialize;

use crate::error::Result;
use crate::github::models::Repo;
use crate::github::{GithubClient, SearchParams};
use crate::i18n::Lang;
use crate::llm::Llm;

fn lang_instr(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "Respond in English.",
        Lang::Ko => "반드시 자연스러운 한국어로 답하세요.",
    }
}

/// A concise, plain-language summary of what a repo is and when to use it.
pub async fn repo_summary(
    llm: &Llm,
    repo: &Repo,
    readme: Option<&str>,
    lang: Lang,
) -> Result<String> {
    let system = format!(
        "You are a concise technical writer who explains what a GitHub repository is and \
         when a developer would use it. {} Write 2-3 plain sentences. No markdown headings, \
         no bullet lists, no preamble like 'This repository'.",
        lang_instr(lang)
    );
    let excerpt: String = readme.unwrap_or("").chars().take(2000).collect();
    let user = format!(
        "Repository: {}\nDescription: {}\nLanguage: {}\nTopics: {}\n\nREADME excerpt:\n{}",
        repo.full_name,
        repo.description.as_deref().unwrap_or("(none)"),
        repo.language.as_deref().unwrap_or("(unknown)"),
        repo.topics.join(", "),
        excerpt,
    );
    llm.complete(&system, &user, 400).await
}

/// A repo recommended for the user's described project.
pub struct Recommendation {
    pub full_name: String,
    pub html_url: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stars: i64,
    pub license: Option<String>,
    pub reason: String,
    pub from_stars: bool,
}

#[derive(Deserialize, Default)]
struct RankItem {
    #[serde(default)]
    repo: String,
    #[serde(default)]
    reason: String,
}

/// Recommend repos for a project description, drawing from the user's stars and
/// a fresh GitHub search, then ranking + explaining with the LLM.
pub async fn discover(
    llm: &Llm,
    github: &GithubClient,
    stars: &[Repo],
    description: &str,
    lang: Lang,
) -> Result<Vec<Recommendation>> {
    // 1) Extract search keywords.
    let keywords = llm
        .complete(
            "Extract GitHub search keywords for finding libraries/tools for a project. \
             Reply ONLY with 2-4 comma-separated keywords (technologies or topics). No other text.",
            &format!("Project: {description}"),
            60,
        )
        .await?;
    let kw_list: Vec<String> = keywords
        .replace('\n', " ")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(4)
        .collect();
    if kw_list.is_empty() {
        return Ok(Vec::new());
    }

    // 2) Candidates from a fresh GitHub search (failures are non-fatal).
    let search = github
        .search_trending(&SearchParams {
            query: format!("{} stars:>30", kw_list.join(" ")),
            per_page: 15,
        })
        .await
        .unwrap_or_default();

    // 3) Candidates from the user's own stars (keyword match).
    let kw_lower: Vec<String> = kw_list.iter().map(|k| k.to_lowercase()).collect();
    let star_matches = stars.iter().filter(|r| {
        let hay = format!(
            "{} {} {}",
            r.full_name,
            r.description.as_deref().unwrap_or(""),
            r.topics.join(" ")
        )
        .to_lowercase();
        kw_lower.iter().any(|k| hay.contains(k))
    });

    // 4) Merge (stars first), dedupe, cap.
    let mut cands: Vec<(Repo, bool)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for r in star_matches {
        if seen.insert(r.full_name.clone()) {
            cands.push((r.clone(), true));
        }
    }
    for r in &search {
        if seen.insert(r.full_name.clone()) {
            cands.push((r.clone(), false));
        }
    }
    cands.truncate(12); // fewer candidates → smaller prompt → faster ranking
    if cands.is_empty() {
        return Ok(Vec::new());
    }

    // 5) Rank + explain with the LLM (strict JSON out).
    let mut listing = String::new();
    for (i, (r, from_star)) in cands.iter().enumerate() {
        listing.push_str(&format!(
            "{}. {} — {} [{}, {}★{}]\n",
            i + 1,
            r.full_name,
            r.description.as_deref().unwrap_or(""),
            r.language.as_deref().unwrap_or("?"),
            r.stargazers_count,
            if *from_star { ", starred" } else { "" }
        ));
    }
    let system = format!(
        "You recommend GitHub repositories for a developer's project. From the candidates, \
         pick the most relevant (up to 6). {} Reply ONLY as a JSON array; each item has \
         \"repo\" (the full_name EXACTLY as listed) and \"reason\" (ONE sentence on why it \
         fits the project). Output nothing except the JSON array.",
        lang_instr(lang)
    );
    let user = format!("Project description:\n{description}\n\nCandidates:\n{listing}");
    let raw = llm.complete(&system, &user, 550).await?;

    // 6) Parse leniently and map back to candidate metadata.
    let parsed: Vec<RankItem> = serde_json::from_str(&extract_json_array(&raw)).unwrap_or_default();
    let index: HashMap<&str, usize> = cands
        .iter()
        .enumerate()
        .map(|(i, (r, _))| (r.full_name.as_str(), i))
        .collect();

    let mut recs = Vec::new();
    for item in parsed {
        if let Some(&i) = index.get(item.repo.as_str()) {
            let (repo, from_star) = &cands[i];
            recs.push(Recommendation {
                full_name: repo.full_name.clone(),
                html_url: repo.html_url.clone(),
                description: repo.description.clone(),
                language: repo.language.clone(),
                stars: repo.stargazers_count,
                license: repo.license_spdx().map(str::to_string),
                reason: item.reason,
                from_stars: *from_star,
            });
        }
    }
    Ok(recs)
}

/// Pull the first `[ ... ]` out of a possibly-chatty LLM reply.
fn extract_json_array(s: &str) -> String {
    match (s.find('['), s.rfind(']')) {
        (Some(a), Some(b)) if b > a => s[a..=b].to_string(),
        _ => "[]".to_string(),
    }
}
