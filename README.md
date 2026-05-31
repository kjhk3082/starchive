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

| Your Stars | Trending + For You |
|---|---|
| ![Stars dashboard](docs/screenshots/stars.png) | ![Trending](docs/screenshots/trending.png) |

## Why

- **Your stars are a knowledge base.** starchive turns them into versioned markdown that
  you (or your coding agent) can grep, read, and reason over offline.
- **Discovery that knows you.** The "For You" feed scores GitHub's trending repos against
  the languages and topics you actually star — with a one-line reason for each pick.
- **Zero infrastructure.** One binary, one SQLite file. No server, no account, no tracking.
  It reuses your existing `gh` login, so setup is nothing.

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
| `RUST_LOG` | `starchive=info` | Log level |

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

The DB is the source of truth; markdown is a regenerable artifact. Both the database and
archive directory are local user data and are git-ignored in this repository.

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
