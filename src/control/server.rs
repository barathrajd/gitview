//! The control socket: one JSON request per line in, one JSON reply per line
//! out. Only the user running Gitview can connect.

use std::{
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, PermissionsExt},
        io::AsRawFd,
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::JoinHandle,
    time::Duration,
};

use serde_json::{Map, Value, json};

use super::tools;

/// The longest request line accepted; a longer one closes the connection.
pub const MAX_REQUEST: usize = 64 * 1024;
/// The most connections served at once; more are turned away.
pub const MAX_CONNECTIONS: usize = 8;
/// How long a connection may wait without sending a request.
pub const IDLE_LIMIT: Duration = Duration::from_secs(600);

/// What the agent asked Gitview to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlCommand {
    /// A commit by its full ID, and optionally one of its files to open.
    SelectCommit {
        id: String,
        path: Option<String>,
    },
    SelectFile {
        path: String,
        staged: bool,
    },
    FocusAgent,
    FocusGitview,
}

pub struct ControlServer {
    path: PathBuf,
    stop: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
    accept: Option<JoinHandle<()>>,
}

impl ControlServer {
    /// Listen in the user's private runtime folder.
    pub fn start(root: &Path) -> Result<(Self, Receiver<ControlCommand>), String> {
        Self::start_in(root, &default_folder())
    }

    /// Listen in `folder`, creating it if needed. An existing folder must be
    /// owned by this user and closed to everyone else.
    pub fn start_in(
        root: &Path,
        folder: &Path,
    ) -> Result<(Self, Receiver<ControlCommand>), String> {
        Self::start_with_idle_limit(root, folder, IDLE_LIMIT)
    }

    /// Like `start_in`, closing connections idle for longer than `idle`.
    pub fn start_with_idle_limit(
        root: &Path,
        folder: &Path,
        idle: Duration,
    ) -> Result<(Self, Receiver<ControlCommand>), String> {
        prepare_folder(folder)?;
        // Unique per server: a second server must never replace this one's
        // socket, or dropping this one would wait on a listener it can't wake.
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = folder.join(format!(
            "gitview-{}-{}.sock",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_file(&path);
        let listener = UnixListener::bind(&path)
            .map_err(|error| format!("Could not open the control socket: {error}"))?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("Could not protect the control socket: {error}"))?;
        let (sender, commands) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let busy = Arc::new(AtomicBool::new(false));
        let accept = {
            let stop = Arc::clone(&stop);
            let busy = Arc::clone(&busy);
            let root = root.to_owned();
            std::thread::spawn(move || accept_loop(listener, root, sender, stop, busy, idle))
        };
        Ok((
            Self {
                path,
                stop,
                busy,
                accept: Some(accept),
            },
            commands,
        ))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Tell agents that a selection will wait, because the user has a prompt
    /// or dialog open.
    pub fn set_busy(&self, busy: bool) {
        self.busy.store(busy, Ordering::Relaxed);
    }
}

impl Drop for ControlServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the blocked accept so the thread sees the flag.
        let _ = UnixStream::connect(&self.path);
        let _ = fs::remove_file(&self.path);
        if let Some(accept) = self.accept.take() {
            let _ = accept.join();
        }
    }
}

fn default_folder() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(runtime) if !runtime.is_empty() => PathBuf::from(runtime).join("gitview"),
        _ => std::env::temp_dir().join(format!("gitview-{}", unsafe { libc::geteuid() })),
    }
}

fn prepare_folder(folder: &Path) -> Result<(), String> {
    match fs::DirBuilder::new().mode(0o700).create(folder) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(format!("Could not create {}: {error}", folder.display())),
    }
    let metadata = fs::symlink_metadata(folder)
        .map_err(|error| format!("Could not inspect {}: {error}", folder.display()))?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err(format!(
            "{} must be a private folder owned by you (mode 0700)",
            folder.display()
        ));
    }
    Ok(())
}

fn accept_loop(
    listener: UnixListener,
    root: PathBuf,
    sender: Sender<ControlCommand>,
    stop: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
    idle: Duration,
) {
    let open = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let Ok(stream) = stream else { continue };
        if peer_uid(&stream) != Some(unsafe { libc::geteuid() }) {
            continue;
        }
        if open.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
            let mut stream = stream;
            let reply = failure(
                &Value::Null,
                "Too many connections to Gitview; close one and retry",
            );
            let _ = writeln!(stream, "{reply}");
            continue;
        }
        if stream.set_read_timeout(Some(idle)).is_err() {
            continue;
        }
        open.fetch_add(1, Ordering::SeqCst);
        let root = root.clone();
        let sender = sender.clone();
        let busy = Arc::clone(&busy);
        let open = Arc::clone(&open);
        std::thread::spawn(move || {
            // An idle connection's read times out and ends `serve`.
            let _ = serve(stream, &root, &sender, &busy);
            open.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

#[cfg(target_os = "linux")]
fn peer_uid(stream: &UnixStream) -> Option<libc::uid_t> {
    let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut length,
        )
    };
    (result == 0).then_some(credentials.uid)
}

