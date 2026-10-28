// Release notes summarized from RELEASE.md. Keep both in step when a release ships.
export interface Release {
  version: string;
  date: string;
  status: "planned" | "released";
  summary: string;
  highlights: string[];
  note?: string;
  links?: { label: string; url: string }[];
}

const tag = (version: string) => ({
  label: `GitHub v${version}`,
  url: `https://github.com/barathrajd/gitview/releases/tag/v${version}`,
});

export const releases: Release[] = [
  {
    version: "0.6.0",
    date: "Targets December 31, 2026",
    status: "planned",
    summary:
      "Completes six everyday local Git workflows. Not published yet; remotes, merge and conflict workflows, and export stay out.",
    highlights: [
      "Repository setup with gitview init and gitview clone.",
      "Ignore-rule lookup for any path, stop tracking, confirmed removal and moves of tracked paths, and discard of a staged rename.",
      "Ctrl+E edits the commit message in your Git editor; saving a changed message commits it without a second Enter. Amend keeps its warning.",
      "History filtered by file or directory, search, and a one-page comparison of two commits.",
      "Rename, delete, and previous-branch return for local branches; list, lightweight create, verify, and delete for local tags.",
      "Object description for commits, tags, trees, and blobs, and comparisons against remote refs already stored locally.",
    ],
    note: "Ships the crate and the macOS Apple Silicon archive. Linux packaging moves to the next release.",
  },
  {
    version: "0.5.0",
    date: "October 28, 2026",
    status: "released",
    summary:
      "Folder-scoped Git configuration profiles, so repositories can use separate author identities and settings.",
    highlights: [
      "Applies the most specific matching profile to Gitview's Git commands; other repositories keep the normal global config.",
      "Adds a profile view, registry editor and reload actions, and the active account in the header.",
      "Adds an opt-in zsh snippet for terminal Git commands.",
      "Updates worktree selection colors and documents the staging shortcuts.",
    ],
    links: [
      { label: "crates.io 0.5.0", url: "https://crates.io/crates/gitview/0.5.0" },
      tag("0.5.0"),
    ],
  },
  {
    version: "0.4.1",
    date: "October 28, 2026",
    status: "released",
    summary: "Crates.io installation and a verified installer for the macOS Apple Silicon binary.",
    highlights: [
      "Publishes the gitview crate with package metadata.",
      "Adds cargo install gitview --locked as an installation path.",
      "Adds release checks for formatting, tests, Clippy, CLI help, installer help, and packaging.",
      "Adds manual recovery paths for publishing the crate and building the macOS archive.",
    ],
    links: [
      { label: "crates.io 0.4.1", url: "https://crates.io/crates/gitview/0.4.1" },
      tag("0.4.1"),
    ],
  },
  {
    version: "0.4.0",
    date: "October 27, 2026",
    status: "released",
    summary: "Hunk-level staging, grouped worktree sections, and page-specific help.",
    highlights: [
      "Stage and unstage selected hunks, whole files, or the whole worktree.",
      "Groups staged changes separately from unstaged and untracked changes.",
      "Refreshes worktree status in the background and adds contextual help overlays.",
      "Opens in the worktree by default and keeps selection in its section after staging.",
    ],
    links: [tag("0.4.0")],
  },
  {
    version: "0.3.3",
    date: "October 27, 2026",
    status: "released",
    summary: "Published by the Release Please GitHub Action with the macOS archive and checksum.",
    highlights: ["Creates the tag and release, builds the macOS Apple Silicon binary, and uploads the archive and checksum."],
    links: [tag("0.3.3")],
  },
  {
    version: "0.3.2",
    date: "October 27, 2026",
    status: "released",
    summary: "A macOS archive containing only the executable, and clearer installation steps.",
    highlights: [
      "Reduces the macOS release archive to the Gitview executable only.",
      "Splits installer, Gitview installation, and launch instructions into separate steps.",
      "Documents the GNU find prerequisite for the GAH installer on macOS.",
    ],
    links: [tag("0.3.2")],
  },
  {
    version: "0.3.1",
    date: "October 27, 2026",
    status: "released",
    summary: "Clear reuse terms and a no-sudo installation path for macOS Apple Silicon.",
    highlights: [
      "Adds the MIT License.",
      "Documents installing Gitview into ~/.local/bin without sudo.",
    ],
    links: [tag("0.3.1")],
  },
  {
    version: "0.3.0",
    date: "October 27, 2026",
    status: "released",
    summary: "Improved history and diff review with a responsive cockpit and clearer search.",
    highlights: [
      "Adds a responsive cockpit for repository status, history, and commit details.",
      "Adds debounced history search with selectable results.",
      "Adds old and new line numbers to diff views.",
      "Replaces wide shortcut tables with a compact shortcut bar, and makes Esc and ? close help.",
    ],
    links: [tag("0.3.0")],
  },
  {
    version: "0.2.1",
    date: "October 27, 2026",
    status: "released",
    summary: "Refactors the terminal UI into testable state, actions, and rendering modules.",
    highlights: [
      "Adds Ratatui TestBackend coverage for help, history, branches, details, worktree states, errors, and diff layouts.",
      "Adds regression coverage for stale branch and unstage selections.",
      "Adds a PTY test that the terminal is restored after a panic.",
    ],
    links: [tag("0.2.1")],
  },
];
