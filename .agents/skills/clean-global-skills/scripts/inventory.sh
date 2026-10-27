#!/usr/bin/env bash
# inventory.sh — READ-ONLY audit of installed agent skills. Changes nothing.
#
# Usage: bash inventory.sh [project-dir] [extra-skills-dir ...]
#
# Lists every skill found in the global and project skill folders of Claude Code,
# Codex, OpenCode and Gemini CLI, with the size of its description (the part an
# agent loads on every run). Then reports name clashes and the cost per scope.
# Folders that are symlinks to the same place are counted once.

set -u
proj="${1:-$PWD}"
[ "$#" -ge 1 ] && shift
proj="$(cd "$proj" 2>/dev/null && pwd -P)" || { echo "ERROR: no such project directory"; exit 1; }

tsv="$(mktemp)"; trap 'rm -f "$tsv"' EXIT

scan() { # scope root
  local scope="$1" root="$2" d name len real link
  [ -d "$root" ] || return 0
  for d in "$root"/*/; do
    [ -f "${d}SKILL.md" ] || continue
    d="${d%/}"
    name="$(basename "$d")"
    real="$(cd "$d" && pwd -P)"
    link="no"; [ -L "$d" ] && link="symlink"
    len="$(awk '
      NR==1 && $0!="---" { exit }
      NR>1 && $0=="---" { exit }
      /^description:/ { on=1; sub(/^description:[ \t]*/, ""); sub(/^[>|][-+]?[ \t]*/, ""); n+=length($0); next }
      on && /^[A-Za-z_-]+:/ { on=0 }
      on { sub(/^[ \t]+/, ""); n+=length($0)+1 }
      END { print n+0 }
    ' "${d}/SKILL.md")"
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$scope" "$root" "$name" "$len" "$link" "$real" >> "$tsv"
  done
}

for r in "$HOME/.claude/skills" "$HOME/.agents/skills" "$HOME/.config/opencode/skills" "$HOME/.gemini/skills"; do scan global "$r"; done
for r in "$proj/.claude/skills" "$proj/.agents/skills" "$proj/.opencode/skills" "$proj/.gemini/skills"; do scan project "$r"; done
for r in "$@"; do scan extra "$r"; done

if [ ! -s "$tsv" ]; then echo "No skills found."; exit 0; fi

echo "== Skills (scope, folder, name, description chars, link, real path) =="
sort -t$'\t' -k1,1 -k3,3 "$tsv" | awk -F'\t' '{ printf "%-8s %-44s %-34s %5s  %-7s %s\n", $1, $2, $3, $4, $5, $6 }'

echo
echo "== Cost: unique skills per scope (descriptions load on every run; ~4 chars per token) =="
awk -F'\t' '!seen[$1 FS $6]++ { c[$1]++; t[$1]+=$4 }
  END { for (s in c) printf "%-8s %3d skills  %6d chars  ~%d tokens\n", s, c[s], t[s], t[s]/4 }' "$tsv" | sort

echo
echo "== Clashes: the same skill name at different real paths =="
awk -F'\t' '!seen[$3 FS $6]++ { n[$3]++; where[$3]=where[$3] "\n    " $1 ": " $6 }
  END { found=0; for (k in n) if (n[k] > 1) { found=1; print k where[k] } if (!found) print "none" }' "$tsv"

echo
echo "== Global skills that also exist in the project (safe to drop from global once the copy is identical) =="
awk -F'\t' 'NR==FNR { if ($1=="project") p[$3]=1; next } $1=="global" && ($3 in p) && !seen[$3]++ { print $3; f=1 } END { if (!f) print "none" }' "$tsv" "$tsv"
