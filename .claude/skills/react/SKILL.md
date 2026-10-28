---
name: react
description: The React umbrella for this stack — house conventions for components, hooks, data fetching, and client state, plus pointers to the dedicated tanstack and zustand skills. Scoped to React 18+, TypeScript, Vite+, TanStack Query, and Zustand. Use when the user says "react", "react conventions", "react best practices", "how should I structure this component/hook", or starts React work in this repo.
---

# React

## Overview
One entry point for React work in this stack. It encodes how *we* write React here and routes the two big external concerns to their own skills:
- **Data fetching / server state** → `tanstack` (official TanStack Intent).
- **Client state** → `zustand`.
- **Primitive components / styling** → `base-ui` / `stylex`, when the project uses those libraries.

Everything else (component shape, hooks, TypeScript, rendering, testing) lives below. Scope: React 18+ and TypeScript, bundler-agnostic (Vite / Vite+ / Rstack / Next), web only (not React Native).

## When to Use
- Starting or reviewing a React component, hook, or feature in any repo.
- Deciding where state belongs (server vs client vs local).
- Setting up a new contributor on the house React conventions.

## Detect the workspace first
Don't assume a layout. Figure out the repo shape before writing files — it changes where React code lives and where skills install. See `references/workspace-detection.md`. In short:
- **Standalone app** — React code under the root `src/`.
- **Monorepo** — the React app is one package (e.g. `apps/<name>` or `packages/<name>`); run React tooling in that package, keep repo-wide config at the root, install skills once at the root.
- **Library** — reusable components, no app shell; library-mode build, and `react`/`react-dom` are `peerDependencies`, not bundled.

## House conventions (the short version)
| Area | Rule |
| --- | --- |
| Server state | TanStack Query owns anything fetched from the API. Never mirror server data into Zustand. See the `tanstack` skill. |
| Client state | Zustand for cross-component UI/app state; `useState` for local. See the `zustand` skill. |
| Components | Function components only. Keep them presentational; push data/effects to hooks. |
| Hooks | One concern per custom hook. Follow the rules of hooks; stable deps; no conditional hooks. |
| TypeScript | No `any` in component/hook public surfaces. Props typed explicitly; infer internals. |
| Effects | `useEffect` is a last resort — not for deriving data (compute in render) or syncing server state (that's Query). |
| Rendering | Measure before memoizing. `memo`/`useMemo`/`useCallback` only with evidence of a real re-render cost. |
| Files | Co-locate component, hook, and styles; tests go in the top-level `tests/` folder (outside `src/`), mirroring `src/`. One public component per file. Folder layout for the app/monorepo/library: see the `project-structure` skill. |

## Routing — where each concern goes
| Concern | Go to |
| --- | --- |
| Fetching, caching, mutations, invalidation, pagination, prefetch | `tanstack` |
| Global/UI client state, stores, selectors, persistence | `zustand` |
| Primitive components (dialog, menu, select, popover), `render` composition | `base-ui` |
| Styling, tokens, themes, typed style props | `stylex` |
| Component/hook structure, TS, effects, rendering | this skill |

## Optional external React skills (vetted, use with judgment)
These are *supplementary* references, not house authority. The strongest matches for this stack (React web, no RN) come from `CJHarmath/claude-agents-skills` — note it's a small personal repo (low traction), so read the content before trusting it:
- `react-performance` — memoization, virtualization, code splitting.
- `react-testing` — Jest + React Testing Library + MSW.
- `react-async-patterns`, `react-state-flows` — async flows and multi-step operations.

These are third-party instructions the agent will follow, so install one only after the user confirms. Use the pinned-commit procedure in `references/external-skills.md`, run from the **repo root** (so every package in a monorepo shares them), project-scoped, and read each skill before copying it. Do not use unpinned `npx skills add` / `dlx` or `-g`.

Skip the repo's `backend` (Python/FastAPI) and `rn-*` (React Native) skills — they don't match this stack.

## Process
1. **Classify the state.** Server data → `tanstack`. Shared client state → `zustand`. Local → `useState`. Don't duplicate across layers.
2. **Shape the component.** Presentational; data and effects in hooks. Explicit prop types.
3. **Reach for the right skill.** Fetching/mutations → `tanstack`. Stores/selectors → `zustand`.
4. **Only optimize with evidence.** Profile first; memoize second.
5. **Add a test in `tests/`.** Mirror the source path, outside `src/`. React Testing Library; test behavior, not implementation.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "Keep the fetched list in a Zustand store so it's easy to read." | Server state belongs to TanStack Query. Mirroring it causes stale/double-source bugs. Use the `tanstack` skill. |
| "A `useEffect` to sync the API into local state is fine." | That's exactly what Query removes. Effects for server sync are a smell. |
| "Wrap everything in `useMemo`/`memo` to be safe." | Unmeasured memoization adds noise and bugs. Profile, then optimize the real cost. |
| "This component can do its own fetching inline." | Push data access into a hook so it's testable and cache-aware. |

## Red Flags — stop and fix
- Server data stored in Zustand or synced via `useEffect`.
- `any` on a component or hook's public types.
- Memoization added with no profiling evidence.
- A component mixing fetching, business logic, and presentation.

## Verification — done when
- State lives in exactly one layer (server→Query, client→Zustand, local→useState).
- Components are presentational; data/effects isolated in hooks.
- `tanstack` and `zustand` were used for their concerns, not reinvented here.
- A test under `tests/` (mirroring the source path) exercises behavior and passes.
