use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const JSONL_FIXTURE: &str = r#"{"schema_version":4,"record_type":"session","session":{"id":"s1","source":"codex","source_id":"source-s1","title":"Codex session","directory":"/tmp/repo","repo_remote":"git@example.com:owner/repo.git","repo_slug":"owner/repo","repo_name":"repo","started_at":1000,"updated_at":1200,"message_count":2,"entrypoint":null,"custom_title":null,"summary":null,"duration_minutes":null,"source_file_path":null},"messages":[{"seq":0,"role":"user","timestamp":1000,"content":"please keep scope small"},{"seq":1,"role":"assistant","timestamp":1100,"content":"I will keep it focused"}],"usage_events":[],"events":[]}
"#;

#[test]
fn reflect_cli_reads_export_jsonl_from_recall_bin() {
    let fake = FakeRecall::new(JSONL_FIXTURE, 0, "");

    let output = Command::new(env!("CARGO_BIN_EXE_recall-reflect"))
        .env("RECALL_BIN", fake.script_path())
        .args([
            "--project",
            "/tmp/repo",
            "--repo",
            "owner/repo",
            "--source",
            "codex",
            "--time",
            "week",
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty(), "stderr should be empty on success");

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["summary"]["sessions"], 1);
    assert_eq!(json["summary"]["timeline_moments"], 2);
    assert_eq!(json["scope"]["project"], "/tmp/repo");
    assert_eq!(json["scope"]["repo"], "owner/repo");
    assert_eq!(json["scope"]["time_range"], "week");
    assert_eq!(json["scope"]["sources"][0], "codex");

    let calls = fake.calls();
    assert_eq!(
        calls,
        ["export --limit 0 --project /tmp/repo --repo owner/repo --source codex --time week"]
    );
}

#[test]
fn reflect_cli_syncs_selected_source_before_export() {
    let fake = FakeRecall::new(JSONL_FIXTURE, 0, "");

    let output = Command::new(env!("CARGO_BIN_EXE_recall-reflect"))
        .env("RECALL_BIN", fake.script_path())
        .args(["--sync", "--source", "opencode"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Recall reflect"));
    assert!(stdout.contains("Sessions: 1"));

    let calls = fake.calls();
    assert_eq!(calls, ["sync --source opencode", "export --limit 0 --source opencode"]);
}

#[test]
fn reflect_cli_reports_recall_command_failures_on_stderr() {
    let fake = FakeRecall::new("", 23, "boom");

    let output = Command::new(env!("CARGO_BIN_EXE_recall-reflect"))
        .env("RECALL_BIN", fake.script_path())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(!output.status.success(), "command should fail");
    assert!(output.stdout.is_empty(), "stdout must be data-only and empty on command errors");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Recall command failed"), "stderr: {stderr}");
    assert!(stderr.contains("boom"), "stderr: {stderr}");

    let calls = fake.calls();
    assert_eq!(calls, ["export --limit 0"]);
}

struct FakeRecall {
    dir: PathBuf,
    script: PathBuf,
    calls: PathBuf,
}

impl FakeRecall {
    fn new(export_stdout: &str, export_exit_code: i32, export_stderr: &str) -> Self {
        let dir = unique_temp_dir();
        fs::create_dir_all(&dir).unwrap();
        let script = dir.join("recall-fake.sh");
        let calls = dir.join("calls.txt");
        let stdout_path = dir.join("export.jsonl");
        let stderr_path = dir.join("export.stderr");
        fs::write(&stdout_path, export_stdout).unwrap();
        fs::write(&stderr_path, export_stderr).unwrap();
        fs::write(&calls, "").unwrap();

        let script_body = format!(
            r#"#!/bin/sh
printf '%s\n' "$*" >> "{calls}"
if [ "$1" = "sync" ]; then
  exit 0
fi
if [ "$1" = "export" ]; then
  cat "{stderr_path}" >&2
  cat "{stdout_path}"
  exit {export_exit_code}
fi
echo "unexpected command: $*" >&2
exit 99
"#,
            calls = calls.display(),
            stdout_path = stdout_path.display(),
            stderr_path = stderr_path.display(),
            export_exit_code = export_exit_code,
        );
        fs::write(&script, script_body).unwrap();
        make_executable(&script);

        Self { dir, script, calls }
    }

    fn script_path(&self) -> &Path {
        &self.script
    }

    fn calls(&self) -> Vec<String> {
        fs::read_to_string(&self.calls).unwrap().lines().map(str::to_string).collect()
    }
}

impl Drop for FakeRecall {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("recall-reflect-test-{}-{nanos}", std::process::id()))
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}
