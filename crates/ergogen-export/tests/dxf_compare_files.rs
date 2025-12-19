use ergogen_export::dxf::{NormalizeOptions, compare_files_semantic};

#[test]
fn compares_two_files_semantically() {
    let tmp = std::env::temp_dir().join("ergogen-export-dxf-compare");
    std::fs::create_dir_all(&tmp).unwrap();

    let left_path = tmp.join("left.dxf");
    let right_path = tmp.join("right.dxf");

    // Same entities, different ordering and LINE direction.
    let left = r#"0
SECTION
2
ENTITIES
0
LINE
10
0
20
0
11
1
21
0
0
CIRCLE
10
5
20
6
40
7
0
ENDSEC
0
EOF
"#;

    let right = r#"0
SECTION
2
ENTITIES
0
CIRCLE
10
5
20
6
40
7
0
LINE
10
1
20
0
11
0
21
0
0
ENDSEC
0
EOF
"#;

    std::fs::write(&left_path, left).unwrap();
    std::fs::write(&right_path, right).unwrap();

    compare_files_semantic(&left_path, &right_path, NormalizeOptions::default()).unwrap();
}
