---
name: project-structure
description: Decides and scaffolds the folder structure for a frontend codebase — a standalone web app, a monorepo (apps + packages/core + packages/ui), or a reusable library — with one-way dependency rules, public-API boundaries, tests in a top-level tests/ folder, and config placement for either Rstack (rs) or Vite+ (vp). Use when the user says "project structure", "folder structure", "how should I organize this app", "monorepo layout", "library structure", "where should this code live", "where do reusable components go", or "restructure this repo".
---

# Project Structure

## Overview
Picks one layout for the repo's shape and applies it, so every app, monorepo, and library here is organized the same way. It produces a folder tree, dependency rules, and public-API boundaries — not business code. In a monorepo, all reusable components live in `packages/ui` and all core logic in `packages/core`. It works with either toolchain, Rstack (`rs`) or Vite+ (`vp`), and verifies with that toolchain's own commands. It does NOT set up the toolchain (`setup-rstack`, `setup-vite-plus`) or Module Federation (`setup-module-federation`); it tells them where their config files go.

## When to Use
- Starting a new app, monorepo, or library and nothing defines its layout yet.
- A codebase has grown organically and "where does this go?" has no clear answer.
- Adding a new app/package/feature to an existing repo and needing to place it.
- Deciding where a reusable component or a piece of core logic belongs.
- The user says "project structure", "folder structure", "monorepo layout", "restructure this".

## Process

### 1. Detect the shape and the toolchain
Follow `references/workspace-detection.md` to classify the repo as **standalone app**, **monorepo**, or **library** (and each package inside a monorepo the same way). Then detect the toolchain:
- `rstack.config.ts`, or `rstack` in devDependencies → **Rstack** → `references/toolchain-rstack.md`
- `vite.config.ts` importing from `vite-plus`, or `vite-plus` in devDependencies → **Vite+** → `references/toolchain-vite-plus.md`
- Both, or a mix across packages → use the toolchain of the package being touched.
- Neither → use the repo's own `package.json` scripts, state that no reference applies, and do not invent `rs`/`vp` commands.

Also check for an existing layout and for `module-federation.config.ts`.
- **Exit criterion:** you can state the shape, the toolchain, and what layout already exists, and you have opened the matching toolchain reference.

### 2. Choose the layout
Open the matching reference and use it as the target; do not invent a variant:
- Standalone app → `references/structure-app.md`
- Monorepo → `references/structure-monorepo.md` (apps follow the app layout; `packages/core` and `packages/ui` follow the library layout flavors)
- Library → `references/structure-library.md`

If the repo already has a layout, propose the smallest move toward the target, not a rewrite.
- **Exit criterion:** the target tree is written down, with each existing top-level folder mapped to where it ends up.

### 3. Scaffold or migrate in slices
Create folders and move code one feature/package at a time. Each slice keeps the build green. Put config where the toolchain reference says (root = shared lint/format/hooks; package = build/test/pack). Add `index.ts` public entries as you go. In a monorepo, move reusable components into `packages/ui` and core logic into `packages/core`. Move any tests found inside `src/` into the package's top-level `tests/`, mirroring the source path, and fix imports to use the `@/` alias or the public entry.

Verification commands come from the toolchain reference, never from memory or from the other toolchain:

| Check | Rstack | Vite+ |
| --- | --- | --- |
| Lint + format + types | `rs check --type-check` | `vp check` |
| Build an app | `rs build` | `vp build` |
| Build a library | `rs lib` | `vp pack` |
| Test | `rs test` | `vp test` |

- **Exit criterion:** every moved slice passes the check and build commands for its toolchain (and the repo's own, if neither toolchain applies) before the next slice starts.

### 4. Enforce the boundaries
Write the dependency rules from the reference into the repo's `AGENTS.md` (via `bootstrap-context` if absent). Add an automated check if the toolchain supports one; if you cannot confirm a tool supports import-boundary rules, say so and use a dependency-graph check (e.g. `madge --circular`) or a small script instead of claiming enforcement.
- **Exit criterion:** a deliberate violation (e.g. a feature deep-importing another feature's internals, or `core` importing React) is caught by the check, or the gap is stated explicitly.

### 5. Record the decision
Any non-obvious choice (layer-first vs feature-first, package scope, where a component lives) goes through `write-adr`.
- **Exit criterion:** an ADR exists for each non-default choice, or none were made.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "Put it in `utils` for now." | `utils` becomes the dumping ground. Name it by what it does, put it in `core` (no React) or the owning feature, and promote only when a second consumer exists. |
| "Keep this reusable component in the app, we'll move it later." | "Later" means two copies. Reusable components go in `packages/ui` when they become reusable. |
| "`core` can import React just for this one hook." | A React import ends `core`'s use outside React and its plain-Node tests. Put React code in `ui` or the app. |
| "Import the other feature's file directly, it's quicker." | Deep imports make features un-movable. Import only through that feature's `index.ts`. |
| "Move everything at once, then fix the build." | A broken build hides which move caused what. Migrate in green slices. |
| "Export everything from the library root." | The `exports` map is your public API; every extra export is a promise to maintain. |
| "Put the test next to the file, it's easier to find." | This repo keeps tests in a top-level `tests/` folder, outside `src/`, mirroring the source path. `src/` stays production code only. |
| "Add build config to the monorepo root so it's shared." | Only lint, format, staged and hooks are shared at the root. Build, test and pack config belong to each project; how they combine differs per toolchain, so check its reference. |
| "Reuse the `rs` commands from the skill, they're close enough." | A Vite+ repo has no `rs`. Use the commands from the detected toolchain's reference, or the repo's own scripts. |

## Red Flags — stop and fix
- A package imports from an app, or a shared package imports from a feature.
- `packages/core` imports `react`, `react-dom`, or browser APIs; or `packages/ui` fetches data, reads routes, or touches app stores.
- The same reusable component exists in two apps, or a reusable component sits in an app's `src/components/` inside a monorepo.
- Two features import each other (cycle).
- A folder named `utils`, `helpers`, `common`, or `misc` with unrelated contents.
- A library lists `react` / `react-dom` under `dependencies` instead of `peerDependencies`.
- Any `*.test.*` / `*.spec.*` file, or test helper, inside `src/`.
- Build/test config duplicated at both the repo root and a package.
- An `rs` command run in a Vite+ repo, or a `vp` command in an Rstack repo.

## Verification — done when
- The tree matches the chosen reference (apps/packages/features/shared in the right places; in a monorepo, reusable components in `ui` and core logic in `core`).
- The toolchain's check and build commands pass in every touched package (table above, or the repo's own scripts).
- Every package has a `tests/` folder mirroring `src/`; `find src -name '*.test.*' -o -name '*.spec.*'` returns nothing; the toolchain's test command discovers and runs the tests in `tests/`.
- `packages/core` has no React/DOM imports (`grep -rE "from 'react|from \"react" packages/core/src` returns nothing).
- The dependency rules are written in `AGENTS.md` and a violation is caught or the gap is stated.
- No cycles: the dependency-graph check reports none.
