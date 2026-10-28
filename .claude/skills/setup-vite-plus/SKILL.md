---
name: setup-vite-plus
description: Sets up or migrates a project onto the Vite+ unified toolchain (the vp CLI — Vite, Vitest, Oxlint, Oxfmt, Rolldown, tsdown, Vite Task). Covers install, config in vite.config.ts, the core vp commands, and native git hooks. Use when the user says "set up vite plus", "use vp", "migrate to vite+", or wants the build/test/lint toolchain configured.
---

# Setup Vite+

## Overview
Get a project onto Vite+ so build, dev, test, lint, format, and git hooks run through one `vp` CLI and one `vite.config.ts`. Use for a new project or to migrate an existing Vite project. Treat the official docs at viteplus.dev as the source of truth — this skill is the workflow, not a replacement for the reference.

## When to Use
- Standing up a new frontend/app project, or migrating an existing Vite project.
- The user says "set up vite plus / vp", "migrate to vite+", "configure the toolchain".

## Process

### 1. Install the CLI
- **Project-local (default):** `pnpm add -D vite-plus@1.1.0`, commit the lockfile, and run through the package manager: `pnpm exec vp <cmd>`. For a one-off, `pnpm dlx --package=vite-plus@1.1.0 vp <cmd>`. Bump the pinned version deliberately (see `references/supply-chain.md`).
- **Global** (manages Node + package managers across projects): the installer at viteplus.dev is a shell script. Do not run it through a pipe (`curl … | bash`) and do not run it as the agent. If the user wants it, point them to the official docs and tell them to download the script, read it, and run it themselves.
- **Exit criterion:** `pnpm exec vp help` works (local), or the user confirms `vp help` works after their own global install.

### 2. Create or migrate
- New project: `vp create` → pick template, target dir, package manager.
- Existing Vite project: `vp migrate` (or hand the coding agent the migration prompt from viteplus.dev). Preserve unrelated changes.

### 3. Configure in vite.config.ts
Import `defineConfig` from `vite-plus` (a superset of Vite's) so Vite+ keys are recognized:
```ts
import { defineConfig } from 'vite-plus';
export default defineConfig({
  // existing Vite options stay as-is
});
```

### 4. Learn the core commands
`vp dev` (dev server), `vp check` (format + lint + typecheck together), `vp test` (Vitest), `vp build` (apps), `vp pack` (libraries), `vp preview`. Project scripts run via `vp run <task>` / `vpr`.

### 5. Enable git hooks (optional but recommended)
`vp config` installs the hook dispatcher and sets `core.hooksPath`. See the `setup-commit-lint` skill for the full commit-message + staged-check wiring. Do not add husky — Vite+ owns the hook path.

### 6. Monorepo
Run per-package with `vp -C <path> <cmd>`. Each package's `vite.config.ts` carries its own config (including the `staged` block).

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "Keep importing defineConfig from 'vite'." | Then Vite+ keys like `staged` are flagged as unknown. Import from `vite-plus`. |
| "Add husky for hooks." | Vite+ manages hooks; husky collides on `core.hooksPath`. Use `vp config`. |
| "Guess the flags." | Vite+ evolves. Check `vp help <cmd>` or viteplus.dev before asserting a flag. |

## Red Flags — stop and fix
- `defineConfig` imported from `vite`, not `vite-plus`, while using Vite+ keys.
- husky installed alongside Vite+ hooks.
- A shell installer piped into `bash`/`sh`, or run by the agent, instead of a pinned project-local install.
- Node runtime below Vite+'s requirement for staged checks (22.22.1+ / 24.11.0+).

## Verification — done when
- `vp check` and `vp build` run successfully.
- `vite.config.ts` imports `defineConfig` from `vite-plus`.
- Hooks (if enabled) show active via `vp hooks status`, with no husky present.
