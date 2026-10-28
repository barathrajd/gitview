---
name: release-plan
description: Creates and maintains RELEASE.md — pre-planned, feature-based releases. Defines each release (version + target), the specific features it will contain pulled from the roadmap/plans, a release gate checklist, and status. Use when planning what goes into a release, grouping features into a versioned cut, or when the user says "plan a release", "what's in the next release", or "cut a release".
---

# Release Plan

## Overview
Maintain `RELEASE.md`: which features ship together, under what version, by when. Unlike a changelog (what already shipped), this is **planned ahead** — you decide the contents of a release up front, then track it to the gate. Each release is a named/versioned bundle of roadmap items.

## When to Use
- Grouping specific features into a planned, versioned release.
- Deciding or reviewing what the next release contains.
- The user says "plan a release", "what's in the next release", "cut a release", "release plan".

## Process

### 1. Create or load RELEASE.md
If absent, create it with two parts: **Upcoming** (planned releases) and **Released** (history). If it exists, read current state before editing.

### 2. Define the release header
For each upcoming release: a **version** (or codename), a **target date**, and a one-line **theme/goal**. Keep a release scoped to a coherent theme, not "everything we can cram in".

### 3. List its features — pulled from the roadmap/plans
Each feature is a line linking to its `ROADMAP.md` item and its `plans/<slug>.md`. Mark each `planned` / `in progress` / `done`. A release is only as real as the plans behind its features — if a feature has no plan, flag it, don't assume it.

### 4. Attach the release gate
Every release carries a checklist that must pass before it ships: all features `done`, tests green, `CONSTRAINTS.md` met, security check done (`security-and-hardening`), migration/rollback ready, docs/CHANGELOG updated. Reference the shared bar in `references/definition-of-done.md` rather than restating it.

### 5. Cut and archive
When the gate passes and it ships, move the release from **Upcoming** to **Released** with the actual date and a link to the PR(s)/tag. Don't edit a released entry afterward — it's history.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "Just ship whatever's ready, skip the plan." | Then "release" means nothing and you can't communicate what's coming. Define the bundle. |
| "This feature will definitely make it, mark it done." | Status reflects reality, not hope. `planned` until its plan says `done`. |
| "Skip the gate, we're in a hurry." | The gate is what makes a release safe to ship. Hurrying past it is how incidents happen. |
| "Cram one more feature into this release." | Scope creep slips the date for everything else in it. Push it to the next release. |

## Red Flags — stop and fix
- A release lists features that have no plan file.
- Feature statuses don't match the underlying plans.
- No release gate, or the gate was skipped.
- A `Released` entry is being edited after the fact.

## Verification — done when
- `RELEASE.md` has Upcoming and Released sections.
- Each upcoming release has a version, target date, theme, and feature list linked to plans.
- Each release has a gate referencing the definition of done.
- Shipped releases are archived with real dates and PR/tag links, unedited after.
