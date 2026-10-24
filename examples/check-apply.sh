#!/usr/bin/env bash
# Checks that every scenario's change.diff applies cleanly to its project's base files.
# Each project is copied to a temporary git repository and `git apply --check` runs there.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

total=0
failed=0
for project in "$here"/*/; do
  project="${project%/}"
  [[ -d "$project/scenarios" ]] || continue
  name="$(basename "$project")"
  work="$tmp/$name"
  mkdir -p "$work"
  # Base files only: scenarios and generated comments stay out of the copy.
  (cd "$project" && tar --exclude=./scenarios -cf - .) | (cd "$work" && tar -xf -)
  git -C "$work" init -q
  for scenario in "$project"/scenarios/*/; do
    scenario="${scenario%/}"
    total=$((total + 1))
    if out="$(git -C "$work" apply --check "$scenario/change.diff" 2>&1)"; then
      echo "ok    $name/$(basename "$scenario")"
    else
      failed=$((failed + 1))
      echo "FAIL  $name/$(basename "$scenario")"
      sed 's/^/      /' <<<"$out"
    fi
  done
done

echo
echo "$((total - failed))/$total diffs apply cleanly"
[[ $failed -eq 0 ]]
