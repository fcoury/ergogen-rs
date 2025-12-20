# Knuckles parity (assets-level)

This fixture exercises **real-world JS footprints** from the bundled Knuckles assets and compares:

- Rust outputs vs **committed goldens** (always)
- Rust outputs vs **upstream JS ergogen** (optional, env-gated)

## What it covers

- `outlines/pcb.dxf` (semantic DXF compare)
- `pcbs/pcb.kicad_pcb` (normalized text compare)

The footprint sources come from `knuckles/ergogen/footprints` in this repo.

## Run locally

Generate goldens:

```bash
cd ergogen-rs
UPDATE_GOLDENS=1 cargo test -p ergogen-pcb --features js-footprints --test knuckles_parity
```

Run against committed goldens:

```bash
cd ergogen-rs
cargo test -p ergogen-pcb --features js-footprints --test knuckles_parity
```

## Upstream baseline (optional)

Generate upstream outputs for this fixture:

```bash
cd ergogen-rs
./scripts/update-upstream-baselines.sh 8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37 fixtures/m8/knuckles/knuckles_assets.yaml knuckles_assets
```

Then compare Rust outputs against those upstream outputs:

```bash
cd ergogen-rs
UPSTREAM_BASELINE_DIR=fixtures/upstream-baselines/8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37/knuckles_assets \
  cargo test -p ergogen-pcb --features js-footprints --test knuckles_parity
```

