#!/bin/sh
# Run every release gate and performance diagnostic once and summarize the results.
#
# Usage: scripts/run-release-pass.sh [--allow-dirty] [--skip-slow] [--output DIR]
#
# The pass refuses to run on a dirty worktree so the evidence names a clean
# revision. Keep the machine otherwise idle while it runs (about 15 minutes).
# --skip-slow omits the three-minute native memory scripts and the 30 second idle
# profile, for a quick check that isn't release evidence. Results go to
# target/release-pass/<revision>-<time> unless --output is given.
set -u
cd "$(dirname "$0")/.." || exit 2
allow_dirty=0; skip_slow=0; output=""
while [ $# -gt 0 ]; do
  case "$1" in
    --allow-dirty) allow_dirty=1 ;;
    --skip-slow) skip_slow=1 ;;
    --output) shift; output="${1:?--output needs a directory}" ;;
    -h|--help) sed -n '2,10p' "$0"; exit 0 ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
  esac
  shift
done
revision=$(git rev-parse HEAD) || exit 2
dirty=$(git status --porcelain)
if [ -n "$dirty" ] && [ "$allow_dirty" -eq 0 ]; then
  echo "Worktree is dirty; commit or stash first, or pass --allow-dirty (not release evidence):" >&2
  echo "$dirty" >&2
  exit 2
fi
[ -n "$output" ] || output="target/release-pass/$(git rev-parse --short HEAD)-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$output" || exit 2
{
  echo "revision: $revision"
  if [ -n "$dirty" ]; then echo "worktree: DIRTY (not clean-revision evidence)"; echo "$dirty"; else echo "worktree: clean"; fi
  echo "slow checks: $([ "$skip_slow" -eq 1 ] && echo skipped || echo included)"
  echo "started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "host: $(uname -srm)"
  rustc --version; cargo --version; git --version; python3 --version
  echo "load average: $(uptime | sed 's/.*load averages*: //')"
} | tee "$output/environment.txt"
load=$(uptime | sed 's/.*load averages*: //' | awk '{gsub(",",""); print $1}')
if [ "$(echo "$load" | awk '{print ($1 > 2)}')" = "1" ]; then
  echo "WARNING: load average $load is high; timings may be perturbed." >&2
fi
failed=""
run() {
  name=$1; shift
  echo "=== $name"
  "$@" > "$output/$name.out" 2>&1
  code=$?
  echo "$name exit=$code" >> "$output/exit-codes.txt"
  if [ "$code" -ne 0 ]; then failed="$failed $name"; echo "    FAILED (exit $code)"; fi
}
: > "$output/exit-codes.txt"
T="cargo test --release --locked --features test-support"
run fmt cargo fmt --check
run diff_check git diff --check
run clippy cargo clippy --locked --all-targets --features test-support -- -D warnings -A clippy::too_many_arguments
run test_suite cargo test --locked --features test-support
run release_build cargo build --release --locked
run cli_help ./target/release/gitview --help
run frames_limits $T --test ordinary_diff_performance measures_supported_limit_layouts -- --ignored --nocapture --test-threads=1
# Known failing: renderer-only RSS growth exceeds the budget. Kept so it stays visible.
run renderer_rss $T --test ordinary_diff_performance measures_diff_cache_lifecycle -- --ignored --nocapture --test-threads=1
run live_heap_diff $T --test diff_cache_live_allocations -- --ignored --nocapture --test-threads=1
run live_heap_comparison $T --test comparison_cache_live_allocations -- --ignored --nocapture --test-threads=1
run large_lists $T --test large_list_performance -- --ignored --nocapture --test-threads=1
run object_tags $T --test object_tag_performance -- --ignored --nocapture --test-threads=1
run languages $T --test language_diff_performance -- --ignored --nocapture --test-threads=1
run ordinary_diff python3 scripts/check-ordinary-diff.py
run preview_retry python3 scripts/check-preview-retry.py
run commit_editor python3 scripts/check-commit-editor.py
run load_cancel python3 scripts/check-load-cancel.py
run navigation python3 scripts/profile-navigation.py --fixture stress --check
run profile_previews python3 scripts/profile-previews.py
if [ "$skip_slow" -eq 0 ]; then
  run diff_memory python3 scripts/check-diff-cache-memory.py
  run page_memory python3 scripts/check-page-memory.py
  run profile_small python3 scripts/profile-tui.py --fixture small --samples 20 --idle-seconds 30 --check-idle --check-preview
fi
echo "finished: $(date -u +%Y-%m-%dT%H:%M:%SZ)" | tee -a "$output/environment.txt"
echo
echo "Results in $output"
if [ -n "$failed" ]; then
  echo "FAILED:$failed"
  echo "renderer_rss is a known failing diagnostic; any other failure is new."
  exit 1
fi
echo "All checks passed."
