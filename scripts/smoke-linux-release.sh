#!/usr/bin/env bash
# Checks a Linux release binary before it is uploaded: statically linked, the
# expected version, working help, and a real run on Alpine (no glibc) where an
# agent in Gitview's pane reaches Gitview through `gitview mcp`.
#
# Usage: scripts/smoke-linux-release.sh <binary> <version>
# Set CONTAINER=podman to use Podman instead of Docker.
set -euo pipefail

binary="$1"
version="$2"
container="${CONTAINER:-docker}"
here="$(cd "$(dirname "$0")" && pwd)"

fail() {
  printf 'smoke test failed: %s\n' "$1" >&2
  exit 1
}

description="$(file -b "$binary")"
case "$description" in
  *"statically linked"* | *"static-pie linked"*) ;;
  *) fail "not statically linked: $description" ;;
esac

[[ "$("$binary" --version)" == "gitview $version" ]] ||
  fail "--version printed '$("$binary" --version)', expected 'gitview $version'"
"$binary" --help >/dev/null || fail "--help failed"

"$container" run --rm \
  -v "$(cd "$(dirname "$binary")" && pwd)/$(basename "$binary"):/usr/local/bin/gitview:ro" \
  -v "$here/smoke-linux-alpine.sh:/smoke.sh:ro" \
  docker.io/library/alpine:latest sh /smoke.sh ||
  fail "the Alpine run failed"

printf 'smoke test passed: %s\n' "$description"
