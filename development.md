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
| [Online servers](apps/game-server/README.md) | The lobby (rooms, Frontline, presence, admission), the battle server each battle runs in, and the deploy step, the only code that knows the host. |
| [Multiplayer contract](packages/multiplayer/README.md) | The wire types, the record stream and its resume rule, and the room and Frontline rules both ends share. |
| [Skirmish setup](packages/skirmish-setup/README.md) | Seats, sides and map choice for offline skirmish, rooms and the server, and the battle request they become. |
| [Browser camera primitives](packages/renderer-core/README.md) | Camera/depth contracts, view control, camera rays and clearance.                                           |
| [Browser render inputs](packages/battle-renderer/README.md) | Plain scene contracts, external geometry and material inputs.                                   |
| [Scene assets](packages/scene-assets/README.md)           | Appearance contracts, validation, baking, loading, and Blender authoring.                            |
| [Battle audio](packages/battle-audio/README.md)           | Recorded and synthetic sound preparation, calibration, and playback.                                 |
| [Shared render runtime](crates/renderer-core/README.md) | Rust GPU composition, canonical preparation, resource lifetime, frame encoding and shared renderer tests across native and browser. |
| [Fixtures](fixtures/README.md)                            | Authored rules, unit/prop catalogs, generator presets, saved maps, and sound assignments.            |
| [Assets](assets/README.md)                                | Source media, prepared runtime files, review sheets, and third-party provenance.                     |
| [Native client](native/client/README.md)                  | The desktop shell over the shared render runtime.                        |
| [Mechanics editor](apps/mechanics-editor/README.md)       | Developer tool: edits resolved rules and unit families.                                              |
| [Map workbench](apps/map-workbench/README.md)             | Developer tool: tunes generation on a live top-down plan.                                            |
| [Sound workbench](apps/sound-workbench/README.md)         | Developer tool: auditions looks and movement, and edits library recipes.                       |
| [Fixture publication](apps/fixture-publication/README.md) | Reviewed fixture saves from the developer tools, and interrupted-write recovery.                     |
| [Capture tool](apps/battle-lab/src/capture/README.md)     | Developer tool: films saved encounters into MP4s from a JSON capture file (the menu backdrop is one), and HDR clips of a native replay. |

Commands, flags, paths, and dependency versions belong in the manifest or README
that owns them, not in this document.

