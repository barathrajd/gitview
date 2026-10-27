# Git config profiles by repository folder

Add account-aware Git configuration before starting the Basic Git workflow
milestones. A primary account remains the default. Repositories under configured
client, office, contract, or other account folders use a matching profile.

## Objective

Let Gitview use the intended author identity and GitHub/GitLab authentication
settings for each repository folder. Keep the primary account as the default,
and make a matching folder profile replace the primary user's global config for
Git commands started by Gitview.

## Success criteria

- Gitview shows the active profile and effective commit identity for the
  current repository before commit or remote actions.
- Repositories without a matching folder profile use the primary account
  configuration.
- Repositories with a matching profile use its dedicated Git config file;
  Gitview's Git subprocesses do not read the primary `~/.gitconfig` or XDG
  global config file.
- Folder matching handles nested paths, repository roots, and linked
  worktrees consistently, and reports ambiguous or overlapping profile rules.
- The user can add, edit, disable, and remove named folder profiles without
  Gitview silently rewriting repository remotes or the primary config.
- Profiles store identity and Git settings only. Gitview never stores access
  tokens or passwords; Git credential helpers and SSH remain responsible for
  authentication secrets.
- The active account is observable using Git's effective config origin and
  scope, and repository-local identity overrides are surfaced before a commit.

## In scope / Out of scope

**In scope**

- A primary profile and named profiles matched by repository worktree path.
- Config files managed under the user's Gitview config directory, with safe
  file permissions and atomic writes.
- Applying the selected profile to every Git subprocess launched by Gitview,
  including status, commit, fetch, pull, and push.
- Identity (`user.name`, `user.email`), signing preferences, credential-helper
  selection, and SSH command settings where the user configures them.
- A profile selector/editor and visible active-profile state in Gitview.

**Out of scope**

- Automatically changing the remote URL, SSH keys, or credential-store entries.
- Storing passwords, access tokens, or private keys in Gitview profile files.
- Changing the user's primary `~/.gitconfig` or repository `.git/config` as a
  side effect of selecting a profile.
- Changing the environment for Git commands started outside Gitview. Standard
  shell `git` commands keep their existing configuration behavior.
- Automatic account guessing from private repository metadata or remote
  network calls.

## Approach

Store one config file per profile and match profiles against the canonical
worktree root. For a matching profile, set `GIT_CONFIG_GLOBAL` in the
`Command` environment for every Git subprocess; for unmatched repositories,
leave the environment unchanged and use the primary profile. Git documents that
`GIT_CONFIG_GLOBAL` selects the global config file instead of `~/.gitconfig` or
`$XDG_CONFIG_HOME/git/config`. Keep system and repository-local scopes intact,
then inspect effective values so Gitview can show when a repository-local
setting overrides the selected profile. Use credential helpers or SSH for
secrets; never copy credentials into the profile file.

A Git-native `includeIf` rule is not the isolation mechanism: it loads the
normal global config and conditionally adds another file. Gitview's per-process
`GIT_CONFIG_GLOBAL` override keeps the primary global file out of Gitview's
commands for matched folders. This does not affect standalone Git commands in a
terminal; covering those commands would need a separate shell-level approach
and would not provide the same guarantee with ordinary `includeIf` rules.

Sources: Git documents [`GIT_CONFIG_GLOBAL`][global-env], conditional
[`includeIf`][include-if], configuration precedence and scopes in
[`git-config`][git-config], and credential helpers in
[`gitcredentials`][credentials].

## Milestones

1. **Define profile matching and precedence:** Specify canonical worktree path
   matching, nested-profile precedence, linked-worktree behavior, and what
   happens when local config overrides the selected identity.
2. **Load profiles for Gitview commands:** Add a central Git command context
   that chooses the profile and applies `GIT_CONFIG_GLOBAL` consistently.
3. **Manage and explain profiles in the UI:** Add profile setup, edit, disable,
   and status views; show effective identity and profile source before writes.
4. **Verify account isolation:** Cover fallback, matching paths, linked
   worktrees, local overrides, credential-helper delegation, and commit author
   identity; manually verify GitHub and GitLab authentication using the user's
   existing credential setup.

## Risks & open questions

- **Standalone shell Git commands:** This plan only isolates commands launched
  by Gitview. If the requirement includes every `git` command launched from a
  terminal in the folder, that needs a separate shell-level design.
- **Local config precedence:** Repository-local values can override the
  selected profile. Gitview must reveal the effective identity and provide a
  clear resolution path without silently editing `.git/config`.
- **Credential selection:** HTTPS helpers may identify accounts by host or
  path; SSH relies on remote host aliases and key configuration. Gitview
  delegates secrets to the configured helper and reports authentication errors.
- **Profile path changes:** Moving or symlinking a repository can change which
  profile matches. Resolve the real worktree path and show the selected rule.

## Boundaries

Keep the primary global config unchanged. Never write tokens or private keys.
Apply profile selection only to Gitview's Git subprocesses unless a later,
separately approved shell integration extends the scope. Preserve Git's normal
system and repository-local configuration, show their effective contribution,
and do not silently rewrite remotes or repository config.

[global-env]: https://git-scm.com/docs/git#Documentation/git.txt-codeGITCONFIGGLOBALcode
[include-if]: https://git-scm.com/docs/git-config#_conditional_includes
[git-config]: https://git-scm.com/docs/git-config
[credentials]: https://git-scm.com/docs/gitcredentials
