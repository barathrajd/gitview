#!/bin/sh
# Create a disposable feature playground and launch Gitview inside it.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
source_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
playground=$("$script_dir/create-feature-playground.sh")

printf 'Opening Gitview playground: %s\n' "$playground" >&2
cd "$playground/repo"
# GITVIEW_BIN runs a prebuilt binary, such as the optimized build.
if [ -n "${GITVIEW_BIN:-}" ]; then
  XDG_CONFIG_HOME="$playground/config" exec "$GITVIEW_BIN" "$@"
fi
XDG_CONFIG_HOME="$playground/config" \
  cargo run --manifest-path "$source_dir/Cargo.toml" --bin gitview -- "$@"
