---
name: clean-global-skills
description: Audits globally installed agent skills, finds ones that clash with a project's skills or waste tokens, and moves them out of global with a backup so each project carries only its own skills. Use when the user says "clean up global skills", "skills are clashing", "too many skills loaded", or "skills burning tokens".
---

# Clean Global Skills

## Overview
Every installed skill puts its name and description into the agent's context on every run, whether or not the task needs it. Skills left in the global folders load in every project, and a global skill can share a name with a project skill, so the agent may load the wrong one. This skill measures that, then moves unwanted or project-specific skills out of global with a backup. It keeps the project folder as the place for stack-specific skills. It does not delete anything, and it does not touch plugin-managed skills.

## When to Use
- Skills from one project show up in another, or a global skill clashes with a project skill of the same name.
- The agent's context looks heavy before any work starts.
- The user says "clean up global skills", "skills clashing", or "move skills to the project".

## Process

### 1. Measure first
Run `bash <this-skill-folder>/scripts/inventory.sh <project-dir>`. It is read-only. It lists every skill in the global and project folders of Claude Code, Codex, OpenCode and Gemini CLI, the size of each description, the cost per scope, and the clashes. Symlinks that point to the same folder count once.
- **Exit criterion:** you can state how many skills and how many description characters are global, and which names clash.

### 2. Classify with the user
Put each global skill in one of three groups and show the list. The user decides; the agent proposes.
- **Keep global:** small, general, and wanted in every repo (writing, review, a router the user uses everywhere).
- **Move to a project:** tied to a stack or product (React, TanStack, Base UI, StyleX, Postgres, a toolchain). It belongs in the repos that use it.
- **Remove:** unused, duplicated, or from a source nobody can name.

For skills that clash, compare the two copies (`diff -r` on the real paths from step 1) before choosing. If the copies differ, say which is newer.
- **Exit criterion:** the user has approved a keep / move / remove list. Without approval, stop here.

### 3. Back up, then move out of global
For each skill to leave global, move the folder (or only the symlink, if it is one) into `~/.skills-backup/<date-time>/<original-folder-name>/`. Do not delete. Write a one-line-per-skill manifest next to the backup with the original path. Cover the global folders of every agent the inventory showed, not only one.
- Skills installed by the skills CLI also appear in its lock file; use the CLI's remove command, pinned to an exact version, when the user prefers that, and still keep the backup.
- **Exit criterion:** every skill on the move/remove list is gone from the global folders and present in the backup, with a manifest.

### 4. Install project skills at the project
For each "move to a project" skill, install it into the project root, project scope only, never global. Use the source the project already records (`skills-lock.json`, or the pinned manifest of external skills in the agent-skills pack), pinned to the recorded version or commit, and only after the user agrees. Install into `.agents/skills/` and link `.claude/skills/` to it, so each agent finds the same copy. If the project already has it, keep that copy and skip.
- **Exit criterion:** each moved skill is in the project folder and is named in the project's lock file or manifest.

### 5. Re-measure and report
Run the inventory again, in the project and in a second project if one is handy. Report global before and after (skills, characters), the clashes resolved, and where the backup is. Tell the user to start a new agent session, because skills are read at start.
- **Exit criterion:** no name appears in both global and the project, and the global total is lower than step 1.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "Just delete the duplicates." | A global copy can be the only copy, or the newer one. Back up, then move. |
| "I know which ones are unused." | Without the user's list you guess. Show the inventory and get approval. |
| "Global is convenient, leave it." | Each global skill costs tokens in every project, including ones that never use it. |
| "Clean one agent's folder." | The same skill often sits in several agents' folders. Check all the folders the inventory lists. |
| "Reinstall the project skills from the latest version." | Reinstall from the recorded pin. An unpinned reinstall changes content nobody reviewed. |

## Red Flags — stop and fix
- Anything was deleted instead of moved to the backup.
- A skill was moved with no user approval.
- A project skill was installed globally, or without a recorded source and version.
- The plugin-managed skills were edited by hand. Those are managed by the agent's plugin commands; disable them there.
- The inventory was not re-run after the changes.

## Verification — done when
- The inventory after the cleanup shows no clashes and a lower global total than before.
- Every removed skill is in `~/.skills-backup/` with a manifest, so it can be restored.
- Each project-specific skill is installed in its project at a recorded version, not global.
- The user was told to restart the agent session.
