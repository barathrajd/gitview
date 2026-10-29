use std::{
    fs,
    path::Path,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("gitview-cli-{name}-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn help_describes_usage_without_a_repository() {
    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .arg("--help")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("Usage: gitview"));
    assert!(help.contains("gitview init [directory]"));
    assert!(help.contains("gitview clone <repository> [directory]"));
}

#[test]
fn init_command_creates_a_repository_at_the_requested_path() {
    let temp = TempDir::new("init");
    let destination = temp.0.join("new repository");
    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .args(["init"])
        .arg(&destination)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(destination.join(".git").exists());
}

#[test]
fn clone_command_clones_a_local_repository_to_a_path_with_spaces() {
    let temp = TempDir::new("clone");
    let source = temp.0.join("source");
    fs::create_dir(&source).unwrap();
    git(&source, &["init", "-q"]);
    git(&source, &["config", "user.name", "Gitview Test"]);
    git(&source, &["config", "user.email", "gitview@example.com"]);
    fs::write(source.join("README.md"), "ready\n").unwrap();
    git(&source, &["add", "README.md"]);
    git(&source, &["commit", "-qm", "initial"]);
    let destination = temp.0.join("cloned repository");
    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .arg("clone")
        .arg(&source)
        .arg(&destination)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(destination.join("README.md")).unwrap(), b"ready\n");
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

#[test]
fn help_documents_the_agent_pane() {
    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .arg("--help")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();

    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "gitview [--ascii] --run <command> [args...]",
        "gitview [--ascii] --claude [args...]",
        "gitview [--ascii] --codex [args...]",
        "Ctrl+\\",
        "Everything after --run, --claude, or --codex goes to that program",
        "full screen",
    ] {
        assert!(
            help.contains(expected),
            "help is missing {expected:?}:\n{help}"
        );
    }
}

#[test]
fn flags_after_an_agent_flag_belong_to_the_program() {
    // `--help` after `--run` is the program's, so Gitview does not print its own
    // help. With no such program installed the run fails with the program error,
    // not with the usage text.
    let output = Command::new(env!("CARGO_BIN_EXE_gitview"))
        .args(["--run", "gitview-no-such-agent", "--help"])
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Usage: gitview"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("`gitview-no-such-agent`"));
}
