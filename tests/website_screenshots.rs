//! Captures the website's terminal screenshots from the real application and
//! writes them as SVGs in `website/src/assets` (140x24) and
//! `website/src/assets/m` (64x30, for phones). Ignored by default because it
//! rewrites files in the repository:
//!
//! ```sh
//! cargo test --features test-support --test website_screenshots -- --ignored
//! ```

#[path = "support/replay.rs"]
mod replay;

use std::{
    fmt::Write as _,
    path::Path,
    sync::{Arc, Mutex},
};

use crossterm::event::KeyCode::Enter;
use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier},
};
use replay::{Repo, Step, Step::*, replay_with_agent};

const CELL_WIDTH: f64 = 8.4;
const CELL_HEIGHT: f64 = 18.0;
const PADDING: f64 = 12.0;
const PAGE: &str = "#080e0c";
const DEFAULT_TEXT: &str = "#e2e8f0";

/// The terminal's 16 named colors, as the website shows them.
fn named(color: Color) -> Option<&'static str> {
    Some(match color {
        Color::Black => "#1e293b",
        Color::Red => "#f87171",
        Color::Green => "#4ade80",
        Color::Yellow => "#facc15",
        Color::Blue => "#60a5fa",
        Color::Magenta => "#c084fc",
        Color::Cyan => "#22d3ee",
        Color::Gray => "#cbd5e1",
        Color::DarkGray => "#64748b",
        Color::LightRed => "#fca5a5",
        Color::LightGreen => "#86efac",
        Color::LightYellow => "#fde68a",
        Color::LightBlue => "#93c5fd",
        Color::LightMagenta => "#d8b4fe",
        Color::LightCyan => "#67e8f9",
        Color::White => "#f8fafc",
        _ => return None,
    })
}

