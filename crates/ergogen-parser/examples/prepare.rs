use std::path::PathBuf;

use ergogen_parser::PreparedIr;

fn main() {
    let mut args = std::env::args_os();
    let _ = args.next();
    let Some(path) = args.next() else {
        eprintln!("usage: prepare <path-to-yaml>");
        std::process::exit(2);
    };

    let path = PathBuf::from(path);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("failed to read {}: {e}", path.display());
        std::process::exit(2);
    });

    let ir = PreparedIr::from_yaml_str(&raw).unwrap_or_else(|e| {
        eprintln!("prepare failed: {e}");
        std::process::exit(1);
    });

    println!("{}", ir.canonical.to_json_compact_string());
}
