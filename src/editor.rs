use std::{io::Write, path::Path, process::Command};

pub fn configured_editor(root: &Path) -> Result<String, String> {
    let output = crate::process::git_command()
        .current_dir(root)
        .args(["var", "GIT_EDITOR"])
        .output()
        .map_err(|error| format!("Could not resolve Git editor: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| "Git editor command is not valid UTF-8".into())
}

/// Run an editor as an executable and arguments, without shell evaluation.
/// The caller must release the terminal and pause its input reader first.
pub fn edit_with_command(root: &Path, editor: &str, message: &str) -> Result<String, String> {
    let args = shell_words::split(editor)
        .map_err(|error| format!("Could not parse Git editor command: {error}"))?;
    if args.is_empty()
        || editor.contains(['$', '`', '\n', '\r'])
        || args
            .iter()
            .any(|arg| arg.contains(['|', ';', '&', '<', '>']))
    {
        return Err(
            "Use an editor executable with arguments; shell expressions are unsupported".into(),
        );
    }
    let mut draft = tempfile::Builder::new()
        .prefix("gitview-commit-")
        .suffix(".txt")
        .tempfile()
        .map_err(|error| format!("Could not create commit draft: {error}"))?;
    draft
        .write_all(message.as_bytes())
        .and_then(|()| draft.flush())
        .map_err(|error| format!("Could not write commit draft: {error}"))?;
    let status = Command::new(&args[0])
        .args(&args[1..])
        .arg(draft.path())
        .current_dir(root)
        .status()
        .map_err(|error| format!("Could not open Git editor: {error}"))?;
    if !status.success() {
        return Err(format!(
            "Git editor exited with {status}; kept the original message"
        ));
    }
    std::fs::read_to_string(draft.path())
        .map_err(|error| format!("Could not read commit draft: {error}"))
}
