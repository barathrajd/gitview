---
name: zustand
description: Client-state conventions for Zustand in this stack, backed by the two best available external skills (there is NO official Zustand skill). Covers store shape, selector stability, async actions, stale-closure traps, and persist. Use when the user says "zustand", "set up a store", "client state", "selectors", or "persist state".
---

# Zustand

## Overview
How we use Zustand for client state here, plus the two external skills worth pulling in. **Important: Zustand (pmndrs) does not publish an official agent skill.** So unlike `tanstack`, there's nothing version-synced to install — these are community references, and this skill is the house authority on top of them.

Scope: client/UI/app state only. Server data belongs to TanStack Query (see the `react` and `tanstack` skills), never a Zustand store.

## The two external skills — verified
1. **`CJHarmath/claude-agents-skills` → `react-zustand-patterns`** (primary).
   - **Verified:** the file exists at `skills/react-zustand-patterns/SKILL.md`, and it's the **shared web+native** variant — the right one for a React **web** app. *Not* `rn-zustand-patterns` (that's React Native only).
   - **Honesty check:** it's a small personal repo (~0 stars, few commits), not a popular pack. Content is solid (set() render timing, stale closures via `getState()`, async actions, selector stability, derived state, `persist` middleware, v4→v5 migration), but read it before trusting it — it's a reference, not an authority.
2. **`microsoft/skills` → `zustand-store-ts`** (alternative).
   - Microsoft's zero-config TypeScript store template (store/action separation, `subscribeWithSelector`). Note: Microsoft-authored, **not** official Zustand.

### Install (project-scoped, pinned, user-confirmed)
These are third-party instructions the agent will follow, so install one only after the user confirms. Use the pinned-commit procedure in `references/external-skills.md` (clone, check out the pinned SHA, read the skill, copy just that folder into the **repo root** skills dir so every package in a monorepo shares it). Skills to copy:
- Recommended: `react-zustand-patterns` from `CJHarmath/claude-agents-skills`.
- Alternative: `zustand-store-ts` from `microsoft/skills`.

Do not run the repo's `install.sh`, `npx skills add`, or `dlx` for these, and do not use `-g`. Pick **one** to avoid two overlapping store-pattern skills. `react-zustand-patterns` is the recommendation.

## House conventions
| Area | Rule |
| --- | --- |
| Scope | Client/UI/app state only. Never store server data (that's TanStack Query). |
| Selectors | Always select the narrowest slice: `useStore(s => s.x)`. Never `useStore()` whole — it re-renders on every change. |
| Object selectors | Returning a new object/array from a selector needs `useShallow` (v5) or a shallow comparator, or it re-renders every time. |
| Actions | Co-locate actions inside the store; mutate via `set`. Keep components calling actions, not reaching into `set`. |
| Async | Async actions call `set` on resolution; read current state with `get()` to avoid stale closures. |
| Derived state | Compute in the selector or in render — don't store derived values. |
| Persist | Use `persist` middleware deliberately; set `partialize` so you don't persist transient/server-ish fields. |
| Structure | One store per concern. Don't build a single global god-store. |

## Process
1. **Confirm it's client state.** If it comes from the API, stop — use TanStack Query, not a store.
2. **Install one external skill** (`react-zustand-patterns` recommended) if the agent needs pattern depth.
3. **Shape the store:** state + co-located actions; one store per concern.
4. **Select narrowly:** slice selectors; `useShallow` for object/array returns.
5. **Handle async with `get()`**, set on resolution; no stale closures.
6. **Persist deliberately** with `partialize` if used.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "Store the fetched data in Zustand too, it's convenient." | Two sources of truth = stale bugs. Server state is Query's job. |
| "`const state = useStore()` is simpler." | It subscribes to the whole store and re-renders on any change. Select the slice you need. |
| "Return `{a, b}` from the selector directly." | New object every render → infinite re-renders. Use `useShallow` / shallow compare. |
| "Read state from the closure in an async action." | Closures capture stale state. Use `get()` for the current value. |
| "There's a Microsoft skill, so it's official." | Microsoft ≠ pmndrs. No official Zustand skill exists; treat both as community references. |

## Red Flags — stop and fix
- Server/API data living in a Zustand store.
- `useStore()` with no selector, or object selectors without shallow compare.
- Async actions relying on closed-over state instead of `get()`.
- `persist` with no `partialize`, persisting transient state.
- Both `react-zustand-patterns` and `zustand-store-ts` installed (pick one).

## Verification — done when
- Stores hold only client state; server data stays in TanStack Query.
- Components use slice selectors (and `useShallow` for object returns).
- Async actions use `get()`/`set` correctly — no stale-closure bugs.
- At most one external Zustand skill is installed, and the house rules above hold.
