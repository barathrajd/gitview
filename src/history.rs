use std::path::Path;

use crate::process;

pub fn load_history(root: &Path) -> Result<Vec<String>, String> {
    load_history_for_path(root, None)
}

/// Reject a history path that is empty, absolute, or leaves the repository.
pub fn check_history_path(path: Option<&Path>) -> Result<(), String> {
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
    Ok(())
}

pub fn load_history_for_path(root: &Path, path: Option<&Path>) -> Result<Vec<String>, String> {
    check_history_path(path)?;
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

/// The full commit hash after the row's unit separator.
pub fn full_hash(row: &str) -> Option<&str> {
    row.split_once('\x1f').map(|(_, hash)| hash.trim())
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

/// Commit rows matching the query, in history order.
pub fn matches_in_order(rows: &[String], query: &str) -> Vec<usize> {
    let mut matches = search_matches(rows, query);
    matches.sort_unstable();
    matches
}

/// The matching commit row after `selected` (before it when `forward` is
/// false), in history order and wrapping around. None without a query or a
/// match.
pub fn next_match(
    rows: &[String],
    query: &str,
    selected: Option<usize>,
    forward: bool,
) -> Option<usize> {
    if query.trim().is_empty() {
        return None;
    }
    step_match(&matches_in_order(rows, query), selected, forward)
}

/// The match after (or before) the selection among `matches`, which are row
/// indices in history order, wrapping around.
pub fn step_match(matches: &[usize], selected: Option<usize>, forward: bool) -> Option<usize> {
    let current = selected.unwrap_or(0);
    if forward {
        matches
            .iter()
            .find(|&&row| row > current)
            .or_else(|| matches.first())
    } else {
        matches
            .iter()
            .rev()
            .find(|&&row| row < current)
            .or_else(|| matches.last())
    }
    .copied()
}

/// Byte offsets in `row` of the characters that match the query, in order.
/// Empty without a query or a match.
pub fn match_positions(row: &str, query: &str) -> Vec<usize> {
    MatchFinder::new(query).positions(row)
}

/// Finds match positions for one query across many rows, building the
/// matcher once.
pub struct MatchFinder {
    matcher: Option<frizbee::Matcher>,
}

impl MatchFinder {
    pub fn new(query: &str) -> Self {
        let config = frizbee::Config::default().casing(frizbee::CaseMatching::Ignore);
        Self {
            matcher: (!query.trim().is_empty())
                .then(|| frizbee::Matcher::from_query(query, &config)),
        }
    }

    /// Byte offsets in `row` of the matching characters, in order.
    pub fn positions(&mut self, row: &str) -> Vec<usize> {
        let Some(matcher) = &mut self.matcher else {
            return Vec::new();
        };
        let mut positions: Vec<usize> = matcher
            .match_one_indices(row, 0)
            .map(|found| {
                found
                    .indices
                    .into_iter()
                    .map(|index| index as usize)
                    .collect()
            })
            .unwrap_or_default();
        positions.sort_unstable();
        positions
    }
}

/// Hide the full object ID used for searching from the visible graph row.
pub fn display_row(row: &str) -> &str {
    row.rsplit_once('\u{1f}')
        .map_or(row, |(visible, _)| visible)
}

/// History drawn from parsed commits: each row's search text, in the
/// format the raw rows use, beside the commit and its graph cells.
#[derive(Debug, Default)]
pub struct Structured {
    /// One per row: `* <short> (<refs>) <subject> - <author>` with the full
    /// hash after a unit separator, which search, `n`/`N`, and commit
    /// lookups read.
    pub rows: Vec<String>,
    /// The commit on each row; `None` for a notice row.
    pub entries: Vec<Option<Entry>>,
    /// More commits follow the loaded ones.
    pub more: bool,
    /// The lanes after the last row, which the next page continues.
    pub layout: crate::graph::Layout,
}

/// A commit on a History row and where its fields sit in the row's text.
#[derive(Clone, Debug)]
pub struct Entry {
    pub commit: crate::commit_log::Commit,
    pub graph: crate::graph::GraphRow,
    /// Byte ranges in the row text of the short hash, each ref, the
    /// subject, and the author, for highlighting search matches.
    pub hash: std::ops::Range<usize>,
    pub refs: Vec<std::ops::Range<usize>>,
    pub subject: std::ops::Range<usize>,
    pub author: std::ops::Range<usize>,
}

/// Commits History loads at a time.
pub const HISTORY_PAGE: usize = 500;

/// Load the first page of History as parsed commits with graph lanes,
/// optionally limited to commits touching `path`.
pub fn load_structured(root: &Path, path: Option<&Path>) -> Result<Structured, String> {
    load_structured_count(root, path, HISTORY_PAGE)
}

/// Load History in the chosen view: the structured graph with at least
/// `count` commits, or with `raw` the plain `git log --graph` rows, which
/// carry no parsed commits and load no more pages.
pub fn load_view(
    root: &Path,
    path: Option<&Path>,
    raw: bool,
    count: usize,
) -> Result<Structured, String> {
    if raw {
        let rows = load_history_for_path(root, path)?;
        return Ok(Structured {
            entries: vec![None; rows.len()],
            rows,
            ..Structured::default()
        });
    }
    load_structured_count(root, path, count)
}

/// Load the newest `count` commits, at least one page, as a reload that
/// keeps as much History as was loaded.
pub fn load_structured_count(
    root: &Path,
    path: Option<&Path>,
    count: usize,
) -> Result<Structured, String> {
    load_more(
        root,
        path,
        0,
        count.max(HISTORY_PAGE),
        crate::graph::Layout::new(),
    )
}

/// Load `count` commits after the first `skip`, continuing `layout`'s
/// lanes from the rows above.
pub fn load_more(
    root: &Path,
    path: Option<&Path>,
    skip: usize,
    count: usize,
    layout: crate::graph::Layout,
) -> Result<Structured, String> {
    let page = crate::commit_log::load_page(root, path, skip, count)?;
    let mut structured = structured_with(page.commits, layout);
    structured.more = page.more;
    Ok(structured)
}

/// Rows and graph cells for `commits`, newest first.
pub fn structured_from(commits: Vec<crate::commit_log::Commit>) -> Structured {
    structured_with(commits, crate::graph::Layout::new())
}

fn structured_with(
    commits: Vec<crate::commit_log::Commit>,
    mut layout: crate::graph::Layout,
) -> Structured {
    let mut structured = Structured::default();
    for commit in commits {
        let graph = layout.push(&commit);
        let (row, entry) = row_text(commit, graph);
        structured.rows.push(row);
        structured.entries.push(Some(entry));
    }
    structured.layout = layout;
    structured
}

/// A commit's row text and the field ranges within it.
fn row_text(commit: crate::commit_log::Commit, graph: crate::graph::GraphRow) -> (String, Entry) {
    use std::fmt::Write;
    let mut row = String::from("* ");
    let span = |row: &mut String, text: &str| {
        let start = row.len();
        row.push_str(text);
        start..row.len()
    };
    let hash = span(&mut row, commit.short_hash());
    let mut refs = Vec::new();
    if !commit.refs.is_empty() {
        row.push_str(" (");
        for (index, name) in commit.refs.iter().enumerate() {
            if index > 0 {
                row.push_str(", ");
            }
            refs.push(span(&mut row, name));
        }
        row.push(')');
    }
    row.push(' ');
    let subject = span(&mut row, &commit.subject);
    row.push_str(" - ");
    let author = span(&mut row, &commit.author);
    let _ = write!(row, "\u{1f}{}", commit.hash);
    (
        row,
        Entry {
            commit,
            graph,
            hash,
            refs,
            subject,
            author,
        },
    )
}

/// A compact age such as `now`, `5m`, `2h`, `3d`, `5w`, or `1y`.
pub fn compact_age(seconds: i64, now: i64) -> String {
    let age = now.saturating_sub(seconds).max(0);
    match age {
        0..60 => "now".to_owned(),
        60..3_600 => format!("{}m", age / 60),
        3_600..86_400 => format!("{}h", age / 3_600),
        86_400..604_800 => format!("{}d", age / 86_400),
        604_800..31_536_000 => format!("{}w", age / 604_800),
        _ => format!("{}y", age / 31_536_000),
    }
}

#[cfg(test)]
mod structured_tests {
    use super::*;

    #[test]
    fn compact_ages_use_the_largest_whole_unit() {
        let now = 1_000_000_000;
        assert_eq!(compact_age(now, now), "now");
        assert_eq!(compact_age(now + 30, now), "now");
        assert_eq!(compact_age(now - 59, now), "now");
        assert_eq!(compact_age(now - 60, now), "1m");
        assert_eq!(compact_age(now - 7_200, now), "2h");
        assert_eq!(compact_age(now - 3 * 86_400, now), "3d");
        assert_eq!(compact_age(now - 35 * 86_400, now), "5w");
        assert_eq!(compact_age(now - 400 * 86_400, now), "1y");
    }

    #[test]
    fn row_text_matches_the_search_format_and_records_fields() {
        let commit = crate::commit_log::Commit {
            hash: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            parents: Vec::new(),
            refs: vec!["HEAD -> main".to_owned(), "tag: v1".to_owned()],
            author: "Zoë".to_owned(),
            time: 0,
            subject: "Fix ünïcode".to_owned(),
        };
        let graph = crate::graph::Layout::new().push(&commit);
        let (row, entry) = row_text(commit, graph);
        assert_eq!(
            row,
            "* 0123456 (HEAD -> main, tag: v1) Fix ünïcode - Zoë\u{1f}0123456789abcdef0123456789abcdef01234567"
        );
        assert_eq!(&row[entry.hash.clone()], "0123456");
        assert_eq!(&row[entry.refs[1].clone()], "tag: v1");
        assert_eq!(&row[entry.subject.clone()], "Fix ünïcode");
        assert_eq!(&row[entry.author.clone()], "Zoë");
        assert_eq!(commit_id(&row), Some("0123456"));
        assert_eq!(
            full_hash(&row),
            Some("0123456789abcdef0123456789abcdef01234567")
        );
    }
}
