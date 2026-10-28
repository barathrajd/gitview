---
name: setup-release-please
description: Sets up GitHub-side automated releases with release-please, driven by Conventional Commits — no local release CLI. Drops in the workflow, config, and manifest; flags the one-time GitHub setting; and enforces the commit conventions that trigger releases. Use when a repo needs release automation, or when the user says "set up releases", "add release-please", or "automate versioning".
---

# Setup Release-Please

## Overview
Wire a repo for hands-off, GitHub-side releases using [release-please](https://github.com/googleapis/release-please-action). It reads Conventional Commits on `main`, maintains a "Release PR" that bumps the version and writes `CHANGELOG.md`, and on merge tags the version and creates a GitHub Release. Nothing runs on a developer's machine. Pairs with `release-plan`, which decides *what* goes in a release; this skill sets up the machinery that *ships* it.

## When to Use
- A repo has no release automation and uses (or will use) Conventional Commits.
- The user says "set up releases", "add release-please", "automate versioning/changelog".

## Process

### 1. Choose the release type
- **`simple`** — repos that aren't published to a package registry (docs, skills, config, generic projects). Version lives in the manifest.
- **`node`** — npm packages (also updates `package.json`).
Pick `simple` unless the repo publishes a package.
- **Exit criterion:** the release type matches what the repo actually ships.

### 2. Add the three files
- `.github/workflows/release-please.yml` — runs on push to `main`, `permissions: contents: write` + `pull-requests: write`, uses `googleapis/release-please-action@v4` with `config-file` and `manifest-file`.
- `release-please-config.json` — the chosen `release-type`, `changelog-path`, and any `extra-files` that also carry a version (e.g. a plugin/package manifest, synced via a `json` + `jsonpath` entry).
- `.release-please-manifest.json` — starting version. Use `0.0.0` so the first release computes to `0.1.0`.

### 3. Enable the one-time GitHub setting
Repo **Settings → Actions → General → Workflow permissions** → check **"Allow GitHub Actions to create and approve pull requests."** Without it, the action can't open the Release PR.
- **Exit criterion:** the setting is enabled (or the user is told to enable it).

### 4. Enforce Conventional Commits
Releases only fire on conventional messages. State the rules so commits actually trigger versions:
- `fix:` → patch · `feat:` → minor · `feat!:` / `BREAKING CHANGE:` → major · `chore:`/`docs:`/`ci:` → no release.
Recommend commitlint + a husky `commit-msg` hook if the team needs enforcement.

### 5. Seed the first release (optional)
To force a specific first version regardless of commit history:
```
git commit --allow-empty -m "chore: release 0.1.0" -m "Release-As: 0.1.0"
```

### 6. Verify the loop
Push a `feat:`/`fix:` commit, confirm release-please opens a Release PR with the right version and a changelog, and that merging it creates the tag and GitHub Release.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "Skip the GitHub permission setting, it'll work." | It won't — the action can't open the Release PR without it. Enable it first. |
| "We don't need strict commit format." | release-please silently ignores non-conventional commits — no prefix, no release. Enforce the format. |
| "Pin the action to a floating tag." | Pin a major (`@v4`). Floating tags break silently when inputs change. |
| "Bump the version by hand too." | Don't — the manifest is the source of truth. Manual edits desync it from the tags. |

## Red Flags — stop and fix
- The workflow lacks `contents: write` or `pull-requests: write`.
- The GitHub "allow Actions to create PRs" setting is off.
- Commits aren't in Conventional format, so no Release PR ever appears.
- Version is edited by hand instead of flowing through the manifest.

## Verification — done when
- The three files exist and the action is pinned to a major version.
- The GitHub Actions PR-creation setting is enabled.
- A test `feat:`/`fix:` commit produces a Release PR with the correct version and changelog.
- Merging the Release PR creates the tag and GitHub Release.
