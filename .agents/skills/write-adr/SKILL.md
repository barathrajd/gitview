---
name: write-adr
description: Records an Architecture Decision Record as docs/adr/NNNN-title.md so the reasoning behind a decision is preserved for the team. Captures status, context, the decision, and its consequences. Use when a non-trivial or hard-to-reverse decision is made, or when the user says "record this decision", "write an ADR", or asks "why did we choose X".
---

# Write ADR

## Overview
Capture *why* a decision was made in a short, numbered, immutable record under `docs/adr/`. ADRs are the team's knowledge-sharing trail: anyone (human or agent) can later read why a path was chosen without asking the people who chose it. One decision per file.

## When to Use
- A decision was made that someone will question in six months (framework, data model, boundary, trade-off).
- The decision is hard or expensive to reverse.
- The user says "record this decision", "write an ADR", "capture why we chose…".

## Process

### 1. Find the next number
Look in `docs/adr/`. Take the highest existing `NNNN` and add one; start at `0001` if empty. Create `docs/adr/` if it doesn't exist.
- **Exit criterion:** you have the next zero-padded number and a short kebab-case title.

### 2. Write one decision, not many
Each ADR records exactly one decision. If you're tempted to record several, write several files.

### 3. Fill the standard format
Create `docs/adr/NNNN-short-title.md` with:
- **Title:** `NNNN. Short decision statement`
- **Status:** Proposed | Accepted | Deprecated | Superseded by NNNN
- **Context:** the forces at play — what made this decision necessary. Neutral, factual.
- **Decision:** what was chosen, stated in active voice ("We will …").
- **Consequences:** what becomes easier and what becomes harder as a result. Include the downsides honestly.

### 4. Keep it immutable
Don't rewrite an accepted ADR. If the decision changes, write a new ADR and mark the old one `Superseded by NNNN`. The history is the point.

### 5. Link it
Reference the ADR from the relevant `DESIGN.md` or plan file so the decision is discoverable from the work it governs.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "The decision is obvious, no need to record it." | Obvious today, mysterious in six months. The ADR is for future-you. |
| "I'll just edit the old ADR." | ADRs are immutable. Supersede it with a new one so the reasoning trail survives. |
| "I'll only write the decision, skip consequences." | A decision without its downsides reads as propaganda. Record what it costs. |
| "One big ADR covers all our choices." | One decision per file, or no one can find or supersede them individually. |

## Red Flags — stop and fix
- An existing accepted ADR is being edited instead of superseded.
- The file lists multiple unrelated decisions.
- Consequences section only lists benefits.
- No status, or status doesn't match reality.

## Verification — done when
- `docs/adr/NNNN-title.md` exists with the next sequential number.
- It has Status, Context, Decision, and Consequences (including downsides).
- It records exactly one decision.
- It's linked from the design/plan it supports.
