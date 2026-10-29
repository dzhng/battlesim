# Developing battlegame

Subsystem READMEs own commands and contracts for their code.

## Where a concept belongs

| Owner                                                     | Responsibility                                                                                       |
| --------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| [Contracts](crates/contract/README.md)                    | Shared commands, observations, map admission, physical templates, and exact content identities.      |
| [Simulation](crates/sim/README.md)                        | Deterministic battle rules, replay digests, encounter placement, and native reports.                 |
| [Map generation](crates/mapgen/README.md)                 | Seeded layout, physical compilation, admission, and generation reports.                              |
| [WebAssembly boundary](crates/game-wasm/README.md)        | Native/browser parity, geometry queries, publication memory, and binding lifetime.                   |
| [Presentation](crates/presentation/README.md)             | Per-side frame publications: interpolation, poses, effects, cook-offs, tiers, and cast lights.       |
| [Browser application](web/README.md)                      | Workers, map adapters, player input, readouts, build configuration, and browser verification.        |
| [Battle lab](apps/battle-lab/README.md)                   | Player pages, shareable battle addresses, and focused developer fixtures.                            |
| [Browser camera primitives](packages/renderer-core/README.md) | Camera/depth contracts, view control and CPU picking.                                           |
| [Browser render inputs](packages/battle-renderer/README.md) | Plain scene contracts, external geometry and material inputs.                                   |
| [Scene assets](packages/scene-assets/README.md)           | Appearance contracts, validation, baking, loading, and Blender authoring.                            |
| [Battle audio](packages/battle-audio/README.md)           | Recorded and synthetic sound preparation, calibration, and playback.                                 |
| [Shared render runtime](crates/renderer-core/README.md) | Rust GPU composition, canonical preparation, resource lifetime and frame encoding across native and browser. |
| [Fixtures](fixtures/README.md)                            | Authored rules, unit/prop catalogs, generator presets, saved maps, and sound assignments.            |
| [Assets](assets/README.md)                                | Source media, prepared runtime files, review sheets, and third-party provenance.                     |
| [Native client](native/client/README.md)                  | The `wgpu` client over the shared presentation frames and renderer pipelines.                        |
| [Mechanics editor](apps/mechanics-editor/README.md)       | Developer tool: edits resolved rules and unit families.                                              |
| [Map workbench](apps/map-workbench/README.md)             | Developer tool: tunes generation on a live top-down plan.                                            |
| [Sound workbench](apps/sound-workbench/README.md)         | Developer tool: auditions the sound library and edits recipes and assignments.                       |
| [Fixture publication](apps/fixture-publication/README.md) | Reviewed fixture saves from the developer tools, and interrupted-write recovery.                     |
| [Capture tool](apps/battle-lab/src/capture/README.md)     | Developer tool: films saved encounters into MP4s from a JSON capture file; the menu backdrop is one. |

Commands, flags, paths, and dependency versions belong in the manifest or README
that owns them, not in this document.

