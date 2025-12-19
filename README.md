# Ergogen‑RS

## CI: Upstream JS Footprint Parity

We run a small JS footprint parity check against upstream Ergogen in CI to catch regressions. The workflow is in `/.github/workflows/js-footprints-upstream.yml` and runs on PRs and nightly. It generates upstream outputs with `npx ergogen@4.1.0`, then runs the env‑gated test `js_footprints_upstream` against those outputs.

There’s a provision to gate this job on a PR label (e.g. `run-upstream-js`) but it’s currently commented out in the workflow until we measure stability/latency. Uncomment the job‑level `if:` line in that workflow to enable label‑based gating.
