# Gitview 0.6.0 performance review

## Objective

Review and fix measured performance problems across the whole Gitview TUI before
0.6.0, for developers doing everyday local Git work. Cover startup, navigation,
previews, Git operations, diffs, and the newer pages while preserving keyboard
behavior, Git safety checks, and truthful release claims. The user confirmed this
scope on October 29, 2026. Implementation was approved on October 29, 2026.

## Success criteria

- Record reproducible release-build baselines and final results on macOS ARM64
  and Linux x86_64, including revision, Rust/Git versions, machine, fixture,
  terminal dimensions, sample count, and warm/cold state. Report loading time
  separately from input response and rendering.
- After warming each supported view/layout at 140x35 and 80x24, the p95 frame
  time for navigation, scrolling, folding, and style/layout switches is at most
  50 ms across 100 actions on each fixed fixture. Record first-use and resize
  costs separately, and fix any UI-thread task exceeding 100 ms.
- A slow Git preview displays loading feedback and processes Escape within
  100 ms p95 while its Git commands incur an injected 250 ms delay. Mutation
  screens acknowledge input within that budget; Git hooks, signing, editors,
  and actual Git completion time have separate measurements.
- The first usable loading frame appears within 500 ms p95 across 20 process
  starts on the small fixture. Actual repository-data completion is measured
  and reported without a universal Git completion-time promise.
- After settling, the Gitview process uses at most 2% of one CPU core averaged
  over 30 idle seconds on the small fixture. Report Git-child CPU separately.
  Background changes still refresh within two seconds on that fixture.
- After warming all variants, 100 repeated open/close, toggle, fold, and resize
  cycles increase settled RSS by no more than the larger of 10 MiB or 10% of
  warm RSS. Caches and pending read requests have explicit finite bounds.
- Safety and rendering regressions pass: stale previews, staging, discard,
  amend, ref operations, terminal/editor restoration, Unicode paths, narrow
  layouts, output limits, cancellation, and external status refresh.
- Every reviewed workflow is recorded as passing, fixed, or an explicit release
  blocker. Full feature-enabled tests, formatting, build, and release Clippy
  checks pass. The 0.6.0 release gate requires this review's completion.

## In scope / Out of scope

**In scope**

- Startup, idle redraw/polling, history/search/path history, and worktree lists.
- Worktree and committed-file diffs, comparison loading, first display,
  scrolling, hunk navigation, folding, IDE/raw, split/unified, and resizing.
- Commit/amend and path/ref previews, tag verification, object inspection,
  mutation progress, cancellation, and editor handoff responsiveness.
- Measured fixes, bounded preparation/caching, regression coverage, and a local
  benchmark/report command. Measurement fixtures use temporary repositories.
- Small fixture: 50 commits, 100 tracked files, and 10 changed paths. Medium:
  200 commits, 1,000 tracked files, 100 changes, and 1,000 local refs. Stress:
  10,000 status paths, supported diff limits (1 MB/5,000 lines), a very long
  line, multiple languages, binary/mode changes, and unusual filenames.

**Out of scope**

- Desktop apps, Windows support, new Git workflows, network synchronization,
  unbounded repository support, or changing the default UI/shortcuts.
- Faster results obtained by skipping validations, suppressing hooks/signing,
  silently clipping more data, or weakening Git safeguards.
- Marketing benchmarks, comparisons with other products, or release publication.

## Approach

**Decided:** use the current architecture and shared theme; optimize bottlenecks
only after recording evidence. Review all workflows, but change only measured
hotspots. Use fixed object IDs and request generations for derived data, retain
finite caches, and keep expensive Git work and syntax preparation away from the
input/render thread. Read-only work may be coalesced or superseded; mutations
must retain their existing checks and completion handling.

Initial evidence at `a320218`, Darwin ARM64, Rust 1.99.1, Git 2.56.0:

- The existing [switch diagnostic](../tests/diff_switching.rs) reports roughly
  2.2–4.3 ms warm frame renders for its 400-line Rust case in a debug build.
  This is one local sample, not an optimized-build or cross-platform result.
- A disposable pseudo-terminal fixture with one staged file injected 250 ms
  into each status/staged-raw-diff call. Opening the commit form took about
  586 ms. The fixture, Git shim, and spike were removed after measurement.
- A two-second debug idle sample showed about 0.13 CPU-seconds and 6.6 MB RSS.
  The sample is too short to establish a release-build idle or memory baseline.
- [Main input handling](../src/main.rs) calls commit/amend, branch, and path
  preview functions synchronously; it also draws before each 50 ms receive
  timeout. These are candidate bottlenecks, not proof that every call is slow.
- [Ordinary diff rendering](../src/ui.rs) caches highlighted rows but prepares
  them on a render cache miss and repeats wrapping/alignment. The comparison
  page already prepares syntax in a worker and retains four layout/style
  variants at one width. Apply measured improvements consistently.

