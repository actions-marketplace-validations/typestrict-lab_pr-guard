#!/usr/bin/env bash
# Packs a built pr-guard into a self-contained release archive and writes its SHA-256.
#
#   scripts/package.sh <target> <out dir> [extra file next to the binary]...
#
# The archive holds, at its root, the binary, the native libraries it loads at start (listed after the out dir), LICENSE
# and README.md. Same name on every platform (pr-guard-<target>.tar.gz); Windows also gets a .zip.
set -euo pipefail
export COPYFILE_DISABLE=1 # no AppleDouble files from macOS tar

target="$1"
out="$(mkdir -p "$2" && cd "$2" && pwd)"
shift 2

exe=pr-guard
[[ "$target" == *windows* ]] && exe=pr-guard.exe

name="pr-guard-$target"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

cp "target/$target/release/$exe" "$stage/"
for f in "$@"; do cp "$f" "$stage/"; done
cp LICENSE README.md "$stage/"

tar -czf "$out/$name.tar.gz" -C "$stage" .
if [[ "$target" == *windows* ]]; then
  zip="$out/$name.zip"
  command -v cygpath >/dev/null 2>&1 && zip="$(cygpath -w "$zip")" # 7z is a native Windows program
  (cd "$stage" && 7z a -tzip -bd "$zip" . >/dev/null)
fi

cd "$out"
for a in "$name".*; do
  [[ "$a" == *.sha256 ]] && continue
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$a" > "$a.sha256"
  else shasum -a 256 "$a" > "$a.sha256"; fi
  cat "$a.sha256"
done
