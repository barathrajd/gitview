export const features = [
...[
  {
    id: "repository",
    title: "Start a repository",
    body: "Create a repository or clone one you name, from the command line. Gitview refuses to initialize inside an existing repository and to clone into a non-empty folder, and shows Git's own progress and errors.",
    transcript: [
      "$ gitview init notes",
      "Initialized Git repository at notes",
      "",
      "$ gitview clone /tmp/demo/sample-repo copy",
      "Cloning /tmp/demo/sample-repo...",
      "Cloning into 'copy'...",
      "done.",
      "Clone complete.",
      "",
      "$ gitview clone /tmp/demo/sample-repo copy",
      "Cloning /tmp/demo/sample-repo...",
      "The clone destination is not empty",
    ],
    shots: [],
  },
  {
    id: "worktree",
    title: "Handle paths with review",
    body: "Look up the ignore rule for any repository-relative path, stop tracking a file but keep your copy, remove a tracked path, move a clean tracked file, and discard a staged rename. Each action is reviewed before it runs, and Git's own refusals stay visible.",
    shots: [
      { name: "ignoreform", label: "Inspect the ignore rule for any repository-relative path" },
      { name: "move", label: "Move a clean tracked file; Git stages the rename" },
    ],
  },
  {
    id: "commit",
    title: "Commit and amend",
    body: "Write a subject and description, or press Ctrl+E to edit them in your configured Git editor. Saving a changed message commits the reviewed staged changes without a second Enter. Amending HEAD first shows the old and replacement messages. Hooks and signing stay on.",
    shots: [
      { name: "commit", label: "Commit composer beside the read-only worktree" },
      { name: "amend", label: "Amend confirmation with a rewrite warning" },
    ],
  },
  {
    id: "history",
    title: "Filter, search, and compare history",
    body: "Filter history to a literal file or directory, search commits as you type, and compare any two local commits on one page with folding per file. Branches and remote refs already stored locally work as revisions; nothing is fetched.",
    shots: [
      { name: "pathhistory", label: "History filtered to docs/notes.md" },
      { name: "search", label: "Debounced history search" },
      { name: "comparison", label: "Comparison of two commits across three files" },
    ],
  },
  {
    id: "refs",
    title: "Branches and tags",
    body: "Rename and delete local branches with the destination rechecked first, and return to the previous branch or detached commit. Tags list, create lightweight tags at the reviewed HEAD, verify signatures with Git, and delete locally. Annotated and signed tag creation isn't available.",
    shots: [
      { name: "branches", label: "Local branches" },
      { name: "tags", label: "Local tags" },
    ],
  },
  {
    id: "inspect",
    title: "Describe any object",
    body: "Enter a commit, tag, tree, or blob revision such as HEAD:src/main.rs to see its fixed ID, type, size, nearest-tag description, and a bounded read-only preview.",
    shots: [{ name: "object", label: "Object description for a commit" }],
  },
  {
    id: "help",
    title: "Help on every page",
    body: "Press ? on any page for the actions available there. The footer always lists the keys for the current view.",
    shots: [{ name: "help", label: "Contextual help over the working tree" }],
  },
].map(item => ({ ...item, since: item.id === "help" ? "0.4.0" : "0.6.0" })),
...[
  {
    id: "agent-pane",
    title: "Run a coding agent beside Gitview",
    body: "Start Claude Code, Codex, or any program in a pane next to the Git views. Ctrl+\\ moves focus between Gitview and the agent, and on a terminal under 119 columns they take turns filling the screen. The views refresh by themselves within about a second of the agent's changes, and an idle Gitview runs no Git commands.",
    transcript: [
      "$ gitview --help",
      "Usage: gitview [--ascii] [--version]",
      "       gitview [--ascii] --run <command> [args...]",
      "       gitview [--ascii] --claude [args...]",
      "       gitview [--ascii] --codex [args...]",
      "       gitview [--ascii] --session (--claude | --codex | --run <command>) [args...]",
      "       gitview mcp",
    ],
    shots: [
      { name: "agent", label: "A shell in the agent pane runs git log and asks Gitview for the status through gitview mcp" },
    ],
  },
  {
    id: "control-channel",
    title: "Let the agent read and steer Gitview",
    body: "An agent started from Gitview reaches it through gitview mcp, a stdio MCP server, over a socket only you can open. It can read status, history, diffs, and commits, and open a commit, a commit's file, or a changed file on your screen. No tool stages, commits, or changes files, and every reply is capped at 64 KiB. --claude registers the server for you; Codex needs a few lines of config.",
    transcript: [
      "$ printf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}' | gitview mcp",
      "get_status  get_log  get_diff  get_show",
      "select_commit  select_file  focus_pane",
    ],
    note: "Tool names from the tools/list reply",
  },
  {
    id: "sessions",
    title: "Give each agent its own worktree",
    body: "With --session, the agent works in a new worktree on its own branch, created from the branch you're on. Press A for the Sessions page: review a session's commits and diff, merge it with a merge commit, or discard it after seeing what would be lost. Gitview refuses a merge if anything moved since your review, and a conflict changes nothing.",
    transcript: [
      "$ cd /tmp/notes && gitview --session --run sh -c 'sleep 30'",
      "Session session-20261030-110040 is kept on branch gitview/session-20261030-110040 at /private/tmp/notes/.git/gitview/sessions/session-20261030-110040.",
    ],
    shots: [
      { name: "sessions", label: "The Sessions page (A): an agent session one commit ahead of main" },
      { name: "session-review", label: "Reviewing the session: its commit and the diff it would bring into main" },
    ],
  },
  {
    id: "install",
    title: "Install with Homebrew, on macOS and Linux",
    body: "Releases now ship a static Linux x86_64 binary next to the macOS Apple Silicon one. Homebrew installs either from the barathrajd/tap tap, and the installer script picks the right archive and verifies its checksum.",
    transcript: [
      "$ brew install barathrajd/tap/gitview",
      "🍺  /opt/homebrew/Cellar/gitview/0.7.0: 4 files, 6.2MB, built in 1 second",
      "",
      "# Alpine Linux, x86_64",
      "$ curl -fsSL https://raw.githubusercontent.com/barathrajd/gitview/main/install.sh | bash",
      "Verifying SHA-256...",
      "gitview-v0.7.0-linux-x86_64.tar.gz: OK",
      "Installed Gitview v0.7.0 to /root/.local/bin/gitview",
    ],
  },
].map(item => ({ ...item, since: "0.7.0" })),
];
