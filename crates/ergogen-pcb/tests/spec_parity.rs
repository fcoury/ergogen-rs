use ergogen_pcb::generate_kicad_pcb_from_yaml_str;

fn normalize(s: &str) -> String {
    fn normalize_fp_text_atom_token(line: &str, prefix: &str) -> String {
        if !line.starts_with(prefix) {
            return line.to_string();
        }
        let rest = &line[prefix.len()..];
        if !rest.starts_with('"') {
            return line.to_string();
        }
        let Some(end_quote) = rest[1..].find('"').map(|idx| idx + 1) else {
            return line.to_string();
        };
        let token = &rest[1..end_quote];
        let safe_unquoted = !token.is_empty()
            && !token
                .chars()
                .any(|c| c.is_whitespace() || c == '(' || c == ')' || c == '"');
        if !safe_unquoted {
            return line.to_string();
        }
        let mut out = String::new();
        out.push_str(prefix);
        out.push_str(token);
        out.push_str(&rest[end_quote + 1..]);
        out
    }

    fn normalize_fp_text_at_rotation_zero(line: &str) -> String {
        if !line.starts_with("(fp_text ") {
            return line.to_string();
        }
        let Some(at_start) = line.find("(at ") else {
            return line.to_string();
        };
        let after_at = &line[at_start + 4..];
        let Some(at_end_rel) = after_at.find(')') else {
            return line.to_string();
        };
        let at_contents = &after_at[..at_end_rel];
        let parts: Vec<&str> = at_contents.split_whitespace().collect();
        if parts.len() != 3 || parts[2] != "0" {
            return line.to_string();
        }
        let mut out = String::new();
        out.push_str(&line[..at_start]);
        out.push_str("(at ");
        out.push_str(parts[0]);
        out.push(' ');
        out.push_str(parts[1]);
        out.push(')');
        out.push_str(&after_at[at_end_rel + 1..]);
        out
    }

    fn normalize_numeric_tokens(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut in_quote = false;
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '"' {
                in_quote = !in_quote;
                out.push(c);
                continue;
            }
            if in_quote {
                out.push(c);
                continue;
            }
            if c == '-' || c == '+' || c.is_ascii_digit() {
                let mut token = String::new();
                token.push(c);
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_digit() || next == '.' {
                        token.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Some(dot) = token.find('.') {
                    let (head, tail) = token.split_at(dot);
                    let tail = &tail[1..];
                    if !tail.is_empty() && tail.chars().all(|d| d == '0') {
                        out.push_str(head);
                        continue;
                    }
                }
                out.push_str(&token);
                continue;
            }
            out.push(c);
        }
        out
    }

    let joined = s
        .replace("\r\n", "\n")
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| {
            // Normalize whitespace artifacts from templates (double spaces before `(net ...)`).
            let line = line
                .replace(")  (net", ") (net")
                .replace(") )", "))")
                .replace(")  )", "))");
            // Templates sometimes contain `-0` literals, while the spec renderer normalizes
            // near-zero values to `0`. Treat these as equivalent for parity checks.
            let line = line.replace("-0 ", "0 ").replace("-0)", "0)");
            // Some upstream templates emit unquoted fp_text user atoms and include an explicit
            // `0` rotation; normalize those variants.
            let line = normalize_fp_text_atom_token(&line, "(fp_text user ");
            let line = normalize_fp_text_atom_token(&line, "(fp_text reference ");
            let line = normalize_fp_text_atom_token(&line, "(fp_text value ");
            normalize_fp_text_at_rotation_zero(&line)
        })
        .collect::<Vec<_>>()
        // Treat line breaks in upstream templates as insignificant; they frequently wrap
        // long pad/fp_text definitions across multiple lines.
        .join(" ");

    let mut joined = normalize_numeric_tokens(&joined);
    // Normalize padding around close-parens produced by line wrapping.
    while joined.contains(" )") {
        joined = joined.replace(" )", ")");
    }
    while joined.contains("  ") {
        joined = joined.replace("  ", " ");
    }
    joined
}

