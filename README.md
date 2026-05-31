# starchive

**Track your GitHub stars, archive them as AI-readable markdown, and discover trending repos — from a single self-hosted Rust binary.**

starchive pulls every repository you've starred into a local database, writes each one
as a structured markdown file an LLM can read at a glance (description, install command,
usage, full README, metadata), and serves a dashboard that recommends *new* trending
repos ranked by your actual taste. Refresh anytime — newly starred repos float to the top
and get archived automatically.

```
GitHub stars ──▶ local SQLite ──▶ AI-readable markdown archive
                      │
                      └──▶ dashboard: your stars + trending + "For You"
```

## Screenshots

| Your Stars | Archive (license + AI summary) |
|---|---|
| ![Stars dashboard](docs/screenshots/stars.png) | ![Archive](docs/screenshots/archive.png) |

## Why

- **Your stars are a knowledge base.** starchive turns them into versioned markdown that
  you (or your coding agent) can grep, read, and reason over offline.
- **Discovery that knows you.** The "For You" feed scores GitHub's trending repos against
  the languages and topics you actually star — with a one-line reason for each pick.
- **Bring your own LLM (optional).** Add an OpenRouter, Anthropic, or OpenAI key to get
  AI repo summaries, plain-language license explanations, and project-based discovery —
  "describe your project, get the right repos." All answers follow the UI language.
- **Zero infrastructure.** One binary, one SQLite file. No server, no account, no tracking.
  It reuses your existing `gh` login, so setup is nothing. Bilingual (English / 한국어).

## Install

Requires [Rust](https://rustup.rs) 1.85+ and either the [GitHub CLI](https://cli.github.com)
(`gh auth login`) or a `GITHUB_TOKEN`.

```sh
cargo install --git https://github.com/kjhk3082/starchive
```

Or build from source:

```sh
git clone https://github.com/kjhk3082/starchive
cd starchive
cargo build --release   # binary at target/release/starchive
```

## Usage

```sh
# 1. Pull your stars into the local DB and archive them as markdown
starchive sync

# 2. Open the dashboard
starchive serve            # http://127.0.0.1:7878

# Re-generate markdown for every starred repo
starchive archive

# Commit the archive after syncing (best for a dedicated archive repo)
starchive sync --git
```

On the dashboard: browse **Trending** and **For You**, click through to GitHub, star what
you like, then hit **Refresh** — new stars are detected, archived, and pinned to the top.
The UI is bilingual (**English / 한국어** — toggle in the header; it also auto-detects your
browser language). Open any repo's `📄 archive` and click **⬇ .md** to download its
markdown file.

## Authentication

starchive resolves a GitHub token in this order:

1. `GITHUB_TOKEN` environment variable
2. `gh auth token` (the GitHub CLI)

A classic or fine-grained token with public read scope is enough (the same one `gh` uses).

## Configuration

| Variable | Default | Purpose |
|---|---|---|
| `GITHUB_TOKEN` | — | GitHub API token (falls back to `gh auth token`) |
| `STARCHIVE_DB` | `data/starchive.db` | SQLite database path |
| `STARCHIVE_ARCHIVE` | `archive/` | Markdown archive directory |
| `OPENROUTER_API_KEY` / `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` | — | Enables AI features (optional) |
| `STARCHIVE_LLM_PROVIDER` | auto-detect | `openrouter` \| `anthropic` \| `openai` |
| `STARCHIVE_LLM_MODEL` | per-provider | e.g. `qwen/qwen3.5-plus-20260420` |
| `RUST_LOG` | `starchive=info` | Log level |

Secrets are read from the environment only. Copy `.env.example` to `.env` (which is
git-ignored) and fill in one key — keys are never written to source or committed.

## AI features (optional)

Set any one provider key and starchive lights up three AI features, all answered in the
UI language (English or Korean):

- **Project discovery** — describe what you're building on the **Discover** tab; starchive
  extracts keywords, searches both your stars and GitHub, then ranks the best fits with a
  one-line reason for each.
- **Repo summaries** — each archive page generates a plain-language "what is this / when to
  use it" summary, cached per repo and language.
- **License explainer** — every repo's license is explained in plain language (this one
  needs no key — it's built in).

No key? Every non-AI feature works exactly the same; the AI bits are simply hidden.

## The markdown archive

Each starred repo becomes `archive/{owner}/{name}.md`, plus an `archive/INDEX.md` table of
contents. Files are structured for machine reading — YAML frontmatter and fixed headings:

```markdown
---
repo: BurntSushi/ripgrep
url: https://github.com/BurntSushi/ripgrep
language: Rust
stars: 45000
topics: [cli, search]
license: MIT
archived_at: 2026-06-01T00:00:00Z
---

# BurntSushi/ripgrep
> recursively search directories for a regex pattern

## What it is
…first paragraph of the README…

## Installation
…extracted install section…
```sh
cargo install ripgrep
```

## Usage
…extracted usage section…

## README (full)
…the complete README…

## AI Notes
- Primary language: Rust
- Install guess: `cargo install ripgrep`
```

All files live under the archive directory (default `./archive/`) on your machine, so the
whole catalog is just a folder you can open, grep, sync to a notes repo, or feed to an LLM.
Individual files are also downloadable from the dashboard. The DB is the source of truth;
markdown is a regenerable artifact. Both the database and archive directory are local user
data and are git-ignored in this repository.

## How "trending" works

GitHub has no official trending API, and the Search API can't sort by *star velocity*.
starchive approximates trending as **recently-created, already-popular** repositories
(`created:>recent stars:>N`) via the Search API — stable and within rate limits. The
**For You** ranking then re-scores those results against your star profile (language match
is weighted heavily, topic overlap adds up, popularity breaks ties).

## Development

```sh
cargo test                       # 19 unit tests (parsing, db, sync diff, archive, ranking)
cargo clippy --all-targets       # lint
cargo fmt                        # format
```

The codebase is split into small, single-responsibility modules: `github` (API client +
pure parsers), `db` (SQLite), `sync` (the reconcile diff), `archive` (markdown rendering),
`recommend` (ranking), and `web` (axum + maud dashboard). Network-free pure logic is
unit-tested; the thin I/O shells are exercised by real runs.

## License

MIT © 2026 kjhk3082
