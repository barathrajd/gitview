use gitview::control::{mcp, server::ControlServer};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn repository(name: &str) -> PathBuf {
    let root = PathBuf::from("/tmp").join(format!("gv-mcp-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).unwrap();
    assert!(
        Command::new("git")
            .current_dir(&root)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    fs::write(root.join("hello.txt"), "hi\n").unwrap();
    root
}

/// Feed `requests` to the MCP server, one per line, and parse its replies.
fn session(socket: Option<&Path>, requests: &[Value]) -> Vec<Value> {
    let input: String = requests.iter().map(|r| format!("{r}\n")).collect();
    let mut output = Vec::new();
    mcp::serve(input.as_bytes(), &mut output, socket).unwrap();
    String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn call(id: u64, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
           "params": {"name": name, "arguments": arguments}})
}

#[test]
fn initializes_and_lists_the_read_only_tools() {
    let replies = session(
        None,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                   "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                              "clientInfo": {"name": "test", "version": "0"}}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        ],
    );

    assert_eq!(
        replies.len(),
        2,
        "a notification gets no reply: {replies:?}"
    );
    assert_eq!(replies[0]["id"], 1);
    assert_eq!(replies[0]["result"]["protocolVersion"], "2025-06-18");
    assert!(replies[0]["result"]["capabilities"]["tools"].is_object());
    assert_eq!(replies[0]["result"]["serverInfo"]["name"], "gitview");
    let names: Vec<_> = replies[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| {
            assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
            assert_eq!(tool["annotations"]["destructiveHint"], false, "{tool}");
            tool["name"].as_str().unwrap().to_owned()
        })
        .collect();
    assert_eq!(
        names,
        [
            "get_status",
            "get_log",
            "get_diff",
            "get_show",
            "select_commit",
            "select_file",
            "focus_pane"
        ]
    );
}

#[test]
fn descriptions_say_which_tools_change_gitviews_screen() {
    let replies = session(
        None,
        &[json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})],
    );
    for tool in replies[0]["result"]["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        let description = tool["description"].as_str().unwrap();
        if name.starts_with("get_") {
            assert!(
                description.contains("doesn't change Gitview's screen"),
                "{name}: {description}"
            );
        } else {
            assert!(
                description.contains("the user wants to see"),
                "{name}: {description}"
            );
        }
    }
    let commit = &replies[0]["result"]["tools"][4];
    assert_eq!(commit["name"], "select_commit");
    assert_eq!(
        commit["inputSchema"]["properties"]["path"]["type"],
        "string"
    );
}

#[test]
fn tool_calls_are_forwarded_to_gitview() {
    let root = repository("forward");
    let folder = PathBuf::from(format!("/tmp/gv-mcp-sock-{}", std::process::id()));
    let (server, commands) = ControlServer::start_in(&root, &folder).unwrap();

    let replies = session(
        Some(server.path()),
        &[
            call(1, "get_status", json!({})),
            call(2, "get_diff", json!({"path": "../outside"})),
            call(3, "select_file", json!({"path": "hello.txt"})),
        ],
    );

    let status = &replies[0]["result"];
    assert_eq!(status["isError"], false);
    assert!(
        status["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("hello.txt")
    );
    let refused = &replies[1]["result"];
    assert_eq!(refused["isError"], true);
    assert!(
        refused["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("repository-relative")
    );
    assert_eq!(replies[2]["result"]["isError"], false);
    assert_eq!(commands.try_iter().count(), 1);
}

#[test]
fn without_gitview_a_call_explains_how_to_start_it() {
    let missing = Path::new("/tmp/gv-mcp-no-such.sock");
    for socket in [None, Some(missing)] {
        let replies = session(socket, &[call(1, "get_status", json!({}))]);
        let result = &replies[0]["result"];
        assert_eq!(result["isError"], true);
        assert!(
            result["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("gitview --claude"),
            "{result}"
        );
    }
}

#[test]
fn protocol_errors_use_json_rpc_codes() {
    let input = "not json\n{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"resources/list\"}\n{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"ping\"}\n";
    let mut output = Vec::new();
    mcp::serve(input.as_bytes(), &mut output, None).unwrap();
    let replies: Vec<Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    assert_eq!(replies[0]["error"]["code"], -32700);
    assert_eq!(replies[0]["id"], Value::Null);
    assert_eq!(replies[1]["error"]["code"], -32601);
    assert_eq!(replies[1]["id"], 5);
    assert_eq!(replies[2]["result"], json!({}));
}
