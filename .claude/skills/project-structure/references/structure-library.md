# Structure: library

Applies to a reusable package — standalone repo or a package inside a monorepo. Built in library mode: `define.lib()` with Rstack, or `vp pack` with Vite+. Config files and commands per toolchain: `references/toolchain-rstack.md`, `references/toolchain-vite-plus.md`.

## Layout
```
<lib>/
├── package.json         # name, version, exports, files, peerDependencies, scripts
├── <toolchain config>   # rstack.config.ts (define.lib, define.test, optional define.doc) OR vite.config.ts (pack, test)
├── tsconfig.json
├── README.md
├── src/
│   ├── index.ts         # the ONLY public entry
│   ├── <module>/
│   │   ├── index.ts     # module's public surface (re-exported from src/index.ts if public)
│   │   └── <module>.ts
│   └── internal/        # shared by modules, never exported
├── tests/               # outside src/, mirrors src/ layout
│   ├── setup.ts         # optional setup, shared helpers, fixtures
│   └── <module>/<module>.test.ts
├── docs/                # optional Rspress site
└── dist/                # build output, git-ignored
```

## Public API rule
- `package.json` `exports` is the contract. Export `src/index.ts` (and deliberate subpaths); everything else is private.
- No deep imports into `src/` from consumers; consumers use the package name.
- Adding an export is a decision; removing one is a breaking change.
- Types are shipped (`dts: true` in the library build config), and `exports` points at them.

## Dependencies
- Framework packages (`react`, `react-dom`, router, state libs) are `peerDependencies`, never bundled and never pinned in `dependencies`.
- Mirror them in `devDependencies` so tests and the dev build resolve.
- A leaf library (e.g. `utils`) imports no internal packages; a UI library may import the leaf library and nothing app-level.

## Rules of placement
- One module per folder; shared private code in `src/internal/`.
- Tests go in top-level `tests/`, never inside `src/`, mirroring the source path (`src/<module>/<module>.ts` → `tests/<module>/<module>.test.ts`). They import through the public entry or a `@/` alias, so `tests/` also checks the public API. Include `tests/` in `tsconfig.json`; keep it out of the build entries and `files`, so it never ships.
- Set the test config's `include` explicitly so `tests/` is picked up, then confirm the test command runs them (see the toolchain reference).
- Component libraries: one public component per file, styles co-located, no app-specific state or routing.
- Keep `scripts` minimal and uniform, mapped to the toolchain's commands: `build` (`rs lib` or `vp pack`), `dev` (`rs lib -w`, Rstack only), `test` (`rs test` or `vp test`), `doc` (`rs doc`, only if configured).

## Flavors used by the monorepo (`references/structure-monorepo.md`)

**`core` — logic library.** Framework-agnostic: no `react`, `react-dom`, or DOM APIs; imports no internal package.
```
packages/core/src/
├── index.ts
├── types/            # shared types and schemas
├── api/              # api client, request functions (no React hooks)
├── <domain>/         # business rules per domain: billing/, auth/, ...
├── format/           # formatters, named by what they do
└── internal/
```
Expose large domains as `exports` subpaths (`@org/core/billing`) instead of one huge root entry.

**`ui` — component library.** Reusable, presentational React components. `react`/`react-dom` are peer dependencies. May import `core` for types and pure helpers only; no data fetching, routing, app stores, or business rules.
```
packages/ui/src/
├── index.ts
├── tokens/           # colors, spacing, typography
├── primitives/       # Button, Input, ...
├── composites/       # DataTable, FormField, ...
├── hooks/            # UI-only hooks (useDisclosure, ...)
├── styles/
└── internal/
```
One public component per file; export each through `index.ts` only after it is reusable. If the project builds on Base UI and StyleX, the `base-ui` and `stylex` skills cover how primitives are wrapped and styled here; `tokens/` holds the StyleX variables.

## Variations
- **Multiple entry points:** declare each subpath in `exports` and the matching entries in the library build config (Rslib, or Vite+ `pack`); keep one `index.ts` per entry.
- **Module Federation producer library:** add `module-federation.config.ts` with `exposes` pointing at public modules only. See `setup-module-federation`.
