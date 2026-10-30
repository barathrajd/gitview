//! Agent sessions: each runs in its own worktree on its own branch, created
//! from the branch the user is on. Worktrees live under
//! `<git common dir>/gitview/sessions`, so the main worktree never sees them,
//! and `manifest.json` beside them records each session, so a crash can't
//! leave one behind unnoticed.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::process;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    pub branch: String,
    /// The branch the session started from, and merges back into.
    pub base_branch: String,
    /// Where the base branch was when the session started.
    pub base_commit: String,
    pub path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Ready,
    /// The worktree folder is gone, for example deleted by hand.
    MissingWorktree,
    /// The session's branch is gone.
    MissingBranch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionStatus {
    pub session: Session,
    pub health: Health,
    /// The worktree has uncommitted or untracked changes.
    pub dirty: bool,
    /// Commits on the session branch that its base commit doesn't have.
    pub ahead: usize,
}

#[derive(Default, Serialize, Deserialize)]
struct Manifest {
    sessions: Vec<Session>,
}

/// Start a session from the branch checked out at `root`.
pub fn create(root: &Path) -> Result<Session, String> {
    let base_commit = git(root, &["rev-parse", "--verify", "--quiet", "HEAD"])
        .map_err(|_| "Make a first commit before starting a session".to_owned())?;
    let base_branch = git(root, &["symbolic-ref", "--short", "-q", "HEAD"])
        .map_err(|_| "Start a session while on a branch: it merges back into the branch it starts from, so it can't start on a detached HEAD".to_owned())?;
    let folder = sessions_dir(root)?;
    fs::create_dir_all(&folder)
        .map_err(|error| format!("Could not create {}: {error}", folder.display()))?;
    let folder = fs::canonicalize(&folder)
        .map_err(|error| format!("Could not resolve {}: {error}", folder.display()))?;
    let mut manifest = read_manifest(&folder)?;
    let name = unique_name(root, &manifest, &timestamp())?;
    let session = Session {
        branch: format!("gitview/{name}"),
        path: folder.join(&name),
        name,
        base_branch,
        base_commit,
    };
    let path = session
        .path
        .to_str()
        .ok_or("The repository path isn't valid UTF-8")?;
    git(
        root,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            &session.branch,
            path,
            &session.base_commit,
        ],
    )?;
    manifest.sessions.push(session.clone());
    if let Err(error) = write_manifest(&folder, &manifest) {
        // Don't leave a worktree the manifest doesn't know about.
        let _ = git(root, &["worktree", "remove", "--force", path]);
        let _ = git(root, &["branch", "-D", &session.branch]);
        return Err(error);
    }
    Ok(session)
}

/// Every recorded session, oldest first, with its current state.
pub fn list(root: &Path) -> Result<Vec<SessionStatus>, String> {
    let folder = sessions_dir(root)?;
    Ok(read_manifest(&folder)?
        .sessions
        .into_iter()
        .map(|session| status(root, session))
        .collect())
}

fn status(root: &Path, session: Session) -> SessionStatus {
    let branch_ref = format!("refs/heads/{}", session.branch);
    let health = if git(root, &["rev-parse", "--verify", "--quiet", &branch_ref]).is_err() {
        Health::MissingBranch
    } else if !session.path.is_dir() {
        Health::MissingWorktree
    } else {
        Health::Ready
    };
    let ahead = git(
        root,
        &[
            "rev-list",
            "--count",
            &format!("{}..{branch_ref}", session.base_commit),
        ],
    )
    .ok()
    .and_then(|count| count.parse().ok())
    .unwrap_or(0);
    let dirty = health == Health::Ready
        && git(
            &session.path,
            &["status", "--porcelain", "--untracked-files=all"],
        )
        .is_ok_and(|output| !output.is_empty());
    SessionStatus {
        session,
        health,
        dirty,
        ahead,
    }
}

fn sessions_dir(root: &Path) -> Result<PathBuf, String> {
    let common = git(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    Ok(PathBuf::from(common).join("gitview").join("sessions"))
}

fn read_manifest(folder: &Path) -> Result<Manifest, String> {
    match fs::read(folder.join("manifest.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("The session manifest is damaged: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Manifest::default()),
        Err(error) => Err(format!("Could not read the session manifest: {error}")),
    }
}

/// Replace the manifest in one step, so a crash leaves the old or the new one.
fn write_manifest(folder: &Path, manifest: &Manifest) -> Result<(), String> {
    let text = serde_json::to_vec_pretty(manifest).map_err(|error| error.to_string())?;
    let temporary = folder.join("manifest.json.tmp");
    fs::write(&temporary, text)
        .and_then(|()| fs::rename(&temporary, folder.join("manifest.json")))
        .map_err(|error| format!("Could not save the session manifest: {error}"))
}

fn unique_name(root: &Path, manifest: &Manifest, stamp: &str) -> Result<String, String> {
    for suffix in 1..100 {
        let name = if suffix == 1 {
            format!("session-{stamp}")
        } else {
            format!("session-{stamp}-{suffix}")
        };
        let taken = manifest.sessions.iter().any(|session| session.name == name)
            || git(
                root,
                &[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    &format!("refs/heads/gitview/{name}"),
                ],
            )
            .is_ok();
        if !taken {
            return Ok(name);
        }
    }
    Err("Too many sessions started this second".into())
}

/// The current UTC time as `yyyymmdd-hhmmss`.
fn timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    let (days, time) = (seconds / 86_400, seconds % 86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        time / 3600,
        time % 3600 / 60,
        time % 60
    )
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = process::git_command()
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|error| format!("Could not run Git: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(format!(
            "Git {} failed: {}",
            args.first().copied().unwrap_or_default(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
