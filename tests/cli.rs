use std::process::Command;

#[test]
fn help_describes_usage_without_a_repository() {
    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .arg("--help")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: gitview"));
}

#[test]
fn outside_a_repository_reports_an_actionable_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Git repository"));
}

#[test]
fn missing_git_reports_how_to_fix_it() {
    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("install Git"));
}

#[test]
fn help_lists_history_and_working_tree_controls() {
    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .arg("--help")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    for word in ["Search", "Stage", "Unstage", "Refresh"] {
        assert!(text.contains(word), "missing {word}");
    }
}

#[test]
fn help_documents_ascii_mode() {
    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains("--ascii"));
}

#[test]
fn version_and_help_show_navigation_keys() {
    let version = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .arg("--version")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        "gitview 0.3.0"
    );
    let help = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .arg("--help")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(text.contains("j/k"));
    assert!(text.contains("Enter"));
    assert!(text.contains("Esc"));
    assert!(text.contains(" h "));
    assert!(text.contains(" l "));
}
