# Toolchain: Vite+ (`vp`)

Used by `project-structure` for config placement and for the commands that verify a layout. Source of truth: viteplus.dev. This file covers only what affects structure; for setup use `setup-vite-plus`.

**Detect:** a `vite.config.ts` that imports `defineConfig` from `vite-plus`, or `vite-plus` in `devDependencies`. Git hooks live in `.vite-hooks/`.

## Command map
| Task | Command |
| --- | --- |
| Check (format + lint + typecheck) | `vp check` (`vp check --fix` to apply fixes) |
| Dev server (app) | `vp dev` |
| Build app | `vp build` |
| Preview app build | `vp preview` |
| Build library | `vp pack` |
| Test | `vp test` |
| Run a `package.json` script | `vp run <script>` (built-ins like `vp build` can't be overridden by scripts) |
| Build every package, dependency order | `vp run -r build` |
| One package | `vp run --filter ./apps/web build`, or `vp -C apps/web build` |
| Staged-file checks | `vp staged` |

## Config placement
Everything is in `vite.config.ts` using `defineConfig` from `vite-plus`. In a monorepo the **root** config owns lint and format.

| Where | Holds |
| --- | --- |
| Repo root `vite.config.ts` | `lint`, `fmt` (with `overrides` for per-package rules), `staged`, optionally `defaultPackage` |
| App package `vite.config.ts` | Vite options and plugins, `test` |
| Library package `vite.config.ts` | `pack` (`dts`, `format`, `sourcemap`), `test` |

- `vp check` always uses the **root** `lint` and `fmt` blocks, even when run from inside a package. Package-level `lint`/`fmt` blocks are discouraged and cannot replace the root's settings.
- Per-package lint or format rules: `lint.overrides` / `fmt.overrides` in the root config, using workspace-relative globs such as `apps/web/**`. Shared override objects can live in a file (for example `tooling/lint/react.ts`) and be imported into the root config.
- Typed linting: `lint.options.typeAware: true` and `lint.options.typeCheck: true` in the root config.
- `staged` goes in the root config next to the hooks. The docs do not say whether it can also be set per package, so keep it at the root.

## Hooks
`vp config` installs the hook dispatcher and sets `core.hooksPath`. The default hooks folder is `.vite-hooks/`. Commit `.vite-hooks/pre-commit` containing `vp staged`, with the rules in the `staged` block (for example `'*': 'vp check --fix'`). Do not add husky.

## Tests in `tests/`
`vp test` reads the `test` block (Vitest options). The docs' example sets `include: ['src/**/*.test.ts']`, so set `include` explicitly to cover `tests/`, for example `['tests/**/*.test.{ts,tsx}']`, and confirm `vp test` runs them.

## Monorepo behaviour to know
- `vp dev`, `vp build`, `vp preview` and `vp pack` do not silently act on the workspace root. With one app package vp runs it; with several it opens a picker, and in a non-interactive shell (CI) it lists the packages and exits with code 1. Use `-C <dir>`, `--filter`, or `defaultPackage` in the root config.
- `vp run -r` runs in dependency order taken from each package's `dependencies`. `--filter` uses pnpm's syntax (`@org/ui`, `"@org/*"`, `./packages/ui`, `"@org/app..."`). A filter that matches nothing only warns and exits 0, so add `--fail-if-no-match` in CI.
- Tasks defined in `vite.config.ts` are cached by default; plain `package.json` scripts are not unless you pass `--cache`.

## Verify a layout
| Check | Command |
| --- | --- |
| Whole repo | `vp check` at the root |
| Every package builds, in order | `vp run -r build` (apps use `vp build`, libraries `vp pack`) |
| One app builds | `vp -C apps/<app> build` |
| One library builds | `vp -C packages/<pkg> pack` |
| Tests discovered in `tests/` | `vp test` in that package |

## Not prescribed by Vite+
The Vite+ docs do not define a folder layout for `src/`, apps or packages, and the `vp create vite:monorepo` template layout is not documented. The folder structure comes from the `structure-*.md` references.
