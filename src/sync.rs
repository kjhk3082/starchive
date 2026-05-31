//! The reconcile routine — the heart of starchive.
//!
//! Given the user's current GitHub stars and the DB's record, it computes the
//! diff: which stars are **new** (insert + flag `is_new`, queue for archive),
//! which are **existing** (refresh metadata), and which were **removed** (soft
//! delete). It performs no network I/O itself; the caller fetches first, which
//! keeps the diff fully unit-testable against an in-memory DB.

use std::collections::HashSet;

use crate::db::Db;
use crate::error::Result;
use crate::github::models::StarredRepo;

#[derive(Debug, Default, Clone)]
pub struct SyncReport {
    pub added: u32,
    pub updated: u32,
    pub removed: u32,
    /// repo ids that are newly starred — the archive queue.
    pub new_repo_ids: Vec<i64>,
}

impl SyncReport {
    pub fn summary(&self) -> String {
        format!(
            "{} added, {} updated, {} removed",
            self.added, self.updated, self.removed
        )
    }
}

/// Reconcile `starred` (freshly fetched from GitHub) against the DB.
pub async fn reconcile(db: &Db, starred: Vec<StarredRepo>, now: &str) -> Result<SyncReport> {
    let active: HashSet<i64> = db.get_active_star_ids().await?.into_iter().collect();
    let mut report = SyncReport::default();
    let mut seen: HashSet<i64> = HashSet::with_capacity(starred.len());

    for s in &starred {
        let id = s.repo.id;
        seen.insert(id);
        db.upsert_repo(&s.repo).await?; // always refresh metadata
        if active.contains(&id) {
            report.updated += 1;
        } else {
            db.add_star(id, &s.starred_at, now).await?;
            report.added += 1;
            report.new_repo_ids.push(id);
        }
    }

    // Anything active in the DB but absent from GitHub was unstarred.
    for id in &active {
        if !seen.contains(id) {
            db.mark_unstarred(*id, now).await?;
            report.removed += 1;
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::models::{Repo, StarredRepo};

    const NOW: &str = "2026-06-01T00:00:00Z";

    fn star(repo: Repo, at: &str) -> StarredRepo {
        StarredRepo {
            starred_at: at.to_string(),
            repo,
        }
    }

    async fn seed_active(db: &Db, repo: &Repo, at: &str) {
        db.upsert_repo(repo).await.unwrap();
        db.add_star(repo.id, at, NOW).await.unwrap();
    }

    #[tokio::test]
    async fn empty_db_adds_all_as_new() {
        let db = Db::memory().await.unwrap();
        let input = vec![
            star(
                Repo::sample(1, "a", "x", Some("Rust"), &[]),
                "2026-05-01T00:00:00Z",
            ),
            star(
                Repo::sample(2, "b", "y", Some("Go"), &[]),
                "2026-05-02T00:00:00Z",
            ),
        ];
        let r = reconcile(&db, input, NOW).await.unwrap();
        assert_eq!((r.added, r.updated, r.removed), (2, 0, 0));
        assert_eq!(r.new_repo_ids.len(), 2);
        let stars = db.get_stars_sorted().await.unwrap();
        assert_eq!(stars.len(), 2);
        assert!(stars.iter().all(|s| s.is_new()));
    }

    #[tokio::test]
    async fn detects_added_and_updated_without_removal() {
        let db = Db::memory().await.unwrap();
        let a = Repo::sample(1, "a", "x", Some("Rust"), &[]);
        seed_active(&db, &a, "2026-05-01T00:00:00Z").await;
        db.clear_is_new().await.unwrap();

        let a2 = Repo::sample(1, "a", "x", Some("Rust"), &["new-topic"]);
        let b = Repo::sample(2, "b", "y", Some("Go"), &[]);
        let r = reconcile(
            &db,
            vec![
                star(a2, "2026-05-01T00:00:00Z"),
                star(b, "2026-05-03T00:00:00Z"),
            ],
            NOW,
        )
        .await
        .unwrap();
        assert_eq!((r.added, r.updated, r.removed), (1, 1, 0));
        assert_eq!(r.new_repo_ids, vec![2]);
        // A's metadata was refreshed.
        assert_eq!(
            db.get_repo(1).await.unwrap().unwrap().topics,
            vec!["new-topic"]
        );
    }

    #[tokio::test]
    async fn detects_removed_star() {
        let db = Db::memory().await.unwrap();
        let a = Repo::sample(1, "a", "x", None, &[]);
        let b = Repo::sample(2, "b", "y", None, &[]);
        seed_active(&db, &a, "t1").await;
        seed_active(&db, &b, "t2").await;
        db.clear_is_new().await.unwrap();

        let r = reconcile(&db, vec![star(a, "t1")], NOW).await.unwrap();
        assert_eq!((r.added, r.updated, r.removed), (0, 1, 1));
        assert_eq!(db.get_active_star_ids().await.unwrap(), vec![1]);
    }

    #[tokio::test]
    async fn re_star_counts_as_added_again() {
        let db = Db::memory().await.unwrap();
        let b = Repo::sample(2, "b", "y", None, &[]);
        seed_active(&db, &b, "t1").await;
        db.mark_unstarred(2, NOW).await.unwrap();
        db.clear_is_new().await.unwrap();

        let r = reconcile(&db, vec![star(b, "t2")], NOW).await.unwrap();
        assert_eq!((r.added, r.removed), (1, 0));
        assert_eq!(db.get_active_star_ids().await.unwrap(), vec![2]);
    }
}
