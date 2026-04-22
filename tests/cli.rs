//! CLI integration tests: spawn the `quire` binary against the real
//! starter policy and assert the exit-code contract end-to-end.
//!
//! These tests complement the library-level evaluator tests in `src/` by
//! covering the one surface callers actually consume: the process exit
//! code. Cargo sets `CARGO_BIN_EXE_quire` to the built binary path when
//! running integration tests, so no `assert_cmd` dependency is needed.

use std::io::Write;
use std::process::{Command, Stdio};

fn quire_bin() -> &'static str {
    env!("CARGO_BIN_EXE_quire")
}

fn run_check(policy: &str, argv: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(quire_bin());
    cmd.arg("check").arg("--policy").arg(policy).arg("--");
    for a in argv {
        cmd.arg(a);
    }
    let out = cmd.output().expect("spawn quire");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (code, stdout, stderr)
}

fn run_check_json(policy: &str, argv: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(quire_bin());
    cmd.arg("check")
        .arg("--policy")
        .arg(policy)
        .arg("--json")
        .arg("--");
    for a in argv {
        cmd.arg(a);
    }
    let out = cmd.output().expect("spawn quire");
    let code = out.status.code().unwrap_or(-1);
    (
        code,
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn starter_allows_ls() {
    let (code, stdout, _) = run_check("examples/starter.policy", &["ls", "-la"]);
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "allow");
}

#[test]
fn starter_allows_cat() {
    let (code, stdout, _) = run_check("examples/starter.policy", &["cat", "README.md"]);
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "allow");
}

#[test]
fn starter_forbids_rm_rf() {
    let (code, stdout, _) = run_check("examples/starter.policy", &["rm", "-rf", "/tmp/x"]);
    assert_eq!(code, 20);
    assert_eq!(stdout.trim(), "forbidden");
}

#[test]
fn starter_prompts_git_push() {
    let (code, stdout, _) = run_check(
        "examples/starter.policy",
        &["git", "push", "origin", "main"],
    );
    assert_eq!(code, 10);
    assert_eq!(stdout.trim(), "prompt");
}

#[test]
fn starter_no_match_on_unknown_command() {
    let (code, stdout, _) = run_check("examples/starter.policy", &["wget", "http://x"]);
    assert_eq!(code, 30);
    assert_eq!(stdout.trim(), "no-match");
}

#[test]
fn json_flag_emits_expected_shape() {
    let (code, stdout, _) = run_check_json("examples/starter.policy", &["ls", "-la"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert_eq!(v["decision"], "allow");
    assert_eq!(v["exit_code"], 0);
    assert_eq!(v["argv"], serde_json::json!(["ls", "-la"]));
}

#[test]
fn missing_policy_file_exits_with_load_error() {
    let (code, _, stderr) = run_check("examples/does-not-exist.policy", &["ls"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("quire:") && stderr.to_lowercase().contains("does-not-exist"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn lying_match_example_exits_with_validation_error() {
    let dir = tempdir();
    let path = dir.join("lying.policy");
    std::fs::write(
        &path,
        r#"prefix_rule(pattern=["ls"], decision="allow",
             justification="ls is read-only", match=["lsof"])
"#,
    )
    .unwrap();
    let (code, _, stderr) = run_check(path.to_str().unwrap(), &["ls"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("match=") && stderr.contains("lsof"),
        "stderr should name the lying example: {stderr}"
    );
}

#[test]
fn invalid_decision_in_policy_exits_with_load_error() {
    let dir = tempdir();
    let path = dir.join("bad-decision.policy");
    std::fs::write(
        &path,
        r#"prefix_rule(pattern=["ls"], decision="maybe", justification="unknown")"#,
    )
    .unwrap();
    let (code, _, stderr) = run_check(path.to_str().unwrap(), &["ls"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("invalid decision") && stderr.contains("maybe"),
        "stderr should reject bad decision: {stderr}"
    );
}

fn tempdir() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("quire-cli-test-{pid}-{n}"));
    std::fs::create_dir_all(&path).unwrap();
    path
}

// Keep a stdin smoke test so regressions to the CLI argv parser don't
// silently hang waiting for input.
#[test]
fn cli_does_not_read_stdin() {
    let mut child = Command::new(quire_bin())
        .args(["check", "--policy", "examples/starter.policy", "--", "ls"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    // Close stdin immediately by dropping it. If quire tries to read
    // stdin it will hit EOF; if it never reads stdin, no effect.
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(b"");
    }
    let out = child.wait_with_output().expect("wait");
    assert_eq!(out.status.code(), Some(0));
}
