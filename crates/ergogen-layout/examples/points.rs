use std::path::PathBuf;

use ergogen_layout::parse_points;
use ergogen_parser::PreparedConfig;

fn main() {
    let mut args = std::env::args_os();
    let _ = args.next();
    let Some(path) = args.next() else {
        eprintln!("usage: points <path-to-yaml>");
        std::process::exit(2);
    };

    let path = PathBuf::from(path);
    let yaml = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("failed to read {}: {e}", path.display());
        std::process::exit(2);
    });

    let prepared = PreparedConfig::from_yaml_str(&yaml).unwrap_or_else(|e| {
        eprintln!("parse failed: {e}");
        std::process::exit(1);
    });

    let points = parse_points(&prepared.canonical, &prepared.units).unwrap_or_else(|e| {
        eprintln!("layout failed: {e}");
        std::process::exit(1);
    });

    let mut names: Vec<&str> = points.keys().map(|s| s.as_str()).collect();
    names.sort_unstable();
    for n in names {
        let p = points.get(n).unwrap();
        println!(
            "{n}\tx={:.6}\ty={:.6}\tr={:.6}\tbind={:?}\tasym={:?}\tskip={}\tmirrored={:?}",
            p.x, p.y, p.r, p.meta.bind, p.meta.asym, p.meta.skip, p.meta.mirrored
        );
    }
}
