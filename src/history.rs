use std::path::Path;

use crate::process;

pub fn load_history(root: &Path) -> Result<Vec<String>, String> {
    load_history_for_path(root, None)
}

pub fn load_history_for_path(root: &Path, path: Option<&Path>) -> Result<Vec<String>, String> {
    if let Some(path) = path
        && (path.as_os_str().is_empty()
            || path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            }))
    {
        return Err("Use a repository-relative history path without parent components".into());
    }
    let mut command = process::git_command();
    command.current_dir(root).arg("--literal-pathspecs").args([
        "log",
        "--all",
        "--graph",
        "--decorate=short",
        "--format=%h %d %s - %an%x1f%H",
        "--color=never",
        "-n",
        "200",
    ]);
    if let Some(path) = path {
        command.arg("--").arg(path);
    }
    let output = command
        .output()
        .map_err(|error| format!("Unable to load history: {error}"))?;
    if !output.status.success() {
        let empty = process::git_command()
            .current_dir(root)
            .args(["rev-parse", "--verify", "HEAD"])
            .output()
            .map_err(|error| format!("Unable to check HEAD: {error}"))?;
        if !empty.status.success() {
            return Ok(Vec::new());
        }
        return Err(format!(
            "Unable to load history: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let mut rows: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    if !rows.is_empty() {
        let mut remaining_command = process::git_command();
        remaining_command
            .current_dir(root)
            .arg("--literal-pathspecs")
            .args(["rev-list", "--all", "--skip=200", "--max-count=1"]);
        if let Some(path) = path {
            remaining_command.arg("--").arg(path);
        }
        let remaining = remaining_command
            .output()
            .map_err(|error| format!("Unable to check history limit: {error}"))?;
        if !remaining.status.success() {
            return Err(format!(
                "Unable to check history limit: {}",
                String::from_utf8_lossy(&remaining.stderr).trim()
            ));
        }
        if !remaining.stdout.is_empty() {
            rows.insert(0, "[history truncated to 200 commits]".to_owned());
        }
    }
    Ok(rows)
}

pub fn commit_id(row: &str) -> Option<&str> {
    let (prefix, suffix) = row.split_once('*')?;
    if !prefix
        .chars()
        .all(|c| matches!(c, ' ' | '|' | '/' | '\\' | '_' | '-'))
    {
        return None;
    }
    let id = suffix.split_whitespace().find(|token| {
        !token
            .chars()
            .all(|c| matches!(c, '|' | '/' | '\\' | '_' | '-' | '.' | '*'))
    })?;
    if (4..=64).contains(&id.len()) && id.bytes().all(|c| c.is_ascii_hexdigit()) {
        Some(id)
    } else {
        None
    }
}

/// Commit rows matching the search query, best match first. Each word must
/// match somewhere in the row, its letters in order; case is ignored, and
/// equal matches keep history order.
pub fn search_matches(rows: &[String], query: &str) -> Vec<usize> {
    let config = frizbee::Config::default().casing(frizbee::CaseMatching::Ignore);
    let mut matcher = frizbee::Matcher::from_query(query, &config);
    matcher
        .match_list(rows)
        .into_iter()
        .map(|found| found.index as usize)
        .filter(|&index| commit_id(&rows[index]).is_some())
        .collect()
}

/// Hide the full object ID used for searching from the visible graph row.
pub fn display_row(row: &str) -> &str {
    row.rsplit_once('\u{1f}')
        .map_or(row, |(visible, _)| visible)
}
