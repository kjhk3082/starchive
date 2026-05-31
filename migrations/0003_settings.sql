-- Local key/value settings (e.g. the LLM provider/key/model entered via the
-- dashboard). Stored in the git-ignored SQLite DB, never committed.

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