Battle rules, visual presentation and the audio mix have independent readers.
Only battle rules enter the rules identity and encounter overrides; the
[weapon-look contract](fixtures/README.md#weapon-looks) connects a resolved
weapon to its authored sight and sound.

Content has three owners, and none reaches another's place. Game battles are
generated and field the roster's units. Test units (`test_*`) and `test` maps
belong to tests, labs, and benchmarks; capture units (`capture_*`) and `capture`
maps belong to the capture tool and the menu backdrop it films. A saved map's metadata says which it is
([saved maps](fixtures/README.md#saved-maps)). The main menu's Developer page lists
tools, labelled as such, that stay in production builds;
[browser checks](web/README.md#checks-and-evidence) say which lab pages are
tests and which are tools.

Players know the game by a public name that may change at any time, owned by
[`gameName.ts`](apps/battle-lab/src/gameName.ts). Code, packages, and the
repository keep their working names and never take it.

## Game rules

The physical properties that rules read:

- Obstacles are bodies with shape, weight class, mover blocking, cover tier, and
  integrity.
- Movers have a footprint and weight.
- Weapons own their engagement classes, layers, ranges, guidance, fuses, and
  interceptability.
- Units own their hull, senses, movement, flight model, and carried capabilities.

Rules follow from those properties alone. A mover can go where its footprint
fits and can shove aside a body lighter than itself. A body is cover when it stands
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
- a supply truck's final positive remainder buys one last service item at its
  normal delivery rate, even if its full price exceeds that remainder; an empty
  truck delivers nothing;
- an attack order pivots a vehicle on the spot to bring a gun fixed in its
  hull onto its target, on wheels as on tracks; unordered, only tracks pivot
  and wheels hold fire;
- of two vehicles that hold each other up, the later one (the higher unit id)
  gives way and works its way round while the other waits, unless it finds
  no way round;
- active protection (a Trophy) is a simple capability, not an interceptor
  weapon: while it has charges, any interceptable missile that comes within its
  standoff is destroyed, with no sight, cone, or range of its own;
- who can hold an objective follows the unit card's category (recon, infantry,
  and vehicles), a game rule rather than a physical one; the finer browsing
  groups the picker and the Armory show (`UNIT_GROUPS`) only present the
  categories, and no rule reads them;
- the neutral side only returns fire, whatever its units author, and takes no
  commands;
- in a skirmish a side's players are one team: they share the side's faction,
  its roads in, the objectives and their score, and everything any of its
  units sees; each player owns its units, its credits (income and kill
  rewards), its own unit cap and its controller, and readies on its own. A
  player's passive income tapers, by the match rules' `upkeep`, as the
  purchase price of the army it fields grows, counting purchases on their
  way, so a big army earns less and a gutted one rebuilds at full rate. A
  player's slot on its side names a lane, one of the side's roads, for the
  team to coordinate by; an AI opens on its own lane and lets the pull fade as
  the battle goes on, and every purchase still arrives by the side's quickest
  road;
- a recorded army hand-over passes a seat’s living units, wallet and pending
  purchases to the other AI-held seat on its side with the fewest living units,
  ties broken by seat order; it may exceed the receiver’s buying cap;
- a recorded seat reset requires no living units or pending purchases, then
  replaces that seat’s wallet with opening credits or zero as its caller
  decides; human identity and once-per-person grants belong to the server;
- the skirmish AI and the scripted encounter defender choose their behaviour by
  role tags (logistics, artillery, recon, AT, MBT). They are opponents'
  playbooks, not rules of the battle.

## Check ladder

1. Run the smallest focused test or checker that can move: one test, one file,
   one crate, or one scene.
2. Run the owning package or scene checks when the slice is coherent.
3. Run the full repository closeout checks once the spec is complete.

**Never run the whole suite (a workspace-wide test command) short of step 3.**
Run only the tests of the domain you changed. A change in one crate is proven
by that crate's tests, not by every package that depends on it: the sim owns
outcomes and pins its own digests and replays, so the observers (the client,
presentation, renderer) have nothing new to say about a sim change. The whole
suite needs nearly all of the LFS assets, which no checkout should fetch unless
it really needs them, and it fills the disk with builds of crates you never
touched. A test outside your domain that refuses an LFS pointer is a missing
asset, not a failure: don't fetch assets to chase it.

Recordings follow the game, never the reverse. The menu reel is promotional
footage on fake stats that changes as the game does
([the menu backdrop](fixtures/README.md#the-menu-reel)); it never blocks or
narrows a mechanic.

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
[lint-staged.config.mjs](lint-staged.config.mjs) owns what runs, and a
pre-push hook holds the [release order](#release-order). Both sit beside Git
LFS's hooks in the shared hooks directory, one installation for every
worktree, so setup never redirects Git's hooks directory.

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

## Release order

Work lands on `staging` first; `main` (production) only ever fast-forwards to a
commit `staging` already published, so `main` is always `staging` or behind it.
Each commit builds once: Vercel builds every `staging` push as a staged
production deployment, reachable at its own deployment URL (linked from the
commit's GitHub status) but not the production domain, and never deploys `main`,
so pushing `main` by hand ships nothing. Push `staging`, then run `bun run
promote:main`: it lists the commits production lacks, and with `--yes` waits for
that commit's staged deployment, promotes it to the production domain without a
rebuild, and fast-forwards remote `main` to it without touching your checkout.
Promotion keeps the live lobby in step: it redeploys only when the promoted
site’s simulation, shared battle documents or multiplayer wire differ from
the lobby’s, or the lobby cannot state an admitting identity.
Promotion ships every commit on `staging`, whoever pushed it, so read the list
before passing `--yes`. If `main` ever gets a commit `staging` lacks (a merge
made on GitHub), merge `main` into `staging` and push `staging` before
promoting; the promotion refuses until then.

A pre-push hook holds the order: [the guard](tools/check-branch-order.mjs)
refuses a push that would put a commit on `main` that remote `staging` lacks,
or move `staging` behind `main`, then hands the push to Git LFS's own pre-push.
It is local, so `--no-verify` and merges made on GitHub pass it by.

## Scratch output

The ignored scratch folder is `throwaway/`.

## Plans and docs

Active plans live under [specs](specs/README.md). A plan's README is its live
handoff, and its decisions ledger records what was decided where the plan was
silent; finished specs preserve durable rationale. Use
[write-spec](.agents/skills/write-spec/SKILL.md) for large or risky features and
[review](.agents/skills/review/SKILL.md) before calling a substantive slice
done.
