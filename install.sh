#!/bin/sh
# Installs pr-guard from its GitHub releases:
#
#   curl -fsSL https://raw.githubusercontent.com/typestrict-lab/pr-guard/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/typestrict-lab/pr-guard/main/install.sh | sh -s -- 0.1.0 --dir /usr/local/bin
#
# Arguments: an optional version (default the latest release) and `--dir <dir>` (default ~/.local/bin). The same through
# the environment: PR_GUARD_VERSION, PR_GUARD_INSTALL_DIR. PR_GUARD_DOWNLOAD_URL replaces the release download base
# (a mirror); the archive's SHA-256 is checked against the .sha256 published next to it before anything is installed.
set -eu

repo="typestrict-lab/pr-guard"
version="${PR_GUARD_VERSION:-latest}"
dir="${PR_GUARD_INSTALL_DIR:-$HOME/.local/bin}"

fail() { echo "pr-guard: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --dir) [ $# -ge 2 ] || fail "--dir needs a directory"; dir="$2"; shift 2 ;;
    --dir=*) dir="${1#--dir=}"; shift ;;
    -h | --help) sed -n '2,10p' "$0" 2>/dev/null || true; exit 0 ;;
    -*) fail "unknown option $1" ;;
    *) version="$1"; shift ;;
  esac
done

exe=pr-guard
case "$(uname -s)" in
  Linux)
    os=unknown-linux-gnu
    # The builds link glibc; ONNX Runtime has no musl build to link against.
    if ldd --version 2>&1 | grep -qi musl || [ -f /etc/alpine-release ]; then
      fail "no musl (Alpine) build: use a glibc image or ghcr.io/$repo"
    fi
    ;;
  Darwin) os=apple-darwin ;;
  MINGW* | MSYS* | CYGWIN*) os=pc-windows-msvc; exe=pr-guard.exe ;;
  *) fail "no build for $(uname -s)" ;;
esac
case "$(uname -m)" in
  x86_64 | amd64) arch=x86_64 ;;
  arm64 | aarch64) arch=aarch64 ;;
  *) fail "no build for $(uname -m)" ;;
esac
# A shell under Rosetta reports x86_64 on Apple silicon; the native build is faster.
if [ "$os" = apple-darwin ] && [ "$arch" = x86_64 ] && [ "$(sysctl -n sysctl.proc_translated 2>/dev/null)" = 1 ]; then
  arch=aarch64
fi
case "$arch-$os" in
  aarch64-pc-windows-msvc) fail "no build for Windows on ARM yet" ;;
esac
target="$arch-$os"

case "$version" in
  latest) base="https://github.com/$repo/releases/latest/download" ;;
  *) version="${version#v}"; base="https://github.com/$repo/releases/download/v$version" ;;
esac
base="${PR_GUARD_DOWNLOAD_URL:-$base}"
archive="pr-guard-$target.tar.gz"

command -v curl >/dev/null 2>&1 || fail "needs curl"
command -v tar >/dev/null 2>&1 || fail "needs tar"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

echo "Downloading pr-guard ($version) for $target..."
curl -fsSL --retry 3 -o "$tmp/$archive" "$base/$archive" || fail "could not download $base/$archive"
curl -fsSL --retry 3 -o "$tmp/$archive.sha256" "$base/$archive.sha256" || fail "could not download $base/$archive.sha256"

expected="$(cut -d ' ' -f 1 < "$tmp/$archive.sha256")"
if command -v sha256sum >/dev/null 2>&1; then actual="$(sha256sum "$tmp/$archive" | cut -d ' ' -f 1)"
elif command -v shasum >/dev/null 2>&1; then actual="$(shasum -a 256 "$tmp/$archive" | cut -d ' ' -f 1)"
else fail "needs sha256sum or shasum to check the download"; fi
[ -n "$expected" ] && [ "$expected" = "$actual" ] || fail "checksum mismatch ($actual, expected $expected), nothing installed"

mkdir -p "$tmp/x"
tar -xzf "$tmp/$archive" -C "$tmp/x"
[ -f "$tmp/x/$exe" ] || fail "the archive has no $exe"
mkdir -p "$dir"
# The binary and the native libraries it loads from its own directory (macOS x64: libonnxruntime, Windows: DirectML.dll).
for f in "$tmp/x/"*; do
  case "$(basename "$f")" in
    LICENSE | README.md) ;;
    *) cp "$f" "$dir/" ;;
  esac
done
chmod +x "$dir/$exe"

echo "Installed $("$dir/$exe" --version) to $dir/$exe"
case ":$PATH:" in
  *":$dir:"*) ;;
  *) echo "Add it to your PATH: export PATH=\"$dir:\$PATH\"" ;;
esac
echo "The model (~165 MB) downloads on the first check; --no-model runs only the deterministic checks."
