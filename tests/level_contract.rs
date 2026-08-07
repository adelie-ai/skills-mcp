//! Acceptance criteria for mcp-core#40's level contract (epic D10) as applied
//! to skills-mcp's own diagnostic: the skipped-entry log site in
//! `repo::list_all`.
//!
//! Each test is named after the criterion it holds, so a failing run names
//! the unmet requirement rather than a line number.

mod support;

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard};

use mcp_core::telemetry::metrics::{self, Label};
use serde_json::Value;
use skills_mcp::repo;
use tracing::Level;

use support::capture;

/// Serialises the tests in this file that mutate the process-global skill-root
/// env vars, mirroring `src/repo.rs`'s own lock: they must not race any other
/// test in this binary that touches the same variables.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_guard() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Stands in for the operator's home directory: the segment of a skipped
/// entry's path that must never reach an INFO line. Distinct from the skill
/// name below so the assertions can tell which one leaked.
const SENTINEL_HOME: &str = "MARKER-7c2e-home-of-someone";

/// The skipped skill's own directory name. This is the identifier the level
/// contract allows at WARN (mcp-core#40: "a skill name is arguably an
/// identifier and fine at INFO").
const SENTINEL_SKILL: &str = "quarterly-notes";

/// Build one skill root containing a real unreadable entry: `SKILL.md` exists
/// as a directory instead of a file, so the read genuinely fails (`fs::
/// read_to_string` on a directory returns an error) rather than the test
/// only pretending it did. The path that fails carries both sentinels.
fn root_with_unreadable_entry(base: &std::path::Path) -> std::path::PathBuf {
    let root = base.join(SENTINEL_HOME).join("skills");
    let skill_md_as_dir = root.join(SENTINEL_SKILL).join(repo::SKILL_FILE);
    std::fs::create_dir_all(&skill_md_as_dir).expect("build the malformed fixture");
    root
}

/// AC: the skipped-entry message is a `warn!` carrying the skill name and a
/// reason, and never the path.
#[test]
fn skipped_entry_reason_reaches_warn_without_the_path() {
    let _guard = env_guard();
    let temp = tempdir();
    let root = root_with_unreadable_entry(&temp.path);
    unsafe {
        std::env::set_var(repo::ROOTS_ENV, root.display().to_string());
    }

    let recorded = capture(|| {
        repo::list_all();
    });

    unsafe {
        std::env::remove_var(repo::ROOTS_ENV);
    }

    let warn = recorded
        .events
        .iter()
        .find(|event| event.level == Level::WARN)
        .unwrap_or_else(|| {
            panic!(
                "a malformed entry must log a WARN; events were {:?}",
                recorded.event_summary()
            )
        });

    assert_eq!(
        warn.fields.get("skill").map(String::as_str),
        Some(SENTINEL_SKILL),
        "the WARN must carry the skill name as an identifier: {:?}",
        warn.fields
    );
    assert!(
        warn.fields.contains_key("reason"),
        "the WARN must carry a reason field: {:?}",
        warn.fields
    );
    assert!(
        !warn.fields.contains_key("path"),
        "a full path must not reach the WARN field set at all: {:?}",
        warn.fields
    );
    for value in warn.fields.values() {
        assert!(
            !value.contains(SENTINEL_HOME),
            "the path's home-directory segment reached a WARN field: {value:?}"
        );
    }
}

/// AC (mcp-core#40, non-negotiable #4): a path stays out of INFO. Drive a
/// real skipped entry with a sentinel path and assert it appears nowhere at
/// INFO or louder, and that it is still available at DEBUG -- otherwise the
/// level contract has nothing to hold back.
#[test]
fn no_info_or_above_event_carries_the_skipped_entry_path() {
    let _guard = env_guard();
    let temp = tempdir();
    let root = root_with_unreadable_entry(&temp.path);
    unsafe {
        std::env::set_var(repo::ROOTS_ENV, root.display().to_string());
    }

    let recorded = capture(|| {
        repo::list_all();
    });

    unsafe {
        std::env::remove_var(repo::ROOTS_ENV);
    }

    for event in &recorded.events {
        if event.level > Level::INFO {
            continue;
        }
        for (key, value) in &event.fields {
            assert!(
                !value.contains(SENTINEL_HOME),
                "an INFO-or-louder event carried the path's home segment, field {key:?}: {value:?}"
            );
        }
    }

    let at_debug = recorded.events.iter().any(|event| {
        event.level == Level::DEBUG
            && event
                .fields
                .values()
                .any(|value| value.contains(SENTINEL_HOME))
    });
    assert!(
        at_debug,
        "the full path must still be available at DEBUG, or the level contract has \
         nothing to hold back; events were {:?}",
        recorded.event_summary()
    );
}

