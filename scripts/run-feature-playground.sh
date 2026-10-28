#!/bin/sh
# Create a disposable feature playground and launch Gitview inside it.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
source_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
playground=$("$script_dir/create-feature-playground.sh")

printf 'Opening Gitview playground: %s\n' "$playground" >&2
cd "$playground/repo"
XDG_CONFIG_HOME="$playground/config" \
  cargo run --manifest-path "$source_dir/Cargo.toml" --bin gitview
