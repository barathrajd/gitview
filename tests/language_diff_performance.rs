use gitview::ui::{self, Screen};
use ratatui::{Terminal, backend::TestBackend};
use std::{path::Path, time::Instant};

const SNIPPETS: &[(&str, &str)] = &[
    (
        "rs",
        "pub fn value_{n}(input: &str) -> Option<usize> {{ input.parse::<usize>().ok().map(|v| v + {n}) }}",
    ),
    (
        "py",
        "def value_{n}(items): return [item * {n} for item in items if item % 2 == 0]  # comment",
    ),
    (
        "js",
        "export const value{n} = async (items) => items.filter((item) => item > {n}).map((item) => `${{item}}`);",
    ),
    (
        "ts",
        "export function value{n}<T extends { id: number }>(items: T[]): Map<number, T> { return new Map(items.map((i) => [i.id + {n}, i])); }",
    ),
    (
        "tsx",
        "export const Item{n} = ({{ label }}: {{ label: string }}) => <li className=\"item-{n}\">{{label}}</li>;",
    ),
    (
        "go",
        "func Value{n}(items []int) (total int) {{ for _, item := range items {{ total += item * {n} }}; return }}",
    ),
    (
        "java",
        "public static int value{n}(List<Integer> items) {{ return items.stream().mapToInt(i -> i * {n}).sum(); }}",
    ),
    (
        "c",
        "static int value_{n}(const int *items, size_t count) {{ int total = {n}; for (size_t i = 0; i < count; i++) total += items[i]; return total; }}",
    ),
    (
        "cpp",
        "template <typename T> auto value_{n}(const std::vector<T>& items) {{ return std::accumulate(items.begin(), items.end(), T{{{n}}}); }}",
    ),
    (
        "html",
        "<div class=\"item-{n}\" data-id=\"{n}\"><a href=\"/items/{n}\">Item {n}</a><span>text</span></div>",
    ),
    (
        "css",
        ".item-{n} > a:hover {{ color: #{n:06x}; margin: 0 auto; background: url(\"img-{n}.png\"); }}",
    ),
    (
        "json",
        "{{\"id\": {n}, \"name\": \"item-{n}\", \"tags\": [\"a\", \"b\"], \"nested\": {{\"ok\": true, \"count\": {n}}}}},",
    ),
    (
        "md",
        "- **Item {n}**: see [the guide](https://example.com/{n}) and `code_{n}()` for details.",
    ),
    (
        "yaml",
        "item_{n}: {{ name: \"item-{n}\", values: [1, 2, 3], enabled: true }}  # comment",
    ),
    (
        "toml",
        "item_{n} = {{ name = \"item-{n}\", values = [1, 2, 3], enabled = true }}",
    ),
    (
        "sh",
        "for file in ./dir-{n}/*.txt; do echo \"$file\" | grep -q '{n}' && cp \"$file\" /tmp/out-{n}; done",
    ),
    (
        "rb",
        "def value_{n}(items) = items.select {{ |item| item > {n} }}.map {{ |item| \"#{{item}}\" }}",
    ),
    (
        "php",
        "function value_{n}(array $items): array {{ return array_map(fn($i) => $i * {n}, $items); }}",
    ),
    (
        "sql",
        "SELECT id, name FROM items_{n} WHERE created_at > '2026-01-01' AND id IN (SELECT id FROM tags) ORDER BY id;",
    ),
    (
        "xml",
        "<item id=\"{n}\" kind=\"x\"><name>Item {n}</name><value>{n}</value></item>",
    ),
];

