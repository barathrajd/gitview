---
name: bootstrap-context
description: Sets up a repo's standing context files — AGENTS.md (the rolling rulebook agents read every session) and GLOSSARY.md (the project's domain vocabulary) — by interviewing the user for stack, commands, conventions, boundaries, and terms. Use once per repo at the start, or when the user says "set up AGENTS.md", "bootstrap context", or "create the agent rules".
---

# Bootstrap Context

## Overview
Create the two standing files every agent reads before doing anything: `AGENTS.md` (how to behave in this repo) and `GLOSSARY.md` (what the project's words mean). These are loaded every session, so they must be short, true, and specific to this codebase — not generic advice. Run once per repo; re-run to update as the project changes.

## When to Use
- Starting a new repo or onboarding agents to an existing one.
- No `AGENTS.md` exists, or it's stale/generic.
- The user says "set up AGENTS.md", "bootstrap context", "create the agent rules".

## Process

### 1. Interview — one question at a time
Prefer reading the repo (package manifests, CI config, existing scripts) over asking. Only ask what you can't find. Gather:
- **Stack**: languages, frameworks, package manager, runtime versions.
- **Commands**: exact lines to install, run, test, typecheck/lint, build. These are the highest-value content — agents waste the most time guessing these.
- **Conventions**: naming, file layout, formatting rules, what the linter already enforces.
- **Boundaries**: what must not change, what must not break, directories to leave alone.
- **Domain terms**: the nouns this project uses that an outsider would misread.
- **Exit criterion:** you can fill every section below without guessing.

### 2. Write AGENTS.md
Keep it tight. Sections:
- **Project** — one paragraph: what this is and who it's for.
- **Stack** — the tools and versions.
- **Commands** — install / run / test / lint / build, as literal copy-paste lines.
- **Conventions** — the handful of rules that actually matter here.
- **Boundaries** — do-not-touch / do-not-break.
- **Where things live** — point to `ROADMAP.md`, `PLAN.md` (overview), `plans/`, `DESIGN.md`, `CONSTRAINTS.md`, `docs/adr/`, `GLOSSARY.md`.
- **Non-negotiables** — the five: surface assumptions before building; stop and ask when requirements conflict; push back when warranted; prefer the boring, obvious solution; touch only what you're asked to touch.

### 3. Write GLOSSARY.md
A table: **Term | Means**. One row per domain word that could be misread. No encyclopedic entries — just the shared definition. Add new terms here whenever the project coins one.

### 4. Point the tools at it
If the repo uses Claude Code, make `CLAUDE.md` a one-line pointer to `AGENTS.md` (don't duplicate). Note that Cursor/Codex/Copilot read `AGENTS.md` natively.

### 5. Keep it current
These are living files. When commands or conventions change, update `AGENTS.md` in the same change — a stale rulebook is worse than none.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "Generic best-practices are fine for AGENTS.md." | Generic advice gets ignored. The value is *this repo's* exact commands and boundaries. |
| "The agent can figure out how to run tests." | Watching it guess wastes a session and fills context. Write the one line. |
| "GLOSSARY is overkill, the terms are obvious." | Obvious to you. The agent mis-maps your domain nouns constantly. One table fixes it. |
| "I'll keep AGENTS.md and CLAUDE.md both detailed." | Two sources drift. One is canonical; the other points to it. |

## Red Flags — stop and fix
- `AGENTS.md` is full of generic advice with no project-specific commands.
- The commands section is missing or untested.
- `CLAUDE.md` and `AGENTS.md` both carry full, divergent content.
- `GLOSSARY.md` has long essays instead of one-line definitions.

## Verification — done when
- `AGENTS.md` exists with every section filled from real repo facts.
- The commands section has literal, working install/run/test/lint/build lines.
- `GLOSSARY.md` lists the project's domain terms as a term→meaning table.
- `CLAUDE.md`, if present, points to `AGENTS.md` rather than duplicating it.
