//! Orchestration shared by the CLI (`sync`/`archive`) and the web dashboard
//! (`POST /stars/refresh`). Keeping it here guarantees both entry points run the
//! exact same fetch → reconcile → archive pipeline.

use std::path::Path;

use chrono::Utc;

use crate::archive;
use crate::db::Db;
use crate::error::Result;
use crate::github::GithubClient;
use crate::sync::{self, SyncReport};

/// Fetch stars, reconcile against the DB, archive newly-starred repos, refresh
/// the index, and log the run. Returns the diff report.
pub async fn run_sync(
    client: &GithubClient,
    db: &Db,
    archive_dir: &Path,
    source: &str,
    do_git: bool,
) -> Result<SyncReport> {
    let started = Utc::now().to_rfc3339();
    let starred = client.fetch_starred().await?;
    let report = sync::reconcile(db, starred, &started).await?;

    // Archive each newly-starred repo (README fetch failures are non-fatal).
    for id in &report.new_repo_ids {
        if let Some(repo) = db.get_repo(*id).await? {
            let readme = client.fetch_readme(repo.owner(), &repo.name).await.unwrap_or(None);
            let md = archive::render_markdown(&repo, readme.as_deref(), &Utc::now().to_rfc3339());
            let path = archive::write_archive(archive_dir, &repo, &md).await?;
            db.set_archived(*id, &path.to_string_lossy(), &Utc::now().to_rfc3339())
                .await?;
        }
    }

    // Regenerate the index from all currently-active stars.
    let active = db.get_active_repos().await?;
    archive::write_index(archive_dir, &active).await?;

    let finished = Utc::now().to_rfc3339();
    db.record_sync(&report, &started, &finished, source).await?;

    if do_git {
        let _ = archive::git_commit(archive_dir, &format!("starchive: sync ({})", report.summary()));
    }

    Ok(report)
}

/// (Re)generate markdown for every currently-starred repo. Returns the count.
pub async fn run_archive_all(client: &GithubClient, db: &Db, archive_dir: &Path) -> Result<usize> {
    let active = db.get_active_repos().await?;
    for repo in &active {
        let readme = client.fetch_readme(repo.owner(), &repo.name).await.unwrap_or(None);
        let md = archive::render_markdown(repo, readme.as_deref(), &Utc::now().to_rfc3339());
        let path = archive::write_archive(archive_dir, repo, &md).await?;
        db.set_archived(repo.id, &path.to_string_lossy(), &Utc::now().to_rfc3339())
            .await?;
    }
    archive::write_index(archive_dir, &active).await?;
    Ok(active.len())
}
