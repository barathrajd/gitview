//! Read-only tools for the control channel. Every argument comes from an
//! untrusted agent, so paths must stay inside the repository, revisions can't
//! be read as options, and output is capped.

use std::path::{Component, Path};

use serde_json::{Map, Value, json};

use crate::{
    process,
    worktree::{self, Entry, Section},
};

/// The most bytes any tool returns.
pub const MAX_OUTPUT: usize = 64 * 1024;
/// The most commits `get_log` returns.
pub const MAX_LOG: usize = 200;
const DEFAULT_LOG: usize = 50;

#[derive(Debug, PartialEq, Eq)]
pub struct ToolOutput {
    pub text: String,
    pub truncated: bool,
}

/// Run the read-only tool `name` in the repository at `root`.
pub fn call(root: &Path, name: &str, args: &Value) -> Result<ToolOutput, String> {
    let args = args.as_object().ok_or("Tool arguments must be an object")?;
    match name {
        "get_status" => get_status(root),
        "get_log" => get_log(root, args),
        "get_diff" => get_diff(root, args),
        "get_show" => get_show(root, args),
        _ => Err(format!("Unknown tool: {name}")),
    }
}

fn get_status(root: &Path) -> Result<ToolOutput, String> {
    let mut entries = worktree::load_status(root)?;
    entries.sort_by_key(|entry| match entry.section {
        Section::Staged => 0,
        Section::Unstaged => 1,
        Section::Untracked => 2,
    });
    let files = entries
        .iter()
        .map(|entry| {
            let mut file = json!({
                "section": match entry.section {
                    Section::Staged => "staged",
                    Section::Unstaged => "unstaged",
                    Section::Untracked => "untracked",
                },
                "path": entry.path.to_string_lossy(),
            });
            if let Some(original) = &entry.original {
                file["original"] = json!(original.to_string_lossy());
            }
            file
        })
        .collect();
    Ok(capped_list("files", files))
}

fn get_log(root: &Path, args: &Map<String, Value>) -> Result<ToolOutput, String> {
    let limit = match args.get("limit") {
        None => DEFAULT_LOG,
        Some(value) => value
            .as_u64()
            .ok_or("limit must be a positive number")?
            .clamp(1, MAX_LOG as u64) as usize,
    };
    let output = process::git_command()
        .current_dir(root)
        .args(["log", "-z", "--no-color"])
        .arg(format!("--max-count={limit}"))
        .arg("--format=%H%x1f%an%x1f%aI%x1f%s")
        .output()
        .map_err(|error| format!("Could not run Git: {error}"))?;
    if !output.status.success() && has_commits(root) {
        return Err(git_error("Could not read history", &output.stderr));
    }
    let commits = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| {
            let record = String::from_utf8_lossy(record);
            let mut fields = record.trim_start_matches('\n').splitn(4, '\u{1f}');
            let mut next = || fields.next().unwrap_or_default().to_owned();
            json!({"id": next(), "author": next(), "date": next(), "subject": next()})
        })
        .collect();
    Ok(capped_list("commits", commits))
}

fn get_diff(root: &Path, args: &Map<String, Value>) -> Result<ToolOutput, String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or("path must be a string")?;
    check_relative(path)?;
    let staged = match args.get("staged") {
        None => false,
        Some(value) => value.as_bool().ok_or("staged must be true or false")?,
    };
    let untracked = !staged
        && worktree::load_status(root)?
            .iter()
            .any(|entry| entry.section == Section::Untracked && entry.path == path);
    let entry = Entry {
        section: if staged {
            Section::Staged
        } else if untracked {
            Section::Untracked
        } else {
            Section::Unstaged
        },
        path: path.into(),
        original: None,
    };
    Ok(capped_text(worktree::load_diff(root, &entry)?, false))
}

fn get_show(root: &Path, args: &Map<String, Value>) -> Result<ToolOutput, String> {
    let revision = args
        .get("revision")
        .and_then(Value::as_str)
        .ok_or("revision must be a string")?;
    let id = resolve_commit(root, revision)?;
    let mut command = process::git_command();
    command
        .current_dir(root)
        .args(["show", "--no-color", "--no-ext-diff", "--stat", "--patch"])
        .arg(&id)
        .arg("--");
    let output = process::limited_output(&mut command, MAX_OUTPUT)?;
    if !output.status.success() && !output.truncated {
        return Err(git_error("Could not show the commit", &output.stderr));
    }
    Ok(capped_text(
        String::from_utf8_lossy(&output.stdout).into_owned(),
        output.truncated,
    ))
}

/// The full ID of the commit `revision` names. A revision can't be an option.
pub(crate) fn resolve_commit(root: &Path, revision: &str) -> Result<String, String> {
    if revision.is_empty() || revision.starts_with('-') {
        return Err("revision must name a commit, not an option".into());
    }
    let verified = process::git_command()
        .current_dir(root)
        .args(["rev-parse", "--verify", "--quiet", "--end-of-options"])
        .arg(format!("{revision}^{{commit}}"))
        .output()
        .map_err(|error| format!("Could not run Git: {error}"))?;
    if !verified.status.success() {
        return Err(format!("Unknown revision: {revision}"));
    }
    Ok(String::from_utf8_lossy(&verified.stdout).trim().to_owned())
}

/// Refuse anything but a plain path inside the repository.
pub(crate) fn check_relative(path: &str) -> Result<(), String> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err("path must be repository-relative, without parent components".into());
    }
    Ok(())
}

/// Wrap `items` as `{key: [...]}`, dropping items from the end until it fits.
fn capped_list(key: &str, mut items: Vec<Value>) -> ToolOutput {
    let mut truncated = false;
    loop {
        let text = json!({ key: items, "truncated": truncated }).to_string();
        if text.len() <= MAX_OUTPUT || items.is_empty() {
            return ToolOutput { text, truncated };
        }
        let keep = items.len() * MAX_OUTPUT / text.len();
        items.truncate(keep.min(items.len() - 1));
        truncated = true;
    }
}

fn capped_text(mut text: String, mut truncated: bool) -> ToolOutput {
    if text.len() > MAX_OUTPUT {
        let mut end = MAX_OUTPUT;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        truncated = true;
    }
    ToolOutput { text, truncated }
}

fn git_error(context: &str, stderr: &[u8]) -> String {
    format!("{context}: {}", String::from_utf8_lossy(stderr).trim())
}

fn has_commits(root: &Path) -> bool {
    process::git_command()
        .current_dir(root)
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .output()
        .is_ok_and(|output| output.status.success())
}
