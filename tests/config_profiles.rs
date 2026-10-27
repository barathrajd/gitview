use gitview::config::{
    FolderProfile, GitConfigContext, config_for_worktree, local_identity_overrides, select_profile,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "gitview-config-profiles-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn profile(name: &str, root: PathBuf) -> FolderProfile {
    FolderProfile {
        name: name.into(),
        worktree_root: root,
        config_path: PathBuf::from(format!("/profiles/{name}.gitconfig")),
        enabled: true,
    }
}

#[test]
fn profile_registry_round_trips_and_keeps_disabled_profiles() {
    let temp = TempDir::new();
    let config_dir = temp.path().join("gitview-config");
    let folder = temp.path().join("client");
    fs::create_dir_all(&folder).unwrap();
    let mut disabled = profile("old-client", folder.clone());
    disabled.enabled = false;
    let profiles = vec![disabled];

    gitview::config::save_profiles(&config_dir, &profiles).unwrap();

    assert_eq!(
        gitview::config::load_profiles(&config_dir).unwrap(),
        profiles
    );
}

#[test]
fn disabled_matching_profile_does_not_replace_the_primary_config() {
    let temp = TempDir::new();
    let folder = temp.path().join("client");
    let worktree = folder.join("repository");
    let config_dir = temp.path().join("gitview-config");
    fs::create_dir_all(&worktree).unwrap();
    fs::create_dir_all(config_dir.join("profiles")).unwrap();
    let profile_config = config_dir.join("profiles/client.gitconfig");
    fs::write(&profile_config, "[user]\n\tname = Client Account\n").unwrap();
    let mut disabled = FolderProfile {
        name: "client".into(),
        worktree_root: folder,
        config_path: PathBuf::from("profiles/client.gitconfig"),
        enabled: false,
    };
    disabled.enabled = false;
    gitview::config::save_profiles(&config_dir, &[disabled]).unwrap();

    let selected = config_for_worktree(&worktree, &config_dir).unwrap();

    assert!(selected.is_none());
}

#[test]
fn relative_profile_folders_resolve_from_gitview_config_directory() {
    let temp = TempDir::new();
    let config_dir = temp.path().join("gitview-config");
    let folder = config_dir.join("client work");
    let worktree = folder.join("repository");
    let profile_config = config_dir.join("profiles/client.gitconfig");
    fs::create_dir_all(&worktree).unwrap();
    fs::create_dir_all(profile_config.parent().unwrap()).unwrap();
    fs::write(&profile_config, "[user]\n\tname = Client Account\n").unwrap();
    gitview::config::save_profiles(
        &config_dir,
        &[FolderProfile {
            name: "client".into(),
            worktree_root: PathBuf::from("client work"),
            config_path: PathBuf::from("profiles/client.gitconfig"),
            enabled: true,
        }],
    )
    .unwrap();

    let selected = config_for_worktree(&worktree, &config_dir).unwrap();

    assert_eq!(selected, Some(fs::canonicalize(profile_config).unwrap()));
}

#[test]
fn uses_primary_profile_when_no_folder_rule_matches() {
    let temp = TempDir::new();
    let repository = temp.path().join("personal/project");
    fs::create_dir_all(&repository).unwrap();

    let selected = select_profile(&repository, &[]).unwrap();

    assert!(selected.is_none());
}

#[test]
fn most_specific_folder_rule_wins_for_nested_repositories() {
    let temp = TempDir::new();
    let office_root = temp.path().join("work");
    let client_root = office_root.join("client");
    let repository = client_root.join("project");
    fs::create_dir_all(&repository).unwrap();
    let profiles = [
        profile("office", office_root),
        profile("client", client_root),
    ];

    let selected = select_profile(&repository, &profiles).unwrap().unwrap();

    assert_eq!(selected.name, "client");
}

#[test]
fn path_prefixes_match_directories_not_similarly_named_siblings() {
    let temp = TempDir::new();
    let profile_root = temp.path().join("client");
    let sibling_repository = temp.path().join("client-tools/repository");
    fs::create_dir_all(&profile_root).unwrap();
    fs::create_dir_all(&sibling_repository).unwrap();
    let profiles = [profile("client", profile_root)];

    let selected = select_profile(&sibling_repository, &profiles).unwrap();

    assert!(selected.is_none());
}

#[test]
fn duplicate_canonical_folder_rules_are_reported_as_ambiguous() {
    let temp = TempDir::new();
    let profile_root = temp.path().join("client");
    let repository = profile_root.join("project");
    fs::create_dir_all(&repository).unwrap();
    let profiles = [
        profile("client-a", profile_root.clone()),
        profile("client-b", profile_root),
    ];

    let result = select_profile(&repository, &profiles);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("ambiguous"));
}