**Unknown:** optimized cold-start/first-diff costs, idle cost by repository size,
maximum cache retention, and responsiveness with slow Git on both platforms.
Milestone 1 measures these using a temporary harness and repeatable fixtures;
any remaining uncertainty stays a named release risk. No dependency or framework
change is selected, and no unverified library-performance claim is assumed.

## Milestones

1. **Reproducible audit and baseline (in progress):** initial native small-fixture
   startup, idle, and delayed commit/amend measurements are recorded in the
   [performance report](../docs/performance-0-6-0.md); the rest of the matrix
   remains pending. Continue to add fixture generation and an explicit
   benchmark/report command; measure every workflow above, startup, idle, CPU,
   RSS, first-use and warm actions, and delayed Git. Rank findings by observed
   input delay, record passing paths, and retain failed cases as regressions.
2. **Responsive previews and operations (in progress):** commit/amend preview
   preparation now uses a bounded, replaceable read worker and cancellable
   loading state. Path/ref and ignore previews now use the same bounded read
   pattern, and tag creation/verification have cancellable read preparation.
   Failed commits refresh their preview asynchronously without losing the draft.
   Native small-fixture checks cover these flows; the full matrix, page lifecycle,
   and external editor/hook/signing measurements remain pending.
   Preserve current confirmation/commit behavior, ignore stale read results,
   and verify Escape responsiveness with delayed Git and changed repository
   state. Do not cancel an already-started mutation as if it never happened.
3. **Consistent diff responsiveness (in progress):** ordinary worktree/commit
   syntax preparation now runs in a bounded worker. Four layouts at one width
   retain wrapped rows and hunk positions; only visible rows are drawn. Native
   400-line diagnostics, both terminal sizes, and four-variant hunk staging pass.
   Unified IDE replacement rows now include additions as well as removals.
   Large plain-text and long-line diagnostics now exercise the supported diff
   sizes and both terminal dimensions. Ordinary layouts evict older variants
   above an 8 MiB owned-allocation estimate, retaining one active layout if it
   alone exceeds the budget. Blank gutters and alignment rows avoid redundant
   allocations. Renderer-only RSS lifecycle checks still exceed the budget;
   native lifecycle evidence is recorded separately in the performance report.
   Comparison cache bounds, other languages, the complete stress matrix, and
   memory investigation remain pending before this milestone can close.
4. **Idle and large-list fixes (in progress):** unchanged main-loop frames are
   skipped; page completions and status updates request a redraw. Native
   small-fixture idle measurements pass; large lists and memory cycles remain
   pending. Continue to use measured evidence to avoid unchanged
   redraw/preparation work and excessive request/clone volume. Preserve timers,
   status-watch freshness, live startup data, debounced search, and navigation.
   Recheck new pages, many refs, and large worktrees with CPU/RSS measurements.
5. **Release evidence and handoff:** rerun the complete workflow matrix and
   budgets in release builds on both target platforms. Record before/after
   results and limitations, run feature-enabled safety/UI tests and Clippy,
   clear blockers, and update the 0.6.0 gate. Packaging and user approval of the
   final release candidate remain required by the existing release plan.

## Risks & open questions

- **Measurement variance — implementation owner:** use controlled native runs,
  p50/p95 and sample counts; keep timing gates separate from ordinary unit
  tests. If a runner is noisy, repeat controlled measurements and disclose it.
- **Budgets not yet proven feasible — implementation owner:** the targets above
  are proposed release acceptance targets, not shipped performance claims. If
  measured work cannot meet one, report the blocker and obtain a revised target
  explicitly; never silently relax it or mark the review complete.
- **Linux measurement access — release owner:** native Linux evidence is still
  required. If unavailable locally, use a dedicated CI/manual Linux run and
  keep this gate pending until results exist.
- **Cache growth/stale results — implementation owner:** use stable content and
  request identities, bound both entries and retained data, and test reload,
  cancellation, mutation, resize, and repeated-view cycles. Fall back to
  eviction/recomputation in a worker rather than unsafe cached state.
- **Polling changes hide updates — implementation owner:** preserve external
  refresh and timer tests; keep current polling cadence until measurements and
  runtime checks justify a change.
- **Slow hooks/signing/editors — implementation owner:** measure UI response
  separately from external completion. Preserve their behavior and expose
  progress/errors rather than claiming that external work is fast.

## Boundaries

Git remains the source of truth. Keep current staged/HEAD/ref/content checks,
confirmation rules, single-Enter commit flow, terminal restoration, output
limits, default IDE/split presentation, and shared colors intact. Use argument
arrays and literal paths. Preserve the release plan's network boundaries and
never add implicit fetching. The user commits through Gitview; provide
Conventional Commit messages per completed slice. Do not tag, publish, or call
0.6.0 ready while performance or existing release gates remain incomplete.
