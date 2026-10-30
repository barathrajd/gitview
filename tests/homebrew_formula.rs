//! `scripts/render-homebrew-formula.sh` fills in the Homebrew formula for a
//! release from its tag and the two archives' `.sha256` files, and refuses
//! anything that doesn't look like a release.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const MACOS_SHA: &str = "73bbdafdd9fbd29f14caf450483e0729124e6c2e0d49b58183ec09b94df96049";
const LINUX_SHA: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn checksum_file(dir: &Path, archive: &str, digest: &str) -> PathBuf {
    let path = dir.join(format!("{archive}.sha256"));
    fs::write(&path, format!("{digest}  {archive}\n")).unwrap();
    path
}

fn render(tag: &str, macos: &Path, linux: &Path) -> Output {
    Command::new("bash")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/scripts/render-homebrew-formula.sh"
        ))
        .arg(tag)
        .arg(macos)
        .arg(linux)
        .output()
        .unwrap()
}

fn release_files(dir: &Path, tag: &str) -> (PathBuf, PathBuf) {
    (
        checksum_file(dir, &format!("gitview-{tag}-macos-arm64.tar.gz"), MACOS_SHA),
        checksum_file(
            dir,
            &format!("gitview-{tag}-linux-x86_64.tar.gz"),
            LINUX_SHA,
        ),
    )
}

#[test]
fn renders_both_platforms_for_a_release() {
    let dir = tempfile::tempdir().unwrap();
    let (macos, linux) = release_files(dir.path(), "v0.7.0");

    let output = render("v0.7.0", &macos, &linux);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let formula = String::from_utf8(output.stdout).unwrap();
    let base = "https://github.com/barathrajd/gitview/releases/download/v0.7.0";
    assert!(
        formula.contains(&format!("url \"{base}/gitview-v0.7.0-macos-arm64.tar.gz\"")),
        "{formula}"
    );
    assert!(
        formula.contains(&format!(
            "url \"{base}/gitview-v0.7.0-linux-x86_64.tar.gz\""
        )),
        "{formula}"
    );
    assert!(
        formula.contains(&format!("sha256 \"{MACOS_SHA}\"")),
        "{formula}"
    );
    assert!(
        formula.contains(&format!("sha256 \"{LINUX_SHA}\"")),
        "{formula}"
    );
    // The macOS checksum belongs to the macOS URL, not the Linux one.
    let macos_url = formula.find("macos-arm64.tar.gz").unwrap();
    let linux_url = formula.find("linux-x86_64.tar.gz").unwrap();
    let macos_sum = formula.find(MACOS_SHA).unwrap();
    assert!(macos_url < macos_sum && macos_sum < linux_url, "{formula}");
    assert!(formula.contains("class Gitview < Formula"));
    assert!(formula.contains("depends_on \"git\""));
    assert!(
        !formula.contains('@'),
        "an unfilled placeholder is left: {formula}"
    );
}

#[test]
fn refuses_input_that_isnt_a_release() {
    let dir = tempfile::tempdir().unwrap();
    let (macos, linux) = release_files(dir.path(), "v0.7.0");
    let bad_digest = checksum_file(dir.path(), "gitview-v0.7.0-linux-x86_64.tar.gz", "abc");
    let wrong_archive = checksum_file(dir.path(), "gitview-v0.6.0-linux-x86_64.tar.gz", LINUX_SHA);

    for (tag, macos, linux, expected) in [
        ("0.7.0", &macos, &linux, "tag"),
        ("v0.7.0\"; system \"x", &macos, &linux, "tag"),
        ("v0.7.0", &linux, &macos, "macos-arm64"),
        ("v0.7.0", &macos, &bad_digest, "SHA-256"),
        ("v0.7.0", &macos, &wrong_archive, "linux-x86_64"),
        (
            "v0.7.0",
            &macos,
            &dir.path().join("missing.sha256"),
            "missing",
        ),
    ] {
        let output = render(tag, macos, linux);
        assert!(!output.status.success(), "{tag} {}", linux.display());
        assert!(output.stdout.is_empty(), "nothing is printed on failure");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(expected), "expected {expected:?} in {error}");
    }
}

/// Removes the throwaway tap even when the test fails.
struct TemporaryTap(&'static str);

impl Drop for TemporaryTap {
    fn drop(&mut self) {
        let _ = Command::new("brew")
            .args(["untap", "--force", self.0])
            .output();
    }
}

/// `brew style` and `brew audit --strict` accept the rendered formula. Ignored
/// by default: Homebrew only checks formulas inside a tap, so this adds a
/// throwaway tap to the local Homebrew and removes it afterwards. Run it with
/// `cargo test --test homebrew_formula -- --ignored`.
#[test]
#[ignore]
fn the_rendered_formula_passes_brew_style_and_audit() {
    const TAP: &str = "gitview-test/formula-check";
    let dir = tempfile::tempdir().unwrap();
    let (macos, linux) = release_files(dir.path(), "v0.7.0");
    let output = render("v0.7.0", &macos, &linux);
    assert!(output.status.success());
    let brew = |args: &[&str]| {
        Command::new("brew")
            .args(args)
            // Not HOMEBREW_NO_INSTALL_FROM_API: that makes audit clone all
            // of homebrew-core.
            .env("HOMEBREW_NO_AUTO_UPDATE", "1")
            .output()
            .unwrap()
    };
    assert!(brew(&["tap-new", "--no-git", TAP]).status.success());
    let _cleanup = TemporaryTap(TAP);
    let repository = String::from_utf8(brew(&["--repository", TAP]).stdout).unwrap();
    fs::write(
        Path::new(repository.trim()).join("Formula/gitview.rb"),
        &output.stdout,
    )
    .unwrap();
    assert!(brew(&["trust", TAP]).status.success());

    for check in [
        &["style", "gitview-test/formula-check/gitview"][..],
        &["audit", "--strict", "gitview-test/formula-check/gitview"],
    ] {
        let result = brew(check);
        assert!(
            result.status.success(),
            "brew {check:?}:\n{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
