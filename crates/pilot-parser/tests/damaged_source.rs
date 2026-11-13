//! The parser never crashes on damaged source: it reports errors instead.
//!
//! Every `.pilot` file under `tests/conformance`, `stdlib` and `selfhost` is
//! damaged at evenly spaced points (truncated, truncated and followed by
//! stray closers, or with one of `,` `{` `(` `"` `[` `f"{` inserted) and both
//! `parse_header` and `parse` must return. A crash is reported with its
//! panic site and the smallest input that hit it.
//!
//! `PILOT_FUZZ_POINTS` sets the points per file (default 16) and
//! `PILOT_FUZZ_SEED` shifts them, so a run with another seed damages other
//! places. AGENTS.md section 11 asks for this test after any parser change.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static PANIC_SITE: Mutex<String> = Mutex::new(String::new());

const INSERTS: [&str; 6] = [",", "{", "(", "\"", "[", "f\"{"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("workspace root")
}

fn pilot_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            pilot_files(&path, out);
        } else if path.extension().is_some_and(|x| x == "pilot") {
            out.push(path);
        }
    }
}

fn env_number(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// The damaged texts of `src` at byte `at` (a character boundary).
fn damaged(src: &str, at: usize) -> Vec<String> {
    let mut out = vec![src[..at].to_string(), format!("{} , ) ] }} ; :", &src[..at])];
    for ins in INSERTS {
        let mut text = src.to_string();
        text.insert_str(at, ins);
        out.push(text);
    }
    out
}

#[test]
#[ignore = "slow: parses every .pilot file damaged at many points; `cargo test-full` runs it"]
fn the_parser_reports_damaged_source_instead_of_crashing() {
    let root = root();
    let mut files = Vec::new();
    for dir in ["tests/conformance", "stdlib", "selfhost"] {
        pilot_files(&root.join(dir), &mut files);
    }
    assert!(files.len() > 100, "found only {} .pilot files under {}", files.len(), root.display());
    let points = env_number("PILOT_FUZZ_POINTS", 16).max(1);
    let seed = env_number("PILOT_FUZZ_SEED", 0);

    std::panic::set_hook(Box::new(|info| {
        let site = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        *PANIC_SITE.lock().unwrap_or_else(|e| e.into_inner()) = site;
    }));
    // Panic site -> (count, smallest damaged input that hit it).
    let mut crashes: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut runs = 0usize;
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else { continue };
        let name = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        for k in 0..points {
            let mut at = (src.len() * k / points + seed * 7919) % (src.len() + 1);
            while !src.is_char_boundary(at) {
                at -= 1;
            }
            for text in damaged(&src, at) {
                runs += 1;
                let result = std::panic::catch_unwind(|| {
                    let _ = pilot_parser::parse_header(&text);
                    let _ = pilot_parser::parse(&text);
                });
                if let Err(payload) = result {
                    let message = payload
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_default();
                    let site = PANIC_SITE.lock().unwrap_or_else(|e| e.into_inner()).clone();
                    let key = format!("{site}: {}", message.chars().take(80).collect::<String>());
                    let lo = at.saturating_sub(60);
                    let lo = (0..=lo).rev().find(|&i| text.is_char_boundary(i)).unwrap_or(0);
                    let hi = (at + 40).min(text.len());
                    let hi = (hi..=text.len()).find(|&i| text.is_char_boundary(i)).unwrap_or(text.len());
                    let example = format!("{name} at byte {at}: {:?}", &text[lo..hi]);
                    let entry = crashes.entry(key).or_insert((0, example.clone()));
                    entry.0 += 1;
                    if example.len() < entry.1.len() {
                        entry.1 = example;
                    }
                }
            }
        }
    }
    let _ = std::panic::take_hook();
    let report: Vec<String> = crashes.iter().map(|(site, (n, ex))| format!("{n} x {site}\n    {ex}")).collect();
    assert!(crashes.is_empty(), "the parser crashed in {} of {runs} damaged inputs:\n{}", crashes.values().map(|c| c.0).sum::<usize>(), report.join("\n"));
}
