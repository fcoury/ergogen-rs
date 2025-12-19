# Scripts

## update-upstream-baselines.sh

Generate JS Ergogen baselines for fixture comparison.

Usage:

```
./scripts/update-upstream-baselines.sh <commit> <fixture-yaml> [fixture-name]
```

Examples:

```
./scripts/update-upstream-baselines.sh 8286e18f fixtures/upstream/fixtures/big.yaml big
./scripts/update-upstream-baselines.sh 8286e18f fixtures/upstream/fixtures/minimal.yaml minimal
```

Notes:
- To pin a local clone, set `ERGOGEN_UPSTREAM_DIR=/path/to/ergogen`.
- To override the command, set `ERGOGEN_UPSTREAM_CMD='npx -y ergogen'` (or another CLI command).
- Outputs land in `fixtures/upstream-baselines/<commit>/<fixture-name>/` with a `MANIFEST.txt`.