#[cfg(not(target_os = "linux"))]
fn peer_uid(stream: &UnixStream) -> Option<libc::uid_t> {
    let (mut uid, mut gid) = (0, 0);
    let result = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    (result == 0).then_some(uid)
}

fn serve(
    stream: UnixStream,
    root: &Path,
    sender: &Sender<ControlCommand>,
    busy: &AtomicBool,
) -> io::Result<()> {
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    loop {
        line.clear();
        (&mut reader)
            .take(MAX_REQUEST as u64 + 1)
            .read_until(b'\n', &mut line)?;
        if line.is_empty() {
            return Ok(());
        }
        if line.len() > MAX_REQUEST {
            let reply = failure(&Value::Null, "Request is larger than 64 KiB");
            return writeln!(writer, "{reply}");
        }
        let reply = respond(&line, root, sender, busy);
        writeln!(writer, "{reply}")?;
    }
}

fn respond(line: &[u8], root: &Path, sender: &Sender<ControlCommand>, busy: &AtomicBool) -> Value {
    let request: Map<String, Value> = match serde_json::from_slice(line) {
        Ok(request) => request,
        Err(error) => return failure(&Value::Null, &format!("Invalid request: {error}")),
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let Some(tool) = request.get("tool").and_then(Value::as_str) else {
        return failure(&id, "Request needs a tool name");
    };
    let args = request.get("args").cloned().unwrap_or_else(|| json!({}));
    let result = match selection(root, tool, &args) {
        Some(selected) => selected.and_then(|(command, mut text)| {
            if busy.load(Ordering::Relaxed) {
                text = format!(
                    "{}, once the user closes the prompt they have open.",
                    text.trim_end_matches('.')
                );
            }
            sender
                .send(command)
                .map_err(|_| "Gitview is closing".to_owned())
                .map(|()| tools::ToolOutput {
                    text,
                    truncated: false,
                })
        }),
        None => tools::call(root, tool, &args),
    };
    match result {
        Ok(output) => json!({
            "id": id,
            "ok": true,
            "text": output.text,
            "truncated": output.truncated,
        }),
        Err(error) => failure(&id, &error),
    }
}

/// Parse a tool that changes what Gitview shows, with a reply saying what it
/// will show, or `None` for other tools.
fn selection(
    root: &Path,
    tool: &str,
    args: &Value,
) -> Option<Result<(ControlCommand, String), String>> {
    let text = |key: &str| {
        args.get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(format!("{key} must be a string"))
    };
    Some(match tool {
        "select_commit" => (|| {
            let id = tools::resolve_commit(root, &text("revision")?)?;
            let commit = crate::details::load_commit(root, &id)?;
            let label = format!("commit {} ({})", &id[..7.min(id.len())], commit.subject);
            match args.get("path") {
                None => Ok((
                    ControlCommand::SelectCommit { id, path: None },
                    format!("Gitview will open {label}."),
                )),
                Some(_) => {
                    let path = text("path")?;
                    tools::check_relative(&path)?;
                    if !commit.files.iter().any(|file| *file == *path) {
                        return Err(format!("{path} didn't change in {label}"));
                    }
                    let reply = format!("Gitview will open the diff of {path} in {label}.");
                    Ok((
                        ControlCommand::SelectCommit {
                            id,
                            path: Some(path),
                        },
                        reply,
                    ))
                }
            }
        })(),
        "select_file" => (|| {
            let path = text("path")?;
            tools::check_relative(&path)?;
            let staged = match args.get("staged") {
                None => false,
                Some(value) => value.as_bool().ok_or("staged must be true or false")?,
            };
            let kind = if staged { "staged" } else { "unstaged" };
            let changed = crate::worktree::load_status(root)?.iter().any(|entry| {
                entry.path == path.as_str()
                    && (entry.section == crate::worktree::Section::Staged) == staged
            });
            if !changed {
                return Err(format!("{path} has no {kind} changes"));
            }
            let reply = format!("Gitview will open the {kind} diff of {path}.");
            Ok((ControlCommand::SelectFile { path, staged }, reply))
        })(),
        "focus_pane" => text("pane").and_then(|pane| match pane.as_str() {
            "agent" => Ok((
                ControlCommand::FocusAgent,
                "Gitview will give keyboard focus to the agent.".to_owned(),
            )),
            "gitview" => Ok((
                ControlCommand::FocusGitview,
                "Gitview will take keyboard focus.".to_owned(),
            )),
            _ => Err("pane must be \"agent\" or \"gitview\"".into()),
        }),
        _ => return None,
    })
}

fn failure(id: &Value, error: &str) -> Value {
    json!({"id": id, "ok": false, "error": error})
}
