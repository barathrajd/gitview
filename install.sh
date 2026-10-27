#!/usr/bin/env bash
set -euo pipefail

readonly REPOSITORY="barathrajd/gitview"
readonly RELEASE_API="https://api.github.com/repos/${REPOSITORY}/releases/latest"
readonly BASE_RELEASE_URL="https://github.com/${REPOSITORY}/releases/download"

fail() {
  printf 'gitview installer: %s\n' "$1" >&2
  exit 1
}

usage() {
  cat <<'EOF'
Install Gitview from a GitHub release.

Usage: install.sh [tag] [--cargo|--prebuilt]

With no tag, installs the latest release. When interactive, choose a prebuilt
binary or install from crates.io with Cargo. For example:
  install.sh v0.4.1 --prebuilt
  install.sh --cargo

Set GITVIEW_INSTALL_DIR to change the prebuilt binary install directory. The
Cargo option installs to Cargo's configured bin directory, usually ~/.cargo/bin.
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi
tag="latest"
method="${GITVIEW_INSTALL_METHOD:-}"
tag_set=false
while (($#)); do
  case "$1" in
    --help|-h)
      usage
      exit 0
      ;;
    --cargo)
      method="cargo"
      ;;
    --prebuilt)
      method="prebuilt"
      ;;
    --tag)
      shift
      (($#)) || fail "--tag requires a version"
      tag="$1"
      tag_set=true
      ;;
    --tag=*)
      tag="${1#*=}"
      tag_set=true
      ;;
    -* )
      fail "unknown option: $1"
      ;;
    *)
      [[ "$tag_set" == false ]] || fail "provide only one release tag"
      tag="$1"
      tag_set=true
      ;;
  esac
  shift
done

if [[ -z "$method" && -r /dev/tty ]]; then
  printf 'Install Gitview:\n  1) Prebuilt macOS Apple Silicon binary (no compilation)\n  2) Build from crates.io with Cargo (requires Rust)\n'
  IFS= read -r -p 'Choose [1]: ' choice </dev/tty || choice="1"
  case "$choice" in
    ""|1) method="prebuilt" ;;
    2) method="cargo" ;;
    *) fail "choose 1 or 2" ;;
  esac
fi
method="${method:-prebuilt}"
case "$method" in
  prebuilt|cargo) ;;
  *) fail "install method must be 'prebuilt' or 'cargo'" ;;
esac
if [[ "$tag" != "latest" ]]; then
  [[ "$tag" != *[!A-Za-z0-9._-]* ]] || fail "invalid release tag: $tag"
fi

if [[ "$method" == "cargo" ]]; then
  command -v cargo >/dev/null 2>&1 || fail "Cargo is required for this option; install Rust first"
  printf 'Cargo installs require this version to be published on crates.io.\n'
  if [[ "$tag" == "latest" ]]; then
    cargo install gitview --locked
  else
    cargo_version="${tag#v}"
    cargo install gitview --version "$cargo_version" --locked
  fi
  exit $?
fi

case "$(uname -s):$(uname -m)" in
  Darwin:arm64|Darwin:aarch64) ;;
  *) fail "prebuilt releases currently support macOS Apple Silicon only; choose Cargo to build from source" ;;
esac

for tool in curl tar shasum install awk; do
  command -v "$tool" >/dev/null 2>&1 || fail "required command not found: $tool"
done

if [[ "$tag" == "latest" ]]; then
  printf 'Looking up the latest Gitview release...\n'
  tag="$(curl --fail --silent --show-error --location "$RELEASE_API" \
    | awk -F '"' '/"tag_name"[[:space:]]*:/ { print $4; exit }')"
fi
[[ -n "$tag" ]] || fail "could not determine the release tag"
[[ "$tag" != *[!A-Za-z0-9._-]* ]] || fail "invalid release tag: $tag"

archive="gitview-${tag}-macos-arm64.tar.gz"
release_url="${BASE_RELEASE_URL}/${tag}"
install_dir="${GITVIEW_INSTALL_DIR:-$HOME/.local/bin}"
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/gitview-install.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT

printf 'Downloading Gitview %s...\n' "$tag"
curl --fail --location --show-error --progress-bar \
  --output "$tmp_dir/$archive" "$release_url/$archive"
curl --fail --location --silent --show-error \
  --output "$tmp_dir/$archive.sha256" "$release_url/$archive.sha256"

printf 'Verifying SHA-256...\n'
(cd "$tmp_dir" && shasum -a 256 -c "$archive.sha256")

printf 'Extracting Gitview...\n'
tar -xzf "$tmp_dir/$archive" -C "$tmp_dir"
[[ -f "$tmp_dir/gitview" ]] || fail "release archive does not contain gitview"

mkdir -p "$install_dir"
install -m 0755 "$tmp_dir/gitview" "$install_dir/gitview"
printf 'Installed Gitview %s to %s/gitview\n' "$tag" "$install_dir"
case ":${PATH:-}:" in
  *":$install_dir:"*) ;;
  *) printf 'Add %s to your PATH to run gitview from any folder.\n' "$install_dir" ;;
esac
