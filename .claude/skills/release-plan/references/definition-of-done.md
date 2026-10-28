<!-- Vendored copy. Source of truth: the repo-root references folder, file definition-of-done.md. Edit it there, then run scripts/sync-references.sh. -->

# Definition of Done

A standing bar every change clears, regardless of the task. Skills reference this file so the bar lives in one place. (Per-task acceptance criteria are separate — they live in `PLAN.md`.)

- [ ] Behavior is covered by a test that existed before the implementation and now passes.
- [ ] The full test suite is green.
- [ ] Build / typecheck / lint is clean.
- [ ] User-facing changes have a runtime check or screenshot of the real behavior.
- [ ] The change touches only what the task required — no drive-by refactors.
- [ ] The change is committed alone, with a standalone, imperative message.
- [ ] No secrets in code, logs, or version control.
- [ ] Any new assumption or decision is written down (in `PLAN.md`, an ADR, or the commit body).
- [ ] Nothing that was working before is now broken.
