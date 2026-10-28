#!/usr/bin/env python3
"""Check the required coverage and status fields in the capability matrix."""

from pathlib import Path


matrix = Path("docs/git-cli-capability-matrix.md")
assert matrix.is_file(), "capability matrix is missing"
text = matrix.read_text()

for required in (
    "Git 2.38",
    "| Tier | Workflow | Git commands | Current status | Safety | Acceptance evidence |",
    "Repository",
    "Worktree",
    "Commit",
    "History",
    "Branches and tags",
    "Remotes",
    "Merge workbench",
    "Diff and inspection",
    "Medium",
    "Advanced",
    "Not implemented",
    "— in progress",
    "— deferred",
    "Excluded",
    "tests/worktree.rs",
):
    assert required in text, f"capability matrix is missing: {required}"

tiers = {"Basic", "Medium", "Advanced", "Excluded"}
seen_tiers = set()
for line in text.splitlines():
    if not line.startswith("| ") or line.startswith("| Tier"):
        continue
    cells = [cell.strip() for cell in line.strip("|").split("|")]
    if cells[0] not in tiers:
        continue
    seen_tiers.add(cells[0])
    status = cells[3]
    assert any(
        marker in status
        for marker in (
            "— in progress",
            "— deferred",
            "Excluded",
        )
    ), f"workflow has no roadmap status: {cells[1]}"

assert seen_tiers == tiers, f"missing capability tiers: {tiers - seen_tiers}"

print("Capability matrix coverage fields are present.")
