use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::Output,
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub name: OsString,
    pub object: String,
    pub kind: String,
}
#[derive(Clone, Debug)]
pub struct CreatePreview {
    name: String,
    target: String,
}
impl CreatePreview {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn target(&self) -> &str {
        &self.target
    }
}
pub struct Verification {
    pub verified: bool,
    pub output: String,
}
fn git(root: &Path, args: &[&str]) -> Result<Output, String> {
    crate::process::git_command()
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|e| format!("Could not run Git: {e}"))
}
fn checked(output: Output) -> Result<Vec<u8>, String> {
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "Git failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
fn reference(name: &OsStr) -> OsString {
    let mut reference = OsString::from("refs/tags/");
    reference.push(name);
    reference
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
pub fn list(root: &Path) -> Result<Vec<Tag>, String> {
    let output = checked(git(
        root,
        &[
            "for-each-ref",
            "--format=%(refname:strip=2)%00%(objectname)%00%(objecttype)%00",
            "refs/tags/",
        ],
    )?)?;
    output
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields: Vec<_> = line.split(|b| *b == 0).collect();
            if fields.len() != 4 {
                return Err("Invalid tag record from Git".into());
            }
            Ok(Tag {
                name: os(fields[0]),
                object: String::from_utf8_lossy(fields[1]).into_owned(),
                kind: String::from_utf8_lossy(fields[2]).into_owned(),
            })
        })
        .collect()
}
fn head(root: &Path) -> Result<String, String> {
    let output = checked(git(root, &["rev-parse", "--verify", "HEAD^{commit}"])?)?;
    Ok(String::from_utf8_lossy(&output).trim().to_owned())
}
pub fn preview_create(root: &Path, name: &str) -> Result<CreatePreview, String> {
    if name.is_empty() || name.starts_with('-') {
        return Err("Enter a valid tag name".into());
    }
    checked(git(
        root,
        &["check-ref-format", &format!("refs/tags/{name}")],
    )?)?;
    if list(root)?.iter().any(|tag| tag.name == name) {
        return Err("Tag already exists".into());
    }
    Ok(CreatePreview {
        name: name.into(),
        target: head(root)?,
    })
}
pub fn create(root: &Path, preview: &CreatePreview) -> Result<(), String> {
    if head(root)? != preview.target {
        return Err("HEAD changed; review the tag target again".into());
    }
    let zero = "0".repeat(preview.target.len());
    checked(git(
        root,
        &[
            "update-ref",
            &format!("refs/tags/{}", preview.name),
            &preview.target,
            &zero,
        ],
    )?)?;
    Ok(())
}
pub fn delete(root: &Path, tag: &Tag) -> Result<(), String> {
    let output = crate::process::git_command()
        .current_dir(root)
        .args(["update-ref", "-d"])
        .arg(reference(&tag.name))
        .arg(&tag.object)
        .output()
        .map_err(|e| e.to_string())?;
    checked(output)?;
    Ok(())
}
pub fn verify(root: &Path, tag: &Tag) -> Result<Verification, String> {
    if !list(root)?.iter().any(|current| current == tag) {
        return Err("Tag changed; refresh and review again".into());
    }
    let mut command = crate::process::git_command();
    // Verify the reviewed object ID so a concurrent ref change cannot select another object.
    command
        .current_dir(root)
        .args(["verify-tag", "--raw", "--"])
        .arg(&tag.object);
    let output = crate::process::limited_output(&mut command, 65536)?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    if output.truncated {
        text.push_str("\n[verification output truncated]");
    }
    Ok(Verification {
        verified: output.status.success(),
        output: text,
    })
}
