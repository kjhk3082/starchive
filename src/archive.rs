//! Markdown archive generation. The renderer is a pure function (repo + README
//! text → markdown string) so it is fully unit-tested; the file/git I/O are thin
//! shells exercised by the real sync run.
//!
//! The output is structured for machine reading: YAML frontmatter, fixed section
//! headings, and an explicit best-effort install command derived from the
//! README or the repo's language.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::Result;
use crate::github::models::Repo;

/// Render a repo (plus optional README) into an AI-readable markdown document.
pub fn render_markdown(repo: &Repo, readme: Option<&str>, archived_at: &str) -> String {
    let lang = repo.language.as_deref().unwrap_or("—");
    let license = repo.license_spdx().unwrap_or("—");
    let desc = repo.description.as_deref().unwrap_or("").trim();
    let topics = repo.topics.join(", ");
    let install_section = readme.and_then(|r| extract_section(r, &["install", "getting started", "setup"]));
    let install_cmd = guess_install(repo, readme);

    let mut out = String::new();

    // Frontmatter.
    out.push_str("---\n");
    out.push_str(&format!("repo: {}\n", repo.full_name));
    out.push_str(&format!("url: {}\n", repo.html_url));
    out.push_str(&format!("language: {lang}\n"));
    out.push_str(&format!("stars: {}\n", repo.stargazers_count));
    out.push_str(&format!("topics: [{}]\n", repo.topics.join(", ")));
    out.push_str(&format!("license: {license}\n"));
    out.push_str(&format!("archived_at: {archived_at}\n"));
    out.push_str("---\n\n");

    // Heading + description.
    out.push_str(&format!("# {}\n\n", repo.full_name));
    if !desc.is_empty() {
        out.push_str(&format!("> {desc}\n\n"));
    }

    // Facts.
    out.push_str(&format!(
        "**Stars** {} · **Forks** {} · **Language** {lang} · **License** {license}\n\n",
        repo.stargazers_count, repo.forks_count
    ));
    if !topics.is_empty() {
        out.push_str(&format!("**Topics** {topics}\n\n"));
    }
    out.push_str(&format!("**Links** [GitHub]({})", repo.html_url));
    if let Some(hp) = repo.homepage() {
        out.push_str(&format!(" · [Homepage]({hp})"));
    }
    out.push_str("\n\n");

    // What it is.
    out.push_str("## What it is\n\n");
    let what = readme
        .and_then(first_paragraph)
        .or_else(|| (!desc.is_empty()).then(|| desc.to_string()))
        .unwrap_or_else(|| "No description available.".to_string());
    out.push_str(&what);
    out.push_str("\n\n");

    // Installation.
    out.push_str("## Installation\n\n");
    if let Some(sec) = &install_section {
        out.push_str(sec);
        out.push_str("\n\n");
    }
    if let Some(cmd) = &install_cmd {
        out.push_str(&format!("```sh\n{cmd}\n```\n\n"));
    }
    if install_section.is_none() && install_cmd.is_none() {
        out.push_str("_See the repository README for installation instructions._\n\n");
    }

    // Usage (only if present).
    if let Some(sec) = readme.and_then(|r| extract_section(r, &["usage", "example", "quick start", "quickstart"])) {
        out.push_str("## Usage\n\n");
        out.push_str(&sec);
        out.push_str("\n\n");
    }

    // Full README.
    out.push_str("## README (full)\n\n");
    match readme {
        Some(r) if !r.trim().is_empty() => {
            out.push_str(r.trim_end());
            out.push('\n');
        }
        _ => out.push_str("_No README available._\n"),
    }
    out.push('\n');

    // AI notes.
    out.push_str("## AI Notes\n\n");
    out.push_str(&format!("- Primary language: {lang}\n"));
    if let Some(cmd) = &install_cmd {
        out.push_str(&format!("- Install guess: `{cmd}`\n"));
    }
    out.push_str(&format!("- Source: {}\n", repo.html_url));

    out
}

