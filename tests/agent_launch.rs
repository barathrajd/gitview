//! Choosing the program for the agent pane from the command line, and telling
//! the user plainly when it is not installed.

use std::{
    ffi::{OsStr, OsString},
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
};

use gitview::pane::{check_installed, find_executable, register_gitview_mcp, split_agent_args};

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn executable(dir: &Path, name: &str) {
    let path = dir.join(name);
    fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn without_an_agent_flag_all_arguments_belong_to_gitview() {
    let (own, agent) = split_agent_args(&os(&["--ascii", "--version"])).unwrap();

    assert_eq!(own, os(&["--ascii", "--version"]));
    assert!(agent.is_none());
}

#[test]
fn run_takes_everything_after_it_as_the_command() {
    let (own, agent) = split_agent_args(&os(&["--ascii", "--run", "sh", "-c", "echo hi"])).unwrap();
    let agent = agent.unwrap();

    assert_eq!(own, os(&["--ascii"]));
    assert_eq!(agent.argv, ["sh", "-c", "echo hi"]);
    assert_eq!(agent.product, None);
}

#[test]
fn flags_after_run_belong_to_the_program_not_to_gitview() {
    let (own, agent) = split_agent_args(&os(&["--run", "ls", "--help", "--version"])).unwrap();

    assert!(own.is_empty());
    assert_eq!(agent.unwrap().argv, ["ls", "--help", "--version"]);
}

#[test]
fn claude_and_codex_are_shortcuts_for_those_commands() {
    let (_, claude) = split_agent_args(&os(&["--claude"])).unwrap();
    let claude = claude.unwrap();
    assert_eq!(claude.argv, ["claude"]);
    assert!(claude.product.unwrap().contains("Claude Code"));

    let (_, codex) = split_agent_args(&os(&["--ascii", "--codex"])).unwrap();
    let codex = codex.unwrap();
    assert_eq!(codex.argv, ["codex"]);
    assert!(codex.product.unwrap().contains("Codex"));
}

#[test]
fn a_shortcut_passes_the_rest_of_the_line_to_the_program() {
    let (own, agent) = split_agent_args(&os(&["--claude", "--resume", "--model", "opus"])).unwrap();

    assert!(own.is_empty());
    assert_eq!(
        agent.unwrap().argv,
        ["claude", "--resume", "--model", "opus"]
    );
}

#[test]
fn run_without_a_command_is_an_error_with_an_example() {
    let error = split_agent_args(&os(&["--run"])).unwrap_err();

    assert!(error.contains("--run needs a command"), "{error}");
    assert!(error.contains("gitview --run claude"), "{error}");
}

#[test]
fn a_command_that_is_not_valid_text_is_an_error() {
    use std::os::unix::ffi::OsStringExt;
    let bad = OsString::from_vec(vec![0x66, 0xff, 0x6f]);

    let error = split_agent_args(&[OsString::from("--run"), bad]).unwrap_err();

    assert!(error.contains("UTF-8"), "{error}");
}

#[test]
fn finds_an_executable_on_the_search_path() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    executable(second.path(), "agent-tool");
    let path = std::env::join_paths([first.path(), second.path()]).unwrap();

    assert_eq!(
        find_executable("agent-tool", Some(&path)),
        Some(second.path().join("agent-tool"))
    );
    assert_eq!(find_executable("missing-tool", Some(&path)), None);
}

#[test]
fn ignores_files_that_are_not_executable_or_are_folders() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("plain-file"), "text").unwrap();
    fs::create_dir(dir.path().join("a-folder")).unwrap();
    let path = std::env::join_paths([dir.path()]).unwrap();

    assert_eq!(find_executable("plain-file", Some(&path)), None);
    assert_eq!(find_executable("a-folder", Some(&path)), None);
}

#[test]
fn a_name_with_a_slash_is_checked_where_it_points_instead_of_on_the_path() {
    let dir = tempfile::tempdir().unwrap();
    executable(dir.path(), "local-tool");
    let elsewhere = tempfile::tempdir().unwrap();
    let path = std::env::join_paths([elsewhere.path()]).unwrap();
    let full = dir.path().join("local-tool");

    assert_eq!(
        find_executable(full.to_str().unwrap(), Some(&path)),
        Some(full)
    );
    assert_eq!(
        find_executable(dir.path().join("nope").to_str().unwrap(), Some(&path)),
        None
    );
}

