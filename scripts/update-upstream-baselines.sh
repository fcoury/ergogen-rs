#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <commit> <fixture-yaml> [fixture-name]" >&2
  exit 1
fi

commit="$1"
fixture="$2"
fixture_name="${3:-$(basename "${fixture%.*}")}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_root="${repo_root}/fixtures/upstream-baselines/${commit}/${fixture_name}"

mkdir -p "${out_root}"

if [[ -n "${ERGOGEN_UPSTREAM_DIR:-}" ]]; then
  if [[ -d "${ERGOGEN_UPSTREAM_DIR}" ]]; then
    git -C "${ERGOGEN_UPSTREAM_DIR}" checkout "${commit}" >/dev/null
  fi
  if [[ -f "${ERGOGEN_UPSTREAM_DIR}/bin/ergogen.js" ]]; then
    cmd=(node "${ERGOGEN_UPSTREAM_DIR}/bin/ergogen.js")
  elif [[ -f "${ERGOGEN_UPSTREAM_DIR}/src/cli.js" ]]; then
    cmd=(node "${ERGOGEN_UPSTREAM_DIR}/src/cli.js")
  else
    echo "Could not find ergogen CLI under ERGOGEN_UPSTREAM_DIR=${ERGOGEN_UPSTREAM_DIR}" >&2
    exit 1
  fi
else
  cmd=(bash -lc "${ERGOGEN_UPSTREAM_CMD:-npx -y ergogen}")
fi

"${cmd[@]}" "${fixture}" -o "${out_root}" --clean

cat > "${out_root}/MANIFEST.txt" <<EOF
commit: ${commit}
fixture: ${fixture}
fixture_name: ${fixture_name}
command: ${cmd[*]}
generated_at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
repo_root: ${repo_root}
EOF

echo "Wrote upstream baselines to ${out_root}"
