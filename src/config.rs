use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::process;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct FolderProfile {
    pub name: String,
    pub worktree_root: PathBuf,
    pub config_path: PathBuf,
    #[serde(default = "profile_enabled_by_default")]
    pub enabled: bool,
}

#[derive(Deserialize, Serialize)]
struct ProfileRegistry {
    #[serde(default)]
    profiles: Vec<FolderProfile>,
}

fn profile_enabled_by_default() -> bool {
    true
}

static NEXT_REGISTRY_TEMP: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitConfigContext {
    profile_name: Option<String>,
    global_config: Option<PathBuf>,
}

impl GitConfigContext {
    pub fn new(global_config: Option<PathBuf>) -> Self {
        Self {
            profile_name: None,
            global_config,
        }
    }

    pub fn for_profile(profile_name: String, global_config: PathBuf) -> Self {
        Self {
            profile_name: Some(profile_name),
            global_config: Some(global_config),
        }
    }

    pub fn apply(&self, command: &mut Command) {
        if let Some(global_config) = &self.global_config {
            command.env("GIT_CONFIG_GLOBAL", global_config);
        }
    }

    pub fn global_config(&self) -> Option<&Path> {
        self.global_config.as_deref()
    }

    pub fn profile_name(&self) -> Option<&str> {
        self.profile_name.as_deref()
    }
}

/// Load the profile config file selected by the repository's folder mapping.
///
/// The registry lives in `<config_dir>/profiles.json`. Missing registries use
/// Git's normal primary global config. Invalid registries and missing selected
/// profile files return errors rather than silently switching accounts.
pub fn config_for_worktree(
    worktree_root: &Path,
    config_dir: &Path,
) -> Result<Option<PathBuf>, String> {
    Ok(profile_for_worktree(worktree_root, config_dir)?.map(|profile| profile.config_path))
}

/// Load the selected profile, including its canonical Git config path.
pub fn profile_for_worktree(
    worktree_root: &Path,
    config_dir: &Path,
) -> Result<Option<FolderProfile>, String> {
    let profiles = load_profiles(config_dir)?;
    let normalized_profiles: Vec<_> = profiles
        .iter()
        .cloned()
        .map(|mut profile| {
            if !profile.worktree_root.is_absolute() {
                profile.worktree_root = config_dir.join(&profile.worktree_root);
            }
            profile
        })
        .collect();
    let Some(mut profile) = select_profile(worktree_root, &normalized_profiles)?.cloned() else {
        return Ok(None);
    };
    let config_path = if profile.config_path.is_absolute() {
        profile.config_path.clone()
    } else {
        config_dir.join(&profile.config_path)
    };
    profile.config_path = canonicalize(&config_path, "profile config")?;
    if !profile.config_path.is_file() {
        return Err(format!(
            "Git profile '{}' config is not a file: '{}'",
            profile.name,
            profile.config_path.display()
        ));
    }
    Ok(Some(profile))
}

/// Resolve the user's Gitview configuration directory.
pub fn config_dir() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(path).join("gitview"));
    }
    let home = std::env::var_os("HOME")
        .ok_or_else(|| "Could not find HOME or XDG_CONFIG_HOME for Gitview settings".to_string())?;
    Ok(PathBuf::from(home).join(".config").join("gitview"))
}

/// Read the profile registry. A missing registry means no folder profiles.
pub fn load_profiles(config_dir: &Path) -> Result<Vec<FolderProfile>, String> {
    let registry_path = config_dir.join("profiles.json");
    let bytes = match fs::read(&registry_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "Could not read Git profile registry '{}': {error}",
                registry_path.display()
            ));
        }
    };
    let registry: ProfileRegistry = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "Could not parse Git profile registry '{}': {error}",
            registry_path.display()
        )
    })?;
    Ok(registry.profiles)
}

/// Atomically save the profile registry with owner-only permissions on Unix.
pub fn save_profiles(config_dir: &Path, profiles: &[FolderProfile]) -> Result<(), String> {
    fs::create_dir_all(config_dir).map_err(|error| {
        format!(
            "Could not create Gitview config folder '{}': {error}",
            config_dir.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(config_dir, fs::Permissions::from_mode(0o700)).map_err(|error| {
            format!(
                "Could not protect Gitview config folder '{}': {error}",
                config_dir.display()
            )
        })?;
    }

    let registry_path = config_dir.join("profiles.json");
    let temp_path = config_dir.join(format!(
        ".profiles.json.{}.{}.tmp",
        std::process::id(),
        NEXT_REGISTRY_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    let data = serde_json::to_vec_pretty(&ProfileRegistry {
        profiles: profiles.to_vec(),
    })
    .map_err(|error| format!("Could not encode Git profile registry: {error}"))?;

    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp_path).map_err(|error| {
            format!(
                "Could not create temporary Git profile registry '{}': {error}",
                temp_path.display()
            )
        })?;
        file.write_all(&data)
            .map_err(|error| format!("Could not write Git profile registry: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Could not sync Git profile registry: {error}"))?;
        fs::rename(&temp_path, &registry_path).map_err(|error| {
            format!(
                "Could not replace Git profile registry '{}': {error}",
                registry_path.display()
            )
        })?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

/// Select the most specific folder profile for a canonical worktree path.
///
/// A profile applies to its configured directory and all descendants. If two
/// profiles resolve to the same directory, selection fails instead of choosing
/// one implicitly.
pub fn select_profile<'a>(
    worktree_root: &Path,
    profiles: &'a [FolderProfile],
) -> Result<Option<&'a FolderProfile>, String> {
    let worktree_root = canonicalize(worktree_root, "worktree")?;
    let mut selected: Option<(&FolderProfile, PathBuf, usize)> = None;

    for profile in profiles {
        if !profile.enabled {
            continue;
        }
        let profile_root = canonicalize(&profile.worktree_root, "profile folder")?;
        if !worktree_root.starts_with(&profile_root) {
            continue;
        }

        let depth = profile_root.components().count();
        if let Some((selected_profile, selected_root, selected_depth)) = &selected
            && depth == *selected_depth
            && profile_root == *selected_root
        {
            return Err(format!(
                "Git profile selection is ambiguous: '{}' and '{}' both match {}",
                selected_profile.name,
                profile.name,
                selected_root.display()
            ));
        }
        if selected
            .as_ref()
            .is_none_or(|(_, _, selected_depth)| depth > *selected_depth)
        {
            selected = Some((profile, profile_root, depth));
        }
    }

    Ok(selected.map(|(profile, _, _)| profile))
}

/// Return repository-local identity keys that take precedence over a profile.
pub fn local_identity_overrides(worktree_root: &Path) -> Result<Vec<String>, String> {
    let output = process::git_command()
        .args([
            "config",
            "--local",
            "--name-only",
            "--null",
            "--get-regexp",
            r"^user\.(name|email|signingkey)$",
        ])
        .current_dir(worktree_root)
        .output()
        .map_err(|error| format!("Could not inspect repository identity settings: {error}"))?;

    if !output.status.success() && output.status.code() != Some(1) {
        return Err(format!(
            "Could not inspect repository identity settings: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|key| !key.is_empty())
        .map(|key| String::from_utf8_lossy(key).into_owned())
        .collect())
}

fn canonicalize(path: &Path, label: &str) -> Result<PathBuf, String> {
    fs::canonicalize(path).map_err(|error| {
        format!(
            "Could not resolve {label} path '{}': {error}",
            path.display()
        )
    })
}
