# Toolchain: Rstack (`rs`)

Used by `project-structure` for config placement and for the commands that verify a layout. Source of truth: rstack.rs. This file covers only what affects structure; for setup use `setup-rstack`.

**Detect:** `rstack.config.ts` at the root or in a package, or `rstack` in `devDependencies`. Git hooks live in `.rstack/hooks/`.

## Command map
| Task | Command |
| --- | --- |
| Check (lint + format, with typecheck) | `rs check --type-check` |
| Dev server (app) | `rs dev` |
| Build app | `rs build` |
| Preview app build | `rs preview` |
| Build library | `rs lib` (`rs lib -w` to watch) |
| Test | `rs test` |
| Docs site | `rs doc` |
| Staged-file checks | `rs staged` |

Run a project's commands from that project's directory, or through the workspace runner (`pnpm -F <pkg> <script>`, `turbo run <task> --filter=<pkg>`, `nx run <pkg>:<target>`).

## Config placement
Two levels. The CLI loads `rstack.config.ts` from the current working directory and **does not merge** a project's config with the root config.

| Where | File | Holds |
| --- | --- | --- |
| Repo root | `rstack.config.ts` | `define.lint()`, `define.fmt()`, `define.staged()` only, plus the shared `rstack` version in `package.json` |
| App package | `rstack.config.ts` | `define.app()` (+ plugins), `define.test()` |
| Library package | `rstack.config.ts` | `define.lib()` (`dts`, `format`), `define.test()`, optional `define.doc()` |

- Each `define.*()` is called at most once per config file.
- Typed linting: list each project's `tsconfig.json` in the root lint config's `parserOptions.project` (for example `./apps/*/tsconfig.json`, `./packages/*/tsconfig.json`).
- Per-project lint rules: `files` patterns in the root lint config, resolved from the repo root.
- Project-only dependencies (Rsbuild plugins, test libraries) go in the project's `package.json`, not the root.
- Root scripts: `"check": "rs check --type-check"`, `"lint": "rs lint"`, `"format": "rs fmt"`, `"staged": "rs staged"`.

## Hooks
`"prepare": "rs hooks"` in the root `package.json` sets `core.hooksPath` to `.rstack/hooks/_`. Put `rs staged` in `.rstack/hooks/pre-commit`, with the `define.staged()` rules in the root config. Do not add husky.

## Tests in `tests/`
`define.test()` takes Rstest options. The Rstest default for which files it finds was not confirmed here, so set `include` explicitly to cover `tests/` (check rstest.rs/config for the option shape) and confirm `rs test` actually runs them.

## Verify a layout
| Check | Command |
| --- | --- |
| Whole repo | `rs check --type-check` at the root |
| One app builds | `rs build` in `apps/<app>` |
| One library builds | `rs lib` in `packages/<pkg>` |
| Tests discovered in `tests/` | `rs test` in that package |

## Not prescribed by Rstack
Rstack does not define a folder layout for `src/`, apps or packages. The folder structure comes from the `structure-*.md` references.
