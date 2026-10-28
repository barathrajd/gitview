use crossterm::event::KeyCode;
use gitview::{
    comparison_page::ComparisonPage,
    ui::{Screen, render},
};
use ratatui::{Terminal, backend::TestBackend};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            LIVE.fetch_add(layout.size(), Ordering::Relaxed);
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
            LIVE.fetch_add(new_size, Ordering::Relaxed);
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn draw(page: &ComparisonPage, width: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, 45)).unwrap();
    terminal
        .draw(|frame| render(frame, Screen::Comparison(page), gitview::ui::ASCII_BORDER))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
#[ignore = "live-allocation diagnostic; run explicitly with --ignored --nocapture"]
fn measures_comparison_render_cache_bytes() {
    let root = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(root.path())
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "commit.gpgsign", "false"]);
    std::fs::write(root.path().join("big.txt"), "base\n").unwrap();
    git(&["add", "--all"]);
    git(&["commit", "-qm", "base"]);
    // About 940 KB of added text, just under the 1 MB comparison output limit.
    let line = "x".repeat(190);
    std::fs::write(
        root.path().join("big.txt"),
        format!("{line}\n").repeat(4900),
    )
    .unwrap();
    git(&["add", "--all"]);
    git(&["commit", "-qm", "target"]);

    let mut page = ComparisonPage::open("HEAD", None);
    page.key(root.path(), KeyCode::Enter);
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        page.tick();
        if !draw(&page, 140).contains("Loading comparison...") {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    let loaded = LIVE.load(Ordering::Relaxed);
    println!("loaded live_kib={}", loaded / 1024);
    for (name, keys) in [
        ("ide-split", vec![]),
        ("raw-split", vec![KeyCode::Char('v')]),
        ("raw-unified", vec![KeyCode::Tab]),
        ("ide-unified", vec![KeyCode::Char('v')]),
    ] {
        for key in keys {
            page.key(root.path(), key);
        }
        draw(&page, 140);
        println!(
            "after {name}: live_delta_kib={}",
            (LIVE.load(Ordering::Relaxed) as isize - loaded as isize) / 1024
        );
    }
}

fn rss_kib() -> u64 {
    let result = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    String::from_utf8(result.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

#[test]
#[ignore = "live-allocation diagnostic; run explicitly with --ignored --nocapture"]
fn live_heap_returns_to_baseline_after_closing_a_comparison_page() {
    let root = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(root.path())
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "commit.gpgsign", "false"]);
    std::fs::write(root.path().join("big.txt"), "base\n").unwrap();
    git(&["add", "--all"]);
    git(&["commit", "-qm", "base"]);
    let line = "x".repeat(190);
    std::fs::write(
        root.path().join("big.txt"),
        format!("{line}\n").repeat(4900),
    )
    .unwrap();
    git(&["add", "--all"]);
    git(&["commit", "-qm", "target"]);

    let mut afters = Vec::new();
    for index in 0..150 {
        let mut page = ComparisonPage::open("HEAD", None);
        page.key(root.path(), KeyCode::Enter);
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            page.tick();
            if !draw(&page, 140).contains("Loading comparison...") {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        for key in [
            KeyCode::Char('v'),
            KeyCode::Tab,
            KeyCode::Char('v'),
            KeyCode::Tab,
        ] {
            page.key(root.path(), key);
            draw(&page, 140);
        }
        drop(page);
        afters.push(LIVE.load(Ordering::Relaxed));
        if index % 10 == 9 || index < 2 {
            println!(
                "cycle={} live_after_kib={} rss_kib={}",
                index + 1,
                afters[index] / 1024,
                rss_kib()
            );
        }
    }
    let first = afters[9];
    let last = *afters.last().unwrap();
    assert!(
        last <= first + 1024 * 1024,
        "live heap grew from {first} to {last} bytes across closed comparison pages"
    );
}
