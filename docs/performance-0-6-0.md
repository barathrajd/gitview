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

## Ordinary worktree and committed-file diffs

The third slice moves ordinary diff highlighting into a single replaceable read
worker. Worktree and commit-file reads, plus refreshed diffs after staging, now
produce immutable prepared data before publication. Closing or superseding a
view invalidates its read result. Mutation execution and Git's checks stay in
the existing adapters; read cancellation does not cancel a confirmed write.

The prepared view retains wrapped rows and hunk positions for four style/layout
variants at one terminal width. Resizing evicts the prior width, and replacing
or closing the diff releases its prepared data. Drawing copies only visible
rows. The legacy `Screen::Diff` path remains available for loading/errors and
existing direct renderer callers; loaded application diffs use `PreparedDiff`.

A 400-line Rust replacement fixture produced these local diagnostic results:

| Check | Result |
| --- | --- |
| Legacy first render, debug sample | 941.8 ms on the render thread |
| Legacy first render, isolated optimized sample | 76.2 ms on the render thread |
| Prepared syntax, isolated optimized sample | 75.9 ms in the preparation thread |
| Prepared first frame, isolated optimized sample | 1.29 ms after preparation |
| Warm frames, 140x35 | 100 actions; p50 0.167 ms; p95 0.202 ms |
| Warm frames, 80x24 | 100 actions; p50 0.073 ms; p95 0.080 ms |
| Delayed-Git loading/Escape runtime sample | 11.1 ms / 12.3 ms |

Preparation work still exists; it is moved out of input/render handling. The
renderer measurements use Ratatui's in-memory test backend and exclude Git,
terminal I/O, and preparation. Runs do not establish maximum-diff, large-list,
Linux, or memory-plateau performance. Cold/resize samples are retained separately
from the warm sample statistics. The [diagnostic output](performance/macos-ordinary-diff-render.txt)
records toolchain, revision, binary identity, and individual first-use samples.
As with the earlier slices, the implementation worktree was dirty; final release
measurements still require an identifiable clean revision.

The runtime check opens actual worktree and committed-file diffs, cancels a
loading read, switches styles/layouts, and stages only the second of two hunks
in each of the four variants. It verifies the index contents, reloads the diff,
and checks that staging keys do not mutate a committed-file view. A regression
also checks changed content and resize invalidation. This check exposed an
existing unified IDE rendering bug: paired replacement rows hid additions.
Unified rendering and hunk geometry now include both sides of replacements.

To reproduce the isolated diagnostics and native check:

```sh
cargo test --release --locked --test ordinary_diff_performance \
  measures_ordinary_diff_first -- --nocapture --test-threads=1
cargo test --release --locked --test ordinary_diff_performance \
  prepares_off_thread -- --nocapture --test-threads=1
cargo build --release --locked
python3 scripts/check-ordinary-diff.py
```

## Large ordinary diffs and retained layouts

The next diagnostic covers a 122 KB text diff, a 958 KB text diff with 4,990
added lines, and a 900 KB diff with two 450,000-character source lines. These
fixtures exercise supported output sizes, but don't represent every language
or the complete repository/workflow matrix.

Ordinary layouts now retain at most four variants at one width and use an
8 MiB owned-allocation estimate to evict older variants. The estimate includes
row/span vector capacities, owned text capacities, and hunk positions. It
excludes allocator metadata and prepared syntax data. One active layout remains
available when it alone exceeds the budget; the budget doesn't truncate content.
An evicted variant is rebuilt when selected. Blank alignment rows, empty line
number gutters, and fixed markers also avoid unnecessary owned allocations.
The comparison page's separate cache remains pending review.

Explicit optimized-build diagnostics check a 100 ms first-layout budget and
50 ms p95 across 100 warm actions at both terminal sizes. These are in-memory
renderer checks, including recomputation after eviction; they exclude Git and
terminal I/O. Run them separately from the regular correctness suite:

```sh
cargo test --release --locked --test ordinary_diff_performance \
  measures_supported_limit_layouts -- --ignored --nocapture --test-threads=1
cargo test --release --locked --test ordinary_diff_performance \
  measures_diff_cache_lifecycle -- --ignored --nocapture --test-threads=1
cargo build --release --locked
python3 scripts/check-diff-cache-memory.py
```

The renderer-only lifecycle diagnostic warms ten cycles, then measures 100
open/drop cycles with four variants and 140x35 / 80x24 / 140x35 resizing.
Its initial settled RSS growth was 16,432 KiB, exceeding the 10,240 KiB budget.
The allocation changes and eviction reduce retained data, but repeated
renderer-only runs still exceed that RSS budget. This diagnostic remains a
known failure; it doesn't establish a memory leak or a passing release gate.

A separate counting-allocator diagnostic compares live heap bytes with RSS:

```sh
cargo test --release --locked --features test-support \
  --test diff_cache_live_allocations -- --ignored --nocapture --test-threads=1
```

It runs the same open, render, and drop cycle 110 times. Live heap after each
close stays at 1,941 KiB (1,942 KiB from cycle 70) against a 1,395 KiB starting
value, while the per-cycle peak stays at about 25,319 KiB. The first closed
view keeps roughly 546 KiB, and nothing grows with the cycle count. In the same
environment the RSS diagnostic still grew 16,320 KiB in steps, then plateaued.
This shows the dropped prepared views and layouts aren't retained by Gitview.
The RSS growth is consistent with the allocator keeping freed pages from a
roughly 25 MiB per-cycle peak. It doesn't show that RSS is bounded in the native
process, and the RSS diagnostic stays failing and unchanged.

The final optimized frame diagnostic passes: p95 is 22.37 ms / 10.54 ms for
958 KB at 140x35 / 80x24, and 7.55 ms / 8.74 ms for the long-line fixture.
The 122 KB case measures 0.324 ms / 0.138 ms. Each value covers 100 actions;
first-use and resize samples are recorded separately in the raw evidence.
Eviction trades some recomputation for lower retention, so these results aren't
a claim that every toggle remains a cache hit.

The native PTY script separately exercises a real Gitview process on 2,490
replaced text lines. It warms ten cycles, samples 100 cycles, and settles on the
same closed-view state before and after. It verifies that the index and working
diff remain unchanged. Native RSS and renderer-only RSS must be reported
separately because their allocation and execution contexts differ. The native
sample passes: settled RSS grows from 66,880 KiB to 71,728 KiB, or 4,848 KiB,
against a 10,240 KiB budget. The working diff is 941,342 bytes. This one passing
native fixture doesn't close the whole-TUI memory gate or erase the failing
renderer-only diagnostic.

The [large-diff evidence](performance/macos-large-diff-cache.txt) includes fixture
sizes, frame timings, failed renderer RSS checks, native RSS measurements,
toolchain, revision, and binary identity. All evidence comes from the macOS
implementation worktree; clean-revision and Linux evidence remain required.

## Review status and remaining work

The following checks remain required before release:

- Complete the workflow matrix and larger-fixture sampling for the newly
  asynchronous path/ref/ignore/tag previews and failed-commit refreshes. Audit
  profile/config/editor handling and read-worker lifecycle across page closure.
- Extend ordinary/comparison diff measurements to the medium/stress fixtures,
  output limits, very long lines, unusual paths, cache memory plateaus, and
  page/request lifecycle cycles. Current warm-frame coverage is the 400-line
  Rust fixture and two terminal sizes, not the whole matrix.
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