fn parity_case(name: &str, template_yaml: &str, spec_yaml: &str) {
    let template = generate_kicad_pcb_from_yaml_str(template_yaml, "pcb").unwrap();
    let spec = generate_kicad_pcb_from_yaml_str(spec_yaml, "pcb").unwrap();
    assert_eq!(
        normalize(&spec),
        normalize(&template),
        "{name} parity mismatch"
    );
}

#[test]
fn pad_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: pad
    params:
      net: P1
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: pad.yaml
      net: P1
"#;

    parity_case("pad", template_yaml, spec_yaml);
}

#[test]
fn diode_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: diode
    params:
      from: D_FROM
      to: D_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: diode.yaml
      from: D_FROM
      to: D_TO
"#;

    parity_case("diode", template_yaml, spec_yaml);
}

#[test]
fn button_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: button
    params:
      from: BTN_FROM
      to: BTN_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: button.yaml
      from: BTN_FROM
      to: BTN_TO
"#;

    parity_case("button", template_yaml, spec_yaml);
}

#[test]
fn mx_base_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: mx
    params:
      from: MX_FROM
      to: MX_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: mx_base.yaml
      from: MX_FROM
      to: MX_TO
"#;

    parity_case("mx_base", template_yaml, spec_yaml);
}

#[test]
fn trrs_base_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: trrs
    params:
      A: net_a
      B: net_b
      C: net_c
      D: net_d
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: trrs_base.yaml
      A: net_a
      B: net_b
      C: net_c
      D: net_d
"#;

    parity_case("trrs_base", template_yaml, spec_yaml);
}

#[test]
fn trrs_reverse_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: trrs
    params:
      reverse: true
      symmetric: false
      A: net_a
      B: net_b
      C: net_c
      D: net_d
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: trrs_reverse.yaml
      A: net_a
      B: net_b
      C: net_c
      D: net_d
"#;

    parity_case("trrs_reverse", template_yaml, spec_yaml);
}

#[test]
fn trrs_reverse_symmetric_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: trrs
    params:
      reverse: true
      symmetric: true
      A: net_a
      B: net_b
      C: net_c
      D: net_d
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: trrs_reverse_symmetric.yaml
      A: net_a
      B: net_b
      C: net_c
      D: net_d
"#;

    parity_case("trrs_reverse_symmetric", template_yaml, spec_yaml);
}

#[test]
fn mx_hotswap_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: mx
    params:
      hotswap: true
      from: MX_FROM
      to: MX_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: mx_hotswap.yaml
      from: MX_FROM
      to: MX_TO
"#;

    parity_case("mx_hotswap", template_yaml, spec_yaml);
}

#[test]
fn mx_reverse_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: mx
    params:
      reverse: true
      from: MX_FROM
      to: MX_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: mx_reverse.yaml
      from: MX_FROM
      to: MX_TO
"#;

    parity_case("mx_reverse", template_yaml, spec_yaml);
}

#[test]
fn mx_reverse_hotswap_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: mx
    params:
      reverse: true
      hotswap: true
      from: MX_FROM
      to: MX_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: mx_reverse_hotswap.yaml
      from: MX_FROM
      to: MX_TO
"#;

    parity_case("mx_reverse_hotswap", template_yaml, spec_yaml);
}

#[test]
fn mx_keycaps_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: mx
    params:
      keycaps: true
      from: MX_FROM
      to: MX_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: mx_keycaps.yaml
      from: MX_FROM
      to: MX_TO
"#;

    parity_case("mx_keycaps", template_yaml, spec_yaml);
}

#[test]
fn mx_keycaps_reverse_hotswap_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: mx
    params:
      keycaps: true
      reverse: true
      hotswap: true
      from: MX_FROM
      to: MX_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: mx_keycaps_reverse_hotswap.yaml
      from: MX_FROM
      to: MX_TO
"#;

    parity_case("mx_keycaps_reverse_hotswap", template_yaml, spec_yaml);
}

#[test]
fn choc_base_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: choc
    params:
      from: CHOC_FROM
      to: CHOC_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: choc_base.yaml
      from: CHOC_FROM
      to: CHOC_TO
