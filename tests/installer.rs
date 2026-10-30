//! `install.sh` against a fake release served from a local folder: it picks the
//! archive for the platform, verifies the checksum with whichever tool exists,
//! and refuses platforms without a prebuilt binary.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const TAG: &str = "v9.9.9";

struct Release {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

/// A release folder holding a macOS and a Linux archive. Each archive's
/// `gitview` prints which one it is.
fn release() -> Release {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_owned();
    let tag_dir = root.join(TAG);
    fs::create_dir(&tag_dir).unwrap();
    for platform in ["macos-arm64", "linux-x86_64"] {
        let staging = root.join(format!("staging-{platform}"));
        fs::create_dir(&staging).unwrap();
        let program = staging.join("gitview");
        fs::write(&program, format!("#!/bin/sh\necho {platform} gitview\n")).unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        let archive = format!("gitview-{TAG}-{platform}.tar.gz");
        run(Command::new("tar")
            .args(["-czf"])
            .arg(tag_dir.join(&archive))
            .arg("-C")
            .arg(&staging)
            .arg("gitview"));
        let digest = sha256(&tag_dir.join(&archive));
        fs::write(
            tag_dir.join(format!("{archive}.sha256")),
            format!("{digest}  {archive}\n"),
        )
        .unwrap();
    }
    Release { _dir: dir, root }
}

fn sha256(path: &Path) -> String {
    let output = Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .or_else(|_| Command::new("sha256sum").arg(path).output())
        .unwrap();
    String::from_utf8(output.stdout)
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .to_owned()
}

fn run(command: &mut Command) {
    assert!(command.status().unwrap().success(), "{command:?}");
}

/// Runs `install.sh <TAG> --prebuilt` as if on `platform`, installing into
/// a new folder. `path` replaces PATH when given.
fn install(release: &Release, platform: &str, path: Option<&str>) -> (Output, PathBuf) {
    let target = release
        .root
        .join(format!("bin-{}", platform.replace(':', "-")));
    let mut command = Command::new("bash");
    command
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/install.sh"))
        .args([TAG, "--prebuilt"])
        .env("GITVIEW_INSTALL_DIR", &target)
        .env(
            "GITVIEW_TEST_RELEASE_URL",
            format!("file://{}", release.root.display()),
        )
        .env("GITVIEW_TEST_PLATFORM", platform)
        .stdin(std::process::Stdio::null());
    if let Some(path) = path {
        command.env("PATH", path);
    }
    (command.output().unwrap(), target)
}

fn installed(target: &Path) -> String {
    let output = Command::new(target.join("gitview")).output().unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[test]
fn linux_x86_64_gets_the_linux_archive() {
    let release = release();
    for platform in ["Linux:x86_64", "Linux:amd64"] {
        let (output, target) = install(&release, platform, None);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(installed(&target), "linux-x86_64 gitview");
    }
}

#[test]
fn macos_apple_silicon_still_gets_the_macos_archive() {
    let release = release();
    for platform in ["Darwin:arm64", "Darwin:aarch64"] {
        let (output, target) = install(&release, platform, None);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(installed(&target), "macos-arm64 gitview");
    }
}

#[test]
fn other_platforms_are_told_what_is_supported() {
    let release = release();
    for platform in ["Linux:aarch64", "Darwin:x86_64", "FreeBSD:amd64"] {
        let (output, target) = install(&release, platform, None);
        assert!(!output.status.success(), "{platform}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("macOS Apple Silicon and Linux x86_64"),
            "{error}"
        );
        assert!(error.contains("Cargo"), "{error}");
        assert!(!target.join("gitview").exists());
    }
}

#[test]
fn a_bad_checksum_installs_nothing() {
    let release = release();
    let checksum = release
        .root
        .join(TAG)
        .join(format!("gitview-{TAG}-linux-x86_64.tar.gz.sha256"));
    let text = fs::read_to_string(&checksum).unwrap();
    fs::write(&checksum, format!("{}{}", "0".repeat(64), &text[64..])).unwrap();

    let (output, target) = install(&release, "Linux:x86_64", None);

    assert!(!output.status.success());
    assert!(!target.join("gitview").exists());
}

/// A PATH holding only the tools the installer needs, plus `extra`: one of
/// the two checksum tools.
fn limited_path(release: &Release, extra: &str) -> String {
    let bin = release.root.join(format!("tools-{extra}"));
    fs::create_dir(&bin).unwrap();
    for tool in [
        "bash", "curl", "tar", "gzip", "install", "awk", "mkdir", "mktemp", "rm", "uname", "cat",
        "dirname", "sh", "chmod", extra,
    ] {
        let found = Command::new("sh")
            .args(["-c", &format!("command -v {tool}")])
            .output()
            .unwrap();
        let found = String::from_utf8(found.stdout).unwrap();
        if !found.trim().is_empty() {
            std::os::unix::fs::symlink(found.trim(), bin.join(tool)).unwrap();
        }
    }
    bin.display().to_string()
}

#[test]
fn the_checksum_is_verified_with_whichever_tool_exists() {
    let release = release();
    for tool in ["sha256sum", "shasum"] {
        let has_tool = Command::new("sh")
            .args(["-c", &format!("command -v {tool}")])
            .status()
            .unwrap()
            .success();
        if !has_tool {
            // Each platform has at least one; the other is checked where it exists.
            continue;
        }
        let path = limited_path(&release, tool);
        let (output, target) = install(&release, "Linux:x86_64", Some(&path));
        assert!(
            output.status.success(),
            "{tool}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(installed(&target), "linux-x86_64 gitview");
    }
}

#[test]
fn without_a_checksum_tool_the_installer_stops() {
    let release = release();
    let path = limited_path(&release, "true");

    let (output, target) = install(&release, "Linux:x86_64", Some(&path));

    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("sha256sum or shasum"), "{error}");
    assert!(!target.join("gitview").exists());
}
