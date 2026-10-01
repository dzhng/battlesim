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
  - the worker that prepares a battle on a generated map (`prepare/`: the simulation's generator, then its encounter planner, off the page's thread);
  - decoding of the packed observation;
  - player input;
  - player readouts.

  Presentation reads only the observation, never simulation state.

- **`packages/`** — the TypeGPU renderer and its assets.
  - `renderer-core` holds device, camera and projection primitives.
  - `battle-renderer` holds scene resources, meshes and overlays.
  - `scene-assets` owns appearance bundles: schema, validation, baking and the one loader. The art itself lives in [`assets/`](assets/README.md).
  - `battle-audio` owns the battle's sound: what is heard and when, from the same feed the effects and poses read, synthesised in code, and heard from the camera.
- **`apps/battle-lab/`** — the lab app. Each lab route is a focused, deterministic fixture for one mechanic, and the village routes are the playable game. `src/fixtures.json` is the registry of lab routes.
- **`fixtures/`** — authored maps, units and rule numbers. `village.json` is the one owner of the game's rules; labs reuse it. `fixtures/units/` and `fixtures/props/` are the catalog: every unit type and every prop type is one entry, a variant an `extends` of another, and behaviour comes from a type's components, never its id. Adding a unit or prop type starts there ([`fixtures/README.md`](fixtures/README.md)).
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

`/` is the main menu: play the village or watch a replay; behind its developer link, run the benchmark or open the lab index at `/labs`, which links every route. The benchmark (`/benchmark`) is the one frame-cost measure.

Floating unit panels show name and health. Own-unit panels show a crossed-out eye beside the name, plus HIDDEN in expanded detail, while foliage or garrison shelter grants a concealment bonus to the hull or at least one living soldier; it does not guarantee enemies cannot identify the unit. Space prioritizes the closest 30% of visible units from the camera's actual position; farther cards expand where room remains. Hovering a unit or its card always reveals its detail. Placement tries expansion in place, then the fewest card shifts, favoring shorter shifts when the counts tie.

A held right-click previews each selected unit’s destination and facing with the same markers shown after release. Dragging rotates about the clicked front center. The [group move placement rationale](specs/done/group-move-preview/README.md) explains its authority, spacing and partial-placement contracts.

## Checks

`package.json` names the gates:

- `check` covers format, lint, typecheck and every Rust and web test.
- `verify` runs every browser scene.
- `bun run --cwd web scene -- <fixture-id>` runs one scene.

[`AGENTS.md`](AGENTS.md) says which runner to reach for while iterating. It also covers the worktree recipe (Git LFS, shared `node_modules` and build directory) that keeps parallel work from exhausting the machine.

## Plans and decisions

Work is planned as specs in [`specs/<feature>/`](specs/). Each spec has:

- a README whose "Next Agent Prompt" is the live handoff;
- a slice ladder;
- `choices.md`, the ledger of decisions made where the spec was silent.

A finished spec moves to [`specs/done/`](specs/done/), rewritten from a build plan into the record of why it works as it does.

[`design/`](design/) keeps the earliest planning map.
