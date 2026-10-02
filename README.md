# battlegame

A browser real-time tactics game, built around reconnaissance, physical fire and positioning. Every round is a flown projectile, and every side sees only what its own units can see. The first playable checkpoint is a replayable assault on a defended village. The full design lives in [`specs/`](specs/).

## How it fits together

The simulation is the one authority; everything else observes it.

- **`crates/`** — the Rust side.
  - `contract` holds the shared data shapes: scenario, commands, and the per-side observation.
  - `sim` owns every rule: movement, sight, sensing, knowledge, weapons, flight, damage, garrisons, supply and deployment. It also places an encounter recipe on a compiled map (`sim::encounter`), by asking those same rules where units may stand.
  - `game-wasm` is the thin WebAssembly boundary.

  New battle state must enter `Battle::digest`, so replays and parity checks catch drift.

- **`web/src/battle/`** — the browser side of that boundary:
  - the worker that runs the simulation (one authority, ordered commands, bounded publications);
  - the worker that prepares a battle (`prepare/`): one request names where the map comes from (a saved map, or a generation request), which encounter is laid on it and the seeds; the worker resolves the map, has the simulation's planner place the encounter, and answers with the scenario a battle runs, off the page's thread;
  - decoding of the packed observation;
  - player input;
  - player readouts.

  Presentation reads only the observation, never simulation state.

- **`web/src/maps/`** — maps as JavaScript reaches them: the catalogue's listing, the browser and Node adapters that fetch a saved map's documents by id and hand them to the simulation's one resolver, and the one way to a map from its source (`source.ts`: a saved map's id, or a request the simulation's generator makes a map from).

- **`packages/`** — the TypeGPU renderer and its assets.
  - `renderer-core` holds device, camera and projection primitives.
  - `battle-renderer` holds scene resources, meshes and overlays.
  - `scene-assets` owns appearance bundles: schema, validation, baking and the one loader. The art itself lives in [`assets/`](assets/README.md).
  - `battle-audio` owns the battle's sound: what is heard and when, from the same feed the effects and poses read, synthesised in code, and heard from the camera.
- **`apps/battle-lab/`** — the lab app. Each lab route is a focused, deterministic fixture for one mechanic, and the village routes are the playable game. `src/fixtures.json` is the registry of lab routes.
- **`fixtures/`** — authored maps, units and rule numbers. `game.json` is the one owner of the game's rules; labs reuse it. `fixtures/maps/<id>/` is the saved-map catalogue: every map is one folder (its physical map, its provenance, its listing metadata and its encounters), resolved by id and never imported into a script. `fixtures/units/` and `fixtures/props/` are the catalog: every unit type and every prop type is one entry, a variant an `extends` of another, and behaviour comes from a type's components, never its id. Adding a unit or prop type starts there ([`fixtures/README.md`](fixtures/README.md)).
- **`web/scenes/`** — one headless browser scene per registered fixture. These scenes are the visual and behavioural checks, run by `web/scene.mjs`.

## Rules from first principles

The simulation builds behaviour from low-level physical properties, never from named special cases.

- **Obstacles are bodies:** a shape, a weight class, which mover classes it blocks, a cover tier, and integrity.
- **Movers have a footprint and a push class.**

The rules then follow from those properties alone:

- **Can go here:** the footprint fits, judged by navigation's clearance field and then per-tick collision.
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

