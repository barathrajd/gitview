use std::{path::Path, process::Command};

#[derive(Debug)]
pub struct CommitDetails {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub subject: String,
    pub files: Vec<String>,
}

pub fn load_commit(root: &Path, revision: &str) -> Result<CommitDetails, String> {
    let metadata = git(
        root,
        &[
            "show",
            "-s",
            "--format=%H%x00%an%x00%aI%x00%s",
            "--end-of-options",
            revision,
        ],
    )?;
    let fields: Vec<_> = metadata.split(|byte| *byte == 0).collect();
    if fields.len() != 4 {
        return Err("Invalid commit metadata from Git".into());
    }
    let to_string = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .trim_end_matches('\n')
            .to_owned()
    };
    let files = git(
        root,
        &[
            "diff-tree",
            "--root",
            "--first-parent",
            "-m",
            "--no-commit-id",
            "--name-only",
            "-r",
            "-z",
            "--no-renames",
            "--end-of-options",
            revision,
        ],
    )?;
    Ok(CommitDetails {
        hash: to_string(fields[0]),
        author: to_string(fields[1]),
        date: to_string(fields[2]),
        subject: to_string(fields[3]),
        files: files
            .split(|byte| *byte == 0)
            .filter(|field| !field.is_empty())
            .map(|field| String::from_utf8_lossy(field).into_owned())
            .collect(),
    })
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|e| format!("Could not run Git: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "Git failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}
