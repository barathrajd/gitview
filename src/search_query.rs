//! Parses a search query into Git-side filters and free text.
//!
//! `author:`, `path:`, `msg:`, `since:`, `until:`, and `ref:` become
//! `git log` options. Values may be double-quoted. A filter repeated is OR
//! (Git's own behavior for `--author` and `--grep`); different filters are
//! AND. Anything else stays free text for the fuzzy matcher.

use std::ffi::OsString;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchQuery {
    pub text: String,
    pub authors: Vec<String>,
    pub paths: Vec<String>,
    pub messages: Vec<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub refs: Vec<String>,
}

impl SearchQuery {
    pub fn is_filtered(&self) -> bool {
        !(self.authors.is_empty()
            && self.paths.is_empty()
            && self.messages.is_empty()
            && self.since.is_none()
            && self.until.is_none()
            && self.refs.is_empty())
    }

    /// Arguments for `git`, fetching at most `limit` commits in the same row
    /// format as History.
    pub fn git_log_args(&self, limit: usize) -> Vec<OsString> {
        let mut args: Vec<OsString> = [
            "--literal-pathspecs",
            "log",
            "--format=* %h %d %s - %an%x1f%H",
            "--decorate=short",
            "--color=never",
            "--regexp-ignore-case",
            "--fixed-strings",
            "-n",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        args.push(limit.to_string().into());
        let option = |name: &str, value: &str| OsString::from(format!("--{name}={value}"));
        args.extend(self.authors.iter().map(|value| option("author", value)));
        args.extend(self.messages.iter().map(|value| option("grep", value)));
        args.extend(self.since.iter().map(|value| option("since", value)));
        args.extend(self.until.iter().map(|value| option("until", value)));
        if self.refs.is_empty() {
            args.push("--all".into());
        } else {
            args.push("--end-of-options".into());
            args.extend(self.refs.iter().map(OsString::from));
        }
        args.push("--".into());
        args.extend(self.paths.iter().map(OsString::from));
        args
    }
}

pub fn parse_query(query: &str) -> SearchQuery {
    let mut parsed = SearchQuery::default();
    let mut text = Vec::new();
    let mut rest = query.trim_start();
    while !rest.is_empty() {
        let token_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let token = &rest[..token_end];
        let filter = token.split_once(':').and_then(|(key, value)| {
            let key = key.to_ascii_lowercase();
            matches!(
                key.as_str(),
                "author" | "path" | "msg" | "since" | "until" | "ref"
            )
            .then_some((key, value))
        });
        let mut consumed = token_end;
        let value = filter.as_ref().map(|(key, value)| {
            if value.starts_with('"') {
                let start = key.len() + 2;
                let close = rest[start..].find('"');
                consumed = close.map_or(rest.len(), |close| start + close + 1);
                rest[start..close.map_or(rest.len(), |close| start + close)].to_owned()
            } else {
                (*value).to_owned()
            }
        });
        match (filter, value) {
            (Some((key, _)), Some(value)) if !value.is_empty() => match key.as_str() {
                "author" => parsed.authors.push(value),
                "path" => parsed.paths.push(value),
                "msg" => parsed.messages.push(value),
                "since" => parsed.since = Some(value),
                "until" => parsed.until = Some(value),
                _ => parsed.refs.push(value),
            },
            _ => text.push(&rest[..consumed]),
        }
        rest = rest[consumed..].trim_start();
    }
    parsed.text = text.join(" ");
    parsed
}

/// Commits found by a filtered search, in the History row format.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filtered {
    pub rows: Vec<String>,
    /// More than the requested limit matched; only the first ones are kept.
    pub truncated: bool,
}

/// Runs the query's filters through `git log`, keeping at most `limit`
/// commits. Blocking: call it off the render loop.
pub fn run_filtered(
    root: &std::path::Path,
    query: &SearchQuery,
    limit: usize,
) -> Result<Filtered, String> {
    let output = crate::process::git_command()
        .current_dir(root)
        .args(query.git_log_args(limit + 1))
        .output()
        .map_err(|error| format!("Unable to search history: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Unable to search history: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let mut rows: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    let truncated = rows.len() > limit;
    rows.truncate(limit);
    Ok(Filtered { rows, truncated })
}
