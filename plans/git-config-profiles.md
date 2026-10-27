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
- Terminal `git` commands use folder profiles only when the user explicitly
  enables shell integration; without the opt-in, terminal behavior is unchanged.
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
- An optional `--shell-integration` setup flow that installs a reversible,
  user-owned shell hook; the hook selects `GIT_CONFIG_GLOBAL` for terminal Git
  commands based on the current worktree folder.
- Identity (`user.name`, `user.email`), signing preferences, credential-helper
  selection, and SSH command settings where the user configures them.
- A profile selector/editor and visible active-profile state in Gitview.

**Out of scope**

- Automatically changing the remote URL, SSH keys, or credential-store entries.
- Storing passwords, access tokens, or private keys in Gitview profile files.
- Changing the user's primary `~/.gitconfig` or repository `.git/config` as a
  side effect of selecting a profile.
- Changing the environment for Git commands started outside Gitview unless the
  user explicitly enables the optional shell integration.
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
commands for matched folders. The optional shell integration uses a reversible
shell hook to export the selected profile file before invoking terminal Git;
it remains disabled until the user opts in and can be removed without changing
the primary config. Supported shells, installation UX, and safe hook updates
must be confirmed before implementation. Never claim shell isolation from an
`includeIf` rule alone.

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
4. **Add optional terminal integration:** Implement `--shell-integration` as
   an explicit, reversible opt-in and document the supported shells.
5. **Verify account isolation:** Cover fallback, matching paths, linked
   worktrees, local overrides, credential-helper delegation, commit author
   identity, and terminal opt-in/off behavior; manually verify GitHub and
   GitLab authentication using the user's existing credential setup.

## Risks & open questions

- **Shell compatibility:** Shell hooks differ and user startup files vary. Keep
  shell integration opt-in, support a documented shell set, back up managed
  edits, and provide an uninstall path. Gitview's own subprocess isolation
  remains available without shell integration.
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
Apply profile selection to Gitview's Git subprocesses by default. Apply it to
terminal Git commands only after the user enables `--shell-integration`.
Preserve Git's normal system and repository-local configuration, show their
effective contribution, and do not silently rewrite remotes or repository
config.

[global-env]: https://git-scm.com/docs/git#Documentation/git.txt-codeGITCONFIGGLOBALcode
[include-if]: https://git-scm.com/docs/git-config#_conditional_includes
[git-config]: https://git-scm.com/docs/git-config
[credentials]: https://git-scm.com/docs/gitcredentials
