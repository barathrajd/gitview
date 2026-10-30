#!/usr/bin/env bash
# Prints the Homebrew formula for a release, filled in from its tag and the
# `.sha256` files of its two archives.
#
# Usage: scripts/render-homebrew-formula.sh <tag> <macos.sha256> <linux.sha256>
set -euo pipefail

fail() {
  printf 'render-homebrew-formula: %s\n' "$1" >&2
  exit 1
}

(($# == 3)) || fail "usage: render-homebrew-formula.sh <tag> <macos.sha256> <linux.sha256>"
tag="$1"
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "tag must look like v1.2.3, not '$tag'"

# Reads "<sha256>  <archive>" and checks it is for the expected archive.
digest_for() {
  local file="$1" archive="$2" digest name
  [[ -r "$file" ]] || fail "missing checksum file: $file"
  read -r digest name <"$file" || true
  [[ "$name" == "$archive" ]] || fail "$file is for '$name', expected $archive"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || fail "$file doesn't hold a SHA-256 digest"
  printf '%s' "$digest"
}

macos="$(digest_for "$2" "gitview-${tag}-macos-arm64.tar.gz")"
linux="$(digest_for "$3" "gitview-${tag}-linux-x86_64.tar.gz")"
template="$(dirname "$0")/../packaging/homebrew/gitview.rb.in"

sed -e "s/@TAG@/${tag}/g" \
  -e "s/@MACOS_SHA256@/${macos}/" \
  -e "s/@LINUX_SHA256@/${linux}/" \
  "$template"
