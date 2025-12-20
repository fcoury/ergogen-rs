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
fn render_bundle_folder_smoke() {
    let bin = env!("CARGO_BIN_EXE_ergogen");
    let input = workspace_root().join("fixtures/upstream/fixtures/bundle");
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
    assert!(status.success());

    assert!(output.join("outlines/box.dxf").is_file());
    assert!(output.join("pcbs/pcb.kicad_pcb").is_file());
    assert!(output.join("pcbs/custom_template.kicad_pcb").is_file());
}

#[test]
fn render_bundle_zip_smoke() {
    let bin = env!("CARGO_BIN_EXE_ergogen");
    let input = workspace_root().join("fixtures/upstream/fixtures/bundle.zip");
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
    assert!(status.success());

    assert!(output.join("outlines/box.dxf").is_file());
    assert!(output.join("pcbs/pcb.kicad_pcb").is_file());
    assert!(output.join("pcbs/custom_template.kicad_pcb").is_file());
}
