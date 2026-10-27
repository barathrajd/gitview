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
    for word in ["Search", "Stage", "Unstage", "Refresh", "H", "S stages all"] {
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
fn shell_integration_flag_prints_a_reversible_zsh_hook() {
    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .arg("--shell-integration")
        .output()
        .unwrap();

    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("git() {"));
    assert!(text.contains("--profile-config-for"));
    assert!(text.contains("GIT_CONFIG_GLOBAL"));
    assert!(text.contains(r#"--profile-config-for "$PWD""#));
    assert!(!text.contains(r#"\""#));
}

#[test]
fn profile_config_flag_prints_the_config_selected_for_a_folder() {
    let temp = std::env::temp_dir().join(format!(
        "gitview-cli-profile-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config_home = temp.join("config-home");
    let profile_folder = temp.join("client work");
    let worktree = profile_folder.join("repository");
    let gitview_config = config_home.join("gitview");
    let profile_file = gitview_config.join("profiles/client.gitconfig");
    std::fs::create_dir_all(&worktree).unwrap();
    std::fs::create_dir_all(profile_file.parent().unwrap()).unwrap();
    std::fs::write(&profile_file, "[user]\n\tname = Client Account\n").unwrap();
    let registry = format!(
        r#"{{"profiles":[{{"name":"client","worktree_root":"{}","config_path":"profiles/client.gitconfig"}}]}}"#,
        profile_folder.display()
    );
    std::fs::write(gitview_config.join("profiles.json"), registry).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .args(["--profile-config-for", worktree.to_str().unwrap()])
        .env("XDG_CONFIG_HOME", &config_home)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        std::fs::canonicalize(profile_file)
            .unwrap()
            .to_string_lossy()
    );
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
fn zsh_hook_uses_the_folder_profile_for_terminal_git_commands() {
    if Command::new("zsh").arg("--version").output().is_err() {
        return;
    }
    let temp = std::env::temp_dir().join(format!(
        "gitview-zsh-profile-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config_home = temp.join("config-home");
    let profile_folder = temp.join("client-work");
    let worktree = profile_folder.join("repository");
    let gitview_config = config_home.join("gitview");
    let profile_file = gitview_config.join("profiles/client.gitconfig");
    let primary_file = temp.join("primary.gitconfig");
    std::fs::create_dir_all(&worktree).unwrap();
    std::fs::create_dir_all(profile_file.parent().unwrap()).unwrap();
    std::fs::write(&profile_file, "[user]\n\tname = Client Account\n").unwrap();
    std::fs::write(&primary_file, "[user]\n\tname = Primary Account\n").unwrap();
    let registry = format!(
        r#"{{"profiles":[{{"name":"client","worktree_root":"{}","config_path":"profiles/client.gitconfig"}}]}}"#,
        profile_folder.display()
    );
    std::fs::write(gitview_config.join("profiles.json"), registry).unwrap();
    let binary = std::path::PathBuf::from(env!("CARGO_BIN_EXE_gitview"));
    let bin_dir = binary.parent().unwrap();
    let path = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let output = Command::new("zsh")
        .args([
            "-c",
            "eval \"$(gitview --shell-integration)\"; cd \"$PROFILE_WORKTREE\" && git config --global user.name",
        ])
        .env("PROFILE_WORKTREE", &worktree)
        .env("XDG_CONFIG_HOME", &config_home)
        .env("GIT_CONFIG_GLOBAL", &primary_file)
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Client Account"
    );
    std::fs::remove_dir_all(temp).unwrap();
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
        concat!("gitview ", env!("CARGO_PKG_VERSION"))
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
