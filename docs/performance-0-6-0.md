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
The comparison page's render cache now uses the same 8 MiB owned-allocation
estimate and keeps one active layout even if it alone exceeds the budget. Before
the change, a 940 KB comparison held about 28 MiB of live heap after all four
variants rendered (13,484 KiB loaded with one variant, plus 15,232 KiB). After
it, the same sequence ends 772 KiB below the loaded value, so about 12.7 MiB.
The active layout alone exceeds 8 MiB there, which is why the total stays above
the budget. Run `comparison_cache_live_allocations` with `--ignored --nocapture`
to reproduce. This is a single local measurement, not p95 or cross-platform proof.
Comparison loading spawns one thread per Enter. Closing the page now sets a
cancellation flag that stops the remaining per-file syntax preparation and skips
the result, and a load that hasn't started returns immediately. Git commands
already running aren't killed; they finish within the 1 MB / 5,000 line output
limit. The object page keeps its simple load thread, because it only runs those
bounded Git reads and does no preparation. The cancellation rule is covered by a
unit test on the preparation loop. The native behavior is checked separately
under "Escape during slow loads".

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

## Large cockpit lists

The cockpit renders its history, worktree, and branch lists from in-memory data.
This diagnostic measures only that rendering. It excludes Git, terminal I/O,
input handling, and search. Run it explicitly in a release build:

```sh
cargo test --release --locked --features test-support \
  --test large_list_performance -- --ignored --nocapture --test-threads=1
```

Each value is the p95 of 100 frames while the selection jumps through both
lists, after one warm-up frame. Lengths are history and worktree entries, with
a tenth as many branches. One run on macOS ARM64 gave:

| List length | p95 at 140x35 | p95 at 80x24 |
| --- | --- | --- |
| 100 | 0.22 to 0.29 ms | 0.07 ms |
| 1,000 | 0.26 to 0.28 ms | 0.10 to 0.14 ms |
| 10,000 | 1.35 to 1.38 ms | 0.47 to 0.83 ms |

Frame time grows with list length but stays far below the 50 ms budget, so no
renderer change was made. The native stress fixture (10,000 staged paths, 1,000
extra refs) also settled at 0.5% of one core over 10 idle seconds, with 25
terminal output bytes, and slow-preview feedback and cancel took about 12 ms
(five samples). Its first startup sample took 689 ms and the other four took
28 to 42 ms; the first run is cold, and the startup budget applies to the small
fixture. These are single local runs, not p95 proof across 20 starts.

History search runs on the UI thread after a 180 ms debounce, over the loaded
history rows. History loads at most 200 rows (`git log -n 200`). A second
diagnostic, `measures_history_search_on_the_ui_thread` in the same test file,
times `search_matches` over 100 mixed queries. One run gave p95 of 0.023 ms at
200 rows, 0.79 ms at 10,000 rows, and 3.8 ms at 100,000 rows. That is well within
the 100 ms UI-thread budget, so search needs no worker. The numbers cover
matching only, not the debounce timer, rendering, or terminal I/O.

## Tag and object pages

Path history uses the same 200-row history loader and cockpit list as the
history view, so the large-list and search results above cover its rendering.
Two further diagnostics in `tests/object_tag_performance.rs` cover the tag and
object pages. Run them explicitly in a release build:

```sh
cargo test --release --locked --features test-support \
  --test object_tag_performance -- --ignored --nocapture --test-threads=1
```

One run on macOS ARM64 gave:

| Fixture | Measurement | 140x35 | 80x24 |
| --- | --- | --- | --- |
| Blob of 4,900 lines, about 940 KB | first frame | 0.86 ms | 0.43 ms |
| Same blob | scroll p95 over 100 actions | 0.58 ms | 0.47 ms |
| 10,000 local tags | first frame | 0.22 ms | 0.09 ms |
| Same tags | selection p95 over 100 jumps of 97 | 0.12 ms | 0.06 ms |

Loading the blob took 20 ms and listing 10,000 tags took 303 ms. Both happen
off the UI thread behind a loading state, and these are Git and thread times, not
promises. All frame times are far below the 50 ms budget, so neither page was
changed. The numbers exclude terminal I/O, input handling, and syntax work, and
the object page's load thread still finishes its bounded Git reads after close.
Linux remains unmeasured.

