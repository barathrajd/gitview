use std::{path::Path, process::Command};

pub fn load_history(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "log",
            "--all",
            "--graph",
            "--decorate=short",
            "--format=%h %d %s - %an%x1f%H",
            "--color=never",
            "-n",
            "200",
        ])
        .output()
        .map_err(|error| format!("Unable to load history: {error}"))?;
    if !output.status.success() {
        let empty = Command::new("git")
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
        let remaining = Command::new("git")
            .current_dir(root)
            .args(["rev-list", "--all", "--skip=200", "--max-count=1"])
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

/// Find the next matching commit row after the selected row, wrapping once.
pub fn find_next(rows: &[String], query: &str, selected: usize) -> Option<usize> {
    if query.is_empty() || rows.is_empty() {
        return None;
    }
    let query = query.to_lowercase();
    (1..=rows.len())
        .map(|offset| (selected + offset) % rows.len())
        .find(|&index| {
            commit_id(&rows[index]).is_some() && rows[index].to_lowercase().contains(&query)
        })
}

/// Return visible commit rows matching the search query, preserving history order.
pub fn search_matches(rows: &[String], query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    rows.iter()
        .enumerate()
        .filter_map(|(index, row)| {
            (commit_id(row).is_some() && row.to_lowercase().contains(&query)).then_some(index)
        })
        .collect()
}

/// Hide the full object ID used for searching from the visible graph row.
pub fn display_row(row: &str) -> &str {
    row.rsplit_once('\u{1f}')
        .map_or(row, |(visible, _)| visible)
}
