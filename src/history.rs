use std::{path::Path, process::Command};

pub fn load_history(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "log",
            "--all",
            "--graph",
            "--decorate=short",
            "--oneline",
            "--color=never",
            "-n",
            "200",
        ])
        .output()
        .map_err(|error| format!("Unable to load history: {error}"))?;
    if !output.status.success() {
        let empty = Command::new("git")
            .current_dir(root)
            .args(["rev-parse", "--verify", "HEAD"])
            .output()
            .map_err(|error| format!("Unable to check HEAD: {error}"))?;
        if !empty.status.success() {
            return Ok(Vec::new());
        }
        return Err(format!(
            "Unable to load history: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect())
}
