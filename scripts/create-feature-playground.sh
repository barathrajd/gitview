#!/bin/sh
# Build an isolated repository with representative data for Gitview's current TUI.
set -eu

# GITVIEW_PLAYGROUND_PARENT keeps playgrounds in one folder, such as
# target/playground, so they can be deleted together.
parent=${GITVIEW_PLAYGROUND_PARENT:-${TMPDIR:-/tmp}}
mkdir -p "$parent"
playground=$(mktemp -d "$parent/gitview-playground.XXXXXXXX")
repo="$playground/repo"
config="$playground/config/gitview"
mkdir -p "$repo" "$config/profiles"

git -C "$repo" init -q -b main
git -C "$repo" config user.name 'Gitview Playground'
git -C "$repo" config user.email 'gitview-playground@example.invalid'
git -C "$repo" config commit.gpgsign false

cat > "$repo/README.md" <<'EOF'
# Playground repository

Use this repository to try Gitview's history, branches, file review, staging,
commit, amend, discard, and folder-scoped profile workflows.
EOF
git -C "$repo" add -- README.md
git -C "$repo" commit -qm 'docs: add playground readme'

git -C "$repo" switch -qc demo-branch
cat > "$repo/branch-notes.txt" <<'EOF'
This branch exists to exercise branch browsing and the commit graph.
EOF
git -C "$repo" add -- branch-notes.txt
git -C "$repo" commit -qm 'feat: add a demo branch'

git -C "$repo" switch -q main
cat > "$repo/main-notes.txt" <<'EOF'
This commit creates a merge graph for history browsing.
EOF
git -C "$repo" add -- main-notes.txt
git -C "$repo" commit -qm 'feat: add main branch notes'
git -C "$repo" merge --no-ff -qm 'merge: include demo branch' demo-branch
git -C "$repo" tag playground-v1

# Enough history and changed files to exercise counts, page keys, gg/G, and zz.
mkdir -p "$repo/notes"
for number in $(seq -w 1 30); do
  printf 'Note %s\n' "$number" > "$repo/notes/note-$number.txt"
done
git -C "$repo" add -- notes
git -C "$repo" commit -qm 'docs: add thirty notes in one commit'
for number in $(seq -w 1 30); do
  git -C "$repo" branch "topic-$number"
done
for number in $(seq -w 1 40); do
  printf 'Entry %s\n' "$number" >> "$repo/log.txt"
  git -C "$repo" add -- log.txt
  git -C "$repo" commit -qm "chore: add log entry $number"
  # Enough tags to page through the tag list (b, then t).
  if [ "$number" -le 30 ]; then
    git -C "$repo" tag "release-$number"
  fi
done

cat > "$repo/hunk-playground.txt" <<'EOF'
Hunk staging fixture
// staged baseline
context line 1
context line 2
context line 3
context line 4
context line 5
context line 6
context line 7
context line 8
// unstaged baseline
context line 10
context line 11
context line 12
EOF
git -C "$repo" add -- hunk-playground.txt
git -C "$repo" commit -qm 'test: add separated hunk fixture'

sed 's|// staged baseline|// staged hunk|' "$repo/hunk-playground.txt" \
  > "$repo/hunk-playground.tmp"
mv "$repo/hunk-playground.tmp" "$repo/hunk-playground.txt"
git -C "$repo" apply --cached <<'PATCH'
diff --git a/hunk-playground.txt b/hunk-playground.txt
--- a/hunk-playground.txt
+++ b/hunk-playground.txt
@@ -1,3 +1,3 @@
 Hunk staging fixture
-// staged baseline
+// staged hunk
 context line 1
PATCH
sed 's|// unstaged baseline|// unstaged hunk|' "$repo/hunk-playground.txt" \
  > "$repo/hunk-playground.tmp"
mv "$repo/hunk-playground.tmp" "$repo/hunk-playground.txt"

cat > "$repo/staged-playground.txt" <<'EOF'
This staged file is ready for the normal commit workflow.
EOF
git -C "$repo" add -- staged-playground.txt
cat > "$repo/unstaged-playground.txt" <<'EOF'
This unstaged file is ready for the discard confirmation workflow.
EOF
cat > "$repo/untracked playground.txt" <<'EOF'
This untracked file is ready for staging or discard confirmation.
EOF

# Agent sessions for the Sessions page (A): worktrees on gitview/<name>
# branches plus the manifest Gitview reads (see src/sessions.rs). The first
# session's long commit exercises review scrolling.
sessions="$repo/.git/gitview/sessions"
mkdir -p "$sessions"
# Git reports resolved paths, as on macOS where /tmp is /private/tmp.
sessions=$(cd "$sessions" && pwd -P)
base_commit=$(git -C "$repo" rev-parse HEAD)
manifest=''
for number in 1 2 3; do
  name="session-playground-$number"
  git -C "$repo" worktree add -q -b "gitview/$name" "$sessions/$name" "$base_commit"
  manifest="$manifest${manifest:+,}
    {\"name\": \"$name\", \"branch\": \"gitview/$name\", \"base_branch\": \"main\", \"base_commit\": \"$base_commit\", \"path\": \"$sessions/$name\"}"
done
seq -f 'Agent line %03g' 1 120 > "$sessions/session-playground-1/agent-long.txt"
git -C "$sessions/session-playground-1" add -- agent-long.txt
git -C "$sessions/session-playground-1" commit -qm 'feat: agent adds a long file'
printf '{"sessions": [%s\n]}\n' "$manifest" > "$sessions/manifest.json"

# Optional: many untracked files for Worktree scrolling.
if [ -n "${GITVIEW_PLAYGROUND_LARGE:-}" ]; then
  for number in $(seq -w 1 40); do
    printf 'new\n' > "$repo/untracked-$number.txt"
  done
fi

cat > "$config/profiles.json" <<EOF
{
  "profiles": [
    {
      "name": "Playground",
      "worktree_root": "$repo",
      "config_path": "profiles/playground.gitconfig",
      "enabled": true
    }
  ]
}
EOF
cat > "$config/profiles/playground.gitconfig" <<'EOF'
[user]
    name = Gitview Playground Profile
    email = gitview-playground-profile@example.invalid
EOF

printf '%s\n' "$playground"
