---
name: setup-commit-lint
description: Sets up Conventional Commit message enforcement plus staged-file checks using the project's own hook manager. Detects the toolchain — Vite+ (vp hooks), Rstack (rs hooks), or falls back to husky — so it never runs two hook managers. Use when the user says "set up commitlint", "enforce conventional commits", "commit message hooks", or "commit workflow".
---

# Setup Commit Lint

## Overview
Wire a repo so bad commit messages are rejected and staged files are checked before every commit — using whatever hook manager the project already has, never a second one. The commit-message layer is `@commitlint/config-conventional`; the staged layer is the toolchain's native command. Conventional Commits here also feed release automation (see `setup-release-please`).

## When to Use
- A repo needs commit-message enforcement and/or pre-commit checks.
- The user says "set up commitlint", "conventional commits", "commit hooks", "commit workflow".

## Process

### 1. Detect the hook manager — pick exactly one
Inspect `package.json` and the repo:
- `vite-plus` present → **Vite+**: hooks via `vp config` / `vp hooks`, staged via `vp staged`.
- `@rstack` / `rstack` / `rs` toolchain present → **Rstack**: hooks via `rs hooks`, staged via `rs staged`.
- Neither → **husky** fallback: `husky` + `lint-staged`.
- **Exit criterion:** one manager chosen; confirm no other is installed (`git config core.hooksPath`).

### 2. Install the hook dispatcher
- Vite+: `vp config` (installs dispatcher, sets `core.hooksPath`; hooks live in `.vite-hooks/`).
- Rstack: add `"prepare": "rs hooks"` to `package.json` and run it once (sets `core.hooksPath` to `.rstack/hooks/_`; hook scripts live in `.rstack/hooks`; use `rs hooks --force` if another path exists).
- husky: `pnpm add -D husky && pnpm exec husky init`.

### 3. Pre-commit — staged checks
- Vite+: add a `staged` block to `vite.config.ts` (`staged: { '*': 'vp check --fix' }`) and put `vp staged` in `.vite-hooks/pre-commit`.
- Rstack: add `define.staged({ ... })` to `rstack.config.ts` (e.g. `'*.{ts,tsx}': ['rs lint --fix', 'rs fmt']`) and put `rs staged` in `.rstack/hooks/pre-commit`.
- husky: `pnpm add -D lint-staged`, add a `lint-staged` block to `package.json`, put `pnpm exec lint-staged` in `.husky/pre-commit`.

### 4. Commit-message — Conventional Commits
Install commitlint and wire the `commit-msg` hook:
```bash
pnpm add -D @commitlint/cli @commitlint/config-conventional
```
`.commitlintrc.json`:
```json
{ "extends": ["@commitlint/config-conventional"] }
```
commit-msg hook contents by manager:
- Vite+ (`.vite-hooks/commit-msg`): `vp exec commitlint --edit "$1"`
- Rstack (`.rstack/hooks/commit-msg`): `pnpm exec commitlint --edit "$1"`
- husky (`.husky/commit-msg`): `pnpm exec commitlint --edit "$1"`

### 5. Commit the project-owned hook scripts
`.vite-hooks/*` / `.rstack/hooks/*` / husky `.husky/*` scripts and the config files are committed; generated dispatcher internals (`.vite-hooks/_`, `.rstack/hooks/_`) stay gitignored.

### 6. Verify
Bad message is rejected; a conventional message passes and runs the pre-commit checks.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "Just use husky everywhere." | If the project has Vite+/Rstack, husky collides on `core.hooksPath`. Use the native manager. |
| "Skip commit-msg, pre-commit is enough." | Without message enforcement, release automation can't derive versions. Enforce the format. |
| "Run both the native hooks and husky." | Two managers fight over the hook path. Exactly one. |
| "Enforce format later." | Later never comes, and history is already non-conventional. Enforce from the first commit. |

## Red Flags — stop and fix
- Two hook managers installed (e.g. Vite+ hooks + husky).
- `core.hooksPath` points somewhere other than the chosen manager.
- No `commit-msg` hook, so non-conventional messages slip through.
- Pre-commit runs full-repo checks instead of staged-only.

## Verification — done when
- Exactly one hook manager owns `core.hooksPath`.
- A non-conventional commit message is rejected; a conventional one passes.
- Staged checks run on commit via the toolchain's staged command.
- Hook scripts and configs are committed to the repo.
