-- starchive schema. DB is the source of truth; markdown is a generated artifact.

CREATE TABLE IF NOT EXISTS repos (
    id                 INTEGER PRIMARY KEY,          -- GitHub repo id
    owner              TEXT    NOT NULL,
    name               TEXT    NOT NULL,
    full_name          TEXT    NOT NULL,
    description        TEXT,
    html_url           TEXT    NOT NULL,
    homepage           TEXT,
    language           TEXT,
    stargazers_count   INTEGER NOT NULL DEFAULT 0,
    forks_count        INTEGER NOT NULL DEFAULT 0,
    open_issues_count  INTEGER NOT NULL DEFAULT 0,
    topics_json        TEXT    NOT NULL DEFAULT '[]',
    license            TEXT,
    default_branch     TEXT,
    pushed_at          TEXT,
    created_at         TEXT,
    updated_at         TEXT
);

CREATE TABLE IF NOT EXISTS stars (
    repo_id            INTEGER PRIMARY KEY REFERENCES repos(id),
    starred_at         TEXT,                          -- when the user starred it
    first_seen_at      TEXT    NOT NULL,              -- when starchive first saw it
    is_new             INTEGER NOT NULL DEFAULT 0,    -- float-to-top flag after refresh
    unstarred_at       TEXT,                          -- soft delete when star removed
    archived_path      TEXT,
    archive_synced_at  TEXT
);

CREATE TABLE IF NOT EXISTS sync_runs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at  TEXT    NOT NULL,
    finished_at TEXT,
    added       INTEGER NOT NULL DEFAULT 0,
    updated     INTEGER NOT NULL DEFAULT 0,
    removed     INTEGER NOT NULL DEFAULT 0,
    source      TEXT    NOT NULL DEFAULT 'manual'
);

CREATE TABLE IF NOT EXISTS trending_cache (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    fetched_at  TEXT NOT NULL,
    params_json TEXT NOT NULL,
    repos_json  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_stars_active ON stars(unstarred_at);
