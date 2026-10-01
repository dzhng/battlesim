# Working in this repo

Read [`README.md`](README.md) first: what the game is, how the repo fits together, and how to run and check it. Active plans live in `specs/<feature>/README.md`, and their "Next Agent Prompt" says what to do next. If a folder you're working in has a `README.md`, read it before continuing. The readmes are written for you.

These are the principles. Commands, flags and paths live with the code that owns them: the readmes, `package.json`, and each tool's own usage text.

## Talking to the user

The user is very technical but doesn't read the code day to day. Pointing at code is fine; introduce a variable, function or module briefly the first time you mention it.

Lead with contracts. When work touches an interface between components (a command, an observation or publication layout, a digest, a fixture schema, a module boundary), say what the contract looks like and how it changed before anything else.

## Provenance

Every reused byte has a recorded source. Code copied from another project and every art source, ours or third-party, is entered in [`reuse-manifest.json`](reuse-manifest.json) with its content hash, an allowed licence and the user's acceptance; the asset validator refuses anything without an entry.

A project we copy from is read-only: never edit it and never import from it at runtime.

Reference images are for judging our work. They never ship.

## Proving a change

Write the test first. Before changing behaviour or fixing a bug, invoke [`write-tests`](.agents/skills/write-tests/SKILL.md) and follow its red/green workflow.

Run the narrowest check that answers your question: one test, then one file, then one crate or scene. The full gates are a closeout, run once at the end of a pass and before a merge, not a feedback loop.

The simulation is deterministic, and the battle digest is the proof. Two runs are the same battle exactly when their digests match. A change that shouldn't alter outcomes (a refactor, a performance change) must leave digests and replays unchanged, or be a named decision.

Play only the battles a change can move. Rendering, UI, sound and docs need none. A rule change needs a quick sample while iterating. The full balance report runs once, at closeout.

Measure cost in units that don't move with machine load, such as instructions retired, not wall time.

Don't wait on a long run. Start it in the background and keep working.

## What the player sees

Look at the picture. A passing check is not evidence that something reads well on screen.

For any visual change:
- get an unprimed second opinion with [`screenshot-critique`](.agents/skills/screenshot-critique/SKILL.md);
- judge before against after, and our shots against references, with [`compare-screenshots`](.agents/skills/compare-screenshots/SKILL.md);
- show the user with [`preview-shots`](.agents/skills/preview-shots/SKILL.md).

Before adding or changing anything the player sees (markers, callouts, HUD, colours), invoke [`game-ui`](.agents/skills/game-ui/SKILL.md). Before renderer work, load [`renderer`](.agents/skills/renderer/SKILL.md).

## Game rules

Before proposing or changing a game mechanic, invoke [`tweak-mechanics`](.agents/skills/tweak-mechanics/SKILL.md). Rules come from physical properties, not named special cases, and are judged by picturing the moment on the battlefield.

## One owner per concept

Use the library the repo already chose before writing your own. For TypeScript vectors, matrices, shapes, noise and seeded randomness that is the [`math`](https://github.com/pmndrs/math) package; load the [`math`](.agents/skills/math/SKILL.md) skill before writing any.

Prefer one general rule to a special case, and data to constants scattered through code.

## Parallel work stays cheap

Every worktree is a full checkout, and large files, installed dependencies and build output multiply with each one.

- Fetch only the large files your task needs. They are in Git LFS and are not downloaded on checkout.
- Share installed dependencies with the main checkout. Don't install again.
- Never share build output between worktrees whose sources differ. They overwrite each other's builds, and the symptom is an error from someone else's change.
- Remove a worktree and its build output when its branch is merged.

Scratch output (evidence, logs, candidate renders) goes in gitignored `throwaway/`, never in a spec or the source tree.

## Skills

Repo skills live in `.agents/skills/<name>/`. Keep them current: when a pass learns a lesson (a gotcha, a pattern that paid off, a rejected approach), add it to the owning skill in the same commit.