You need [Bun](https://bun.sh), a Rust toolchain with the `wasm32-unknown-unknown` target, [wasm-pack](https://rustwasm.github.io/wasm-pack/), and a WebGPU browser.

```bash
bun run setup   # install web dependencies
bun run dev     # build the WebAssembly, start the lab app
```

`/` is the main menu: start a battle on a generated map (its type and its size), play a saved battlefield of the catalogue, or watch a replay; behind its developer link, play the test village, run the benchmark or open the lab index at `/labs`, which links every route. A generated battle's address (`/battle?type=&size=&seed=`) is its share identity: the same address prepares the same battle on the same build. The benchmark (`/benchmark`) is the one frame-cost measure.

Floating unit panels show name and health. Own-unit panels show a crossed-out eye beside the name, plus HIDDEN in expanded detail, when forest or garrison concealment covers enough of the living unit and there is no known engagement or currently visible enemy observer spotting it. The [sensing rules](crates/sim/src/sensing.rs) own squad thresholds; concealment uses forest ground, independently of tree crowns. Space prioritizes the closest 30% of visible units from the camera's actual position; farther cards expand where room remains. Hovering a unit or its card always reveals its detail. Placement tries expansion in place, then the fewest card shifts, favoring shorter shifts when the counts tie.

A held right-click previews each selected unit’s destination and facing with the same markers shown after release. Dragging rotates about the clicked front center. The [group move placement rationale](specs/done/group-move-preview/README.md) explains its authority, spacing and partial-placement contracts.

The [projectile review lab](apps/battle-lab/src/projectileReview.ts) keeps the fog-lit street fight running alongside firing lanes and midpoint recon teams. It uses the shared battle view’s unit panels and player controls, with gameplay flight and weapon cycles, private unlimited reserves and nonlethal rounds. Repeated visual review does not change gameplay rules. The [guided-fire contract](specs/battle-foundation/contracts.md#guided-flight) separates shared spotting from the launcher’s physical line of sight.

## Checks

`package.json` names the gates:

- `check` covers format, lint, typecheck and every Rust and web test.
- `verify` builds the WebAssembly and runs every browser scene.

Both are slow and run once, when a spec's implementation is finished ([`AGENTS.md`](AGENTS.md)). Everything else is checked with the narrowest thing that covers the change:

```bash
cargo test -p sim --test sim village::a_replay_matches  # one test
cargo test -p sim --test sim village::                  # one file (the sim tests are one binary)
cargo test -p sim                                       # one crate
bun run --cwd web test -- tests/observation.test.ts     # one web test file
bun run --cwd web scene -- village                      # one browser scene
bun run --cwd web scene -- --list                       # scene ids
```

Web tests and scenes need the WebAssembly built once first (`bun run build:wasm`).

Every scene, and anything else that renders, shares the machine's one GPU. When more than one session is working, run each through the shared lock so they take turns; two at once slow each other and distort every timing:

```bash
lockf -k <main checkout>/throwaway/gpu.lock bun run --cwd web scene -- village
```
 Scenes write their evidence into gitignored `throwaway/evidence/<fixture-id>/`.

The simulation's reports are examples of the `sim` crate (`crates/sim/examples/`); each prints its own flags when given one it doesn't know. `village_report` plays blue's comparison scripts against the red defender and ends every row in the battle's digest. Its `--quick` mode is the feedback loop for a rule change, and `--compare main` sets a run beside main's. `endurance_report` prints instructions retired over a long battle.

## Worktrees

Large binaries (models, textures, reference images) are in Git LFS, set up once per clone so that checkouts get small pointer files:

```bash
git lfs install --local --skip-smudge
git lfs pull                                 # in the main checkout only
git lfs pull --include="<path>/**"           # in a worktree: only what the task needs
```

In a worktree, symlink `web/node_modules` to the main checkout's, and give it its own Rust build directory with `CARGO_TARGET_DIR`. Cargo leaves a workspace crate's path out of its build hash, so two worktrees sharing one `target/` overwrite each other's builds. Delete that directory with the worktree.

## Plans and decisions

Work is planned as specs in [`specs/<feature>/`](specs/). Each spec has:

- a README whose "Next Agent Prompt" is the live handoff;
- a slice ladder;
- `choices.md`, the ledger of decisions made where the spec was silent.

A finished spec moves to [`specs/done/`](specs/done/), rewritten from a build plan into the record of why it works as it does.

[`design/`](design/) keeps the earliest planning map.
