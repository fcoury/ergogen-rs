use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn normalize_text(input: &str) -> String {
    input.replace("\r\n", "\n").trim().to_string()
}

#[test]
fn cli_logs_match_upstream_kle_fixture() {
    let exe = env!("CARGO_BIN_EXE_ergogen");
    let root = workspace_root();
    let fixture_dir = root.join("fixtures/upstream/test/cli/minimal_kle");
    let input = root.join("fixtures/upstream/fixtures/minimal_kle.json");
    let expected_log = std::fs::read_to_string(fixture_dir.join("log")).unwrap();

    let temp = tempfile::tempdir().unwrap();
    let output = Command::new(exe)
        .current_dir(temp.path())
        .args(["render", input.to_str().unwrap(), "--output", "output"])
        .output()
        .expect("run ergogen cli");
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let got = String::from_utf8_lossy(&output.stdout);
    assert_eq!(normalize_text(&got), normalize_text(&expected_log));
}
