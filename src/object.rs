use std::path::Path;
#[derive(Debug)]
pub struct ObjectDescription {
    pub id: String,
    pub kind: String,
    pub size: u64,
    pub description: Option<String>,
    pub preview: String,
    pub truncated: bool,
}
fn command(root: &Path) -> std::process::Command {
    let mut command = crate::process::git_command();
    command.current_dir(root).env("GIT_NO_LAZY_FETCH", "1");
    command
}
fn output(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = command(root)
        .args(args)
        .output()
        .map_err(|error| format!("Could not inspect Git object: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Git object inspection failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end_matches('\n')
        .to_owned())
}
pub fn inspect(root: &Path, revision: &str) -> Result<ObjectDescription, String> {
    let id = output(
        root,
        &["rev-parse", "--verify", "--end-of-options", revision],
    )?;
    if !(id.len() == 40 || id.len() == 64) || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Invalid Git object ID".into());
    }
    let kind = output(root, &["cat-file", "-t", &id])?;
    let size = output(root, &["cat-file", "-s", &id])?
        .parse::<u64>()
        .map_err(|_| "Invalid Git object size")?;
    let description = if kind == "commit" {
        Some(output(root, &["describe", "--always", "--tags", &id])?)
    } else {
        None
    };
    let mut command = command(root);
    if kind == "commit" {
        command.args([
            "show",
            "--no-patch",
            "--no-notes",
            "--no-show-signature",
            "--format=fuller",
            &id,
            "--",
        ]);
    } else {
        command.args(["cat-file", "-p", &id]);
    }
    let result = crate::process::limited_output(&mut command, 1_000_000)?;
    if !result.status.success() && !result.truncated {
        return Err(format!(
            "Git object preview failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    let preview = if kind == "blob" && result.stdout.contains(&0) {
        "Binary blob; content preview is not displayed.".into()
    } else {
        String::from_utf8_lossy(&result.stdout).into_owned()
    };
    Ok(ObjectDescription {
        id,
        kind,
        size,
        description,
        preview,
        truncated: result.truncated,
    })
}
