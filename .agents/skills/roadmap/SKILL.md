---
name: roadmap
description: Creates and maintains ROADMAP.md — the Now / Next / Later sequencing of features above the per-feature level. Adds items, moves them across buckets, and links each to its plans/<slug>.md once work starts. Use when deciding what to build next or when the user says "roadmap", "add to roadmap", "what's next", or "prioritize".
---

# Roadmap

## Overview
Maintain a single `ROADMAP.md` that answers "what are we building, in what order?" at the feature level. It sits above `plans/` and `SPEC.md`: items here are one-liners that become full plans only when they move into active work. It's a living file you edit, not regenerate.

## When to Use
- Deciding or reordering what to build next.
- Capturing an idea that isn't ready to plan yet.
- The user says "roadmap", "prioritize", "what's next", "add to roadmap".

## Process

### 1. Create or load ROADMAP.md
If it doesn't exist, create it with three buckets: **Now**, **Next**, **Later**. If it exists, read the current state first — never overwrite it blind.

### 2. Place each item in exactly one bucket
- **Now** — actively being worked (should have a plan in `plans/`).
- **Next** — committed, planned soon, not started.
- **Later** — ideas and intentions, deliberately vague.
One line per item: a short title and a half-sentence of intent. No detail — detail lives in the plan.

### 3. Keep Now small
"Now" is work-in-progress, not a wish list. If more than a few items sit in Now, that's a focus problem — push some back to Next.

### 4. Link items to their plans
When an item moves into **Now**, link it to its `plans/<slug>.md` (and `DESIGN.md` if one exists) so the roadmap is the index into active work.

### 5. Move, don't delete
When something ships, move it to a short **Shipped** section with a date rather than deleting it — the roadmap doubles as a lightweight history.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "I'll just keep the roadmap in my head." | Then no one else can see priorities, and future-you forgets why order changed. Write it. |
| "Let me add full detail to each item." | Detail belongs in the plan file. A roadmap item is a one-liner or it rots. |
| "Everything is 'Now'." | Then nothing is. Now is WIP only; be honest about Next and Later. |
| "Delete shipped items to keep it clean." | Move them to Shipped — the history of what you chose and when is valuable. |

## Red Flags — stop and fix
- `ROADMAP.md` was overwritten instead of edited.
- "Now" holds many items (no real focus).
- Items carry paragraphs of detail instead of a line.
- Active items don't link to their plan file.

## Verification — done when
- `ROADMAP.md` has Now / Next / Later (+ Shipped) and each item sits in exactly one.
- "Now" is small and every Now item links to a plan.
- Shipped items are dated, not deleted.
