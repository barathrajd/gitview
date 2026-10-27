#!/bin/sh
# Create an isolated sample repository without changing an existing repository.
set -eu
repo=$(mktemp -d "${TMPDIR:-/tmp}/gitview-demo.XXXXXXXX")
git init -q "$repo"
git -C "$repo" config user.name 'Gitview Demo'
git -C "$repo" config user.email 'gitview-demo@example.invalid'
git -C "$repo" config commit.gpgsign false
printf 'Initial content\n' > "$repo/readme.txt"
git -C "$repo" add -- readme.txt
git -C "$repo" commit -qm 'Add initial file'
git -C "$repo" switch -qc demo-branch
printf 'Branch content\n' > "$repo/branch.txt"
git -C "$repo" add -- branch.txt
git -C "$repo" commit -qm 'Add branch file'
git -C "$repo" switch -q -
printf 'Main content\n' > "$repo/main.txt"
git -C "$repo" add -- main.txt
git -C "$repo" commit -qm 'Add main file'
git -C "$repo" merge --no-ff -qm 'Merge demo branch' demo-branch
git -C "$repo" tag demo-v1
printf 'Staged content\n' >> "$repo/readme.txt"
git -C "$repo" add -- readme.txt
printf 'Unstaged content\n' >> "$repo/readme.txt"
printf 'Untracked content\n' > "$repo/untracked.txt"
printf '%s\n' "$repo"
