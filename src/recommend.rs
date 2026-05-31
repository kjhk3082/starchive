//! Personalized ranking. Builds a taste profile from the user's stars
//! (language + topic frequencies) and scores trending repos by overlap, with a
//! small popularity tiebreaker. Pure functions — fully unit-tested.

use std::collections::HashMap;

use crate::github::models::Repo;

/// Frequency profile of the user's stars.
#[derive(Debug, Default)]
pub struct Profile {
    pub languages: HashMap<String, usize>,
    pub topics: HashMap<String, usize>,
    pub total: usize,
}

impl Profile {
    /// The user's most-starred languages, most frequent first.
    pub fn top_languages(&self, n: usize) -> Vec<(String, usize)> {
        let mut v: Vec<(String, usize)> = self
            .languages
            .iter()
            .map(|(k, c)| (k.clone(), *c))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }
}

pub fn build_profile(repos: &[Repo]) -> Profile {
    let mut p = Profile {
        total: repos.len(),
        ..Default::default()
    };
    for r in repos {
        if let Some(lang) = &r.language {
            *p.languages.entry(lang.clone()).or_insert(0) += 1;
        }
        for t in &r.topics {
            *p.topics.entry(t.clone()).or_insert(0) += 1;
        }
    }
    p
}

/// A trending repo scored against the user's profile.
#[derive(Debug, Clone)]
pub struct Scored {
    pub repo: Repo,
    pub score: f64,
    pub reasons: Vec<String>,
}

const LANG_WEIGHT: f64 = 3.0;
const TOPIC_WEIGHT: f64 = 1.0;
const POPULARITY_WEIGHT: f64 = 0.01;

/// Score and rank trending repos for the user. Language match dominates, topics
/// add up, and a logarithmic popularity term breaks ties.
pub fn score_trending(profile: &Profile, trending: &[Repo]) -> Vec<Scored> {
    let max_lang = profile
        .languages
        .values()
        .copied()
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    let max_topic = profile.topics.values().copied().max().unwrap_or(1).max(1) as f64;

    let mut scored: Vec<Scored> = trending
        .iter()
        .map(|r| {
            let mut score = 0.0;
            let mut reasons = Vec::new();

            if let Some(lang) = &r.language
                && let Some(&c) = profile.languages.get(lang)
            {
                score += (c as f64 / max_lang) * LANG_WEIGHT;
                reasons.push(format!("{lang} matches your stars"));
            }

            let matched: Vec<String> = r
                .topics
                .iter()
                .filter(|t| profile.topics.contains_key(*t))
                .cloned()
                .collect();
            for t in &matched {
                let c = profile.topics[t] as f64;
                score += (c / max_topic) * TOPIC_WEIGHT;
            }
            if !matched.is_empty() {
                reasons.push(format!("topics: {}", matched.join(", ")));
            }

            // Tiny tiebreaker so popular repos win among equal matches.
            score += (r.stargazers_count.max(1) as f64).ln() * POPULARITY_WEIGHT;

            Scored {
                repo: r.clone(),
                score,
                reasons,
            }
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::models::Repo;

    #[test]
    fn profile_counts_languages_and_topics() {
        let stars = vec![
            Repo::sample(1, "a", "r1", Some("Rust"), &["cli"]),
            Repo::sample(2, "b", "r2", Some("Rust"), &["cli", "tui"]),
            Repo::sample(3, "c", "r3", Some("Python"), &[]),
        ];
        let p = build_profile(&stars);
        assert_eq!(p.total, 3);
        assert_eq!(p.languages["Rust"], 2);
        assert_eq!(p.languages["Python"], 1);
        assert_eq!(p.topics["cli"], 2);
        assert_eq!(p.top_languages(1), vec![("Rust".to_string(), 2)]);
    }

    #[test]
    fn ranks_matching_repo_above_nonmatching_with_reasons() {
        let stars = vec![
            Repo::sample(1, "a", "r1", Some("Rust"), &["cli"]),
            Repo::sample(2, "b", "r2", Some("Rust"), &["cli", "tui"]),
            Repo::sample(3, "c", "r3", Some("Python"), &[]),
        ];
        let profile = build_profile(&stars);

        let trending = vec![
            Repo::sample(10, "x", "match", Some("Rust"), &["cli"]),
            Repo::sample(11, "y", "nomatch", Some("Haskell"), &["web"]),
        ];
        let scored = score_trending(&profile, &trending);
        assert_eq!(scored[0].repo.name, "match");
        assert!(scored[0].score > scored[1].score);
        assert!(scored[0].reasons.iter().any(|r| r.contains("Rust")));
        assert!(scored[0].reasons.iter().any(|r| r.contains("cli")));
        assert!(scored[1].reasons.is_empty());
    }

    #[test]
    fn empty_profile_falls_back_to_popularity() {
        let profile = build_profile(&[]);
        let mut low = Repo::sample(1, "a", "low", Some("Go"), &[]);
        low.stargazers_count = 10;
        let mut high = Repo::sample(2, "b", "high", Some("Go"), &[]);
        high.stargazers_count = 100_000;
        let scored = score_trending(&profile, &[low, high]);
        assert_eq!(scored[0].repo.name, "high");
    }
}
