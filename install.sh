#!/usr/bin/env bash
# kendex CLI installer. Downloads the prebuilt `kendex` binary for this
# machine from the latest GitHub release and puts it on your PATH.
#
#   curl -fsSL https://kendex.ai/install.sh | sh
#
# The desktop app is a separate download: https://kendex.ai/download
set -euo pipefail

repo="vanillagreencom/kendex"
version="latest"

while [ $# -gt 0 ]; do
  case "$1" in
    --version) version="${2:?missing version after --version}"; shift 2 ;;
    -h|--help)
      echo "Usage: install.sh [--version vX.Y.Z]"
      exit 0
      ;;
    *) echo "install.sh: unknown option: $1" >&2; exit 2 ;;
  esac
done

for cmd in curl install; do
  command -v "$cmd" >/dev/null || { echo "install.sh: missing required command: $cmd" >&2; exit 1; }
done

os="$(uname -s)"
arch="$(uname -m)"
case "$os-$arch" in
  Linux-x86_64|Linux-amd64)   target="x86_64-unknown-linux-gnu" ;;
  Darwin-arm64|Darwin-aarch64) target="aarch64-apple-darwin" ;;
  Darwin-x86_64)
    echo "install.sh: no prebuilt binary for Intel macOS yet." >&2
    echo "  Use Homebrew instead: brew install vanillagreencom/kendex/kendex" >&2
    exit 1 ;;
  *)
    echo "install.sh: unsupported platform: $os $arch" >&2
    echo "  See https://kendex.ai/download for the desktop app, or build from source." >&2
    exit 1 ;;
esac

if [ "$version" = latest ]; then
  version="$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
fi
[ -n "$version" ] || { echo "install.sh: could not resolve the latest release" >&2; exit 1; }

asset="kendex-$target"
url="https://github.com/$repo/releases/download/$version/$asset"

# Pick a bin dir already on PATH; prefer a writable user dir over sudo.
bindir=""
for candidate in "$HOME/.local/bin" "/usr/local/bin"; do
  case ":$PATH:" in *":$candidate:"*) bindir="$candidate"; break ;; esac
done
[ -n "$bindir" ] || bindir="$HOME/.local/bin"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
echo "Downloading kendex $version ($target)…"
curl -fSL --proto '=https' -o "$tmp/kendex" "$url"
chmod +x "$tmp/kendex"

if [ -w "$bindir" ] || mkdir -p "$bindir" 2>/dev/null; then
  install -m 0755 "$tmp/kendex" "$bindir/kendex"
else
  echo "Installing to $bindir needs elevated permissions."
  sudo install -D -m 0755 "$tmp/kendex" "$bindir/kendex"
fi

echo "Installed kendex to $bindir/kendex"
case ":$PATH:" in
  *":$bindir:"*) ;;
  *) echo "Note: $bindir is not on your PATH. Add it, e.g.:"
     echo "  echo 'export PATH=\"$bindir:\$PATH\"' >> ~/.profile" ;;
esac
"$bindir/kendex" --version || true
