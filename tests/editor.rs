#[cfg(unix)]
#[test]
fn edits_a_draft_with_a_quoted_editor_path_and_arguments() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("test editor");
    std::fs::write(&script, "#!/bin/sh\n[ \"$1\" = '--wait' ] || exit 3\nprintf 'feat: edited\\n\\nDescription\\n' > \"$2\"\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let command = format!("'{}' --wait", script.display());
    let message =
        gitview::editor::edit_with_command(directory.path(), &command, "original").unwrap();
    assert_eq!(message, "feat: edited\n\nDescription\n");
}

#[test]
fn rejects_shell_commands_and_reports_failed_editors() {
    let directory = tempfile::tempdir().unwrap();
    assert!(gitview::editor::edit_with_command(directory.path(), "editor | cat", "keep").is_err());
    assert!(
        gitview::editor::edit_with_command(directory.path(), "missing-gitview-test-editor", "keep")
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn reports_a_nonzero_editor_exit() {
    let directory = tempfile::tempdir().unwrap();
    let error =
        gitview::editor::edit_with_command(directory.path(), "/usr/bin/false", "keep").unwrap_err();
    assert!(error.contains("kept the original message"));
}
