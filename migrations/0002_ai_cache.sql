-- Cache for LLM-generated text (e.g. repo summaries), keyed by repo + language
-- + kind so English and Korean are stored separately and regenerated lazily.

CREATE TABLE IF NOT EXISTS ai_cache (
    repo_id    INTEGER NOT NULL,
    lang       TEXT    NOT NULL,
    kind       TEXT    NOT NULL,   -- e.g. 'summary'
    content    TEXT    NOT NULL,
    model      TEXT,
    created_at TEXT    NOT NULL,
    PRIMARY KEY (repo_id, lang, kind)
);
