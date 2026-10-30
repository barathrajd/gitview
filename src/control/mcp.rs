//! `gitview mcp`: a stdio MCP server that forwards tool calls to the control
//! socket of the Gitview that started the agent.

use std::{
    io::{self, BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::Path,
    time::Duration,
};

use serde_json::{Value, json};

/// The protocol version used when the client doesn't name one.
const PROTOCOL_VERSION: &str = "2025-06-18";
const NOT_RUNNING: &str = "Gitview isn't reachable. Start the agent from Gitview, for example with `gitview --claude`, so it can connect.";

/// Answer MCP requests from `input` until it ends.
pub fn serve(input: impl BufRead, mut output: impl Write, socket: Option<&Path>) -> io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = respond(&line, socket) {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}

fn respond(line: &str, socket: Option<&Path>) -> Option<Value> {
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return Some(error(Value::Null, -32700, "Parse error"));
    };
    // A message without an id is a notification and gets no reply.
    let id = message.get("id")?.clone();
    let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
    Some(match message.get("method").and_then(Value::as_str) {
        Some("initialize") => success(
            id,
            json!({
                "protocolVersion": params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .unwrap_or(PROTOCOL_VERSION),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "gitview", "version": env!("CARGO_PKG_VERSION")},
            }),
        ),
        Some("ping") => success(id, json!({})),
        Some("tools/list") => success(id, json!({"tools": tool_list()})),
        Some("tools/call") => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let (text, is_error) = match forward(socket, name, arguments) {
                Ok(text) => (text, false),
                Err(text) => (text, true),
            };
            success(
                id,
                json!({"content": [{"type": "text", "text": text}], "isError": is_error}),
            )
        }
        _ => error(id, -32601, "Method not found"),
    })
}

/// Send one tool call over the socket and return its text.
fn forward(socket: Option<&Path>, tool: &str, args: Value) -> Result<String, String> {
    let socket = socket.ok_or(NOT_RUNNING)?;
    let mut stream = UnixStream::connect(socket).map_err(|_| NOT_RUNNING.to_owned())?;
    let timeout = Some(Duration::from_secs(30));
    stream
        .set_read_timeout(timeout)
        .map_err(|e| e.to_string())?;
    writeln!(stream, "{}", json!({"tool": tool, "args": args}))
        .map_err(|error| format!("Could not reach Gitview: {error}"))?;
    let mut reply = String::new();
    BufReader::new(stream)
        .read_line(&mut reply)
        .map_err(|error| format!("Gitview didn't answer: {error}"))?;
    let reply: Value =
        serde_json::from_str(&reply).map_err(|_| "Gitview sent an invalid reply".to_owned())?;
    if reply["ok"] != true {
        return Err(reply["error"]
            .as_str()
            .unwrap_or("Gitview refused")
            .to_owned());
    }
    let mut text = reply["text"].as_str().unwrap_or_default().to_owned();
    if reply["truncated"] == true {
        text.push_str("\n[output truncated at 64 KiB]");
    }
    Ok(text)
}

fn tool_list() -> Value {
    let read = |name: &str, description: &str, properties: Value, required: &[&str]| {
        tool(name, description, properties, required, true)
    };
    json!([
        read(
            "get_status",
            "List changed files in the repository Gitview shows, grouped as staged, unstaged, and untracked.",
            json!({}),
            &[],
        ),
        read(
            "get_log",
            "List recent commits on the current branch, newest first (default 50, at most 200).",
            json!({"limit": {"type": "integer", "minimum": 1, "maximum": 200}}),
            &[],
        ),
        read(
            "get_diff",
            "Show the diff of one changed file. Set staged to see what is staged for commit.",
            json!({"path": {"type": "string", "description": "Repository-relative path"},
                   "staged": {"type": "boolean"}}),
            &["path"],
        ),
        read(
            "get_show",
            "Show a commit's message, file summary, and patch.",
            json!({"revision": {"type": "string", "description": "A commit ID, branch, or tag"}}),
            &["revision"],
        ),
        tool(
            "select_commit",
            "Select a commit in Gitview's history so the user sees it.",
            json!({"revision": {"type": "string"}}),
            &["revision"],
            false,
        ),
        tool(
            "select_file",
            "Open a changed file's diff in Gitview's working-tree view so the user sees it.",
            json!({"path": {"type": "string"}, "staged": {"type": "boolean"}}),
            &["path"],
            false,
        ),
        tool(
            "focus_pane",
            "Move keyboard focus to Gitview or back to the agent.",
            json!({"pane": {"type": "string", "enum": ["gitview", "agent"]}}),
            &["pane"],
            false,
        ),
    ])
}

fn tool(
    name: &str,
    description: &str,
    properties: Value,
    required: &[&str],
    read_only: bool,
) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {"type": "object", "properties": properties, "required": required},
        "annotations": {"readOnlyHint": read_only, "destructiveHint": false},
    })
}

fn success(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}