## Native navigation latency

`scripts/profile-navigation.py` starts the release binary in a pseudo-terminal
on a temporary fixture, sends one `j` at a time, and records the time until the
reconstructed screen changes. The time includes the pseudo-terminal and screen
reconstruction, so it is an upper bound for Gitview's own response. Run it with:

```sh
cargo build --release --locked
python3 scripts/profile-navigation.py --fixture stress --check
```

One run on macOS ARM64 with the stress fixture (10,000 staged paths, 200 commits,
1,000 extra refs) sent 100 keys down the worktree list and 100 down the history
list at each size. The p95 values were:

| Series | 140x35 | 80x24 |
| --- | --- | --- |
| Worktree list | 4.71 ms | 2.47 ms |
| History list | 2.34 ms | 2.51 ms |

These are below the 100 ms UI-thread budget. The script counts the first screen
change, which is the moving selection marker; later inspector updates arrive
asynchronously and aren't timed here. This is one local run, not Linux or
clean-revision evidence.

## Page open and close memory

`scripts/check-page-memory.py` drives the release binary in a pseudo-terminal.
It builds a temporary repository with 2,000 local tags and a 2,490-line
replaced file in the last commit. For each page it warms ten open and close
cycles, runs 100 measured cycles, and compares settled RSS with the larger of
10 MiB or 10% of warm RSS. Each cycle does the following:

- **Tags:** open the branches view, open the tag list, move down 20 rows, and
  close both.
- **Object:** inspect the selected commit, scroll 20 lines, and close it.
- **Comparison:** compare the selected commit with its parent, toggle IDE/raw
  and split/unified twice each, and close it.

```sh
cargo build --release --locked
python3 scripts/check-page-memory.py
```

One run on macOS ARM64 gave:

| Page | Warm RSS | Final RSS | Growth | Budget |
| --- | --- | --- | --- | --- |
| Tags | 8,048 KiB | 8,256 KiB | 208 KiB | 10,240 KiB |
| Object | 8,432 KiB | 8,464 KiB | 32 KiB | 10,240 KiB |
| Comparison | 40,608 KiB | 44,864 KiB | 4,256 KiB | 10,240 KiB |

All three pass. The pages run in order in one process, so each warm value
includes memory left by the earlier pages. Comparison grew the most, using about
42% of its budget without flattening in this run, so a longer run is needed
before calling it a plateau. This is one local run on one small fixture. It doesn't
cover Linux or a clean revision.

### Large comparison fixture

The same script accepts `--pages`, `--lines`, and `--cycles`. This command uses
4,900 replaced lines of about 190 characters, a roughly 940 KB comparison
patch, for 300 measured cycles:

```sh
python3 scripts/check-page-memory.py --pages comparison --lines 4900 --cycles 300
```

The script judges the plan's 100-cycle budget and reports longer-run growth
separately. Two runs on macOS ARM64 disagreed:

| Run | Warm RSS | Growth at 100 cycles | Growth at 300 cycles | Budget |
| --- | --- | --- | --- | --- |
| 1 | 44,528 KiB | 3,696 KiB | 11,360 KiB | 10,240 KiB |
| 2 | 43,536 KiB | 13,552 KiB | 16,688 KiB | 10,240 KiB |

Run 1 passed at 100 cycles and exceeded the budget at 300. Run 2 **failed the
100-cycle budget**. In both, RSS rose in steps and stayed flat for long stretches
(run 2 held 57,696 KiB for about 100 cycles), but I haven't shown that it stops
rising. This is an open finding on the large fixture, not a passing check. After the
load-thread cancellation change, which doesn't affect cycles whose loads finish,
two more 100-cycle runs grew 1,248 KiB and 9,744 KiB, both within the budget but
the second close to it. Four runs on the same fixture ranged from 1,248 to
13,552 KiB, so the variance itself is the finding.

I tested one explanation: that spawning a thread per Enter scatters allocations
across allocator arenas. An experiment replaced it with one reused worker thread.
Growth at 100 cycles, in KiB, over six runs each:

| Variant | Runs | Mean |
| --- | --- | --- |
| Thread per Enter | 1,248; 9,744; 4,336; 256; 6,128; 2,240 | 4,000 |
| Single reused worker | 7,520; 7,744; 10,736; 5,344; 2,752; 2,368 | 6,077 |

