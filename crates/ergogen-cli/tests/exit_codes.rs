use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn exit_code_usage_is_1_for_missing_args() {
    let bin = env!("CARGO_BIN_EXE_ergogen");
    let status = Command::new(bin)
        .args(["render"])
        .status()
        .expect("run ergogen");
    assert_eq!(status.code(), Some(1));
}

#[test]
fn exit_code_input_is_2_for_missing_file() {
    let bin = env!("CARGO_BIN_EXE_ergogen");
    let out_dir = tempfile::tempdir().expect("tempdir");
    let output = out_dir.path().join("output");
    let missing = out_dir.path().join("nope.yaml");

    let status = Command::new(bin)
        .args([
            "render",
            missing.to_string_lossy().as_ref(),
            "--output",
            output.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("run ergogen render");
    assert_eq!(status.code(), Some(2));
}

#[test]
fn exit_code_input_is_2_for_invalid_yaml() {
    let bin = env!("CARGO_BIN_EXE_ergogen");
    let out_dir = tempfile::tempdir().expect("tempdir");
    let output = out_dir.path().join("output");
    let bad = out_dir.path().join("bad.yaml");
    std::fs::write(&bad, "a: [1, 2,").expect("write bad yaml");

    let status = Command::new(bin)
        .args([
            "render",
            bad.to_string_lossy().as_ref(),
            "--output",
            output.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("run ergogen render");
    assert_eq!(status.code(), Some(2));
}

#[test]
fn exit_code_input_is_2_for_zip_missing_config() {
    let bin = env!("CARGO_BIN_EXE_ergogen");
    let out_dir = tempfile::tempdir().expect("tempdir");
    let output = out_dir.path().join("output");

    let zip_path = out_dir.path().join("empty.zip");
    {
        let f = std::fs::File::create(&zip_path).expect("create zip");
        let w = zip::ZipWriter::new(f);
        w.finish().expect("finish zip");
    }

    let status = Command::new(bin)
        .args([
            "render",
            zip_path.to_string_lossy().as_ref(),
            "--output",
            output.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("run ergogen render");
    assert_eq!(status.code(), Some(2));
}

#[test]
fn exit_code_success_is_0() {
    let bin = env!("CARGO_BIN_EXE_ergogen");
    let input = workspace_root().join("fixtures/upstream/fixtures/minimal.yaml");
    let out_dir = tempfile::tempdir().expect("tempdir");
    let output = out_dir.path().join("output");

    let status = Command::new(bin)
        .args([
            "render",
            input.to_string_lossy().as_ref(),
            "--output",
            output.to_string_lossy().as_ref(),
            "--clean",
        ])
        .status()
        .expect("run ergogen render");
    assert_eq!(status.code(), Some(0));
}
