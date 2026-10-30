use gitview::control::server::{ControlCommand, ControlServer, MAX_REQUEST};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn unique(kind: &str) -> PathBuf {
    // Short names: a Unix socket path must fit in about 100 bytes.
    let path = PathBuf::from("/tmp").join(format!(
        "gv-{kind}-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&path);
    path
}

fn repository() -> PathBuf {
    let root = unique("repo");
    fs::create_dir(&root).unwrap();
    let ok = Command::new("git")
        .current_dir(&root)
        .args(["init", "-q"])
        .status()
        .unwrap()
        .success();
    assert!(ok);
    fs::write(root.join("new file.txt"), "x\n").unwrap();
    root
}

fn start() -> (
    ControlServer,
    std::sync::mpsc::Receiver<ControlCommand>,
    PathBuf,
) {
    let root = repository();
    let (server, commands) = ControlServer::start_in(&root, &unique("sock")).unwrap();
    (server, commands, root)
}

struct Client {
    writer: UnixStream,
    reader: BufReader<UnixStream>,
}

impl Client {
    fn connect(path: &Path) -> Self {
        let writer = UnixStream::connect(path).unwrap();
        writer
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let reader = BufReader::new(writer.try_clone().unwrap());
        Self { writer, reader }
    }

    fn send_raw(&mut self, line: &[u8]) -> Option<Value> {
        self.writer.write_all(line).ok()?;
        let mut reply = String::new();
        match self.reader.read_line(&mut reply) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(serde_json::from_str(&reply).unwrap()),
        }
    }

    fn send(&mut self, request: Value) -> Value {
        self.send_raw(format!("{request}\n").as_bytes()).unwrap()
    }
}

#[test]
fn a_client_calls_a_read_tool() {
    let (server, _commands, _root) = start();
    let mut client = Client::connect(server.path());

    let reply = client.send(json!({"id": 7, "tool": "get_status", "args": {}}));

    assert_eq!(reply["id"], 7);
    assert_eq!(reply["ok"], true, "{reply}");
    assert!(reply["text"].as_str().unwrap().contains("new file.txt"));
    assert_eq!(reply["truncated"], false);
}

#[test]
fn the_socket_and_its_folder_are_private() {
    let (server, _commands, _root) = start();
    let socket = fs::metadata(server.path()).unwrap().permissions().mode();
    let folder = fs::metadata(server.path().parent().unwrap())
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(socket & 0o777, 0o600);
    assert_eq!(folder & 0o777, 0o700);
}

#[test]
fn a_shared_folder_is_refused() {
    let root = repository();
    let folder = unique("open");
    fs::create_dir(&folder).unwrap();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o777)).unwrap();

    let error = ControlServer::start_in(&root, &folder).err().unwrap();

    assert!(error.contains("private"), "{error}");
}

#[test]
fn malformed_requests_get_errors_and_the_connection_survives() {
    let (server, _commands, _root) = start();
    let mut client = Client::connect(server.path());

    for bad in [
        &b"not json\n"[..],
        b"[1, 2]\n",
        b"{\"id\": 1}\n",
        b"{\"id\": 2, \"tool\": 5}\n",
        b"\xff\xfe\n",
    ] {
        let reply = client.send_raw(bad).unwrap();
        assert_eq!(reply["ok"], false, "{reply}");
        assert!(reply["error"].is_string(), "{reply}");
    }
    let reply = client.send(json!({"id": 3, "tool": "push", "args": {}}));
    assert!(reply["error"].as_str().unwrap().contains("Unknown tool"));
    assert_eq!(client.send(json!({"id": 4, "tool": "get_log"}))["ok"], true);
}

#[test]
fn an_oversized_request_closes_the_connection() {
    let (server, _commands, _root) = start();
    let mut client = Client::connect(server.path());
    let mut huge = vec![b'x'; MAX_REQUEST + 1];
    huge.push(b'\n');

    let reply = client.send_raw(&huge).unwrap();

    assert_eq!(reply["ok"], false);
    assert!(client.send_raw(b"{\"tool\": \"get_status\"}\n").is_none());
    // Other clients are unaffected.
    assert_eq!(
        Client::connect(server.path()).send(json!({"tool": "get_status"}))["ok"],
        true
    );
}

#[test]
fn selection_tools_reach_gitview() {
    let (server, commands, _root) = start();
    let mut client = Client::connect(server.path());

    let replies = [
        client.send(json!({"tool": "select_commit", "args": {"revision": "main"}})),
        client.send(json!({"tool": "select_file", "args": {"path": "a b.txt", "staged": true}})),
        client.send(json!({"tool": "focus_pane", "args": {"pane": "agent"}})),
    ];

    for reply in &replies {
        assert_eq!(reply["ok"], true, "{reply}");
    }
    let received: Vec<_> = commands.try_iter().collect();
    assert_eq!(
        received,
        [
            ControlCommand::SelectCommit("main".into()),
            ControlCommand::SelectFile {
                path: "a b.txt".into(),
                staged: true
            },
            ControlCommand::FocusAgent,
        ]
    );
}

#[test]
fn selection_arguments_are_checked() {
    let (server, commands, _root) = start();
    let mut client = Client::connect(server.path());

    for request in [
        json!({"tool": "select_commit", "args": {"revision": "--all"}}),
        json!({"tool": "select_file", "args": {"path": "../x"}}),
        json!({"tool": "focus_pane", "args": {"pane": "elsewhere"}}),
    ] {
        assert_eq!(client.send(request.clone())["ok"], false, "{request}");
    }
    assert_eq!(commands.try_iter().count(), 0);
}

#[test]
fn dropping_the_server_removes_the_socket() {
    let (server, _commands, _root) = start();
    let path = server.path().to_owned();
    assert!(path.exists());

    drop(server);

    assert!(!path.exists());
    assert!(UnixStream::connect(&path).is_err());
}