The worker was not better, and one of its runs exceeded the 10,240 KiB budget.
That doesn't support the per-thread explanation, so I reverted the experiment
and kept the thread per Enter. Six runs per variant on a noisy measurement can't
rank them reliably; they only show that the change doesn't remove the variance.
The cause remains unknown.

To separate retained data from allocator behavior, the `live_heap_returns_to_baseline_after_closing_a_comparison_page`
diagnostic in `tests/comparison_cache_live_allocations.rs` opens, loads, renders
four variants of, and drops the same 940 KB comparison 150 times in one
process with a counting allocator:

```sh
cargo test --release --locked --features test-support \
  --test comparison_cache_live_allocations live_heap -- --ignored --nocapture --test-threads=1
```

Live heap after each close stayed between 565 and 588 KiB, while RSS stepped from
34,208 to 53,408 KiB by cycle 100 and 56,576 KiB at cycle 150. This shows a
closed comparison page releases its data and the RSS steps coincide with the
allocator holding freed memory. It doesn't prove the native process behaves the
same way, and the budget stays at 10,240 KiB. The test excludes Git, the load
threads' real timing, and terminal I/O.

## Diffs in other languages

`tests/language_diff_performance.rs` times ordinary diff preparation and warm
frames for 20 file types. Each fixture replaces 400 lines of a short, typical
statement in that language, so the patch has 800 rows. It runs after one
unrelated warm-up so shared syntax setup isn't charged to the first language.
Run both diagnostics explicitly in a release build:

```sh
cargo test --release --locked --features test-support \
  --test language_diff_performance -- --ignored --nocapture --test-threads=1
```

One run on macOS ARM64 gave these results. Warm p95 covers 100 frames at each of
140x35 and 80x24 across all four layouts, with the first four of each excluded.

- **Preparation** takes 0.3 to 330 ms and runs off the UI thread behind the
  loading state. The slowest were C++ (330 ms), C (185 ms), CSS (168 ms),
  JavaScript (155 ms), and Python (134 ms). The first four layout draws took at
  most 2.5 ms.
- **Warm frames** have p95 between 0.15 and 0.19 ms for every language, far
  below the 50 ms budget.
- **TypeScript, TSX, and TOML** prepare in about 0.3 ms because Gitview's bundled
  syntax set has no definition for them, so they show without highlighting. That
  is a coverage gap, not a performance result, and I didn't change it.

A second diagnostic uses one very long line (10,000, 100,000, and 450,000
characters, replaced once) in JavaScript, JSON, HTML, and Python. At 450,000
characters, preparation took 748 ms (JavaScript), 668 ms (Python), 310 ms
(JSON), and 120 ms (HTML). Warm p95 was 8.1 ms (JavaScript), 8.2 ms (Python),
8.4 ms (HTML), and **29.6 ms for JSON**, whose first layout draw took 37 ms. The
JSON line has about 225,000 tokens, so it stays under 50 ms with the least
margin. Shorter lines were faster. These cover diffs rendered by the ordinary
diff view, not the comparison page, and the numbers exclude Git and terminal I/O.

## Escape during slow loads

`scripts/check-load-cancel.py` injects a 250 ms delay into every Git command,
opens the comparison or object page from the history pane, presses Enter, and
presses Escape while the load is pending. It repeats this five times per page,
then waits 2.5 seconds to confirm the abandoned load never reopens a result
screen. It then removes the delay and checks that the next load succeeds.

```sh
cargo build --release --locked
python3 scripts/check-load-cancel.py
```

One run gave Escape-to-cockpit times of 2.3 to 2.8 ms for the comparison page and
2.4 to 2.5 ms for the object page, against the 100 ms budget. No stale result
appeared and the next load succeeded. The times include the pseudo-terminal. The
script checks visible behavior only; it doesn't observe whether syntax
preparation actually stopped, which the unit test covers.

## Pre-release pass on macOS

