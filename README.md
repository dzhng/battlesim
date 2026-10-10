# battlegame

A browser real-time tactics game, built around reconnaissance, physical fire and positioning. Every round is a flown projectile, and every side sees only what its own units can see. Players fight on fresh seeded generated maps with the faction roster. The full design lives in [`specs/`](specs/).

Players know the game by a public name that may change at any time, owned by [`gameName.ts`](apps/battle-lab/src/gameName.ts); code, packages and the repo keep their working names and never take it.

Content has three owners, and none reaches another's place. Game battles are generated and field the roster's units. Test units (`test_*`) and `test` maps are the tests', labs' and benchmarks' own; menu units and `menu` maps are the menu backdrop's own. A saved map's `meta.json` says which it is ([saved maps](fixtures/README.md#saved-maps)). The developer menu's entries are tools, labelled as such, and stay in production builds.

## How it fits together

The simulation is the one authority; everything else observes it. Start with the
owner of the thing you want to change:

| Owner | Responsibility and entry point |
|---|---|
| [Contracts](crates/contract/README.md) | Shared commands, observations, map admission, physical templates and exact content identities. |
| [Simulation](crates/sim/README.md) | Deterministic battle rules, replay digests, encounter placement and native reports. |
| [Map generation](crates/mapgen/README.md) | Seeded layout, physical compilation, admission and generation reports; shared by native tools and WebAssembly. |
| [WebAssembly boundary](crates/game-wasm/README.md) | Native/browser parity, geometry queries, packed publication memory and binding lifetime. |
| [Browser application](web/README.md) | Workers, map adapters, player input/readouts, build configuration and browser verification. |
| [Battle lab](apps/battle-lab/README.md) | Player pages and focused developer fixtures, composed from the same battle view. |
| [Renderer primitives](packages/renderer-core/README.md) | Shared camera/depth contracts, device admission and GPU allocation accounting. |
| [Battle renderer](packages/battle-renderer/README.md) | Observed feeds, terrain, models, light, fog, effects, overlays and GPU lifetime. |
| [Scene assets](packages/scene-assets/README.md) | Appearance contracts, validation, baking and loading; includes [Blender authoring](packages/scene-assets/README.md#authored-sources-blender). |
| [Battle audio](packages/battle-audio/README.md) | Recorded and synthetic sound preparation, source calibration and playback from observed causes. |
| [Fixtures](fixtures/README.md) | Authored rules, unit/prop catalogs, generator presets, saved maps and sound assignments. |
| [Assets](assets/README.md) | Source media, prepared runtime files, review sheets and third-party provenance. |

Development tools edit those existing owners: the [mechanics editor](apps/mechanics-editor/README.md),
[map workbench](apps/map-workbench/README.md) and [sound workbench](apps/sound-workbench/README.md).
They share [fixture publication](apps/fixture-publication/README.md), which keeps
reviewed saves consistent and recovers interrupted writes.

[The root task runner](package.json) composes Rust and web commands; [the Rust
workspace](Cargo.toml) and [web manifest](web/package.json) own members,
dependencies and exact command definitions. Packages and apps are source directories;
the web application owns their JavaScript dependencies.

## Rules from first principles

The simulation builds behaviour from low-level physical properties, never from named special cases.

- **Obstacles are bodies:** a shape, a weight class, which mover classes it blocks, a cover tier, and integrity.
- **Movers have a footprint and a push class.**

The rules then follow from those properties alone:

- **Can go here:** the footprint fits, judged by navigation's clearance field (in a tight spot, by the bodies and ground themselves, anywhere the hull fits) and then per-tick collision.
- **Can clear it:** the mover's push class exceeds the body's weight class.
- **Is cover:** a body stands between the soldier and the threat.
- **Breaks:** its integrity runs out.
- **Stops a round, or not:** a round flies through a body that doesn't stop rounds, and wears it if the body has integrity.
- **Holds fire:** a gun holds fire only for what its rounds can't break or can't see past, and fires into anything else on its line until it breaks.

So there is no "wall", "road block" or "tank trap" in the code. Dragon's teeth are just small heavy bodies that block vehicles, and a squad takes cover behind each one because each is a body. Vehicles and props follow the same rules. A new obstacle is an entry in the prop catalog (`fixtures/props/`), not new code.

A catalog row describes a body, while its placement supplies geometry ownership. Garrison seats and building-scaled integrity require a placed building aggregate; ordinary props, generated forest bodies, bridge decks and vehicle wrecks have no such owner. This constraint follows the body's destruction chain. Aggregate bodies remain immovable until the simulation supports moving all their parts together.

It's a game, not a physics simulation. The target is **Hollywood realism**: the battle should look and behave the way a war film makes it look, not the way a ballistics table says. Keep what a viewer expects, even exaggerated, like cover blown apart, shells felling trees and sparks off armour. Drop what looks silly on screen, even when it's physically defensible, like a squad's stray rifle fire mowing down a forest. Sustained, aimed fire may fell one tree; incidental fire shouldn't clear woods. Use first principles where they stay simple, and hard-code a clear game rule where a principled version would be complex. Today's hard-coded rules:

- cover only helps infantry, and crouching is an animation;
- garrisons are a named fighting-position mechanic;
- a soldier's own rounds pass untouched through the tall cover he leans round to fire (he steps out past its edge, fires a burst and tucks back in); everyone else's rounds still hit it.

## Running it

You need [Bun](https://bun.sh), [rustup](https://rustup.rs) (it installs the pinned Rust and its `wasm32-unknown-unknown` target from [`rust-toolchain.toml`](rust-toolchain.toml), the same release the deployment builds with), [wasm-pack](https://rustwasm.github.io/wasm-pack/), and a WebGPU browser.

```bash
bun run setup   # install web dependencies
bun run dev     # build the WebAssembly, start the lab app
```

`/` is the main menu: start a battle on a generated map (its type, its size and optionally its region), watch a replay, or read the tutorial on how to play; behind its developer link, run the benchmark or open the lab index at `/labs`, which links every route. A generated battle's address (`/battle?type=&size=&seed=&faction=&enemy=`, with `&region=` when one was chosen) is its share identity: the same address prepares the same battle on the same build. An address without the player's faction, or with a parameter the battle does not read, is refused by name. The [benchmark](web/src/battle/benchmark/README.md) is the one frame-cost measure: `/benchmark` runs explicitly synthetic local contact on a complete generated world (`?preset=city-contact` names it).

Ordinary Play chooses an admitted battlefield from the selected type and size, with bounded fresh candidates. Once admitted, its address names the actual battlefield. Explicit seed addresses and saved replays remain exact.

The local [map workbench](apps/map-workbench/README.md) tunes generation on a live
top-down plan. The [mechanics editor](apps/mechanics-editor/README.md) edits resolved
rules and unit families. The [sound workbench](apps/sound-workbench/README.md)
auditions the whole library and edits recipes and assignments. All are available
from the developer menu during `bun run dev`; each README explains its workflow.

[Battle lab documentation](apps/battle-lab/README.md) explains developer routes,
player presentation and their links to shipped design rationales. [Browser
configuration](web/README.md#build-and-serving) owns local serving and deployment
requirements; [native reports](crates/sim/README.md#reports-and-cost) own simulation
and balance measurements.

## Checks

[The task runner](package.json) defines `check` (format, lint, types and Rust/web
tests) and `verify` (browser scenes). Both run once at a spec's closeout; iteration
uses the narrowest check the change can move, as [AGENTS.md](AGENTS.md) requires.
For an interface change that check is `visual`: the scenes whose captures must
match approved pictures.

[Simulation checks](crates/sim/README.md#checks) explain deterministic outcomes and
native tests. [Browser checks](web/README.md#checks-and-evidence) explain test and
scene selection, prerequisites, GPU serialization and evidence. Documentation and
data that no code reads need no battle run.

## Worktrees

Large binaries (models, textures, reference images) are in Git LFS, set up once per clone so that checkouts get small pointer files:

```bash
git lfs install --local --skip-smudge
git lfs pull                                 # in the main checkout only
git lfs pull --include="<path>/**"           # in a worktree: only what the task needs
```

[LFS attributes](.gitattributes) define which inputs are large-file objects, and
[ignore rules](.gitignore) separate generated output and scratch from source.

In a worktree, symlink `web/node_modules` to the main checkout's, and give it its own Rust build directory with `CARGO_TARGET_DIR`. Cargo leaves a workspace crate's path out of its build hash, so two worktrees sharing one `target/` overwrite each other's builds. Delete that directory with the worktree.

## Plans and decisions

[Specs](specs/README.md) hold active plans and finished rationale. A plan's own
README is its live handoff; its choices ledger records decisions made where the
plan was silent. [Design references](design/) retain the earliest planning map.
Repository working principles and skill entry points live in [AGENTS.md](AGENTS.md);
[CLAUDE.md](CLAUDE.md) supplies the corresponding agent entry point. [Repository
skills](.agents/skills/) own procedure-specific tools; `.claude/skills/` links to
those same owners, and [the skill lock](skills-lock.json) records acquired skill
provenance. Those are agent workflows rather than game runtime scripts.
