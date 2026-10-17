# Developing battlegame

This document owns the engineering principles behind the game. The root
[README](README.md) explains the product; subsystem READMEs own commands and
contracts for their code.

## Where a concept belongs

The simulation is the one authority. Everything else observes it, and each
side receives only the publications its own units can see.

| Owner                                                     | Responsibility                                                                                  |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| [Contracts](crates/contract/README.md)                    | Shared commands, observations, map admission, physical templates, and exact content identities. |
| [Simulation](crates/sim/README.md)                        | Deterministic battle rules, replay digests, encounter placement, and native reports.            |
| [Map generation](crates/mapgen/README.md)                 | Seeded layout, physical compilation, admission, and generation reports.                         |
| [WebAssembly boundary](crates/game-wasm/README.md)        | Native/browser parity, geometry queries, publication memory, and binding lifetime.              |
| [Presentation](crates/presentation/README.md)             | Per-side frame publications: interpolation, poses, effects, cook-offs, tiers, and cast lights.  |
| [Browser application](web/README.md)                      | Workers, map adapters, player input, readouts, build configuration, and browser verification.   |
| [Battle lab](apps/battle-lab/README.md)                   | Player pages and focused developer fixtures composed from the shared battle view.               |
| [Renderer primitives](packages/renderer-core/README.md)   | Camera/depth contracts, device admission, and GPU allocation accounting.                        |
| [Battle renderer](packages/battle-renderer/README.md)     | Observed feeds, terrain, models, lights, fog, effects, overlays, and GPU lifetime.              |
| [Scene assets](packages/scene-assets/README.md)           | Appearance contracts, validation, baking, loading, and Blender authoring.                       |
| [Battle audio](packages/battle-audio/README.md)           | Recorded and synthetic sound preparation, calibration, and playback.                            |
| [Pipelines](crates/pipelines/README.md)                   | Browser shaders and pipeline settings consumed by the renderer and native client.               |
| [Fixtures](fixtures/README.md)                            | Authored rules, unit/prop catalogs, generator presets, saved maps, and sound assignments.       |
| [Assets](assets/README.md)                                | Source media, prepared runtime files, review sheets, and third-party provenance.                |
| [Fixture publication](apps/fixture-publication/README.md) | Reviewed fixture saves, generated catalogs, and interrupted-write recovery.                     |
| [Native client](native/client/README.md)                  | The `wgpu` client over the shared presentation frames and renderer pipelines.                   |

Find the existing owner before introducing a new abstraction. Commands, flags,
paths, and dependency versions belong in the manifest or README that owns them,
not in this document.

## Game rules

Rules come from physical properties, not named special cases. Express a
restriction through the data each unit or weapon carries; do not add a rule
that repeats the same restriction by role or label.

- Obstacles are bodies with shape, weight class, mover blocking, cover tier, and
  integrity.
- Movers have a footprint and push class.
- Weapons own their engagement classes, layers, ranges, guidance, fuses, and
  interceptability.
- Units own their hull, senses, movement, flight model, and carried capabilities.

The target is Hollywood realism. Keep what a viewer expects in a war film, even
when it is exaggerated. Drop what looks silly on screen, even when a more exact
physical model could defend it. Hard-coded rules are reserved for clearly
game-shaped mechanics such as garrisons, suppression, objectives, and the
neutral side's return-fire policy.

Before proposing a mechanic or changing who can see, move, shoot, block, push,
destroy, or be hit, use the [tweak-mechanics](.agents/skills/tweak-mechanics/SKILL.md)
workflow.

## Working loop

Optimize for the time to feedback you can trust. Run the narrowest check that
answers the question: one test, one file, one crate, or one scene. Run the full
closeout checks once, when a spec's implementation is finished. A change that
reaches the whole system does not move that full run earlier while more work is
still coming.

Write the test first for behavior changes and regressions. Test what the game
does and how it fails, not which helper or branch happened to implement it. The
simulation is deterministic: a refactor or performance change that should not
alter behavior must preserve battle digests and replays, or name the decision
that changes them.

Do not loosen an acceptance requirement to make a check pass. Say what a narrow
pass proves, what it assumes, and what remains unfinished. Long-running checks
belong in the background with a visible progress marker and a stopping point;
never repeat an unchanged failure.

The root task runner owns the repository-wide commands. The package manifests
and subsystem READMEs own their exact command details.

## Checks and hooks

After setup, Husky runs [lint-staged.config.mjs](lint-staged.config.mjs) before
each commit. It formats staged files and blocks staged JavaScript or TypeScript
that fails Oxlint or exceeds the complexity budget; staged Rust also runs Cargo
fmt and Clippy.

Cyclomatic complexity is an error at 20. A function may exceed that budget only
when its branches form one cohesive state machine, renderer pass, or
data-packaging pipeline. Such an exception needs an inline ESLint disable
comment naming that ownership and why extraction would obscure it. The full
complexity check must finish with zero errors and zero warnings.

Use the repository's check ladder:

1. Run the smallest focused test or checker that can move.
2. Run the owning package or scene checks when the slice is coherent.
3. Run the full repository closeout checks once the spec is complete.

Rendering and UI changes need visual evidence. Get an unprimed second opinion
with [screenshot-critique](.agents/skills/screenshot-critique/SKILL.md), compare
before and after with [compare-screenshots](.agents/skills/compare-screenshots/SKILL.md),
and show the result with [preview-shots](.agents/skills/preview-shots/SKILL.md).
Before changing player-facing UI, use [game-ui](.agents/skills/game-ui/SKILL.md);
before changing GPU passes, buffers, shaders, or frame orchestration, use
[renderer](.agents/skills/renderer/SKILL.md).

## Player-facing work

Look at the picture. A passing test does not prove that a marker, HUD panel,
callout, range ring, colour, or contact glyph reads correctly on screen.

Keep player-facing UI in the visual language of a tactical battlefield: clear
hierarchy, readable state, grounded marks, and restrained colour. It should feel
like a command view, not a business dashboard.

## Parallel work and scratch output

Every parallel checkout multiplies dependencies and build output. Fetch only
large files the task needs, share installed dependencies when safe, and never
share build output between checkouts with different sources. Remove a checkout
and its build output when its branch is merged.

Put screenshots, logs, probes, and candidate renders in the ignored scratch
folder. Do not put disposable evidence in specs or source directories.

## Plans and docs

Active plans live under [specs](specs/). A plan's README is its live handoff;
finished specs preserve durable rationale. Use [write-spec](.agents/skills/write-spec/SKILL.md)
for large or risky features, [refactor-clean](.agents/skills/refactor-clean/SKILL.md)
when a change exposes duplicate or stale ownership, and [review](.agents/skills/review/SKILL.md)
before calling a substantive slice done.
