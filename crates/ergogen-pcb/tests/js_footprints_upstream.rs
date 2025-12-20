#![cfg(feature = "js-footprints")]

use std::path::PathBuf;

use ergogen_pcb::generate_kicad_pcb_from_yaml_str;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn fixture_dir() -> PathBuf {
    workspace_root().join("fixtures/m7/js_footprints")
}

fn normalize_upstream(s: &str) -> String {
    let mut out = String::new();
    for line in s.replace("\r\n", "\n").lines() {
        let trimmed = line.trim_end();
        if trimmed.contains("(date ") || trimmed.contains("(generator_version ") {
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        out.push_str(trimmed);
        out.push('\n');
    }
    out.trim_end_matches('\n').to_string()
}

fn upstream_dir() -> Option<PathBuf> {
    std::env::var("ERGOGEN_UPSTREAM_OUTPUT_DIR")
        .ok()
        .map(|raw| {
            let path = PathBuf::from(raw);
            if path.is_absolute() {
                path
            } else {
                workspace_root().join(path)
            }
        })
}

#[test]
fn js_footprints_match_upstream_outputs_when_configured() {
    let upstream_dir = match upstream_dir() {
        Some(dir) => dir,
        None => return,
    };

    let dir = fixture_dir();
    let mut yamls: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().map(|ext| ext == "yaml").unwrap_or(false))
        .filter(|path| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("knuckles_"))
                .unwrap_or(false)
        })
        .collect();
    yamls.sort();

    if yamls.is_empty() {
        return;
    }

    for yaml_path in yamls {
        let stem = yaml_path
            .file_stem()
            .expect("fixture name")
            .to_string_lossy()
            .to_string();
        let expected_path = upstream_dir.join(format!("{stem}___pcbs_pcb.kicad_pcb"));
        assert!(
            expected_path.exists(),
            "missing upstream output at {}",
            expected_path.display()
        );

        let yaml = std::fs::read_to_string(&yaml_path).unwrap();
        let got = generate_kicad_pcb_from_yaml_str(&yaml, "pcb").unwrap();
        let expected = std::fs::read_to_string(&expected_path).unwrap();
        assert_eq!(
            normalize_upstream(&got),
            normalize_upstream(&expected),
            "fixture {stem} mismatch"
        );
    }
}
