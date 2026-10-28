use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use crate::process;

/// Git prints one terminating newline; preserve all bytes belonging to the path.
pub fn parse_root(output: &[u8]) -> PathBuf {
    let path = output.strip_suffix(b"\n").unwrap_or(output);
    #[cfg(unix)]
    let value = {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(path.to_vec())
    };
    #[cfg(not(unix))]
    let value = OsString::from(String::from_utf8_lossy(path).into_owned());
    PathBuf::from(value)
}

pub fn initialize(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("Choose a directory to initialize".into());
    }
    if path.exists() && !path.is_dir() {
        return Err("The initialization destination is not a directory".into());
    }
    if let Some(existing) = existing_repository_ancestor(path) {
        return Err(format!(
            "The destination is already inside a Git repository: {}",
            existing.display()
        ));
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.is_dir()
    {
        return Err(format!(
            "The parent directory does not exist: {}",
            parent.display()
        ));
    }

    let output = process::git_command()
        .args(["init", "--quiet", "--"])
        .arg(path)
        .output()
        .map_err(|error| format!("Could not run Git init: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Git init failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn existing_repository_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| ancestor.is_dir() && is_repository(ancestor))
        .map(Path::to_path_buf)
}

fn is_repository(path: &Path) -> bool {
    process::git_command()
        .current_dir(path)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .is_ok_and(|output| output.status.success())
}

pub fn clone_repository(source: &Path, destination: Option<&Path>) -> Result<(), String> {
    if source.as_os_str().is_empty() {
        return Err("Provide a repository URL or path to clone".into());
    }
    if let Some(destination) = destination {
        if destination.exists() && !destination.is_dir() {
            return Err("The clone destination exists and is not a directory".into());
        }
        if destination.is_dir()
            && fs::read_dir(destination)
                .map_err(|error| format!("Could not inspect clone destination: {error}"))?
                .next()
                .is_some()
        {
            return Err("The clone destination is not empty".into());
        }
    }

    let mut command = process::git_command();
    command.args(["clone", "--progress", "--"]);
    command.arg(source);
    if let Some(destination) = destination {
        command.arg(destination);
    }
    let status = command
        .status()
        .map_err(|error| format!("Could not start Git clone: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Git clone failed with {status}; review the Git output above"
        ))
    }
}