#[cfg(unix)]
#[test]
fn canonicalizes_worktree_paths_before_matching() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new();
    let real_root = temp.path().join("client");
    let repository = real_root.join("project");
    let alias = temp.path().join("client-alias");
    fs::create_dir_all(&repository).unwrap();
    symlink(&real_root, &alias).unwrap();
    let profiles = [profile("client", real_root)];

    let selected = select_profile(&alias.join("project"), &profiles)
        .unwrap()
        .unwrap();

    assert_eq!(selected.name, "client");
}

#[test]
fn reports_repository_local_identity_overrides() {
    let temp = TempDir::new();
    let repository = temp.path().join("repository");
    fs::create_dir_all(&repository).unwrap();
    let init = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&repository)
        .status()
        .unwrap();
    assert!(init.success());
    let config = std::process::Command::new("git")
        .args(["config", "--local", "user.email", "client@example.com"])
        .current_dir(&repository)
        .status()
        .unwrap();
    assert!(config.success());

    let overrides = local_identity_overrides(&repository).unwrap();

    assert_eq!(overrides, ["user.email"]);
}

#[test]
fn loads_the_selected_profile_global_config_from_registry() {
    let temp = TempDir::new();
    let worktree = temp.path().join("clients/office/repository");
    let config_dir = temp.path().join("gitview-config");
    let profile_config = config_dir.join("profiles/office.gitconfig");
    fs::create_dir_all(&worktree).unwrap();
    fs::create_dir_all(profile_config.parent().unwrap()).unwrap();
    fs::write(&profile_config, "[user]\n\tname = Office Account\n").unwrap();
    let registry = format!(
        r#"{{"profiles":[{{"name":"office","worktree_root":"{}","config_path":"{}"}}]}}"#,
        temp.path().join("clients/office").display(),
        profile_config.display(),
    );
    fs::write(config_dir.join("profiles.json"), registry).unwrap();

    let selected = config_for_worktree(&worktree, &config_dir).unwrap();

    assert_eq!(selected, Some(fs::canonicalize(profile_config).unwrap()));
}

#[test]
fn malformed_profile_registry_fails_instead_of_falling_back_to_primary() {
    let temp = TempDir::new();
    let worktree = temp.path().join("repository");
    let config_dir = temp.path().join("gitview-config");
    fs::create_dir_all(&worktree).unwrap();
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("profiles.json"), "not JSON").unwrap();

    let result = config_for_worktree(&worktree, &config_dir);

    assert!(result.is_err());
}

#[test]
fn selected_profile_replaces_global_config_for_git_commands() {
    let temp = TempDir::new();
    let primary_config = temp.path().join("primary.gitconfig");
    let profile_config = temp.path().join("office.gitconfig");
    fs::write(&primary_config, "[user]\n\tname = Primary Account\n").unwrap();
    fs::write(&profile_config, "[user]\n\tname = Office Account\n").unwrap();
    let context = GitConfigContext::new(Some(profile_config.clone()));
    let mut command = std::process::Command::new("git");
    command.args(["config", "--global", "user.name"]);
    command.env("GIT_CONFIG_GLOBAL", &primary_config);

    context.apply(&mut command);

    assert!(command.get_envs().any(
        |(key, value)| key == "GIT_CONFIG_GLOBAL" && value == Some(profile_config.as_os_str())
    ));
    let output = command.output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Office Account"
    );
}

#[test]
fn no_selected_profile_leaves_git_global_config_unmodified() {
    let context = GitConfigContext::new(None);
    let mut command = std::process::Command::new("git");

    context.apply(&mut command);

    assert!(
        !command
            .get_envs()
            .any(|(key, _)| key == "GIT_CONFIG_GLOBAL")
    );
}

#[test]
fn shared_git_command_factory_uses_the_installed_profile() {
    let temp = TempDir::new();
    let profile_config = temp.path().join("office.gitconfig");
    let updated_config = temp.path().join("contract.gitconfig");
    fs::write(&profile_config, "[user]\n\tname = Office Account\n").unwrap();
    fs::write(&updated_config, "[user]\n\tname = Contract Account\n").unwrap();
    gitview::process::install_git_config_context(GitConfigContext::new(Some(profile_config)))
        .unwrap();

    let read_name = || {
        gitview::process::git_command()
            .args(["config", "--global", "user.name"])
            .output()
            .unwrap()
    };
    let output = read_name();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Office Account"
    );

    gitview::process::update_git_config_context(GitConfigContext::new(Some(updated_config)))
        .unwrap();
    let output = read_name();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Contract Account"
    );
}

#[test]
fn missing_config_for_a_matching_profile_fails_closed() {
    let temp = TempDir::new();
    let worktree = temp.path().join("client/repository");
    let config_dir = temp.path().join("gitview-config");
    fs::create_dir_all(&worktree).unwrap();
    fs::create_dir_all(&config_dir).unwrap();
    let registry = format!(
        r#"{{"profiles":[{{"name":"client","worktree_root":"{}","config_path":"missing.gitconfig"}}]}}"#,
        temp.path().join("client").display(),
    );
    fs::write(config_dir.join("profiles.json"), registry).unwrap();

    let result = config_for_worktree(&worktree, &config_dir);

    assert!(result.is_err());
}