/// AC: skipped entries are counted through the mcp-core metrics facade, by a
/// bounded `reason` label (mcp-core#40: "reason is a bounded label; a skill
/// name from a directory scan is not").
#[test]
fn skipped_entries_are_counted_by_a_bounded_reason_label() {
    let _guard = env_guard();
    let temp = tempdir();
    let root = root_with_unreadable_entry(&temp.path);
    unsafe {
        std::env::set_var(repo::ROOTS_ENV, root.display().to_string());
    }

    let labels = [Label::new("reason", "read_failed")];
    let before = counter_total("skills.skipped_entries", &labels);

    repo::list_all();

    let after = counter_total("skills.skipped_entries", &labels);

    unsafe {
        std::env::remove_var(repo::ROOTS_ENV);
    }

    assert_eq!(
        after,
        before + 1,
        "a read failure must increment skills.skipped_entries, labelled reason=read_failed"
    );
}

/// AC (mcp-core#40, non-negotiable #3): with `RUST_LOG=trace`, every line the
/// stdio transport writes to stdout parses as JSON-RPC, even while a real
/// skipped skill logs its WARN/DEBUG pair on stderr in the same run.
#[test]
fn stdio_stdout_carries_only_jsonrpc_at_trace() {
    let temp = tempdir();
    let root = root_with_unreadable_entry(&temp.path);

    let requests = [
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
        serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {"name": "skills_list_skills", "arguments": {}},
        }),
    ];

    let mut child = Command::new(env!("CARGO_BIN_EXE_skills-mcp"))
        .arg("serve")
        .env("RUST_LOG", "trace")
        .env("HOME", temp.path.display().to_string())
        .env(repo::ROOTS_ENV, root.display().to_string())
        .env_remove("SKILLS_MCP_WRITE_ROOT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("skills-mcp must start");

    {
        let stdin = child.stdin.as_mut().expect("skills-mcp has a piped stdin");
        for request in &requests {
            writeln!(stdin, "{request}").expect("skills-mcp must accept its input");
        }
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("skills-mcp must exit");
    assert!(
        output.status.success(),
        "skills-mcp must exit cleanly, otherwise an empty stdout proves nothing: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");

    let mut replies = 0;
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value = serde_json::from_str(line).unwrap_or_else(|e| {
            panic!("every stdout line must be JSON-RPC, but {line:?} is not: {e}")
        });
        assert_eq!(
            value.get("jsonrpc").and_then(Value::as_str),
            Some("2.0"),
            "every stdout line must carry the JSON-RPC envelope: {line:?}"
        );
        replies += 1;
    }
    assert_eq!(replies, 3, "skills-mcp must answer all three requests");
    assert!(
        !stdout.contains(SENTINEL_HOME),
        "the skipped entry's path leaked onto stdout: {stdout:?}"
    );

    assert!(
        stderr.contains(SENTINEL_HOME),
        "the skipped entry must have actually fired for this test to prove anything; \
         stderr was: {stderr:?}"
    );
    assert!(
        stderr.contains("DEBUG") || stderr.contains("TRACE"),
        "at RUST_LOG=trace the logs must reach stderr, or the subscriber was never \
         installed: {stderr:?}"
    );
}

fn counter_total(name: &str, labels: &[Label]) -> u64 {
    metrics::global()
        .snapshot()
        .counters
        .iter()
        .find(|counter| counter.name == name && same_labels(&counter.labels, labels))
        .map_or(0, |counter| counter.total)
}

fn same_labels(recorded: &[Label], wanted: &[Label]) -> bool {
    recorded.len() == wanted.len()
        && wanted.iter().all(|want| {
            recorded
                .iter()
                .any(|have| have.key() == want.key() && have.value() == want.value())
        })
}

/// Tiny in-tree temp-dir helper (mirrors `src/repo.rs`'s own) to avoid adding
/// a tempfile dev-dependency for these tests.
struct TempDir {
    path: std::path::PathBuf,
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn tempdir() -> TempDir {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let path = std::env::temp_dir().join(format!(
        "skills-mcp-level-contract-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).unwrap();
    TempDir { path }
}
