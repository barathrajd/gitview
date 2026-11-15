//! History as parsed commits: hashes, parents, refs, author, time, and
//! subject from NUL-delimited `git log`, loaded a page at a time.

use std::path::Path;

use crate::process;

/// Fields per commit in the `git log` output.
const FIELDS: usize = 6;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    /// The full object ID.
    pub hash: String,
    /// Full parent IDs, first parent first.
    pub parents: Vec<String>,
    /// Decorations as Git prints them, such as `HEAD -> main` or `tag: v1`.
    pub refs: Vec<String>,
    pub author: String,
    /// Author time, in seconds since the Unix epoch.
    pub time: i64,
    pub subject: String,
}

impl Commit {
    /// The abbreviated ID shown in History.
    pub fn short_hash(&self) -> &str {
        &self.hash[..self.hash.len().min(7)]
    }
}

/// One page of history, newest first.
#[derive(Debug, Default)]
pub struct Page {
    pub commits: Vec<Commit>,
    /// More commits follow this page.
    pub more: bool,
}

/// Load `count` commits from every ref after skipping `skip`, children
/// before their parents, optionally limited to those touching `path`. An
/// empty repository has no commits.
pub fn load_page(
    root: &Path,
    path: Option<&Path>,
    skip: usize,
    count: usize,
) -> Result<Page, String> {
    crate::history::check_history_path(path)?;
    let mut command = process::git_command();
    command
        .current_dir(root)
        .arg("--literal-pathspecs")
        .args([
            "log",
            "--all",
            "--topo-order",
            // Rewrite parents to the commits a path filter keeps, so the
            // graph's lanes end at commits that are listed.
            "--parents",
            "-z",
            "--decorate=short",
            "--format=%H%x00%P%x00%D%x00%an%x00%at%x00%s",
            "--color=never",
            "--no-show-signature",
        ])
        .arg(format!("--skip={skip}"))
        // One extra commit tells whether another page follows.
        .arg(format!("--max-count={}", count.saturating_add(1)));
    if let Some(path) = path {
        command.arg("--").arg(path);
    }
    let output = command
        .output()
        .map_err(|error| format!("Unable to load history: {error}"))?;
    if !output.status.success() {
        let head = process::git_command()
            .current_dir(root)
            .args(["rev-parse", "--verify", "--quiet", "HEAD"])
            .output()
            .map_err(|error| format!("Unable to check HEAD: {error}"))?;
        if !head.status.success() {
            return Ok(Page::default());
        }
        return Err(format!(
            "Unable to load history: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let mut commits = parse(&output.stdout);
    let more = commits.len() > count;
    commits.truncate(count);
    Ok(Page { commits, more })
}

/// Parse `git log -z` output in the six-field format above. `-z` ends each
/// commit with a NUL, so the stream is one NUL-separated field list.
fn parse(output: &[u8]) -> Vec<Commit> {
    let text = String::from_utf8_lossy(output);
    let fields: Vec<&str> = text.split('\0').collect();
    fields
        .as_chunks::<FIELDS>()
        .0
        .iter()
        .map(|field| Commit {
            // `-z` may leave a newline before the next commit's hash.
            hash: field[0].trim().to_owned(),
            parents: field[1].split_whitespace().map(str::to_owned).collect(),
            refs: field[2]
                .split(", ")
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .collect(),
            author: field[3].to_owned(),
            time: field[4].trim().parse().unwrap_or_default(),
            subject: field[5].to_owned(),
        })
        .collect()
}
