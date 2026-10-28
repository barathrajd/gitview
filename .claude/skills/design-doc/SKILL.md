---
name: design-doc
description: Writes a technical design document (DESIGN.md) for a feature before implementation. Covers context, goals/non-goals, the proposed approach, alternatives considered, data/API shape, risks, and rollout. Use after a spec exists and before building anything architecturally non-trivial, or when the user says "design this", "write a design doc", or "how should we build X".
---

# Design Doc

## Overview
Produce a `DESIGN.md` that explains *how* a feature will be built and *why* this approach over the alternatives. Sits between the spec (what) and the plan (tasks). Skip it for trivial changes; write it when a choice is hard to reverse or crosses a system boundary.

## When to Use
- A plan in `plans/` or a `SPEC.md` exists and the work is architecturally non-trivial.
- A change touches data models, public APIs, or multiple systems.
- The user says "design this", "design doc", "how should we build…".

## Process

### 1. Anchor to the spec
Read the spec/plan. Restate in one line what this design must satisfy. If no spec exists, stop and route to `/spec` or `rnd-product-plan` first.
- **Exit criterion:** the design's target behavior is stated and tied to a source.

### 2. State goals and non-goals
List what this design must achieve and — just as important — what it explicitly will not do. Non-goals prevent scope creep in the build.

### 3. Draft the proposed approach
Describe the chosen design concretely: components, data flow, the data model and/or API shape, and where it fits existing architecture. Prefer the boring, obvious design over the clever one.

### 4. Record alternatives considered
At least one real alternative, with why it was rejected. A design doc with no rejected options is a decision no one can audit later.

### 5. Call out risks and rollout
Name the risks, each with a mitigation or fallback. Note how it ships (flag, migration, staged rollout) and how it rolls back.

### 6. Write DESIGN.md
Sections, in order: **Context**, **Goals / Non-goals**, **Proposed design** (with data model/API), **Alternatives considered**, **Risks & mitigations**, **Rollout & rollback**. For any decision worth preserving beyond this feature, hand off to `write-adr`.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "I'll just start coding, the design is in my head." | A design in your head can't be reviewed or revisited. Write the one page. |
| "There's only one way to do this." | Then naming the alternative and why it's worse takes two minutes and proves it. |
| "Non-goals are obvious." | They're the first thing that gets violated mid-build. Write them. |
| "Rollback is a deploy concern, not design." | A design you can't back out of is a risk you chose silently. Decide it now. |

## Red Flags — stop and fix
- You're writing DESIGN.md with no spec to anchor to.
- There are zero alternatives considered.
- No non-goals and no rollback story.
- The doc describes code line-by-line instead of the shape of the solution.

## Verification — done when
- `DESIGN.md` exists with every section filled.
- At least one alternative is documented with a reason for rejection.
- Risks each have a mitigation; rollout and rollback are stated.
- Durable decisions have been captured as ADRs via `write-adr`.
