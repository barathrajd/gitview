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
    Ok(Branches { head, branches })
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
