//! Pass times of the milestone tests, kept across runs.
//!
//! Each test that passes appends one line to
//! `<dir>/M<milestone>/<test name>.tsv`: the Unix time in seconds it finished,
//! a tab, and how long its body ran in microseconds. A test that fails (or
//! never reaches its end) deletes its file, so a broken test leaves no
//! history. `pilot bench` (`selfhost/cli/bench_cmd.pilot`) reads the files
//! and prints the average, fastest, slowest and latest time of each test.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Where the milestone tests keep their pass times.
pub fn metrics_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("target/test-metrics")
}

/// The history file of one test.
pub fn history_file(dir: &Path, milestone: u32, test: &str) -> PathBuf {
    dir.join(format!("M{milestone}")).join(format!("{test}.tsv"))
}

/// Times one run of a test. Dropping it without `record` erases the test's
/// history: the run did not pass.
pub struct TestTimer {
    file: PathBuf,
    start: Instant,
    recorded: bool,
}

impl TestTimer {
    pub fn start(dir: &Path, milestone: u32, test: &str) -> Self {
        TestTimer {
            file: history_file(dir, milestone, test),
            start: Instant::now(),
            recorded: false,
        }
    }

    /// How long the test has run so far.
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Appends a passing run of `took` to the history. Returns the times in
    /// the history, oldest first, this run last.
    pub fn record(mut self, took: Duration) -> Vec<u64> {
        self.recorded = true;
        let finished = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent).expect("create the test-metrics directory");
        }
        let line = format!("{finished}\t{}\n", took.as_micros());
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)
            .and_then(|mut f| f.write_all(line.as_bytes()))
            .expect("append to the test's history");
        read_history(&self.file)
    }
}

impl Drop for TestTimer {
    fn drop(&mut self) {
        if !self.recorded {
            let _ = std::fs::remove_file(&self.file);
        }
    }
}

/// The run times (microseconds) in a history file, oldest first.
pub fn read_history(file: &Path) -> Vec<u64> {
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split('\t').nth(1)?.trim().parse().ok())
        .collect()
}

/// The slowdown over the fastest time, in percent, above which a test's
/// latest time is reported: `bench_regression_percent()` in
/// `selfhost/cli/bench_cmd.pilot`, the one place it is set.
pub fn regression_percent(workspace_root: &Path) -> u64 {
    let src = std::fs::read_to_string(workspace_root.join("selfhost/cli/bench_cmd.pilot"))
        .expect("read selfhost/cli/bench_cmd.pilot");
    let after = src
        .split("fn bench_regression_percent() -> i64 =>")
        .nth(1)
        .expect("bench_regression_percent() in selfhost/cli/bench_cmd.pilot");
    after
        .split(';')
        .next()
        .and_then(|n| n.trim().parse().ok())
        .expect("bench_regression_percent() returns a number")
}

/// The warning for a latest time more than `percent` slower than the
/// fastest in `history` (this run last), or `None`.
pub fn regression_warning(test: &str, history: &[u64], percent: u64) -> Option<String> {
    let (&latest, _) = history.split_last()?;
    let fastest = *history.iter().min()?;
    if fastest == 0 || latest * 100 <= fastest * (100 + percent) {
        return None;
    }
    let over = (latest - fastest) * 100 / fastest;
    Some(format!(
        "{test}: latest time {latest}us is {over}% over its fastest time {fastest}us (more than {percent}%)"
    ))
}
