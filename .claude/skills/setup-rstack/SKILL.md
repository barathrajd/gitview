---
name: setup-rstack
description: Sets up or migrates a project onto the Rstack toolchain via the rs CLI — Rsbuild, Rslib, Rspress, Rstest, Rslint, plus formatting and git hooks — through one rstack.config.ts. Use when the user says "set up rstack", "use rs", "migrate to rstack", or wants the Rust-based build/test/lint toolchain configured.
---

# Setup Rstack

## Overview
Get a project onto Rstack so build, dev, test, lint, format, docs, and git hooks run through one `rs` CLI and one `rstack.config.ts`. Rstack is the Rust-powered web toolchain; the CLI composes its tools behind `define.*()` APIs. Source of truth: rstack.rs (per-tool docs linked below).

## The Rstack toolchain (hardened reference)
| Tool | Role | Config API | Command | Docs |
| --- | --- | --- | --- | --- |
| **Rspack** | Rust bundler that powers Rsbuild | (under `app`) | — | rspack.rs |
| **Rsbuild** | App build tool on Rspack | `define.app()` | `rs dev` / `rs build` / `rs preview` | rsbuild.rs/config/ |
| **Rslib** | Library builder | `define.lib()` | `rs lib` | rslib.rs/config/ |
| **Rspress** | Docs site builder (needs `@rspress/core`) | `define.doc()` | `rs doc` | rspress.rs |
| **Rstest** | Test runner | `define.test()` | `rs test` | rstest.rs/config/ |
| **Rslint** | Linter | `define.lint()` | `rs lint` | rslint.rs/config/ |
| **Prettier** | Formatter | `define.fmt()` | `rs fmt` | prettier.io/docs/options |
| **lint-staged** | Staged-file tasks | `define.staged()` | `rs staged` | lint-staged README |
| **Rsdoctor** | Build analyzer / diagnostics | — (add per rsdoctor.rs) | — | rsdoctor.rs |

`rs check` runs lint + format together (optional typecheck). `define.extends()` shares config across a monorepo.

## When to Use
- Standing up a new project, or migrating an existing one onto Rstack.
- The user says "set up rstack / rs", "migrate to rstack", "configure the Rstack toolchain".

## Process

### 1. Check the runtime and install
Rstack CLI needs **Node.js 22.18.0+ or 24.3.0+** (or Deno/Bun).
- New project: `pnpm create rstack@3.4.3` (templates: app/lib/doc/monorepo across React, Vue, Svelte, Solid, etc.). Non-interactive: `pnpm dlx create-rstack@3.4.3 my-app -t app-react-ts`. Versions are pinned, never `@latest`, and no `npx -y` (see `references/supply-chain.md`).
- Existing project: `pnpm add -D rstack@0.8.3` and commit the lockfile, then follow the migration guide (rstack.rs/guide/migration — it has a migration skill). Bump the pin deliberately.
- **Exit criterion:** `pnpm rs --help` resolves (`rstack` is an alias for `rs`).

### 2. Configure rstack.config.ts
Import `define` from `rstack` and call only the APIs the project needs:
```ts
import { define } from 'rstack';

define.app({ /* Rsbuild config */ });
define.test({ /* Rstest config */ });
define.lint(({ js, ts }) => [js.configs.recommended, ts.configs.recommendedTypeChecked]);
define.fmt({ singleQuote: true });
```
Each `define.*()` may be called at most once. `define.test()` auto-inherits `define.app()` (or `define.lib()`) unless you set `extends`. For `rs doc`, also `pnpm add -D @rspress/core`.

### 3. Learn the core commands
`rs dev`, `rs build`, `rs preview`, `rs lib`, `rs doc`, `rs test`, `rs check`, `rs lint`, `rs fmt`. Add the common ones to `package.json` scripts; they use the project-local `rs` binary (no global install needed).

### 4. Git hooks
Add `"prepare": "rs hooks"` to `package.json` and run it once. `rs hooks` sets `core.hooksPath` to `.rstack/hooks/_` (hook scripts live in `.rstack/hooks`). Put `rs staged` in `.rstack/hooks/pre-commit` and configure `define.staged()`:
```ts
define.staged({
  '*.{js,jsx,ts,tsx,mjs,cjs,mts,cts}': ['rs lint --fix', 'rs fmt'],
  '*.{json,md,mdx,css,scss,less,html,yml,yaml}': 'rs fmt',
});
```
If another hooks path already exists, `rs hooks --force`. For commit-message enforcement, use the `setup-commit-lint` skill (Rstack branch). Do not add husky — Rstack owns the hook path.

### 5. Monorepo
Share build/test/lint/format settings with `define.extends([sharedConfig])`; see rstack.rs/guide/monorepo.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "Guess the per-tool options from memory." | Options follow each underlying tool. Check the linked docs (rsbuild.rs, rstest.rs, rslint.rs) before asserting. |
| "Add husky for hooks." | Rstack owns `core.hooksPath` via `rs hooks`; husky collides. |
| "Call define.app() twice to merge." | Each `define.*()` runs at most once — a second call throws. Use `define.extends()` to share. |
| "Import every tool at the top of the config." | That adds startup cost to `rs lint`/`rs fmt`. Dynamically import inside the relevant `define.*()` function. |

## Red Flags — stop and fix
- A second hook manager (husky) installed alongside `rs hooks`.
- `@latest`, `npx -y`, or any unpinned scaffolder/CLI version in an install command.
- Node runtime below 22.18.0 / 24.3.0.
- A `define.*()` API called more than once.
- Asserting a tool option not confirmed against its linked docs.

## Verification — done when
- `rs check` and `rs build` run successfully.
- Config lives in `rstack.config.ts` via `define.*()`.
- `git config --get core.hooksPath` returns `.rstack/hooks/_` (if hooks enabled), with no husky present.
