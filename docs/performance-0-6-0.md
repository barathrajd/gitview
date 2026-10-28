# Gitview 0.6.0 performance review

This report records partial execution of the approved
[performance plan](../plans/gitview-0-6-0-performance.md). The whole-TUI review
and the 0.6.0 release gate remain open. Measurements below are local results,
not product-wide performance guarantees.

## Measured fixes

Commit and amend previews now prepare their Git snapshots in a read worker with
one active job and at most one queued replacement. Escape invalidates a preview;
its completion cannot reopen a cancelled form. Submission still validates the
reviewed staged state and, for amend, `HEAD`.

The main loop now draws after input, resize, elapsed search timers, or worker
results. Completed tag, comparison, and object loads request a redraw. Status
polling retains its 750 ms interval. The live terminal regression checks both
startup data and an externally created file without keyboard input.

## Native small-fixture results

These runs used an optimized binary on Darwin ARM64 with Rust 1.99.1 and Git
2.56.0, a 140x35 pseudo-terminal, 50 commits, 100 tracked files, and 10 staged
paths. Each matching Git status/staged-diff call incurred a 250 ms injected
delay during the preview check. Fixture setup and compilation are excluded.

| Measurement | Baseline | After fixes |
| --- | --- | --- |
| Commit loading feedback p95 | 587.9 ms, 5 starts | 12.5 ms, 20 starts |
| Escape response p95 after feedback | 12.7 ms, 5 starts | 12.6 ms, 20 starts |
| First loading frame p95 | 433.3 ms, 5 starts | 48.3 ms, 20 starts |
| Gitview idle CPU, one core | 1.8%, 5 seconds | 0.2%, 30 seconds |
| Idle terminal output | Not captured | 19 bytes over 30 seconds |
| Idle RSS | 4,784 to 4,960 KiB | 4,544 to 4,704 KiB |
| Amend loading feedback p95 | Not captured | 12.5 ms, 3 starts |
| Amend Escape response p95 | Not captured | 12.5 ms, 3 starts |

Before the fix, the first preview feedback was the completed form; afterward it
is a loading screen while Git still runs. Baseline cancellation was therefore
measured after blocking preparation finished. The after-fix check cancels during
preparation and waits to ensure a late result does not reopen the form.

Startup distributions have different sample counts and are subject to process
and filesystem cache effects. Disk caches were not cleared. The first process
start and subsequent starts are retained in the raw samples; do not treat this
as proof of a startup speedup. Idle samples have different durations and exclude
Git-child CPU. CPU/RSS come from platform `ps` and have limited precision; the
three-second amend sample reporting zero CPU does not establish zero CPU usage.
Response sampling uses 10 ms read windows and includes harness overhead.
Idle RSS is not the memory-plateau/cycling test required by the plan.

Raw evidence is checked in with machine/toolchain and fixture metadata:

- [Baseline samples](performance/macos-baseline.json).
- [Commit samples after fixes](performance/macos-commit-after.json).
- [Amend samples after fixes](performance/macos-amend-after.json).

The baseline binary was built from application revision `a320218`; planning
files were dirty. After-fix reports also identify `a320218` because implementation
was uncommitted, with `worktree_dirty: true` and the binary SHA-256 recorded.
Rerun the final matrix at an identifiable committed revision before release.

## Reproducing the checks

Use Python 3 and Git on native macOS or Linux. The script creates and removes
its own temporary repository and Git-delay shim. Build the binary explicitly;
then run:

```sh
cargo build --release --locked
python3 scripts/profile-tui.py --samples 20 --idle-seconds 30 \
  --check-preview --check-idle --output /tmp/gitview-performance.json
python3 scripts/profile-tui.py --action amend --samples 20 \
  --idle-seconds 30 --check-preview --output /tmp/gitview-amend.json
```

Use `--fixture medium` or `--fixture stress` and `--width 80 --height 24` for
additional cases. Those combinations have not yet been measured here.
`--check-preview` currently requires every sampled loading/cancel response to
stay within 100 ms. `--check-idle` checks at most 2% process CPU and 256 terminal
output bytes over the selected idle interval. These checks cover this harness's
startup/idle/commit/amend slice, not the whole plan. Its JSON explicitly lists
unmeasured workflows.

