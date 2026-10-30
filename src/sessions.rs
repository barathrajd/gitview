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

/// What a session would bring back to its base branch, as of when it was
/// reviewed. Merging checks that nothing has moved since.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Review {
    pub session: Session,
    /// The session's commits, newest first, as `<short id> <subject>`.
    pub commits: Vec<String>,
    /// The session's changes against where it started.
    pub diff: String,
    base_tip: String,
    session_tip: String,
}

pub fn review(root: &Path, session: &Session) -> Result<Review, String> {
    let base_tip = branch_tip(root, &session.base_branch)?;
    let session_tip = branch_tip(root, &session.branch)?;
    let range = format!("{}..{session_tip}", session.base_commit);
    let commits = git(root, &["log", "--format=%h %s", &range])?
        .lines()
        .map(str::to_owned)
        .collect();
    let mut command = process::git_command();
    command
        .current_dir(root)
        .args(["diff", "--no-color", "--no-ext-diff", "--find-renames"])
        .arg(format!("{}...{session_tip}", session.base_commit))
        .arg("--");
    let output = process::limited_output(&mut command, 1_000_000)?;
    let mut diff = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.truncated {
        diff.insert_str(0, "[diff truncated at 1 MB or 5000 lines]\n");
    }
    Ok(Review {
        session: session.clone(),
        commits,
        diff,
        base_tip,
        session_tip,
    })
}

/// Merge a reviewed session into its base branch with a merge commit, in the
/// worktree at `root`. Refuses unless that worktree is on the base branch and
/// clean, the session has no uncommitted work, and neither branch moved since
/// the review. A conflict aborts the merge and leaves everything as it was.
pub fn merge(root: &Path, review: &Review) -> Result<(), String> {
    let session = &review.session;
    let current = git(root, &["symbolic-ref", "--short", "-q", "HEAD"]).unwrap_or_default();
    if current != session.base_branch {
        return Err(format!(
            "Switch to {} before merging {}: the session merges into the branch it started from",
            session.base_branch, session.name
        ));
    }
    if !git(root, &["status", "--porcelain", "--untracked-files=no"])?.is_empty() {
        return Err(
            "The working tree has uncommitted changes; commit or stash them before merging".into(),
        );
    }
    if session.path.is_dir()
        && !git(
            &session.path,
            &["status", "--porcelain", "--untracked-files=all"],
        )?
        .is_empty()
    {
        return Err(format!(
            "{} has uncommitted work that a merge would leave out; ask the agent to commit it or discard it",
            session.name
        ));
    }
    if branch_tip(root, &session.base_branch)? != review.base_tip
        || branch_tip(root, &session.branch)? != review.session_tip
    {
        return Err(format!(
            "{} or {} changed since you reviewed; review again",
            session.base_branch, session.branch
        ));
    }
    let message = format!("Merge {} ({})", session.name, session.branch);
    let merged = process::git_command()
        .current_dir(root)
        .args([
            "merge",
            "--no-ff",
            "--no-edit",
            "-m",
            &message,
            &review.session_tip,
        ])
        .output()
        .map_err(|error| format!("Could not run Git: {error}"))?;
    if merged.status.success() {
        return Ok(());
    }
    let _ = git(root, &["merge", "--abort"]);
    let detail = String::from_utf8_lossy(&merged.stdout);
    if detail.contains("CONFLICT") {
        Err(format!(
            "Merging {} stopped on a conflict, so nothing changed; resolve it in the session first",
            session.name
        ))
    } else {
        Err(format!(
            "Could not merge {}: {}",
            session.name,
            String::from_utf8_lossy(&merged.stderr).trim()
        ))
    }
}

fn branch_tip(root: &Path, branch: &str) -> Result<String, String> {
    git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .map_err(|_| format!("The branch {branch} no longer exists"))
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