"#;

    parity_case("choc_base", template_yaml, spec_yaml);
}

#[test]
fn choc_hotswap_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: choc
    params:
      hotswap: true
      from: CHOC_FROM
      to: CHOC_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: choc_hotswap.yaml
      from: CHOC_FROM
      to: CHOC_TO
"#;

    parity_case("choc_hotswap", template_yaml, spec_yaml);
}

#[test]
fn choc_reverse_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: choc
    params:
      reverse: true
      from: CHOC_FROM
      to: CHOC_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: choc_reverse.yaml
      from: CHOC_FROM
      to: CHOC_TO
"#;

    parity_case("choc_reverse", template_yaml, spec_yaml);
}

#[test]
fn choc_reverse_hotswap_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: choc
    params:
      reverse: true
      hotswap: true
      from: CHOC_FROM
      to: CHOC_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: choc_reverse_hotswap.yaml
      from: CHOC_FROM
      to: CHOC_TO
"#;

    parity_case("choc_reverse_hotswap", template_yaml, spec_yaml);
}

#[test]
fn choc_keycaps_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: choc
    params:
      keycaps: true
      from: CHOC_FROM
      to: CHOC_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: choc_keycaps.yaml
      from: CHOC_FROM
      to: CHOC_TO
"#;

    parity_case("choc_keycaps", template_yaml, spec_yaml);
}

#[test]
fn choc_keycaps_reverse_hotswap_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: choc
    params:
      keycaps: true
      reverse: true
      hotswap: true
      from: CHOC_FROM
      to: CHOC_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: choc_keycaps_reverse_hotswap.yaml
      from: CHOC_FROM
      to: CHOC_TO
"#;

    parity_case("choc_keycaps_reverse_hotswap", template_yaml, spec_yaml);
}

#[test]
fn pad_right_text_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: pad
    params:
      net: P1
      align: right
      text: any_value
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: pad_right_text.yaml
      net: P1
"#;

    parity_case("pad_right_text", template_yaml, spec_yaml);
}

#[test]
fn pad_up_back_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: pad
    params:
      net: P1
      front: false
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: pad_up_back.yaml
      net: P1
"#;

    parity_case("pad_up_back", template_yaml, spec_yaml);
}

#[test]
fn button_back_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: button
    params:
      side: B
      from: BTN_FROM
      to: BTN_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: button_back.yaml
      from: BTN_FROM
      to: BTN_TO
"#;

    parity_case("button_back", template_yaml, spec_yaml);
}

#[test]
fn chocmini_base_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: chocmini
    params:
      from: MINI_FROM
      to: MINI_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: chocmini_base.yaml
      from: MINI_FROM
      to: MINI_TO
"#;

    parity_case("chocmini_base", template_yaml, spec_yaml);
}

#[test]
fn chocmini_reverse_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: chocmini
    params:
      reverse: true
      from: MINI_FROM
      to: MINI_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: chocmini_reverse.yaml
      from: MINI_FROM
      to: MINI_TO
"#;

    parity_case("chocmini_reverse", template_yaml, spec_yaml);
}

#[test]
fn chocmini_keycaps_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: chocmini
    params:
      keycaps: true
      from: MINI_FROM
      to: MINI_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: chocmini_keycaps.yaml
      from: MINI_FROM
      to: MINI_TO
"#;

    parity_case("chocmini_keycaps", template_yaml, spec_yaml);
}

#[test]
fn chocmini_keycaps_reverse_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: chocmini
    params:
      keycaps: true
      reverse: true
      from: MINI_FROM
      to: MINI_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: chocmini_keycaps_reverse.yaml
      from: MINI_FROM
      to: MINI_TO
"#;

    parity_case("chocmini_keycaps_reverse", template_yaml, spec_yaml);
}

#[test]
fn rest_alps_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: alps
    params:
      from: ALPS_FROM
      to: ALPS_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: rest_alps.yaml
      from: ALPS_FROM
      to: ALPS_TO
"#;

    parity_case("rest_alps", template_yaml, spec_yaml);
}

