# Lazygit v0.66.2 workflow checklist

This checklist pins Gitview 1.0.0 compatibility to Lazygit v0.66.2. It groups
documented workflows so each can receive a Gitview implementation and an
acceptance test without tracking every upstream keybinding as a separate
feature.

| Workflow area | Documented behavior to cover | Status |
| --- | --- | --- |
| Navigation and repository state | Panels, focus, filtering, search, history graph, commit details, help, loading/errors | Not started |
| Working-tree review | Staged/unstaged/untracked files, file tree, file and hunk/line diff review, stage/unstage, discard | Not started |
| Commit workflows | Create, amend, edit message, commit with editor, fixup target selection | Not started |
| Stashes | Create, inspect, apply, pop, drop, staged/unstaged variants | Not started |
| Branches | Create, switch, rename, delete, checkout previous, move unpushed commits to a branch | Not started |
| Remote branches and remotes | Fetch, pull, push, upstream settings, reset, merge, squash merge, rebase, fast-forward | Not started |
| Merge safety | Preview fast-forward-only, merge-commit, and squash results; conflict status; editor handoff; continue/abort | Not started |
| History editing | Interactive rebase, reorder, squash, fixup, drop, edit, reword, cherry-pick, revert, custom patches | Not started |
| History diagnostics | Bisect, compare revisions, commit graph, reflog undo/redo | Not started |
| Tags and worktrees | Create/delete/manage tags and create/switch/remove worktrees | Not started |
| Submodules | Review submodule state and perform supported submodule actions | Not started |
| Pull requests | Discover, create, open, and inspect configured provider pull requests through supported integrations | Not started |
| Customization | Config file, keybindings, themes, custom commands, services, external editor and diff tool | Not started |

AI chat and Windows support are deliberately outside this 1.0.0 checklist. Git
view keeps its own keybindings and visual design; parity refers to workflow
coverage, not a copied interface.

## Reference documents

- [Lazygit v0.66.2 README](https://github.com/jesseduffield/lazygit/blob/v0.66.2/README.md)
- [Lazygit v0.66.2 configuration guide](https://github.com/jesseduffield/lazygit/blob/v0.66.2/docs/Config.md)
- [Lazygit v0.66.2 keybinding guide](https://github.com/jesseduffield/lazygit/blob/v0.66.2/docs/keybindings/Keybindings_en.md)
- [Lazygit v0.66.2 custom command guide](https://github.com/jesseduffield/lazygit/blob/v0.66.2/docs/Custom_Command_Keybindings.md)
- [Lazygit v0.66.2 stacked branch guide](https://github.com/jesseduffield/lazygit/blob/v0.66.2/docs/Stacked_Branches.md)
- [Lazygit v0.66.2 undo guide](https://github.com/jesseduffield/lazygit/blob/v0.66.2/docs/Undoing.md)