/// Best-effort install command: an explicit command found in the README wins;
/// otherwise guess from the repo's primary language.
pub fn guess_install(repo: &Repo, readme: Option<&str>) -> Option<String> {
    const PREFIXES: &[&str] = &[
        "cargo install",
        "cargo binstall",
        "npm install",
        "npm i ",
        "pnpm add",
        "yarn add",
        "pip install",
        "pipx install",
        "uv add",
        "uv tool install",
        "go install",
        "gem install",
        "brew install",
        "docker pull",
    ];
    if let Some(rd) = readme {
        for line in rd.lines() {
            let l = line.trim().trim_start_matches('$').trim();
            if let Some(p) = PREFIXES.iter().find(|p| l.starts_with(**p)) {
                // Keep it to a single command (strip trailing comments).
                let cmd = l.split(" #").next().unwrap_or(l).trim();
                let _ = p;
                return Some(cmd.to_string());
            }
        }
    }
    let name = &repo.name;
    match repo.language.as_deref() {
        Some("Rust") => Some(format!("cargo install {name}")),
        Some("JavaScript") | Some("TypeScript") => Some(format!("npm install {name}")),
        Some("Python") => Some(format!("pip install {name}")),
        Some("Go") => Some(format!("go install github.com/{}@latest", repo.full_name)),
        Some("Ruby") => Some(format!("gem install {name}")),
        _ => None,
    }
}

/// Extract the body of the first markdown section whose heading contains any of
/// `keywords` (case-insensitive), up to the next same-or-higher-level heading.
pub fn extract_section(readme: &str, keywords: &[&str]) -> Option<String> {
    let lines: Vec<&str> = readme.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let Some(level) = heading_level(line) else { continue };
        let title = line.trim_start_matches('#').trim().to_lowercase();
        if !keywords.iter().any(|k| title.contains(k)) {
            continue;
        }
        let mut body = Vec::new();
        for next in &lines[i + 1..] {
            if let Some(l2) = heading_level(next) {
                if l2 <= level {
                    break;
                }
            }
            body.push(*next);
        }
        let text = body.join("\n").trim().to_string();
        if !text.is_empty() {
            return Some(text);
        }
    }
    None
}

fn heading_level(line: &str) -> Option<usize> {
    let t = line.trim_start();
    if !t.starts_with('#') {
        return None;
    }
    let hashes = t.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && t.chars().nth(hashes) == Some(' ') {
        Some(hashes)
    } else {
        None
    }
}

/// First prose paragraph of a README (skips headings, badges, and raw HTML).
fn first_paragraph(readme: &str) -> Option<String> {
    let mut para: Vec<&str> = Vec::new();
    for line in readme.lines() {
        let t = line.trim();
        if t.is_empty() {
            if !para.is_empty() {
                break;
            }
            continue;
        }
        if heading_level(line).is_some() || t.starts_with("![") || t.starts_with("[![") || t.starts_with('<') {
            if !para.is_empty() {
                break;
            }
            continue;
        }
        para.push(t);
    }
    (!para.is_empty()).then(|| para.join(" "))
}

/// Write `md` to `{dir}/{owner}/{name}.md`, returning the path written.
pub async fn write_archive(dir: &Path, repo: &Repo, md: &str) -> Result<PathBuf> {
    let repo_dir = dir.join(repo.owner());
    tokio::fs::create_dir_all(&repo_dir).await?;
    let path = repo_dir.join(format!("{}.md", repo.name));
    tokio::fs::write(&path, md).await?;
    Ok(path)
}