fn hex(color: Color) -> Option<String> {
    match color {
        Color::Reset => None,
        Color::Rgb(r, g, b) => Some(format!("#{r:02x}{g:02x}{b:02x}")),
        Color::Indexed(index) if index < 16 => named(
            [
                Color::Black,
                Color::Red,
                Color::Green,
                Color::Yellow,
                Color::Blue,
                Color::Magenta,
                Color::Cyan,
                Color::Gray,
                Color::DarkGray,
                Color::LightRed,
                Color::LightGreen,
                Color::LightYellow,
                Color::LightBlue,
                Color::LightMagenta,
                Color::LightCyan,
                Color::White,
            ][index as usize],
        )
        .map(str::to_owned),
        Color::Indexed(index) if (16..232).contains(&index) => {
            let level = |n: u8| if n == 0 { 0 } else { 55 + n * 40 };
            let n = index - 16;
            Some(format!(
                "#{:02x}{:02x}{:02x}",
                level(n / 36),
                level(n / 6 % 6),
                level(n % 6)
            ))
        }
        Color::Indexed(index) => {
            let gray = 8 + (index - 232) * 10;
            Some(format!("#{gray:02x}{gray:02x}{gray:02x}"))
        }
        other => named(other).map(str::to_owned),
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Renders `buffer` as an SVG in the format of the existing screenshots.
fn svg(buffer: &Buffer, label: &str) -> String {
    let (width, height) = (buffer.area.width, buffer.area.height);
    let view_width = f64::from(width) * CELL_WIDTH + 2.0 * PADDING;
    let view_height = f64::from(height) * CELL_HEIGHT + 2.0 * PADDING;
    let mut rects = String::new();
    let mut text = String::new();
    for y in 0..height {
        let top = PADDING + f64::from(y) * CELL_HEIGHT;
        // Background runs.
        let mut x = 0;
        while x < width {
            let cell = &buffer[(x, y)];
            let reversed = cell.modifier.contains(Modifier::REVERSED);
            let background = if reversed {
                hex(cell.fg).or(Some(DEFAULT_TEXT.into()))
            } else {
                hex(cell.bg)
            };
            let start = x;
            while x < width {
                let next = &buffer[(x, y)];
                let next_bg = if next.modifier.contains(Modifier::REVERSED) {
                    hex(next.fg).or(Some(DEFAULT_TEXT.into()))
                } else {
                    hex(next.bg)
                };
                if next_bg != background {
                    break;
                }
                x += 1;
            }
            if let Some(color) = background {
                let _ = write!(
                    rects,
                    r#"<rect x="{:.2}" y="{top:.2}" width="{:.2}" height="18" fill="{color}"/>"#,
                    PADDING + f64::from(start) * CELL_WIDTH,
                    f64::from(x - start) * CELL_WIDTH,
                );
            }
        }
        // Text runs of one color and weight, skipping blanks.
        let mut line = String::new();
        let mut x = 0;
        while x < width {
            let cell = &buffer[(x, y)];
            let reversed = cell.modifier.contains(Modifier::REVERSED);
            let style = (
                if reversed {
                    hex(cell.bg).unwrap_or(PAGE.into())
                } else {
                    hex(cell.fg).unwrap_or(DEFAULT_TEXT.into())
                },
                cell.modifier.contains(Modifier::BOLD),
            );
            let start = x;
            let mut run = String::new();
            while x < width {
                let next = &buffer[(x, y)];
                let next_reversed = next.modifier.contains(Modifier::REVERSED);
                let next_style = (
                    if next_reversed {
                        hex(next.bg).unwrap_or(PAGE.into())
                    } else {
                        hex(next.fg).unwrap_or(DEFAULT_TEXT.into())
                    },
                    next.modifier.contains(Modifier::BOLD),
                );
                if next_style != style {
                    break;
                }
                run.push_str(next.symbol());
                x += 1;
            }
            let trimmed = run.trim_end();
            let leading = trimmed.len() - trimmed.trim_start().len();
            let visible = trimmed.trim_start();
            if visible.is_empty() {
                continue;
            }
            let skipped = run[..leading].chars().count() as u16;
            let shown = visible.chars().count() as u16;
            let _ = write!(
                line,
                r#"<tspan x="{:.2}" textLength="{:.2}" lengthAdjust="spacingAndGlyphs" fill="{}"{}>{}</tspan>"#,
                PADDING + f64::from(start + skipped) * CELL_WIDTH,
                f64::from(shown) * CELL_WIDTH,
                style.0,
                if style.1 { r#" font-weight="700""# } else { "" },
                escape(visible),
            );
        }
        if !line.is_empty() {
            let _ = write!(text, r#"<text y="{:.2}">{line}</text>"#, top + 13.0);
        }
    }
    let label = escape(label);
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {view_width:.0} {view_height:.0}" role="img" aria-label="{label}" style="display:block;width:100%;height:auto"><title>{label}</title><rect width="100%" height="100%" fill="{PAGE}"/><g shape-rendering="crispEdges">{rects}</g><g font-family="'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, monospace" font-size="14" xml:space="preserve">{text}</g></svg>"#
    )
}

type Slot = Arc<Mutex<Option<Buffer>>>;

/// Runs `steps` at a size, then writes the captured screen as `name`.
fn capture(
    repository: &Repo,
    agent: Option<&[&str]>,
    (width, height): (u16, u16),
    file: &Path,
    label: &str,
    steps: impl FnOnce(Slot) -> Vec<Step>,
) {
    let slot: Slot = Arc::default();
    replay_with_agent(
        &repository.path(),
        agent,
        width,
        height,
        steps(Arc::clone(&slot)),
    );
    let buffer = slot
        .lock()
        .unwrap()
        .take()
        .expect("the script captured the screen");
    std::fs::write(file, svg(&buffer, label)).unwrap();
}

/// Runs `steps`, which capture into the given slots in order, and writes one
/// SVG per frame to `website/src/assets/demo` (or `m/demo` for phones).
fn capture_frames(
    repository: &Repo,
    agent: Option<&[&str]>,
    (width, height): (u16, u16),
    phone: bool,
    frames: &[(&str, &str)],
    steps: impl FnOnce(Vec<Slot>) -> Vec<Step>,
) {
    let slots: Vec<Slot> = frames.iter().map(|_| Slot::default()).collect();
    replay_with_agent(
        &repository.path(),
        agent,
        width,
        height,
        steps(slots.clone()),
    );
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("website/src/assets")
        .join(if phone { "m/demo" } else { "demo" });
    std::fs::create_dir_all(&folder).unwrap();
    for ((name, label), slot) in frames.iter().zip(slots) {
        let buffer = slot
            .lock()
            .unwrap()
            .take()
            .expect("the script captured every frame");
        std::fs::write(folder.join(format!("{name}.svg")), svg(&buffer, label)).unwrap();
    }
}

fn asset(name: &str, phone: bool) -> std::path::PathBuf {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("website/src/assets");
    if phone {
        base.join("m").join(format!("{name}.svg"))
    } else {
        base.join(format!("{name}.svg"))
    }
}

/// A repository with history, and a session whose agent committed a file.
fn with_session() -> (Repo, gitview::sessions::Session) {
    let repository = Repo::with_changes_at(Path::new(SAMPLE_PATH));
    let session = gitview::sessions::create(&repository.path()).unwrap();
    std::fs::write(
        session.path.join("NOTES.md"),
        "# Release notes\n\n- Linux and Homebrew installs\n- Agent sessions\n",
    )
    .unwrap();
    for args in [
        &["add", "NOTES.md"][..],
        &[
            "-c",
            "user.name=Agent",
            "-c",
            "user.email=agent@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "Draft the release notes",
        ],
    ] {
        assert!(
            std::process::Command::new("git")
                .current_dir(&session.path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    (repository, session)
}

/// Where the sample repository lives, so the header shows a clean path.
const SAMPLE_PATH: &str = "/tmp/gitview-demo/sample-repo";

const SIZES: [((u16, u16), bool); 2] = [((140, 24), false), ((64, 30), true)];

#[test]
#[ignore]
fn capture_the_website_screenshots() {
    let gitview = env!("CARGO_BIN_EXE_gitview");
    // A shell in the pane runs real commands, including a call through
    // gitview mcp, and stays open for the capture.
    let agent_script = format!(
        r#"printf '$ git log --oneline -3\n'; git log --oneline -3; printf '\n$ gitview mcp  # tools/call get_status\n'; printf '%s\n' '{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"get_status","arguments":{{}}}}}}' | '{gitview}' mcp | sed 's/.*"text":"\(.*\)","type".*/\1/' | sed 's/\\"/"/g'; sleep 30"#
    );

    for (size, phone) in SIZES {
        let repository = Repo::with_changes_at(Path::new(SAMPLE_PATH));
        capture(
            &repository,
            Some(&["sh", "-c", &agent_script]),
            size,
            &asset("agent", phone),
            if phone {
                "Gitview on a narrow terminal with an agent running: the agent's pane fills the screen and a bar below shows its status and the focus key."
            } else {
                "Gitview with a shell running in the agent pane: the worktree on the left, and on the right the shell's git log and a get_status call answered through gitview mcp."
            },
            |slot| {
                // On a phone-sized terminal Gitview starts in front; the focus
                // key brings the agent forward.
                let mut steps = if phone {
                    vec![WaitFor("notes.txt"), Ctrl('\\'), WaitFor("get_status")]
                } else {
                    vec![WaitFor("get_status"), WaitFor("notes.txt")]
                };
                steps.extend([Pause(std::time::Duration::from_millis(300)), Capture(slot)]);
                steps
            },
        );

        let (repository, session) = with_session();
        let name: &'static str = session.name.clone().leak();
        capture(
            &repository,
            None,
            size,
            &asset("sessions", phone),
            "The Sessions page listing an agent session, one commit ahead of main, with keys to review, discard, and refresh.",
            |slot| vec![Char('A'), WaitFor(name), Capture(slot)],
        );
        capture(
            &repository,
            None,
            size,
            &asset("session-review", phone),
            "Reviewing an agent session: its commit and the diff it would bring into main, with keys to scroll, merge, and go back.",
            |slot| {
                vec![
                    Char('A'),
                    WaitFor(name),
                    Key(Enter),
                    WaitFor("Draft the release notes"),
                    Capture(slot),
                ]
            },
        );

        // Demo: the agent opens a file's diff on the user's screen.
        let repository = Repo::with_changes_at(Path::new(SAMPLE_PATH));
        let select = format!(
            r#"printf '$ gitview mcp  # select_file README.md\n'; sleep 1; printf '%s\n' '{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"select_file","arguments":{{"path":"README.md"}}}}}}' | '{gitview}' mcp | sed 's/.*"text":"\([^"]*\)".*/\1/'; sleep 30"#
        );
        capture_frames(
            &repository,
            Some(&["sh", "-c", &select]),
            size,
            phone,
            &[
                (
                    "agent-1",
                    "The agent asks Gitview to open README.md through gitview mcp.",
                ),
                (
                    "agent-2",
                    "Gitview opens the diff on the user's screen, and the agent is told what it will show.",
                ),
            ],
            |slots| {
                let mut steps = vec![WaitFor("notes.txt")];
                if phone {
                    steps.extend([Ctrl('\\'), WaitFor("select_file")]);
                } else {
                    steps.push(WaitFor("select_file"));
                }
                steps.push(Capture(slots[0].clone()));
                if phone {
                    steps.extend([WaitFor("Gitview will open"), Ctrl('\\')]);
                }
                steps.extend([
                    WaitFor("more"),
                    Pause(std::time::Duration::from_millis(200)),
                    Capture(slots[1].clone()),
                ]);
                steps
            },
        );

        // Demo: review an agent session and merge it.
        let repository = Repo::with_history_at(Path::new(SAMPLE_PATH));
        let session = gitview::sessions::create(&repository.path()).unwrap();
        std::fs::write(
            session.path.join("NOTES.md"),
            "# Release notes\n\n- Agent sessions\n",
        )
        .unwrap();
        for args in [
            &["add", "NOTES.md"][..],
            &[
                "-c",
                "user.name=Agent",
                "-c",
                "user.email=agent@example.com",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "Draft the release notes",
            ],
        ] {
            assert!(
                std::process::Command::new("git")
                    .current_dir(&session.path)
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let name: &'static str = session.name.clone().leak();
        capture_frames(
            &repository,
            None,
            size,
            phone,
            &[
                (
                    "sessions-1",
                    "Press A: the Sessions page lists the agent's session, one commit ahead of main.",
                ),
                (
                    "sessions-2",
                    "Enter reviews it: the commit and the diff it would bring into main.",
                ),
                (
                    "sessions-3",
                    "m asks to merge it with a merge commit, and says when Gitview would refuse.",
                ),
                ("sessions-4", "y merges it into main."),
            ],
            |slots| {
                vec![
                    Char('A'),
                    WaitFor(name),
                    Capture(slots[0].clone()),
                    Key(Enter),
                    WaitFor("Draft the release notes"),
                    Capture(slots[1].clone()),
                    Char('m'),
                    WaitFor("CONFIRM MERGE"),
                    Capture(slots[2].clone()),
                    Char('y'),
                    WaitFor("Merged"),
                    Capture(slots[3].clone()),
                ]
            },
        );
    }
}