fn replaced_patch(extension: &str, template: &str, lines: usize) -> String {
    let name = format!("sample.{extension}");
    let mut text = format!(
        "diff --git a/{name} b/{name}\n--- a/{name}\n+++ b/{name}\n@@ -1,{lines} +1,{lines} @@\n"
    );
    for marker in ['-', '+'] {
        for n in 0..lines {
            // Fill the template by hand because snippets contain literal braces.
            let line = template
                .replace("{{", "\u{1}")
                .replace("}}", "\u{2}")
                .replace(
                    "{n:06x}",
                    &format!("{:06x}", n + usize::from(marker == '+')),
                )
                .replace("{n}", &(n + usize::from(marker == '+')).to_string())
                .replace('\u{1}', "{")
                .replace('\u{2}', "}");
            text.push(marker);
            text.push_str(&line);
            text.push('\n');
        }
    }
    text
}

fn p95(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    samples[(samples.len() * 95).div_ceil(100) - 1]
}

fn measure(extension: &str, patch: String) -> (f64, f64, f64) {
    let start = Instant::now();
    let prepared = ui::PreparedDiff::new(patch, Some(Path::new(&format!("sample.{extension}"))));
    let preparation = start.elapsed().as_secs_f64() * 1000.0;
    let diff = (1, Some(Ok(prepared.text().to_owned())));
    let mut worst_first = 0.0f64;
    let mut samples = Vec::new();
    for (width, height) in [(140, 35), (80, 24)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        for step in 0..100 {
            let (raw, split) =
                [(false, true), (true, true), (false, false), (true, false)][step % 4];
            let frame = Instant::now();
            terminal
                .draw(|frame| {
                    ui::render(
                        frame,
                        Screen::PreparedDiff {
                            diff: &diff,
                            prepared: &prepared,
                            scroll: (step % 40) as u16,
                            selected_hunk: 0,
                            raw,
                            side_by_side: split,
                            worktree_entry: None,
                        },
                        ui::ASCII_BORDER,
                    )
                })
                .unwrap();
            let elapsed = frame.elapsed().as_secs_f64() * 1000.0;
            if step < 4 {
                worst_first = worst_first.max(elapsed);
            } else {
                samples.push(elapsed);
            }
        }
    }
    (preparation, worst_first, p95(samples))
}

#[test]
#[ignore = "isolated release-build timing diagnostic; run with --ignored --nocapture --test-threads=1"]
fn measures_diffs_in_common_languages() {
    // Initialize shared syntax data first so each language reports its own cost.
    measure("rs", replaced_patch("rs", SNIPPETS[0].1, 50));
    let mut worst_prepare = 0.0f64;
    let mut worst_frame = 0.0f64;
    for (extension, template) in SNIPPETS {
        let (prepare, first, warm) = measure(extension, replaced_patch(extension, template, 400));
        println!(
            "language={extension} prepare_ms={prepare:.1} first_variants_max_ms={first:.2} warm_p95_ms={warm:.3}"
        );
        worst_prepare = worst_prepare.max(prepare);
        worst_frame = worst_frame.max(warm);
    }
    println!("worst prepare_ms={worst_prepare:.1} worst warm_p95_ms={worst_frame:.3}");
    assert!(worst_frame <= 50.0, "warm frame p95 exceeded 50 ms");
}

#[test]
#[ignore = "isolated release-build timing diagnostic; run with --ignored --nocapture --test-threads=1"]
fn measures_long_single_line_diffs_in_syntax_languages() {
    measure("rs", replaced_patch("rs", SNIPPETS[0].1, 50));
    for extension in ["js", "json", "html", "py"] {
        for length in [10_000usize, 100_000, 450_000] {
            let name = format!("sample.{extension}");
            let body = if extension == "json" {
                format!("[{}1]", "1,".repeat(length / 2))
            } else {
                format!("var a = \"{}\";", "x".repeat(length))
            };
            let patch = format!(
                "diff --git a/{name} b/{name}\n--- a/{name}\n+++ b/{name}\n@@ -1 +1 @@\n-{body}\n+{body}y\n"
            );
            let (prepare, first, warm) = measure(extension, patch);
            println!(
                "long_line language={extension} chars={length} prepare_ms={prepare:.1} first_variants_max_ms={first:.2} warm_p95_ms={warm:.3}"
            );
        }
    }
}
