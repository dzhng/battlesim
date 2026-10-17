# Working in this repo

Read [`README.md`](README.md) first: what the game is, how the repo fits
together, and how to run it. Read [`development.md`](development.md) before
changing code: it owns the architecture, game-rule principles, validation
ladder, visual-work rules, and contributor workflow. Active plans live with
their specs, and each one says what to do next. If a folder you're working in
has a README, read it before continuing. The READMEs are written for you.

These are the principles. Commands, flags and paths live with the code that
owns them: the READMEs, the manifests, and each tool's own usage text.

## Talking to the user

The user is very technical but doesn't read the code day to day. Pointing at
code is fine; introduce a variable, function, or module briefly the first time
you mention it.

Lead with contracts. When work touches an interface between components—a
command, observation or publication layout, digest, fixture schema, or module
boundary—say what the contract looks like and how it changed before anything
else.

Answer routine questions from the evidence. Ask the user only when the answer
changes a decision that matters and the repository cannot settle it.

## Proving a change

Follow the check ladder in [`development.md`](development.md): optimize for the
fastest feedback you can trust, run the narrowest check that answers the
question, and run the full closeout checks once a spec's implementation is
finished.

Write the test first for behavior changes and regressions. Use
[`write-tests`](.agents/skills/write-tests/SKILL.md) and test what the game does
and how it fails, not how the code is shaped.

The simulation is deterministic, and the battle digest is the proof. A change
that should not alter outcomes must leave digests and replays unchanged, or be
a named decision.

Never loosen a requirement to make a check pass. Say what you verified, what
you assumed, and what is unfinished. Do not repeat an unchanged failure.

## What the player sees

Look at the picture. A passing check is not evidence that something reads well
on screen.

For any visual change:

- get an unprimed second opinion with
  [`screenshot-critique`](.agents/skills/screenshot-critique/SKILL.md);
- judge before against after, and our shots against references, with
  [`compare-screenshots`](.agents/skills/compare-screenshots/SKILL.md);
- show the user with [`preview-shots`](.agents/skills/preview-shots/SKILL.md).

Before adding or changing anything the player sees, use
[`game-ui`](.agents/skills/game-ui/SKILL.md). Before renderer work, use
[`renderer`](.agents/skills/renderer/SKILL.md).

## Game rules

Rules come from physical properties, not named special cases. Express a
restriction through the data each unit or weapon carries, never through a rule
that repeats the same restriction by role or label. The target is Hollywood
realism: keep what a viewer expects in a war film and drop what looks silly on
screen.

The simulation is the one authority; everything else observes it. Each side
sees only what its own units can see.

Before proposing or changing a game mechanic, use
[`tweak-mechanics`](.agents/skills/tweak-mechanics/SKILL.md). The detailed game
rules and their deliberate exceptions live in
[`development.md`](development.md#game-rules).

## One owner per concept

Use what the repo already chose before writing your own. Find the existing owner
of a concept before creating another. Prefer one general rule to a special
case, data to constants scattered through code, and a simple structure to an
abstraction nobody needs yet.

Before writing TypeScript vectors, matrices, shapes, noise, or seeded
randomness, load [`math`](.agents/skills/math/SKILL.md). When a change exposes a
duplicate or stale owner, use
[`refactor-clean`](.agents/skills/refactor-clean/SKILL.md).

## Parallel work stays cheap

Every parallel checkout is a full copy, and large files, installed dependencies,
and build output multiply with each one.

- Fetch only the large files your task needs.
- Share installed dependencies when safe; never share build output between
  checkouts whose sources differ.
- Remove a checkout and its build output when its branch is merged.
- Put scratch output—evidence, logs, probes, and candidate renders—in the
  ignored scratch folder, never in specs or source directories.

## Skills

Skills hold the procedures behind these principles. Load the one that covers
your work before you start. The complete development workflow and its skill
links live in [`development.md`](development.md).

Before changing this file, invoke
[`audit-agents`](.agents/skills/audit-agents/SKILL.md).
