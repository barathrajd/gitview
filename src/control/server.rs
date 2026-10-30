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
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::JoinHandle,
};

use serde_json::{Map, Value, json};

use super::tools;

/// The longest request line accepted; a longer one closes the connection.
pub const MAX_REQUEST: usize = 64 * 1024;

/// What the agent asked Gitview to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlCommand {
    /// A full commit ID.
    SelectCommit(String),
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
        let accept = {
            let stop = Arc::clone(&stop);
            let root = root.to_owned();
            std::thread::spawn(move || accept_loop(listener, root, sender, stop))
        };
        Ok((
            Self {
                path,
                stop,
                accept: Some(accept),
            },
            commands,
        ))
    }

    pub fn path(&self) -> &Path {
        &self.path
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
) {
    for stream in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let Ok(stream) = stream else { continue };
        if peer_uid(&stream) != Some(unsafe { libc::geteuid() }) {
            continue;
        }
        let root = root.clone();
        let sender = sender.clone();
        std::thread::spawn(move || {
            let _ = serve(stream, &root, &sender);
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

fn serve(stream: UnixStream, root: &Path, sender: &Sender<ControlCommand>) -> io::Result<()> {
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
        let reply = respond(&line, root, sender);
        writeln!(writer, "{reply}")?;
    }
}

fn respond(line: &[u8], root: &Path, sender: &Sender<ControlCommand>) -> Value {
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
        Some(command) => command.and_then(|command| {
            sender
                .send(command)
                .map_err(|_| "Gitview is closing".to_owned())
                .map(|()| tools::ToolOutput {
                    text: "Requested".into(),
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

/// Parse a tool that changes what Gitview shows, or `None` for other tools.
fn selection(root: &Path, tool: &str, args: &Value) -> Option<Result<ControlCommand, String>> {
    let text = |key: &str| {
        args.get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(format!("{key} must be a string"))
    };
    Some(match tool {
        "select_commit" => text("revision")
            .and_then(|revision| tools::resolve_commit(root, &revision))
            .map(ControlCommand::SelectCommit),
        "select_file" => text("path").and_then(|path| {
            tools::check_relative(&path)?;
            let staged = match args.get("staged") {
                None => false,
                Some(value) => value.as_bool().ok_or("staged must be true or false")?,
            };
            Ok(ControlCommand::SelectFile { path, staged })
        }),
        "focus_pane" => text("pane").and_then(|pane| match pane.as_str() {
            "agent" => Ok(ControlCommand::FocusAgent),
            "gitview" => Ok(ControlCommand::FocusGitview),
            _ => Err("pane must be \"agent\" or \"gitview\"".into()),
        }),
        _ => return None,
    })
}

fn failure(id: &Value, error: &str) -> Value {
    json!({"id": id, "ok": false, "error": error})
}
