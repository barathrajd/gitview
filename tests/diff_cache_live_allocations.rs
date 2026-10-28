use gitview::ui::{self, Screen};
use ratatui::{Terminal, backend::TestBackend};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
            let live = LIVE.fetch_add(new_size, Ordering::Relaxed) + new_size;
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn stress_patch(lines: usize, length: usize) -> String {
    let mut text = format!(
        "diff --git a/sample.txt b/sample.txt\n--- a/sample.txt\n+++ b/sample.txt\n@@ -0,0 +1,{lines} @@\n"
    );
    for _ in 0..lines {
        text.push('+');
        text.push_str(&"x".repeat(length));
        text.push('\n');
    }
    text
}

fn cycle(patch: &str) -> usize {
    let prepared =
        ui::PreparedDiff::new(patch.to_owned(), Some(std::path::Path::new("sample.txt")));
    let diff = (991, Some(Ok(prepared.text().to_owned())));
    for (width, height) in [(140, 35), (80, 24), (140, 35)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        for (raw, split) in [(false, true), (true, true), (false, false), (true, false)] {
            terminal
                .draw(|frame| {
                    ui::render(
                        frame,
                        Screen::PreparedDiff {
                            diff: &diff,
                            prepared: &prepared,
                            scroll: 10,
                            selected_hunk: 0,
                            raw,
                            side_by_side: split,
                            worktree_entry: None,
                        },
                        ui::ASCII_BORDER,
                    )
                })
                .unwrap();
        }
    }
    // Peak bytes allocated while the prepared view and its layouts were alive.
    PEAK.load(Ordering::Relaxed)
}

#[test]
#[ignore = "live-allocation diagnostic; run explicitly with --ignored --nocapture"]
fn live_heap_returns_to_baseline_after_closing_a_diff() {
    let patch = stress_patch(4990, 190);
    let baseline = LIVE.load(Ordering::Relaxed);
    let mut afters = Vec::new();
    for index in 0..110 {
        PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
        let peak = cycle(&patch);
        let after = LIVE.load(Ordering::Relaxed);
        afters.push(after);
        if index % 10 == 9 || index < 3 {
            println!(
                "cycle={} live_after_kib={} peak_kib={} baseline_kib={}",
                index + 1,
                after / 1024,
                peak / 1024,
                baseline / 1024
            );
        }
    }
    // Obsolete prepared data must be released: live heap after close stays near
    // the first cycle's value instead of growing with the cycle count.
    let first = afters[10];
    let last = *afters.last().unwrap();
    assert!(
        last <= first + 1024 * 1024,
        "live heap grew from {first} to {last} bytes across 100 closed diffs"
    );
}
