use std::{
    ffi::OsString,
    path::Path,
    process::{Command, Output},
};

#[derive(Debug, PartialEq, Eq)]
pub enum HeadState {
    Attached(OsString),
    Detached,
    Unborn(OsString),
}
#[derive(Debug)]
pub struct Branches {
    pub head: HeadState,
    pub branches: Vec<OsString>,
    pub head_commit: Option<String>,
}

fn git(root: &Path, args: &[&str]) -> Result<Output, String> {
    Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|error| format!("Could not run Git: {error}"))
}
fn error(output: &Output) -> String {
    format!(
        "Git failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}
#[cfg(unix)]
fn os(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes.to_vec())
}
#[cfg(not(unix))]
fn os(bytes: &[u8]) -> OsString {
    OsString::from(String::from_utf8_lossy(bytes).into_owned())
}

pub fn load_branches(root: &Path) -> Result<Branches, String> {
    let refs = git(
        root,
        &[
            "for-each-ref",
            "--format=%(refname:strip=2)%00",
            "refs/heads/",
        ],
    )?;
    if !refs.status.success() {
        return Err(error(&refs));
    }
    let branches = refs
        .stdout
        .split(|b| *b == 0)
        .filter_map(|item| {
            let item = item.strip_prefix(b"\n").unwrap_or(item);
            (!item.is_empty() && item != b"\n").then(|| os(item))
        })
        .collect();
    let symbolic = git(root, &["symbolic-ref", "--quiet", "HEAD"])?;
    let exists = git(root, &["rev-parse", "--verify", "--quiet", "HEAD"])?;
    let head = if symbolic.status.success() {
        let value = symbolic
            .stdout
            .strip_suffix(b"\n")
            .unwrap_or(&symbolic.stdout);
        let name = value
            .strip_prefix(b"refs/heads/")
            .ok_or("HEAD points outside local branches")?;
        if exists.status.success() {
            HeadState::Attached(os(name))
        } else if exists.status.code() == Some(1) {
            HeadState::Unborn(os(name))
        } else {
            return Err(error(&exists));
        }
    } else if symbolic.status.code() == Some(1) && exists.status.success() {
        HeadState::Detached
    } else if !symbolic.status.success() && symbolic.status.code() != Some(1) {
        return Err(error(&symbolic));
    } else {
        return Err(error(&exists));
    };
    let head_commit = if exists.status.success() {
        let value = String::from_utf8(exists.stdout).map_err(|_| "Invalid Git object ID")?;
        let id = value.trim_end_matches('\n');
        if !(id.len() == 40 || id.len() == 64) || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Invalid Git object ID".into());
        }
        Some(id.to_owned())
    } else {
        None
    };
    Ok(Branches {
        head,
        branches,
        head_commit,
    })
}

pub fn switch_branch(root: &Path, name: &std::ffi::OsStr) -> Result<(), String> {
    let data = load_branches(root)?;
    if !data.branches.iter().any(|branch| branch == name) {
        return Err("The selected local branch no longer exists; refresh and try again".into());
    }
    let output = Command::new("git")
        .current_dir(root)
        .args(["switch", "--no-guess", "--no-recurse-submodules", "--"])
        .arg(name)
        .output()
        .map_err(|e| format!("Could not run Git: {e}"))?;
    if !output.status.success() {
        return Err(error(&output));
    }
    Ok(())
}

/// Create only the confirmed name at the confirmed current commit, then switch to it.
pub fn create_branch(root: &Path, name: &str, expected_head: &str) -> Result<(), String> {
    if name.is_empty()
        || name.starts_with('-')
        || name.starts_with("@{-")
        || name.contains(['\n', '\r', '\0'])
    {
        return Err("Invalid branch name".into());
    }
    let check = git(root, &["check-ref-format", "--branch", name])?;
    if !check.status.success() || check.stdout.strip_suffix(b"\n") != Some(name.as_bytes()) {
        return Err(format!("Invalid branch name: {name}"));
    }
    let state = load_branches(root)?;
    let Some(head) = state.head_commit else {
        return Err("Create the first commit before creating a branch".into());
    };
    if head != expected_head {
        return Err("HEAD changed; refresh branches and confirm again".into());
    }
    if state.branches.iter().any(|branch| branch == name) {
        return Err("A local branch with this name already exists".into());
    }
    let output = git(
        root,
        &[
            "switch",
            "--no-recurse-submodules",
            "--no-track",
            "-c",
            name,
            &head,
        ],
    )?;
    if !output.status.success() {
        return Err(error(&output));
    }
    Ok(())
}