The existing `cargo test --release --locked --test diff_switching -- --nocapture`
command measures the comparison fixture separately. It does not measure process
startup, Git completion time, or ordinary worktree/commit diff first display.

## Path, ref, and tag preview response

The second slice moves discard, untrack, remove/move, branch rename/delete,
previous checkout, and ignore inspection into a shared bounded read worker.
Tag creation previews and signature verification use another bounded read
worker owned by the tag page. Each worker retains one active request and at
most one queued replacement. Escape invalidates the read result and returns
to the source screen; an active Git command may finish in the background.
Confirmed mutations keep their existing adapters and checks.

Failed commit/amend attempts now refresh their preview in the background while
retaining the subject, description, and error. Cancelling that refresh preserves
the message but invalidates the expected snapshot; submission requires reopening
the composer to review staged state. The rejected-hook runtime check verifies
cancellation and a normal refreshed retry, including a single successful commit
after the temporary fixture hook is removed.

The delayed-Git debug baseline took 871.6 ms to show the discard confirmation
(one sample). The native optimized follow-up uses a two-file repository with
one unstaged change, a previous checkout, and a lightweight tag. It runs three
samples per case, injecting 250 ms into **every** Git command during preparation.
The table reports sample maxima, not a statistically established p95:

| Preview case | Maximum loading response | Maximum Escape response |
| --- | --- | --- |
| discard | 14.8 ms | 14.8 ms |
| untrack | 14.9 ms | 14.9 ms |
| remove | 12.4 ms | 12.4 ms |
| move | 12.5 ms | 12.5 ms |
| branch-rename | 12.5 ms | 12.4 ms |
| branch-delete | 12.2 ms | 12.4 ms |
| previous | 12.5 ms | 12.5 ms |
| ignore | 12.3 ms | 12.4 ms |
| tag-create | 12.5 ms | 12.5 ms |
| tag-verify | 12.6 ms | 12.5 ms |
| ignore-path | 12.4 ms | 12.5 ms |

All 33 feedback/cancel samples pass the diagnostic's 100 ms limit. Repository
status and refs remain unchanged after the checks. This does not establish the
full fixture/platform performance gate. Verification uses an unsigned tag;
external signing tools, key retrieval, and signed-tag cases remain unaudited.

- [Debug discard baseline](performance/macos-discard-preview-baseline.json).
- [Native path/ref/tag samples](performance/macos-path-ref-previews.json).

The harness now reconstructs cursor movement and erase sequences instead of
searching only raw terminal byte fragments, so incremental redraws are measured
as visible screen state. Its sampling still uses 10 ms read windows.

To repeat the diagnostics against an explicitly built binary:

```sh
cargo build --release --locked
python3 scripts/profile-previews.py --samples 3 --check \
  --output /tmp/gitview-previews.json
python3 scripts/check-preview-retry.py
```

Use `--case` to isolate a preview and `--binary` to select a build. These scripts
use temporary repositories; the retry check exercises a temporary pre-commit
hook and creates a commit only inside its fixture. The native reports above
identify the prior committed revision with a dirty worktree and record binary
SHA-256; final release evidence still requires an identifiable clean revision.

## Review status and remaining work

The following checks remain required before release:

- Complete the workflow matrix and larger-fixture sampling for the newly
  asynchronous path/ref/ignore/tag previews and failed-commit refreshes. Audit
  profile/config/editor handling and read-worker lifecycle across page closure.
- Profile ordinary worktree/commit diff first display, wrapping, hunk movement,
  scrolling, and resizing; their render cache miss still prepares syntax in the
  UI thread. Complete the 100-action matrix for all supported diff variants.
- Cover navigation, debounced search, large status/ref lists, signed-tag/key
  verification behavior, object inspection, mutation progress, and external
  editor/hook/signing responsiveness.
- Run medium/stress fixtures, cache memory plateaus, and request lifecycle checks.
- Collect native Linux evidence and report Git-child CPU separately.
- Clear the release Clippy gate and complete the existing release gates.

Feature-enabled tests pass, including stale-state, terminal restoration, and
live external refresh coverage. The current release Clippy command still fails
on pre-existing findings in `comparison.rs`, `history.rs`, and `ui.rs` (nested
conditions, complex types, prefix stripping, and a one-range vector). This is
an open release blocker, not a passing check.
