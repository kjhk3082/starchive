//! GitHub API data models and pure JSON parsing.
//!
//! These functions take `&str` and return typed values — no network — so they
//! are exhaustively unit-testable. The HTTP shell in `super` calls them.
//!
//! Repo objects come from three endpoints with the *same* inner shape:
//!   - `GET /user/starred` (with `star+json`) → array of `{ starred_at, repo }`
//!   - `GET /search/repositories`             → `{ items: [repo, ...] }`
//! Search results frequently omit fields, so every non-identifying field has a
//! `#[serde(default)]` and tolerates `null`.

use serde::Deserialize;

use crate::error::Result;

#[derive(Debug, Clone, Deserialize)]
pub struct Owner {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct License {
    pub spdx_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Repo {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub owner: Owner,
    pub html_url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub stargazers_count: i64,
    #[serde(default)]
    pub forks_count: i64,
    #[serde(default)]
    pub open_issues_count: i64,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub license: Option<License>,
    #[serde(default)]
    pub default_branch: Option<String>,
    #[serde(default)]
    pub pushed_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl Repo {
    /// The owning user/org login (e.g. `BurntSushi`).
    pub fn owner(&self) -> &str {
        &self.owner.login
    }

    /// SPDX license id, ignoring GitHub's `NOASSERTION` / empty placeholders.
    pub fn license_spdx(&self) -> Option<&str> {
        self.license
            .as_ref()
            .and_then(|l| l.spdx_id.as_deref())
            .filter(|s| !s.is_empty() && *s != "NOASSERTION")
    }

    /// A non-empty homepage URL, if any.
    pub fn homepage(&self) -> Option<&str> {
        self.homepage.as_deref().filter(|s| !s.trim().is_empty())
    }

    /// Topics serialized as a JSON array string for DB storage.
    pub fn topics_json(&self) -> String {
        serde_json::to_string(&self.topics).unwrap_or_else(|_| "[]".to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StarredRepo {
    pub starred_at: String,
    pub repo: Repo,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    items: Vec<Repo>,
}

/// Parse the `GET /user/starred` (star+json) response.
pub fn parse_starred(json: &str) -> Result<Vec<StarredRepo>> {
    Ok(serde_json::from_str(json)?)
}

/// Parse the `GET /search/repositories` response, returning just the items.
pub fn parse_search(json: &str) -> Result<Vec<Repo>> {
    let resp: SearchResponse = serde_json::from_str(json)?;
    Ok(resp.items)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STARRED_JSON: &str = r#"[
      {
        "starred_at": "2025-05-30T12:00:00Z",
        "repo": {
          "id": 123,
          "name": "ripgrep",
          "full_name": "BurntSushi/ripgrep",
          "owner": { "login": "BurntSushi" },
          "html_url": "https://github.com/BurntSushi/ripgrep",
          "description": "recursively search directories for a regex pattern",
          "homepage": "",
          "language": "Rust",
          "stargazers_count": 45000,
          "forks_count": 1800,
          "open_issues_count": 90,
          "topics": ["cli", "search"],
          "license": { "spdx_id": "MIT" },
          "default_branch": "master",
          "pushed_at": "2025-01-01T00:00:00Z",
          "created_at": "2016-01-01T00:00:00Z",
          "updated_at": "2025-05-01T00:00:00Z"
        }
      }
    ]"#;

    #[test]
    fn parses_starred_with_timestamp_owner_and_metadata() {
        let stars = parse_starred(STARRED_JSON).unwrap();
        assert_eq!(stars.len(), 1);
        let s = &stars[0];
        assert_eq!(s.starred_at, "2025-05-30T12:00:00Z");
        assert_eq!(s.repo.id, 123);
        assert_eq!(s.repo.owner(), "BurntSushi");
        assert_eq!(s.repo.name, "ripgrep");
        assert_eq!(s.repo.language.as_deref(), Some("Rust"));
        assert_eq!(s.repo.stargazers_count, 45000);
        assert_eq!(s.repo.topics, vec!["cli", "search"]);
        assert_eq!(s.repo.license_spdx(), Some("MIT"));
        // empty homepage string should read as "none"
        assert_eq!(s.repo.homepage(), None);
    }

    #[test]
    fn parses_search_items_and_tolerates_missing_fields() {
        let json = r#"{ "total_count": 1, "incomplete_results": false, "items": [
            { "id": 7, "name": "x", "full_name": "a/x",
              "owner": {"login":"a"}, "html_url": "https://github.com/a/x" }
        ]}"#;
        let repos = parse_search(json).unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].owner(), "a");
        assert_eq!(repos[0].language, None);
        assert_eq!(repos[0].stargazers_count, 0);
        assert!(repos[0].topics.is_empty());
        assert_eq!(repos[0].license_spdx(), None);
        assert_eq!(repos[0].topics_json(), "[]");
    }

    #[test]
    fn license_noassertion_is_treated_as_none() {
        let json = r#"{ "items": [
            { "id": 1, "name":"n", "full_name":"o/n", "owner":{"login":"o"},
              "html_url":"https://github.com/o/n",
              "license": {"spdx_id": "NOASSERTION"} }
        ]}"#;
        let repos = parse_search(json).unwrap();
        assert_eq!(repos[0].license_spdx(), None);
    }
}