#[test]
fn rest_jstph_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: jstph
    params:
      pos: JST_POS
      neg: JST_NEG
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: rest_jstph.yaml
      pos: JST_POS
      neg: JST_NEG
"#;

    parity_case("rest_jstph", template_yaml, spec_yaml);
}

#[test]
fn rest_jumper_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: jumper
    params:
      from: J_FROM
      to: J_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: rest_jumper.yaml
      from: J_FROM
      to: J_TO
"#;

    parity_case("rest_jumper", template_yaml, spec_yaml);
}

#[test]
fn rest_oled_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: oled
    params:
      SDA: NET_SDA
      SCL: NET_SCL
      VCC: NET_VCC
      GND: NET_GND
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: rest_oled.yaml
      SDA: NET_SDA
      SCL: NET_SCL
      VCC: NET_VCC
      GND: NET_GND
"#;

    parity_case("rest_oled", template_yaml, spec_yaml);
}

#[test]
fn rest_omron_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: omron
    params:
      from: O_FROM
      to: O_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: rest_omron.yaml
      from: O_FROM
      to: O_TO
"#;

    parity_case("rest_omron", template_yaml, spec_yaml);
}

#[test]
fn rest_slider_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: slider
    params:
      from: SL_FROM
      to: SL_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: rest_slider.yaml
      from: SL_FROM
      to: SL_TO
"#;

    parity_case("rest_slider", template_yaml, spec_yaml);
}

#[test]
fn rest_slider_back_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: slider
    params:
      side: B
      from: SL_FROM
      to: SL_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: rest_slider_back.yaml
      from: SL_FROM
      to: SL_TO
"#;

    parity_case("rest_slider_back", template_yaml, spec_yaml);
}

#[test]
fn rest_via_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: via
    params:
      net: VIA_NET
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: rest_via.yaml
      net: VIA_NET
"#;

    parity_case("rest_via", template_yaml, spec_yaml);
}

#[test]
fn rest_rgb_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: rgb
    params:
      VCC: NET_VCC
      dout: NET_DOUT
      GND: NET_GND
      din: NET_DIN
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: rest_rgb.yaml
      VCC: NET_VCC
      dout: NET_DOUT
      GND: NET_GND
      din: NET_DIN
"#;

    parity_case("rest_rgb", template_yaml, spec_yaml);
}

#[test]
fn rest_rotary_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: rotary
    params:
      A: ROT_A
      C: ROT_C
      B: ROT_B
      from: ROT_FROM
      to: ROT_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: rest_rotary.yaml
      A: ROT_A
      C: ROT_C
      B: ROT_B
      from: ROT_FROM
      to: ROT_TO
"#;

    parity_case("rest_rotary", template_yaml, spec_yaml);
}

#[test]
fn rest_scrollwheel_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: scrollwheel
    params:
      from: SW_FROM
      to: SW_TO
      A: SW_A
      B: SW_B
      C: SW_C
      D: SW_D
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: rest_scrollwheel.yaml
      from: SW_FROM
      to: SW_TO
      A: SW_A
      B: SW_B
      C: SW_C
      D: SW_D
"#;

    parity_case("rest_scrollwheel", template_yaml, spec_yaml);
}

#[test]
fn rest_scrollwheel_reverse_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: scrollwheel
    params:
      from: SW_FROM
      to: SW_TO
      A: SW_A
      B: SW_B
      C: SW_C
      D: SW_D
      reverse: true
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: rest_scrollwheel_reverse.yaml
      from: SW_FROM
      to: SW_TO
      A: SW_A
      B: SW_B
      C: SW_C
      D: SW_D
"#;

    parity_case("rest_scrollwheel_reverse", template_yaml, spec_yaml);
}

#[test]
fn promicro_down_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: promicro
    params:
      orientation: down
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: promicro_down.yaml
"#;

    parity_case("promicro_down", template_yaml, spec_yaml);
}

#[test]
fn promicro_up_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: promicro
    params:
      orientation: up
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: promicro_up.yaml
"#;

    parity_case("promicro_up", template_yaml, spec_yaml);
}