/// Regenerate `{dir}/INDEX.md` — a stars-sorted table of contents.
pub async fn write_index(dir: &Path, repos: &[Repo]) -> Result<()> {
    let mut sorted: Vec<&Repo> = repos.iter().collect();
    sorted.sort_by(|a, b| b.stargazers_count.cmp(&a.stargazers_count));

    let mut s = String::from("# starchive — Starred Repository Index\n\n");
    s.push_str(&format!("{} repositories archived.\n\n", repos.len()));
    s.push_str("| Repository | Language | Stars | Description |\n|---|---|---|---|\n");
    for r in sorted {
        let lang = r.language.as_deref().unwrap_or("—");
        let desc: String = r
            .description
            .as_deref()
            .unwrap_or("")
            .replace('|', "\\|")
            .chars()
            .take(80)
            .collect();
        s.push_str(&format!(
            "| [{}]({}/{}.md) | {} | {} | {} |\n",
            r.full_name,
            r.owner(),
            r.name,
            lang,
            r.stargazers_count,
            desc
        ));
    }
    tokio::fs::create_dir_all(dir).await?;
    tokio::fs::write(dir.join("INDEX.md"), s).await?;
    Ok(())
}

/// Best-effort commit of the archive directory (opt-in via `--git`). Only stages
/// paths under `dir`, so it's safe even when the archive lives in a larger repo.
/// Returns whether a commit was created.
pub fn git_commit(dir: &Path, msg: &str) -> Result<bool> {
    let add = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["add", "."])
        .status()?;
    if !add.success() {
        return Ok(false);
    }
    let commit = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["commit", "-q", "-m", msg, "--", "."])
        .output()?;
    Ok(commit.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::models::Repo;

    const AT: &str = "2026-06-01T00:00:00Z";

    #[test]
    fn renders_frontmatter_sections_and_install_guess() {
        let repo = Repo::sample(1, "BurntSushi", "ripgrep", Some("Rust"), &["cli", "search"]);
        let readme = "# ripgrep\n\nripgrep recursively searches directories.\n\n## Installation\n\nRun the installer.\n\n## Usage\n\nrg pattern path\n";
        let md = render_markdown(&repo, Some(readme), AT);

        assert!(md.starts_with("---\n"));
        assert!(md.contains("repo: BurntSushi/ripgrep"));
        assert!(md.contains("stars: 100"));
        assert!(md.contains("topics: [cli, search]"));
        assert!(md.contains("## What it is"));
        assert!(md.contains("ripgrep recursively searches directories."));
        assert!(md.contains("## Installation"));
        assert!(md.contains("Run the installer."));
        assert!(md.contains("cargo install ripgrep")); // language guess
        assert!(md.contains("## Usage"));
        assert!(md.contains("rg pattern path"));
        assert!(md.contains("## README (full)"));
        assert!(md.contains("## AI Notes"));
    }

    #[test]
    fn handles_missing_readme() {
        let repo = Repo::sample(2, "o", "lib", Some("Python"), &[]);
        let md = render_markdown(&repo, None, AT);
        assert!(md.contains("_No README available._"));
        assert!(md.contains("pip install lib"));
    }

    #[test]
    fn guess_install_prefers_explicit_readme_command() {
        let repo = Repo::sample(3, "o", "tool", Some("Rust"), &[]);
        let readme = "## Install\n\n```\nbrew install tool\n```\n";
        assert_eq!(
            guess_install(&repo, Some(readme)).as_deref(),
            Some("brew install tool")
        );
    }

    #[test]
    fn guess_install_language_fallback_and_unknown() {
        let go = Repo::sample(4, "owner", "cli", Some("Go"), &[]);
        assert_eq!(
            guess_install(&go, None).as_deref(),
            Some("go install github.com/owner/cli@latest")
        );
        let unknown = Repo::sample(5, "o", "x", Some("Haskell"), &[]);
        assert_eq!(guess_install(&unknown, None), None);
    }

    #[test]
    fn extract_section_stops_at_next_heading() {
        let readme = "# Title\n\n## Installation\n\nstep one\nstep two\n\n## Usage\n\nuse it\n";
        let sec = extract_section(readme, &["install"]).unwrap();
        assert!(sec.contains("step one") && sec.contains("step two"));
        assert!(!sec.contains("use it"));
    }
}
