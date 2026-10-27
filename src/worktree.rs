use std::{
    ffi::OsString,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Staged,
    Unstaged,
    Untracked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub section: Section,
    pub path: OsString,
    pub original: Option<OsString>,
}

pub fn load_status(root: &Path) -> Result<Vec<Entry>, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["status", "--porcelain=v2", "-z", "--untracked-files=all"])
        .output()
        .map_err(|e| format!("Could not run Git: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "Unable to read status: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let mut records = output.stdout.split(|byte| *byte == 0);
    let mut entries = Vec::new();
    while let Some(record) = records.next() {
        if record.is_empty() {
            continue;
        }
        match record[0] {
            b'?' if record.starts_with(b"? ") => entries.push(Entry {
                section: Section::Untracked,
                path: path_from_bytes(&record[2..]),
                original: None,
            }),
            b'1' | b'2' => {
                let rename = record[0] == b'2';
                let fields: Vec<_> = record
                    .splitn(if rename { 10 } else { 9 }, |b| *b == b' ')
                    .collect();
                let expected = if rename { 10 } else { 9 };
                if fields.len() != expected || fields[1].len() != 2 {
                    return Err("Invalid Git status record".into());
                }
                let path = path_from_bytes(fields[expected - 1]);
                let original = if rename {
                    Some(path_from_bytes(
                        records
                            .next()
                            .filter(|r| !r.is_empty())
                            .ok_or("Incomplete Git rename record")?,
                    ))
                } else {
                    None
                };
                if fields[1][0] != b'.' {
                    entries.push(Entry {
                        section: Section::Staged,
                        path: path.clone(),
                        original: original.clone(),
                    });
                }
                if fields[1][1] != b'.' {
                    entries.push(Entry {
                        section: Section::Unstaged,
                        path,
                        original: original.clone(),
                    });
                }
            }
            b'u' => {
                return Err(
                    "Unmerged files are not supported in the MVP; resolve conflicts with Git first"
                        .into(),
                );
            }
            _ => return Err("Unrecognized Git status record".into()),
        }
    }
    entries.sort_by_key(|entry| match entry.section {
        Section::Staged => 0,
        Section::Unstaged => 1,
        Section::Untracked => 2,
    });
    Ok(entries)
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes.to_vec())
}
#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> OsString {
    OsString::from(String::from_utf8_lossy(bytes).into_owned())
}

