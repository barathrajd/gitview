---
name: run-the-plan
description: Executes a confirmed PLAN.md by building it in thin, test-first slices with a verification gate and a commit per slice. Use after rnd-product-plan when the user says "build it", "run the plan", "implement this", or "start on the milestones". Reads PLAN.md as the source of truth and stays inside its scope.
---

# Run the Plan

## Overview
Take a confirmed plan (`plans/<slug>.md`) and ship it one thin slice at a time. Each slice is implemented test-first, verified with real evidence, and committed on its own. The plan is the contract: build what it says, flag anything it doesn't cover, and never silently expand scope.

## When to Use
- A confirmed plan in `plans/` exists and the user wants it built.
- The user says "build", "implement", "run the plan", "start the milestones", or "ship it".

## Process

### 1. Load the plan and confirm the slice
Pick the plan: the path the user gave, else the `Now` item in `ROADMAP.md`, else the single entry in `PLAN.md`; if several qualify, ask which. Read only that plan file, not the others. Take the next unbuilt milestone. If the plan is missing, incomplete, or unconfirmed, stop and route back to `rnd-product-plan` — do not invent requirements.
- Restate the slice's acceptance criteria in one line before writing anything.

### 2. Red — write the failing test first
Write the smallest test that captures the slice's expected behavior. Run it. Watch it fail for the right reason. Code written before its test gets deleted and rewritten test-first.

### 3. Green — minimum code to pass
Write the least code that makes the test pass. Resist building for imagined future needs. Run the test. Watch it pass.

### 4. Refactor — only with tests green
Clean up names, duplication, and structure while the suite stays green. Don't fold new behavior into this step; that's a new slice.

### 5. Verify with evidence
Produce proof the slice works, not a claim that it does:
- Full test suite passes (paste the result).
- Build/typecheck is clean.
- For user-facing changes, a runtime check or screenshot of the actual behavior.
"Seems right" does not close a slice.

### 6. Commit this slice alone
One slice = one atomic commit with a standalone, imperative message ("Add rate limiter to login", not "Phase 1"). Keep changes small — if a slice balloons past a reviewable size, split it.

### 7. Loop or stop
Update the plan's milestone status. Move to the next slice. When all milestones are done, run the full suite once more and report what shipped against the plan's success criteria.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "I'll write the test after the code works." | After means never, and after-the-fact tests test the code you wrote, not the behavior you wanted. Test first. |
| "This slice is too simple to test." | Then the test is two lines. Write it — it documents the behavior and catches the regression. |
| "The plan didn't mention this, but I'll just add it." | Out-of-plan work is scope creep. Flag it to the user and let them decide; don't silently expand. |
| "Tests pass, ship it." | Passing tests are evidence, not proof. Did you check the runtime? Did the build stay clean? |
| "I'll commit everything at the end." | Big-bang commits are unreviewable and unrevertable. One slice, one commit. |
| "While I'm here I'll refactor this other file." | Touch only what the slice needs. Unrelated cleanup is its own change. |

## Red Flags — stop and fix
- You're implementing something that isn't in the plan file.
- You wrote code before a failing test existed.
- A single commit spans multiple milestones or mixes a refactor with new behavior.
- You're reporting "done" without a test run, build result, or runtime check.
- The slice grew to touch far more than the plan implied — split it or re-plan.

## Verification — a slice is done when
- Its test existed before its implementation and now passes.
- The full suite is green and the build is clean.
- User-facing behavior has a runtime check or screenshot.
- The change is committed alone with a descriptive message.
- The plan file's milestone status is updated.
