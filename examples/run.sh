#!/usr/bin/env bash
# Runs pr-guard on every scenario under examples/ and prints what it decided next to what we expect.
#
#   examples/run.sh                         # builds and runs with cargo
#   PR_GUARD=target/release/pr-guard examples/run.sh
#   examples/run.sh ruby-blog               # only the scenarios of one project
#
# Exit code 1 from pr-guard means "blocked by the policy", which is a valid outcome here,
# so the loop keeps going. The sticky comment of each run lands in <scenario>/comment.md.
#
# The policy path must not exist so pr-guard falls back to its default policy. pr-guard only
# treats "file not found" as missing; a path like /dev/null/none fails with "not a directory",
# so the default is a file name inside a fresh empty temp dir. Set PR_GUARD_POLICY to try a real one.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
cd "$root"

read -r -a pr_guard <<<"${PR_GUARD:-cargo run --release --quiet --}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
policy="${PR_GUARD_POLICY:-$tmp/no-policy.yml}"
filter="${1:-}"

passed=0
blocked=0
errors=0
for scenario in "$here"/*/scenarios/*/; do
  scenario="${scenario%/}"
  rel="${scenario#"$root"/}"
  project="$(basename "$(dirname "$(dirname "$scenario")")")"
  [[ -z "$filter" || "$project" == "$filter" ]] || continue

  echo "=== $project / $(basename "$scenario")"
  echo "    title:    $(head -n 1 "$scenario/description.md")"
  echo "    expected: $(sed -n 's/^note: //p' "$scenario/expected.yaml" | sed 's/^"//; s/"$//; s/\\"/"/g')"

  status=0
  stderr="$("${pr_guard[@]}" check \
    --diff "$rel/change.diff" \
    --description-file "$rel/description.md" \
    --policy "$policy" \
    --comment "$rel/comment.md" 2>&1 >/dev/null)" || status=$?

  grep '^pr-guard:' <<<"$stderr" | sed 's/^/    /' || true
  case $status in
    0) passed=$((passed + 1)); echo "    result:   passed" ;;
    1) blocked=$((blocked + 1)); echo "    result:   blocked" ;;
    *) errors=$((errors + 1)); echo "    result:   error (exit $status)"; grep -v '^pr-guard:' <<<"$stderr" | sed 's/^/    /' || true ;;
  esac
  echo
done

echo "passed: $passed  blocked: $blocked  errors: $errors"
