#!/usr/bin/env bash
# Renders the Homebrew formula for a release from its SHA256SUMS.
#
#   scripts/homebrew-formula.sh <version> <SHA256SUMS> > pr-guard.rb
set -euo pipefail

version="${1#v}"
sums="$2"
formula="$(cat "$(dirname "$0")/../integrations/homebrew/pr-guard.rb.tmpl")"
formula="${formula//@VERSION@/$version}"
for target in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu; do
  sha="$(awk -v f="pr-guard-$target.tar.gz" '$2 == f || $2 == "*" f { print $1 }' "$sums")"
  [[ -n "$sha" ]] || { echo "homebrew-formula: no pr-guard-$target.tar.gz in $sums" >&2; exit 1; }
  formula="${formula//@SHA_$target@/$sha}"
done
printf '%s\n' "$formula"
