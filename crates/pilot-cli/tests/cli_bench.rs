//! Milestone test timing: the recorder the milestone tests use
//! (crates/pilot-analysis/tests/common/test_metrics.rs) and the table
//! `pilot bench` prints from what it records (selfhost/cli/bench_cmd.pilot).
//! Every expected number below is worked out by hand from the times written.

// `metrics_dir` is the milestone tests' own; these tests use their own directories.
#[allow(dead_code)]
#[path = "../../pilot-analysis/tests/common/test_metrics.rs"]
mod test_metrics;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// A fresh, empty directory for one test.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli_bench").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create test directory");
    dir
}

fn bench(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pilot"))
        .arg("bench")
        .args(args)
        .current_dir(root())
        .output()
        .expect("spawn pilot binary")
}

fn write_history(dir: &Path, milestone: &str, test: &str, lines: &str) {
    let m = dir.join(milestone);
    fs::create_dir_all(&m).expect("create milestone directory");
    fs::write(m.join(format!("{test}.tsv")), lines).expect("write history");
}

#[test]
fn passing_runs_are_recorded_and_a_failing_run_erases_the_history() {
    let dir = fresh_dir("recorder");
    let file = test_metrics::history_file(&dir, 7, "test_m7_1_sample");

    let first = test_metrics::TestTimer::start(&dir, 7, "test_m7_1_sample");
    assert_eq!(first.record(Duration::from_micros(1_500)), vec![1_500]);
    let second = test_metrics::TestTimer::start(&dir, 7, "test_m7_1_sample");
    assert_eq!(second.record(Duration::from_micros(2_250)), vec![1_500, 2_250]);
    assert_eq!(test_metrics::read_history(&file), vec![1_500, 2_250]);
    let text = fs::read_to_string(&file).expect("history file");
    assert!(text.lines().all(|l| l.split('\t').count() == 2), "{text}");

    // A run that never records (its test panicked) deletes the history.
    let broken = test_metrics::TestTimer::start(&dir, 7, "test_m7_1_sample");
    drop(broken);
    assert!(!file.exists(), "a failing test must leave no history");

    // The next passing run starts a new history.
    let again = test_metrics::TestTimer::start(&dir, 7, "test_m7_1_sample");
    assert_eq!(again.record(Duration::from_micros(900)), vec![900]);
}

#[test]
fn a_latest_time_more_than_the_limit_over_the_fastest_is_reported() {
    // The limit is the one `pilot bench` uses.
    let percent = test_metrics::regression_percent(&root());
    assert_eq!(percent, 50, "bench_regression_percent() in selfhost/cli/bench_cmd.pilot");

    // 1500 is exactly 50% over 1000: not more than 50%.
    assert_eq!(test_metrics::regression_warning("t", &[1000, 1500], 50), None);
    // 1600 is 60% over the fastest 1000.
    assert_eq!(
        test_metrics::regression_warning("t", &[1200, 1000, 1600], 50).as_deref(),
        Some("t: latest time 1600us is 60% over its fastest time 1000us (more than 50%)")
    );
    // Only the latest run counts: an old slow run is not reported.
    assert_eq!(test_metrics::regression_warning("t", &[3000, 1000, 1100], 50), None);
    assert_eq!(test_metrics::regression_warning("t", &[], 50), None);
}

#[test]
fn pilot_bench_prints_each_tests_times_by_milestone() {
    let dir = fresh_dir("table");
    // M59: runs 2.0s, 1.5s, 2.4s. Average 5.9 / 3 = 1.966s (truncated),
    // fastest 1.5s, slowest 2.4s, latest 2.4s = 60% over the fastest.
    write_history(&dir, "M59", "test_59_1", "100\t2000000\n200\t1500000\n300\t2400000\n");
    // M1: one run of 0.5s. M10: a malformed line is skipped, one run of 1ms.
    write_history(&dir, "M1", "test_m1_1", "100\t500000\n");
    write_history(&dir, "M10", "test_m10_2", "junk\n100\t1000\n");
    // M10 again: 0.3s then 0.45s is exactly 50% over, which is not flagged.
    write_history(&dir, "M10", "test_m10_1", "100\t300000\n200\t450000\n");
    // Files that are not histories are ignored.
    fs::write(dir.join("notes.txt"), "not a history").expect("write");
    write_history(&dir, "scratch", "test_x", "100\t5\n");

    let out = bench(&[dir.to_str().expect("utf-8 path")]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let want = "\
MILESTONE TEST         RUNS    AVERAGE    FASTEST    SLOWEST     LATEST  STATUS
M1        test_m1_1       1     0.500s     0.500s     0.500s     0.500s  ok
M10       test_m10_1      2     0.375s     0.300s     0.450s     0.450s  ok
M10       test_m10_2      1     0.001s     0.001s     0.001s     0.001s  ok
M59       test_59_1       3     1.966s     1.500s     2.400s     2.400s  look: +60% over fastest
RESULT: 4 tests, 1 more than 50% over their fastest time (worth a look, not a failure)
";
    assert_eq!(stdout, want);

    // --filter keeps the tests whose name contains the text.
    let out = bench(&[dir.to_str().unwrap(), "--filter", "m10"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("test_m10_1") && stdout.contains("test_m10_2"), "{stdout}");
    assert!(!stdout.contains("test_59_1") && !stdout.contains("test_m1_1 "), "{stdout}");
    assert!(stdout.ends_with("RESULT: 2 tests, 0 more than 50% over their fastest time (worth a look, not a failure)\n"), "{stdout}");
}

#[test]
fn pilot_bench_reads_what_the_recorder_writes() {
    let dir = fresh_dir("round_trip");
    for us in [4_000_000, 3_000_000, 5_000_000] {
        let timer = test_metrics::TestTimer::start(&dir, 61, "test_61_9_round_trip");
        timer.record(Duration::from_micros(us));
    }
    let out = bench(&[dir.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Average 12 / 3 = 4s; 5s is 66% over the fastest 3s.
    assert!(
        stdout.contains("M61       test_61_9_round_trip      3     4.000s     3.000s     5.000s     5.000s  look: +66% over fastest"),
        "{stdout}"
    );
}

#[test]
fn pilot_bench_without_history_or_with_bad_usage() {
    let dir = fresh_dir("empty");
    let out = bench(&[dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("no recorded test times in `"));

    let out = bench(&["one", "two"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("`pilot bench` takes at most one directory"));
}