#[test]
fn an_empty_search_path_finds_nothing() {
    assert_eq!(find_executable("sh", Some(OsStr::new(""))), None);
    assert_eq!(find_executable("sh", None), None);
}

#[test]
fn a_missing_shortcut_program_gets_a_message_that_says_what_to_do() {
    let (_, agent) = split_agent_args(&os(&["--codex"])).unwrap();
    let empty = tempfile::tempdir().unwrap();
    let path = std::env::join_paths([empty.path()]).unwrap();

    let error = check_installed(&agent.unwrap(), Some(&path)).unwrap_err();

    assert!(error.contains("`codex`"), "{error}");
    assert!(error.contains("Codex"), "{error}");
    assert!(error.contains("PATH"), "{error}");
    assert!(error.contains("gitview --run"), "{error}");
}

#[test]
fn a_missing_run_command_names_the_command_and_suggests_a_path() {
    let (_, agent) = split_agent_args(&os(&["--run", "no-such-agent", "--flag"])).unwrap();
    let empty = tempfile::tempdir().unwrap();
    let path = std::env::join_paths([empty.path()]).unwrap();

    let error = check_installed(&agent.unwrap(), Some(&path)).unwrap_err();

    assert!(error.contains("`no-such-agent`"), "{error}");
    assert!(error.contains("PATH"), "{error}");
    assert!(error.contains("./"), "{error}");
}

#[test]
fn an_installed_program_passes_the_check() {
    let dir = tempfile::tempdir().unwrap();
    executable(dir.path(), "claude");
    let path = std::env::join_paths([dir.path()]).unwrap();
    let (_, agent) = split_agent_args(&os(&["--claude"])).unwrap();

    assert!(check_installed(&agent.unwrap(), Some(&path)).is_ok());
}

#[test]
fn the_binary_reports_a_missing_agent_before_anything_else() {
    // Not a Git repository and no `claude`: the program problem is reported
    // first, and the terminal is never touched.
    let folder = tempfile::tempdir().unwrap();
    let empty = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .current_dir(folder.path())
        .env("PATH", empty.path())
        .arg("--claude")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`claude`"), "{stderr}");
    assert!(!stderr.contains("Not in a Git repository"), "{stderr}");
    assert!(!String::from_utf8_lossy(&output.stdout).contains('\u{1b}'));
}

#[test]
fn a_program_in_the_current_folder_is_not_found_through_an_empty_path_entry() {
    // A repository could ship an executable named like the agent. An empty
    // search path must not make Gitview run it.
    let repository = tempfile::tempdir().unwrap();
    executable(repository.path(), "claude");

    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .current_dir(repository.path())
        .env("PATH", "")
        .arg("--claude")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("was not found on your PATH"), "{stderr}");
}

#[test]
fn claude_is_given_the_gitview_mcp_server_before_its_own_arguments() {
    let (_, request) = split_agent_args(&os(&["--claude", "explain this repo"])).unwrap();
    let request = register_gitview_mcp(request.unwrap(), Path::new("/opt/my bin/gitview"));

    assert_eq!(request.argv[0], "claude");
    let config = request.argv[1].strip_prefix("--mcp-config=").unwrap();
    let config: serde_json::Value = serde_json::from_str(config).unwrap();
    assert_eq!(
        config,
        serde_json::json!({"mcpServers": {"gitview": {
            "type": "stdio", "command": "/opt/my bin/gitview", "args": ["mcp"]}}})
    );
    assert_eq!(request.argv[2..], ["explain this repo"]);
}

#[test]
fn other_agents_are_left_as_given() {
    for args in [&["--codex", "x"][..], &["--run", "claude", "x"]] {
        let (_, request) = split_agent_args(&os(args)).unwrap();
        let request = request.unwrap();
        assert_eq!(
            register_gitview_mcp(request.clone(), Path::new("/g")),
            request
        );
    }
}
