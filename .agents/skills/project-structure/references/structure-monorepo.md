# Structure: monorepo

Applies to a pnpm workspace (with or without Turborepo/Nx) holding several apps and shared packages. The folders below are toolchain-neutral. Which config file sits where, and which commands verify it, depends on the toolchain: see `references/toolchain-rstack.md` or `references/toolchain-vite-plus.md`.

Shared code starts as two packages: **`core`** (logic) and **`ui`** (reusable components).

## Layout
```
.
├── package.json             # private; root scripts run check / lint / format across the repo
├── pnpm-workspace.yaml      # packages: apps/*, packages/*; catalog: for shared versions
├── <root toolchain config>  # rstack.config.ts OR vite.config.ts: shared lint, format, staged only
├── tsconfig.base.json       # shared compiler options; each project extends it
├── <hooks dir>/             # .rstack/hooks OR .vite-hooks: one set for the whole repo
├── AGENTS.md  GLOSSARY.md   # repo-wide context
├── docs/adr/                # decisions
├── .claude/skills/          # skills installed once at the root
├── apps/
│   └── <app>/               # own package.json + toolchain config (build, test)
│       ├── src/             # laid out per references/structure-app.md
│       └── tests/           # outside src/, mirrors src/
└── packages/
    ├── core/                # all core logic: framework-agnostic, no React, no DOM
    └── ui/                  # all reusable components: presentational, no data or business rules
```
Each package follows `references/structure-library.md` (the `core` and `ui` flavors are described there), including its own `tests/` folder. Tests are per project (`apps/<app>/tests`, `packages/<pkg>/tests`); there is no repo-wide `tests/` at the root.

## What goes where
| Code | Lives in |
| --- | --- |
| Reusable component (used, or clearly reusable, by more than one screen or app): primitives, composites, tokens, theme, UI-only hooks | `packages/ui` |
| Core logic: types and schemas, business rules, API client and request functions, validators, formatters, constants, pure helpers | `packages/core` |
| Component used by exactly one feature | `apps/<app>/src/features/<feature>/components` |
| Data hooks (TanStack Query), client stores (Zustand), routes, screens, app wiring | the app |

Rule of thumb: if it has no React, it belongs in `core` or the app's `lib/`; if it is a reusable React component, it belongs in `ui`; if it fetches, routes or holds app state, it stays in the app.

## Dependency rule
`apps → ui → core`, and `apps → core`. Imports only point down.
- Nothing imports from `apps/*`.
- `core` imports no internal package, and no `react`, `react-dom`, or browser/DOM APIs. That keeps it testable in plain Node and usable outside React.
- `ui` may import `core` for types and pure helpers only. `ui` never fetches data, reads routes, touches app stores, or contains business rules.
- Declare every internal dependency in `package.json` with `workspace:*`; never reach across packages by relative path.

## Config placement (neutral)
- **Root:** shared lint, format, staged-file checks, git hooks, the toolchain's version, release automation, CI.
- **Project:** that project's build, test, and library-pack config. How root and project config combine differs per toolchain, so follow the toolchain reference rather than assuming root settings reach a project.
- Project-only dependencies (build plugins, test libraries) go in the project, not the root.

## Rules of placement
- Naming: scope internal packages (`@org/core`, `@org/ui`); folder name = package name without scope.
- Promote reusable components into `ui` when they become reusable; do not leave copies in two apps.
- Keep the app's `src/components/` for app-specific composites that wire `ui` pieces to the app's data; reusable pieces live in `ui`.
- Shared versions: pin framework singletons (`react`, `react-dom`, router, state libs) in the pnpm `catalog:` and reference them as `catalog:` in packages.
- Run package tasks in the owning package or through the workspace runner (`pnpm -F <pkg> <script>`, `turbo run <task> --filter=<pkg>`, `nx run <pkg>:<target>`, or the toolchain's own: see the toolchain reference).
- A third shared package needs a second consumer or a clear public API before it exists; otherwise the code stays in `core`, `ui`, or the app.

## Variations
- **Growing `core`:** organize it by module folders (`src/billing/`, `src/auth/`) and expose each through `exports` subpaths (`@org/core/billing`). Split into separate packages only when modules need a different release cadence or dependency footprint.
- **Shared React hooks across apps:** keep them in `ui` if they serve components; if they wrap `core` and are needed by several apps, add `packages/hooks` once there is a second consumer.
- **Micro-frontends (Module Federation):** each producer/consumer is its own app with its own `module-federation.config.ts`; the host lives in `apps/<shell>`, remotes in `apps/<remote>`. Share singletons through MF `shared` and the catalog. See `setup-module-federation`.
- **Docs site:** `apps/docs` (or a package) using the toolchain's docs builder if it has one.
- **Nx instead of plain pnpm tasks:** keep the same `apps/` + `packages/` shape; the task graph comes from Nx project config rather than `pnpm -F`.