fn run_action(
    root: &Path,
    args: &[&str],
    path: &std::ffi::OsStr,
    original: Option<&std::ffi::OsStr>,
) -> Result<(), String> {
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .arg("--literal-pathspecs")
        .args(args)
        .arg("--")
        .arg(path);
    if let Some(original) = original {
        command.arg(original);
    }
    let output = command
        .output()
        .map_err(|e| format!("Could not run Git: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Git operation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

pub fn stage(root: &Path, entry: &Entry) -> Result<(), String> {
    if entry.section == Section::Staged {
        return Err("Select an unstaged or untracked file".into());
    }
    if !load_status(root)?.iter().any(|item| {
        item.section == entry.section && item.path == entry.path && item.original == entry.original
    }) {
        return Err("File status changed; refresh and try again".into());
    }
    run_action(root, &["add", "-A"], &entry.path, None)
}

pub fn stage_all(root: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["add", "-A"])
        .output()
        .map_err(|error| format!("Could not stage all changes: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Could not stage all changes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

pub fn unstage_all(root: &Path) -> Result<(), String> {
    let head = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .output()
        .map_err(|error| format!("Could not check HEAD: {error}"))?;
    let output = if head.status.success() {
        Command::new("git")
            .current_dir(root)
            .args(["reset", "HEAD", "--", "."])
            .output()
    } else {
        Command::new("git")
            .current_dir(root)
            .args(["rm", "--cached", "--ignore-unmatch", "-r", "--", "."])
            .output()
    }
    .map_err(|error| format!("Could not unstage all changes: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Could not unstage all changes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

pub fn unstage(root: &Path, entry: &Entry) -> Result<(), String> {
    if entry.section != Section::Staged {
        return Err("Select a staged file".into());
    }
    if !load_status(root)?.iter().any(|item| {
        item.section == entry.section && item.path == entry.path && item.original == entry.original
    }) {
        return Err("File status changed; refresh and try again".into());
    }
    let head = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .output()
        .map_err(|e| format!("Could not check HEAD: {e}"))?;
    if head.status.success() {
        return run_action(
            root,
            &["restore", "--staged"],
            &entry.path,
            entry.original.as_deref(),
        );
    }
    let symbolic = Command::new("git")
        .current_dir(root)
        .args(["symbolic-ref", "-q", "HEAD"])
        .output()
        .map_err(|e| format!("Could not check branch: {e}"))?;
    if !symbolic.status.success() || head.status.code() != Some(1) {
        return Err("Cannot determine whether HEAD is unborn; index unchanged".into());
    }
    run_action(
        root,
        &["rm", "--cached", "-f"],
        &entry.path,
        entry.original.as_deref(),
    )
}

pub fn stage_hunk(root: &Path, entry: &Entry, hunk_index: usize) -> Result<(), String> {
    if entry.section != Section::Unstaged {
        return Err("Select a tracked unstaged file to stage a hunk".into());
    }
    ensure_current_entry(root, entry)?;
    let patch = selected_hunk_patch(root, entry, false, hunk_index)?;
    apply_hunk_patch(root, &patch, false)
}

pub fn unstage_hunk(root: &Path, entry: &Entry, hunk_index: usize) -> Result<(), String> {
    if entry.section != Section::Staged {
        return Err("Select a staged file to unstage a hunk".into());
    }
    ensure_current_entry(root, entry)?;
    let patch = selected_hunk_patch(root, entry, true, hunk_index)?;
    apply_hunk_patch(root, &patch, true)
}

fn ensure_current_entry(root: &Path, entry: &Entry) -> Result<(), String> {
    if load_status(root)?.iter().any(|current| {
        current.section == entry.section
            && current.path == entry.path
            && current.original == entry.original
    }) {
        Ok(())
    } else {
        Err("File status changed; refresh and try again".into())
    }
}

fn selected_hunk_patch(
    root: &Path,
    entry: &Entry,
    staged: bool,
    hunk_index: usize,
) -> Result<Vec<u8>, String> {
    let mut command = Command::new("git");
    command.current_dir(root).arg("--literal-pathspecs").args([
        "diff",
        "--no-ext-diff",
        "--no-color",
        "--unified=0",
    ]);
    if staged {
        command.arg("--cached");
    }
    command.args(["--find-renames", "--"]).arg(&entry.path);
    if staged && let Some(original) = &entry.original {
        command.arg(original);
    }
    let output = command
        .output()
        .map_err(|error| format!("Could not load diff: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Could not load diff: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    extract_hunk(&output.stdout, hunk_index)
}

fn extract_hunk(diff: &[u8], hunk_index: usize) -> Result<Vec<u8>, String> {
    let lines: Vec<_> = diff.split_inclusive(|byte| *byte == b'\n').collect();
    let hunks: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.starts_with(b"@@ ").then_some(index))
        .collect();
    let Some(&first_hunk) = hunks.first() else {
        return Err("Selected change has no text hunks".into());
    };
    let Some(&selected_start) = hunks.get(hunk_index) else {
        return Err("Selected hunk no longer exists; refresh and try again".into());
    };
    let selected_end = hunks.get(hunk_index + 1).copied().unwrap_or(lines.len());
    let mut patch = Vec::new();
    for line in &lines[..first_hunk] {
        patch.extend_from_slice(line);
    }
    for line in &lines[selected_start..selected_end] {
        patch.extend_from_slice(line);
    }
    Ok(patch)
}

fn apply_hunk_patch(root: &Path, patch: &[u8], reverse: bool) -> Result<(), String> {
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .args(["apply", "--cached", "--unidiff-zero"]);
    if reverse {
        command.arg("--reverse");
    }
    let mut child = command
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not apply hunk: {error}"))?;
    child
        .stdin
        .take()
        .ok_or("Could not pass hunk to Git")?
        .write_all(patch)
        .map_err(|error| format!("Could not pass hunk to Git: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("Could not apply hunk: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Git could not apply the selected hunk: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

pub fn load_diff(root: &Path, entry: &Entry) -> Result<String, String> {
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .arg("--literal-pathspecs")
        .arg("diff");
    if entry.section == Section::Untracked {
        command
            .args([
                "--no-index",
                "--no-ext-diff",
                "--no-color",
                "--",
                "/dev/null",
            ])
            .arg(&entry.path);
    } else {
        command.args(["--no-ext-diff", "--no-color", "--find-renames"]);
        if entry.section == Section::Staged {
            command.arg("--cached");
        }
        command.arg("--").arg(&entry.path);
        if entry.section == Section::Staged
            && let Some(original) = &entry.original
        {
            command.arg(original);
        }
    }
    let output = crate::process::limited_output(&mut command, 1_000_000)?;
    if !output.status.success()
        && !output.truncated
        && !(entry.section == Section::Untracked && output.status.code() == Some(1))
    {
        return Err(format!(
            "Could not load diff: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.truncated {
        text.insert_str(0, "[diff truncated at 1 MB or 5000 lines]\n");
    }
    Ok(text)
}