Content has three owners, and none reaches another's place. Game battles are
generated and field the roster's units. Test units (`test_*`) and `test` maps
belong to tests, labs, and benchmarks; capture units (`capture_*`) and `capture`
maps belong to the capture tool and the menu backdrop it films. A saved map's metadata says which it is
([saved maps](fixtures/README.md#saved-maps)). The developer menu's entries are
tools, labelled as such, and stay in production builds;
[browser checks](web/README.md#checks-and-evidence) say which lab pages are
tests and which are tools.

Players know the game by a public name that may change at any time, owned by
[`gameName.ts`](apps/battle-lab/src/gameName.ts). Code, packages, and the
repository keep their working names and never take it.

## Game rules

The physical properties that rules read:

- Obstacles are bodies with shape, weight class, mover blocking, cover tier, and
  integrity.
- Movers have a footprint and push class.
- Weapons own their engagement classes, layers, ranges, guidance, fuses, and
  interceptability.
- Units own their hull, senses, movement, flight model, and carried capabilities.

Rules follow from those properties alone. A mover can go where its footprint
fits and can clear a body its push class exceeds. A body is cover when it stands
between a soldier and the threat, breaks when its integrity runs out, and stops a
round only if it is authored to. A gun holds fire only for what its rounds can't
break or can't see past. So there is no "wall" or "tank trap" in the code:
dragon's teeth are small heavy bodies that block vehicles, and a new obstacle is
a prop catalog entry, not new code.

A weapon's mechanisms (guidance, seekers, decoys, clear-launch checks,
interceptability, firing on its own) are opt-in flags that compose. A radar contact's source
changes only its evidence, never who may fire at it.

Roles and families group the roster and pick symbols and voices; they never
decide behaviour. If a rule asks "is this an X unit?", find the property that
makes it one and read that instead.

Values live on the unit, not in code. When a value differs between units,
author it on each unit type; a shared profile (`extends`) sets it for a whole
group, and a script may rewrite many at once. Code never derives such a value
from a unit's body, class or another field: no fallback tables, no class
splits, no capability that silently overrides an authored value. Leaving a
field out may only mean "none" or "off", never a value code picks or copies.

A catalog row describes a body, while its placement supplies geometry
ownership. Garrison slots and building-scaled integrity require a placed
building aggregate; ordinary props, generated forest bodies, bridge decks, and
vehicle wrecks have no such owner. Aggregate bodies stay immovable until the
simulation can move all their parts together.

Use first principles where they stay simple, and hard-code a clear game rule
where a principled version would be complex or look silly on screen. This is
the one list of deliberate hard-coded rules:

- cover only helps infantry, and crouching is an animation;
- garrisons are a named fighting-position mechanic;
- carriers come in three sizes only (3, 8 or 16 seats); riders can't fire, be
  seen, be selected or be ordered; an explicit unload with no room to stand is
  refused rather than costing lives, while a destroyed host's bail-out may kill;
- a soldier's own rounds pass untouched through the tall cover he leans round
  to fire (he steps out past its edge, fires a burst, and tucks back in);
  everyone else's rounds still hit it;
- suppression pins squads only; a vehicle's crew isn't suppressed;
- active protection (a Trophy) is a simple capability, not an interceptor
  weapon: while it has charges, any interceptable missile that comes within its
  standoff is destroyed, with no sight, cone, or range of its own;
- who can hold an objective follows the unit card's category (recon, infantry,
  and vehicles), a game rule rather than a physical one;
- the neutral side only returns fire, whatever its units author, and takes no
  commands;
- in a skirmish a side's players are one team: they share the side's faction,
  its roads in, the objectives and their score, and everything any of its
  units sees; each player owns its units, its credits (income and kill
  rewards), its own unit cap and its controller, and readies on its own;
- the skirmish AI and the scripted encounter defender choose their behaviour by
  role tags (logistics, artillery, recon, AT, MBT). They are opponents'
  playbooks, not rules of the battle.

## Check ladder

1. Run the smallest focused test or checker that can move: one test, one file,
   one crate, or one scene.
2. Run the owning package or scene checks when the slice is coherent.
3. Run the full repository closeout checks once the spec is complete.

Played battles, browser scenes, and the balance report are the expensive runs.
Rendering, UI, sound, and docs need no battle. A rule change needs a quick
sample while iterating, and the full balance report runs once, at closeout.
Measure simulation cost in units that don't move with machine load, such as
instructions retired, not wall time.

The root task runner owns the repository-wide commands. The package manifests
and subsystem READMEs own their exact command details. Packages and apps are
source directories; the [web manifest](web/package.json) owns their JavaScript
dependencies.

## Checks and hooks

After setup, a pre-commit hook formats and lints what the commit touches;
[lint-staged.config.mjs](lint-staged.config.mjs) owns what runs. It sits beside
Git LFS's hooks, so setup never redirects Git's hooks directory.

Source files (what GitHub Linguist calls a programming language) have a size
budget of 8,000 tokens (bytes / 4), so an agent can read any file whole. It is
a ratchet: a file over the budget may shrink but not grow. The
[ratchet-files](.agents/skills/ratchet-files/SKILL.md) skill owns the check,
lists every file over the budget, and says how to split one or, as a last
resort, opt it out with a pragma.

Cyclomatic complexity has a loose budget, owned by
[web/eslint.config.mjs](web/eslint.config.mjs). It is a tripwire for functions
that have grown too large to follow, not a target: a long function that reads
top to bottom is better than the same logic scattered across helpers that pass
state back and forth. Split a function when its branches fall into named steps
with their own inputs, never just to get under the number.

An inline `eslint-disable-next-line complexity` comment is technical debt: a
function nobody has found a clean split for yet. Keep them rare. The comment
says what blocks the split, not why the function is fine as it is. Read every
one you touch as a cue to try the split again. When a function drops back under
the budget, the lint fails on the stale comment until you delete it. For the
same reason, suppress an Oxlint rule with an `oxlint-disable` comment: ESLint
reports an `eslint-disable` for a rule it doesn't run.

## Scratch output

The ignored scratch folder is `throwaway/`.

## Plans and docs

Active plans live under [specs](specs/README.md). A plan's README is its live
handoff, and its decisions ledger records what was decided where the plan was
silent; finished specs preserve durable rationale. Use
[write-spec](.agents/skills/write-spec/SKILL.md) for large or risky features and
[review](.agents/skills/review/SKILL.md) before calling a substantive slice
done.
