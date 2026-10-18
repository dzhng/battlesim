---
name: ratchet-files
description: Ratchet source file size so every file stays small enough for an agent to read whole: find every file over a token budget, split oversized files by concept, and set up the pre-commit ratchet that stops them from growing. Use when a commit is blocked because a file is over the size budget, when asked to find or split oversized/long/god files, when a file you need to change is too large to read in one pass, or when asked to add a file-size limit or hook to a repository.
---

# Ratchet Files

A source file is over budget when it is past what an agent can read in one
pass (8,000 tokens, counted as bytes / 4, unless the repository sets
another). [`scripts/check-file-size.mjs`](scripts/check-file-size.mjs) owns
what counts as source, the budget, the ratchet, and the split rules; run it
with `--help` before anything else.

## Splitting

1. Run the script with `--all` from the repository root, passing the
   repository's `--max-tokens` if its hook sets one. Its list is the backlog.
2. Pick the files to split: the one blocking the work in front of you, or
   with no blocker, the largest ones a change is about to touch.
3. Split each one by the rules the script prints. Use
   [refactor-clean](../refactor-clean/SKILL.md) to find the seams: a seam is
   a concept the file already owns, not a line count.
4. Prove the split changed no behaviour with the repository's own checks,
   and commit it alone, before the change that needed the room.

Done when every file you picked is under budget in its own commit, the
behaviour checks pass, and `--all` no longer lists those files.

## Setting up the ratchet

Never install the hook without the user's explicit yes, even when they asked
to set up the repository. Installing it freezes every file already over
budget, so first run `--all` and tell the user how many files it would
freeze and which ones are largest, then ask.

Once they agree, set up everything the check needs, as the script's `--help`
describes:

- Add `linguist-languages` as a pinned dev dependency, with the repository's
  own package manager.
- Commit the script into the repository, unless this skill already lives
  there; the hook runs the repository's copy.
- Wire it into the existing pre-commit runner (lint-staged, or whatever the
  repository already uses) on every staged file, with `bun` or `node`,
  whichever the repository runs its tools with. If the repository has no
  pre-commit runner, say which one you would add before adding it.
- Mark generated and vendored code `linguist-generated` or
  `linguist-vendored` in `.gitattributes` instead of adding skip lists.
- Document the budget where the repository documents its other hooks.

Done when a commit of an over-budget new file is refused by the installed
hook.
