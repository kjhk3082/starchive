//! SQLite persistence (the source of truth). Uses sqlx **runtime** queries so
//! the build needs no `DATABASE_URL` and no sqlx-cli. Timestamps are RFC3339
//! TEXT. Repos read back from the DB are reconstructed into [`github::Repo`] so
//! `archive`/`recommend` consume one uniform type.

use std::path::Path;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use crate::error::Result;
use crate::github::models::{License, Owner, Repo};
use crate::sync::SyncReport;

/// One row of the stars-with-repo join, shaped for dashboard display.
#[derive(Debug, sqlx::FromRow)]
pub struct StarView {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub language: Option<String>,
    pub stargazers_count: i64,
    pub topics_json: String,
    pub is_new: i64,
    pub archived_path: Option<String>,
}

impl StarView {
    pub fn topics(&self) -> Vec<String> {
        serde_json::from_str(&self.topics_json).unwrap_or_default()
    }
    pub fn is_new(&self) -> bool {
        self.is_new != 0
    }
}

/// Flat row mirroring the `repos` table; converted into a rich [`Repo`].
#[derive(Debug, sqlx::FromRow)]
struct RepoRow {
    id: i64,
    owner: String,
    name: String,
    full_name: String,
    description: Option<String>,
    html_url: String,
    homepage: Option<String>,
    language: Option<String>,
    stargazers_count: i64,
    forks_count: i64,
    open_issues_count: i64,
    topics_json: String,
    license: Option<String>,
    default_branch: Option<String>,
    pushed_at: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

impl RepoRow {
    fn into_repo(self) -> Repo {
        Repo {
            id: self.id,
            name: self.name,
            full_name: self.full_name,
            owner: Owner { login: self.owner },
            html_url: self.html_url,
            description: self.description,
            homepage: self.homepage,
            language: self.language,
            stargazers_count: self.stargazers_count,
            forks_count: self.forks_count,
            open_issues_count: self.open_issues_count,
            topics: serde_json::from_str(&self.topics_json).unwrap_or_default(),
            license: self.license.map(|s| License { spdx_id: Some(s) }),
            default_branch: self.default_branch,
            pushed_at: self.pushed_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    /// Open (creating if needed) a file-backed database with WAL enabled, then
    /// run migrations.
    pub async fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new().connect_with(opts).await?;
        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Insert or update repo metadata.
    pub async fn upsert_repo(&self, repo: &Repo) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO repos
                (id, owner, name, full_name, description, html_url, homepage,
                 language, stargazers_count, forks_count, open_issues_count,
                 topics_json, license, default_branch, pushed_at, created_at, updated_at)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
               ON CONFLICT(id) DO UPDATE SET
                 owner=excluded.owner, name=excluded.name, full_name=excluded.full_name,
                 description=excluded.description, html_url=excluded.html_url,
                 homepage=excluded.homepage, language=excluded.language,
                 stargazers_count=excluded.stargazers_count, forks_count=excluded.forks_count,
                 open_issues_count=excluded.open_issues_count, topics_json=excluded.topics_json,
                 license=excluded.license, default_branch=excluded.default_branch,
                 pushed_at=excluded.pushed_at, created_at=excluded.created_at,
                 updated_at=excluded.updated_at"#,
        )
        .bind(repo.id)
        .bind(repo.owner())
        .bind(&repo.name)
        .bind(&repo.full_name)
        .bind(&repo.description)
        .bind(&repo.html_url)
        .bind(repo.homepage())
        .bind(&repo.language)
        .bind(repo.stargazers_count)
        .bind(repo.forks_count)
        .bind(repo.open_issues_count)
        .bind(repo.topics_json())
        .bind(repo.license_spdx())
        .bind(&repo.default_branch)
        .bind(&repo.pushed_at)
        .bind(&repo.created_at)
        .bind(&repo.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// repo_ids of stars that are currently active (not unstarred).
    pub async fn get_active_star_ids(&self) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT repo_id FROM stars WHERE unstarred_at IS NULL")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Record a (new or re-activated) star. Marks `is_new=1` and clears any
    /// previous soft-delete, preserving the original `first_seen_at`.
    pub async fn add_star(&self, repo_id: i64, starred_at: &str, now: &str) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO stars (repo_id, starred_at, first_seen_at, is_new, unstarred_at)
               VALUES (?1, ?2, ?3, 1, NULL)
               ON CONFLICT(repo_id) DO UPDATE SET
                 starred_at = excluded.starred_at,
                 unstarred_at = NULL,
                 is_new = 1"#,
        )
        .bind(repo_id)
        .bind(starred_at)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Soft-delete a star that no longer appears in the user's GitHub stars.
    pub async fn mark_unstarred(&self, repo_id: i64, now: &str) -> Result<()> {
        sqlx::query("UPDATE stars SET unstarred_at = ?2 WHERE repo_id = ?1")
            .bind(repo_id)
            .bind(now)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Record where a repo's markdown archive was written.
    pub async fn set_archived(&self, repo_id: i64, path: &str, now: &str) -> Result<()> {
        sqlx::query(
            "UPDATE stars SET archived_path = ?2, archive_synced_at = ?3 WHERE repo_id = ?1",
        )
        .bind(repo_id)
        .bind(path)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Clear the float-to-top highlight (called after the user has seen them).
    pub async fn clear_is_new(&self) -> Result<()> {
        sqlx::query("UPDATE stars SET is_new = 0 WHERE is_new = 1")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn record_sync(
        &self,
        report: &SyncReport,
        started_at: &str,
        finished_at: &str,
        source: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO sync_runs (started_at, finished_at, added, updated, removed, source)
             VALUES (?1,?2,?3,?4,?5,?6)",
        )
        .bind(started_at)
        .bind(finished_at)
        .bind(report.added as i64)
        .bind(report.updated as i64)
        .bind(report.removed as i64)
        .bind(source)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Active stars for the dashboard: new ones first, then newest-starred.
    pub async fn get_stars_sorted(&self) -> Result<Vec<StarView>> {
        let rows = sqlx::query_as::<_, StarView>(
            r#"SELECT r.owner, r.name, r.full_name, r.description, r.html_url,
                      r.language, r.stargazers_count, r.topics_json,
                      s.starred_at, s.is_new, s.archived_path
               FROM stars s JOIN repos r ON r.id = s.repo_id
               WHERE s.unstarred_at IS NULL
               ORDER BY s.is_new DESC, s.starred_at DESC"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// All currently-starred repos (for archive regeneration and profiling).
    pub async fn get_active_repos(&self) -> Result<Vec<Repo>> {
        let rows = sqlx::query_as::<_, RepoRow>(
            "SELECT r.* FROM repos r JOIN stars s ON s.repo_id = r.id
             WHERE s.unstarred_at IS NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(RepoRow::into_repo).collect())
    }

    pub async fn get_repo(&self, id: i64) -> Result<Option<Repo>> {
        let row = sqlx::query_as::<_, RepoRow>("SELECT * FROM repos WHERE id = ?1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(RepoRow::into_repo))
    }

    #[cfg(test)]
    pub async fn memory() -> Result<Self> {
        // A single shared connection so the in-memory DB persists across queries.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(":memory:"))
            .await?;
        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-06-01T00:00:00Z";

    #[tokio::test]
    async fn upsert_and_read_back_round_trips() {
        let db = Db::memory().await.unwrap();
        let repo = Repo::sample(1, "BurntSushi", "ripgrep", Some("Rust"), &["cli", "search"]);
        db.upsert_repo(&repo).await.unwrap();

        let got = db.get_repo(1).await.unwrap().unwrap();
        assert_eq!(got.owner(), "BurntSushi");
        assert_eq!(got.name, "ripgrep");
        assert_eq!(got.topics, vec!["cli", "search"]);
        assert_eq!(got.license_spdx(), Some("MIT"));
    }

    #[tokio::test]
    async fn add_star_makes_it_active_and_unstar_removes_it() {
        let db = Db::memory().await.unwrap();
        let repo = Repo::sample(2, "owner", "tool", Some("Go"), &[]);
        db.upsert_repo(&repo).await.unwrap();
        db.add_star(2, "2026-05-30T00:00:00Z", NOW).await.unwrap();

        assert_eq!(db.get_active_star_ids().await.unwrap(), vec![2]);

        db.mark_unstarred(2, NOW).await.unwrap();
        assert!(db.get_active_star_ids().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn re_star_clears_soft_delete() {
        let db = Db::memory().await.unwrap();
        let repo = Repo::sample(3, "o", "r", None, &[]);
        db.upsert_repo(&repo).await.unwrap();
        db.add_star(3, "t1", NOW).await.unwrap();
        db.mark_unstarred(3, NOW).await.unwrap();
        assert!(db.get_active_star_ids().await.unwrap().is_empty());

        db.add_star(3, "t2", NOW).await.unwrap(); // re-star
        assert_eq!(db.get_active_star_ids().await.unwrap(), vec![3]);
    }

    #[tokio::test]
    async fn stars_sorted_puts_new_first_then_recent() {
        let db = Db::memory().await.unwrap();
        for (id, owner) in [(10, "a"), (11, "b"), (12, "c")] {
            db.upsert_repo(&Repo::sample(id, owner, "r", Some("Rust"), &[]))
                .await
                .unwrap();
        }
        // 10 starred earliest, 12 latest; then mark 11 as freshly-new.
        db.add_star(10, "2026-01-01T00:00:00Z", NOW).await.unwrap();
        db.add_star(12, "2026-03-01T00:00:00Z", NOW).await.unwrap();
        db.clear_is_new().await.unwrap(); // 10 and 12 are now "seen"
        db.add_star(11, "2026-02-01T00:00:00Z", NOW).await.unwrap(); // is_new=1

        let stars = db.get_stars_sorted().await.unwrap();
        let order: Vec<&str> = stars.iter().map(|s| s.owner.as_str()).collect();
        // b (is_new) first; then c, a by starred_at desc.
        assert_eq!(order, vec!["b", "c", "a"]);
        assert!(stars[0].is_new());
    }
}
