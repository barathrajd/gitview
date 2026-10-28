use std::{ffi::OsString, path::Path};
#[derive(Debug)]
pub struct Comparison {
    pub base: String,
    pub target: String,
    pub path: Option<OsString>,
    pub summary: String,
    pub patch: String,
    pub files: Vec<ComparisonFile>,
    pub truncated: bool,
}
#[derive(Debug)]
pub struct ComparisonFile {
    pub path: OsString,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub patch: String,
}
#[cfg(unix)]
fn os_path(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes.to_vec())
}
#[cfg(not(unix))]
fn os_path(bytes: &[u8]) -> OsString {
    String::from_utf8_lossy(bytes).into_owned().into()
}

fn resolve(root: &Path, revision: &str) -> Result<String, String> {
    let output = crate::process::git_command()
        .current_dir(root)
        .args(["rev-parse", "--verify", "--end-of-options"])
        .arg(format!("{revision}^{{commit}}"))
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Cannot resolve local commit revision {revision:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let id = String::from_utf8(output.stdout).map_err(|_| "Invalid object ID")?;
    let id = id.trim();
    if !(id.len() == 40 || id.len() == 64) || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid object ID".into());
    }
    Ok(id.into())
}
pub fn load(
    root: &Path,
    base: &str,
    target: &str,
    path: Option<&Path>,
) -> Result<Comparison, String> {
    if let Some(path) = path {
        if path.is_absolute()
            || path.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(
                "Use a repository-relative comparison path without parent components".into(),
            );
        }
    }
    let base = resolve(root, base)?;
    let target = resolve(root, target)?;
    let output = |summary: bool| -> Result<String, String> {
        let mut command = crate::process::git_command();
        command.current_dir(root).args([
            "--literal-pathspecs",
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--no-renames",
        ]);
        if summary {
            command.args(["--stat", "--summary"]);
        } else {
            command.arg("--patch");
        }
        command.arg(&base).arg(&target).arg("--");
        if let Some(path) = path {
            command.arg(path);
        }
        let result = crate::process::limited_output(&mut command, 1_000_000)?;
        if !result.status.success() && !result.truncated {
            return Err(format!(
                "Git comparison failed: {}",
                String::from_utf8_lossy(&result.stderr).trim()
            ));
        }
        let mut text = String::from_utf8_lossy(&result.stdout).into_owned();
        if result.truncated {
            text.insert_str(0, "[comparison output truncated at 1 MB or 5000 lines]\n");
        }
        Ok(text)
    };
    let summary = output(true)?;
    let patch = output(false)?;
    let mut command = crate::process::git_command();
    command
        .current_dir(root)
        .args([
            "--literal-pathspecs",
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-renames",
            "--numstat",
            "-z",
        ])
        .arg(&base)
        .arg(&target)
        .arg("--");
    if let Some(path) = path {
        command.arg(path);
    }
    let stats = crate::process::limited_output(&mut command, 1_000_000)?;
    if !stats.status.success() && !stats.truncated {
        return Err(format!(
            "Git file statistics failed: {}",
            String::from_utf8_lossy(&stats.stderr).trim()
        ));
    }
    let mut patches = Vec::<String>::new();
    for line in patch.split_inclusive('\n') {
        if line.starts_with("diff --git ") {
            patches.push(String::new());
        }
        if let Some(file) = patches.last_mut() {
            file.push_str(line);
        }
    }
    let mut files = Vec::new();
    for record in stats.stdout.split_inclusive(|byte| *byte == 0) {
        let Some(record) = record.strip_suffix(&[0]) else {
            break;
        };
        if record.is_empty() {
            continue;
        }
        let fields = record.splitn(3, |byte| *byte == b'\t').collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err("Invalid Git file statistics".into());
        }
        let number = |value: &[u8]| -> Result<Option<u64>, String> {
            if value == b"-" {
                return Ok(None);
            }
            std::str::from_utf8(value)
                .ok()
                .and_then(|value| value.parse().ok())
                .map(Some)
                .ok_or_else(|| "Invalid Git line count".into())
        };
        let index = files.len();
        files.push(ComparisonFile {
            path: os_path(fields[2]),
            additions: number(fields[0])?,
            deletions: number(fields[1])?,
            patch: patches
                .get(index)
                .cloned()
                .unwrap_or_else(|| "[Patch omitted by comparison output limit.]\n".into()),
        });
    }
    let truncated = stats.truncated
        || patch.starts_with("[comparison output truncated")
        || summary.starts_with("[comparison output truncated");
    Ok(Comparison {
        base,
        target,
        path: path.map(|p| p.as_os_str().to_owned()),
        summary,
        patch,
        files,
        truncated,
    })
}
