use std::{io::Write, path::Path, process::Stdio};

use crate::{process, worktree};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitPreview {
    pub entries: Vec<worktree::Entry>,
    staged_diff: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmendPreview {
    pub entries: Vec<worktree::Entry>,
    pub head: String,
    pub message: String,
    staged_diff: Vec<u8>,
}

/// Capture staged paths and index object IDs for the commit editor.
pub fn preview_commit(root: &Path) -> Result<CommitPreview, String> {
    let entries = staged_entries(root)?;
    if entries.is_empty() {
        return Err("Stage changes before creating a commit".into());
    }
    Ok(CommitPreview {
        entries,
        staged_diff: staged_diff(root)?,
    })
}

/// Capture HEAD and the staged index shown before an amend warning.
pub fn preview_amend(root: &Path) -> Result<AmendPreview, String> {
    let head = git_output(root, &["rev-parse", "--verify", "HEAD"])
        .map_err(|_| "Create the first commit before amending".to_owned())?;
    let message = git_output(root, &["log", "-1", "--format=%B"])?;
    Ok(AmendPreview {
        entries: staged_entries(root)?,
        head: String::from_utf8_lossy(&head).trim().to_owned(),
        message: String::from_utf8_lossy(&message).trim_end().to_owned(),
        staged_diff: staged_diff(root)?,
    })
}

/// Commit exactly the staged set shown in the editor.
pub fn create_commit(root: &Path, message: &str, expected: &CommitPreview) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("Commit message cannot be empty".into());
    }
    if expected.entries.is_empty() {
        return Err("Stage changes before creating a commit".into());
    }

    if preview_commit(root)? != *expected {
        return Err("Staged changes changed; review them and press Enter again".into());
    }

    write_commit(root, message, false)
}

/// Amend HEAD only if HEAD and the staged index still match the warning preview.
pub fn amend_commit(root: &Path, message: &str, expected: &AmendPreview) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("Commit message cannot be empty".into());
    }
    if preview_amend(root)? != *expected {
        return Err("HEAD or staged changes changed; review the amend warning again".into());
    }
    write_commit(root, message, true)
}

fn write_commit(root: &Path, message: &str, amend: bool) -> Result<(), String> {
    let mut command = process::git_command();
    command.current_dir(root).arg("commit");
    if amend {
        command.arg("--amend");
    }
    let mut child = command
        .arg("--file=-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not run Git commit: {error}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or("Could not pass the commit message to Git")?;
    stdin
        .write_all(message.trim_end().as_bytes())
        .and_then(|()| stdin.write_all(b"\n"))
        .map_err(|error| format!("Could not pass the commit message to Git: {error}"))?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .map_err(|error| format!("Could not read Git commit result: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if detail.is_empty() {
            format!("Git commit failed with {}", output.status)
        } else {
            format!("Git commit failed: {detail}")
        })
    }
}

fn staged_entries(root: &Path) -> Result<Vec<worktree::Entry>, String> {
    Ok(worktree::load_status(root)?
        .into_iter()
        .filter(|entry| entry.section == worktree::Section::Staged)
        .collect())
}

fn git_output(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = process::git_command()
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|error| format!("Could not run Git: {error}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "Git command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn staged_diff(root: &Path) -> Result<Vec<u8>, String> {
    let output = process::git_command()
        .current_dir(root)
        .args(["diff", "--cached", "--raw", "-z", "--no-renames"])
        .output()
        .map_err(|error| format!("Could not inspect staged changes: {error}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "Could not inspect staged changes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