This pass ran every gate and diagnostic from this report once, in release
builds, on macOS ARM64 (Rust 1.99.1, Git 2.56.0). The source matched revision
`3e68f3f`. The worktree also held uncommitted additions to the documentation,
the plan, `scripts/check-load-cancel.py`, and `tests/language_diff_performance.rs`
(no `src/` changes), so this is not clean-revision evidence. A website sub-agent
ran headless Chrome and a site build during the first part of the pass, so
timings and RSS may be perturbed. Rerun the pass on a clean commit with the
machine otherwise idle before treating any value as release evidence.

`scripts/run-release-pass.sh` runs the whole pass in one command. It refuses a
dirty worktree unless you pass `--allow-dirty`, records the revision and tool
versions, warns if the load average is high, and writes each check's output and
exit code to `target/release-pass/<revision>-<time>`. The full pass takes about
15 minutes. `--skip-slow` omits the native memory scripts and the 30 second idle
profile for a quick check that isn't release evidence. The command exits with
status 1 if any check fails. The known-failing `renderer_rss` diagnostic always
fails today, so expect that status until it is resolved.

```sh
scripts/run-release-pass.sh
```

| Check | Result |
| --- | --- |
| `cargo fmt --check`, `git diff --check` | Pass |
| Clippy (CI command), full suite, release build | Pass |
| Warm ordinary-diff frames, three limit fixtures, both sizes | p95 0.09 to 10.1 ms; pass |
| Cockpit frames, 10,000 entries | p95 at most 1.36 ms; pass |
| History search, 10,000 rows | p95 0.37 ms; pass |
| Tag page (10,000 tags) and object page frames | p95 at most 0.59 ms; pass |
| 20 languages, warm frames | p95 at most 0.19 ms; pass |
| Native list navigation, stress fixture | p95 at most 5.25 ms; pass |
| Startup first frame, small fixture | p95 45.3 ms; pass |
| Idle CPU, small fixture | 0.2% of one core, 25 output bytes; pass |
| Slow-preview feedback and cancel | p95 about 12.3 ms; pass |
| Escape during slow comparison and object loads | at most 3 ms; pass |
| Commit editor save, retry, ordinary diff, preview scripts | Pass |
| Native ordinary-diff memory, 100 cycles | 7,184 KiB growth; pass |
| Native tag and object page memory | 592 and 16 KiB growth; pass |
| Native comparison page memory, small fixture | 9,664 KiB growth; pass, 94% of budget |
| Renderer-only diff RSS diagnostic | 17,296 KiB growth; **fails** |
| Large-comparison native RSS | Variable; one earlier run failed; deferred |

Two values moved the wrong way against earlier single runs: native diff memory
grew 7,184 KiB (earlier 4,848 KiB) and the small comparison page grew
9,664 KiB (earlier 4,256 KiB). Both stay inside the 10,240 KiB budget, but the
comparison value is close and agrees with the run-to-run variance already
recorded for that page.

Release 0.6.0 still isn't ready. Open items: the failing renderer-only RSS
diagnostic, the deferred comparison memory variance, native Linux x86_64
evidence, a clean-revision rerun, and the Clippy rerun on the CI toolchain.

## Review status and remaining work

The following checks remain required before release:

- Complete the workflow matrix and larger-fixture sampling for the newly
  asynchronous path/ref/ignore/tag previews and failed-commit refreshes. Audit
  profile/config/editor handling and read-worker lifecycle across page closure.
- Extend ordinary/comparison diff measurements to the medium/stress fixtures,
  output limits, very long lines, unusual paths, cache memory plateaus, and
  page/request lifecycle cycles. Current warm-frame coverage is the 400-line
  Rust fixture and two terminal sizes, not the whole matrix.
- Cover navigation, debounced search, native large-list navigation, signed-tag/key
  verification behavior, object inspection, mutation progress, and external
  editor/hook/signing responsiveness.
- Resolve or explain the large-comparison native RSS variance, run medium/stress
  fixtures, and complete request lifecycle checks.
- Collect native Linux evidence and report Git-child CPU separately.
- Clear the release Clippy gate and complete the existing release gates.

Feature-enabled tests pass, including stale-state, terminal restoration, and
live external refresh coverage. The release CI Clippy command, `cargo clippy
--locked --all-targets --features test-support -- -D warnings -A
clippy::too_many_arguments`, now passes locally on Rust 1.99.1. The earlier
findings were fixed without behavior changes. Rerun it on the release CI
toolchain and a clean revision before treating the gate as closed.
