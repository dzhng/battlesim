# Fixtures

`village.json` is the one owner of the game's rules and look numbers. Labs and scenes reuse it. `map-presets.json` holds the map generator's layout presets; [`crates/mapgen`](../crates/mapgen/README.md) reads and validates it. `units/` and `props/` are the catalog: every unit type, soldier kind, role and upgrade part, and every prop type. `biomes/` holds the terrain palettes. A number that changes how the battle plays or looks belongs here, validated by the module that reads it, never as a constant in code.

## The unit catalog

A unit type is **one catalog entry**, addressed by its string id (`"tank"`, later `"m1a2_sepv3"`). No code lists unit types. Scenarios, spawn rows and commands name types by id; the publication sends the id list (`unitKinds`) and each unit's index into it.

- **Files:** `units/<faction>/<family>.json`, one family per file, plus `units/roles.json` (the role registry) and, when parts exist, a parts file. Each file is an object with any of five sections: `roles`, `parts`, `soldiers`, `units` and `props` (the prop types, under `props/`). An id is defined once across all files.
- **A type is its components.** Behaviour comes from them, never from the id:
  - `body`: `{ "squad": { "slots": [soldier kinds] } }` or `{ "hull": { half_extents_m, eye_m, hp, armor, weight_class, push_class, wreck } }`, where `wreck` names a prop type;
  - `mobility`: `foot`, `tracked` or `wheeled`, each with its own speeds and turning;
  - `sensors`: the one sight (`ground_m`, `sight_shape`, and `on`, the turret mount the optics turn with);
  - `mounts`: a hull's weapons, each row with its carrier (`on`), `pivot_m` and `muzzle_m`. A squad's come from its soldiers;
  - `capabilities`: optional abilities such as `deploy` and `supply`;
  - `roles` (what scripts and the AI select by), `cost`, `sound`, `name`, `description`, `faction`, `family`, and a hull's `appearance`.
- **Soldier kinds** are a catalog of their own: `hp`, the `appearance` set a soldier of the kind wears (one picked per soldier), and the `mounts` he carries. A `special` mount passes to the next living soldier when its carrier falls; any other is lost with him. A squad's slots name soldier kinds, so hundreds of squads reuse a few kinds.
- **A variant is `extends` plus overrides.** `"m1a1": { "extends": "m1", "body": { "hull": { "armor": { "front": 180 } } } }` inherits everything else. An `abstract` entry only exists to be extended. Weapon rows in `village.json` extend the same way. The merge:
  - objects merge key by key, and a list of named objects (mounts) merges by name, a new name appended;
  - a unit's one-key variant component (`body`, `mobility`) written as another variant replaces the parent's: `"mobility": { "wheeled": … }` over a tracked parent is wheeled;
  - a unit's `parts` gather along the chain, the parent's first;
  - anything else is replaced, a list of strings (`roles`, `slots`) whole.
  - What a variant can't do: drop a key or a named mount it inherits. A type that loses a mount extends a common parent instead.
- **Parts are upgrades:** `"parts": ["trophy_aps"]` merges each part's `patch` into the type after inheritance, by the same rules. A part that needs a capability the simulation doesn't build yet is refused at load, because the type no longer parses. A part names the model nodes that show its hardware (`nodes`), and the type's model must draw them.
- **Resolution happens once, in the simulation** (`contract::catalog`), and a broken catalog fails at load with an error naming the entry:
  - a key written twice inside one file;
  - cycles, unknown parents, roles, soldiers or parts, and incomplete types;
  - structure: a hull mount on anything but the hull or an earlier turret mount, or without its `muzzle_m`; a soldier's mount with `turret`, `on`, `pivot_m` or `muzzle_m`, or both `squad` and `special`; a mount naming a weapon row `village.json` lacks; a wreck whose cover tier isn't its vehicle's;
  - numbers out of range: speeds, turning, sight, hit points.
- **The browser reads the resolved view,** `catalog.json`, which also carries `village.json`'s weapon rows resolved (`weapons`); presentation reads rows there, never the raw ones. After editing the catalog, regenerate it: `BLESS_CATALOG=1 cargo test -p sim --test sim catalog::` (the test fails while it is stale). Then regenerate the icons (each type's silhouette is rendered from its baked model): `bun run --cwd web asset -- icons`.

## Weapon cycles

A weapon row's `ammo` is total carried rounds, including loaded magazines; `"unlimited"` means unlimited reserves. `magazine` adds a physical gun's capacity and interval between shots, while `reload_s` is the pause to replace its magazine or belt. An optional `burst` groups rapid shots, with a fresh independently sampled aim delay before each burst. A burst pause keeps the magazine loaded; only an empty magazine starts a reload. A zero `reload_s` refills immediately without showing unloaded readiness, as used for infantry rifles. The physical shot interval remains a lower bound even when the sampled aim delay is zero. Without `magazine`, the gun reloads after each round. Intervals complete on the next simulation tick.

Each physical weapon owns its targeting, ammunition and firing cycle. Identical special weapons stay separate, so their rows can show different rounds and reload progress. Ammo quantities are authored per mount: adding another identical gun adds another authored load. The only shared row is infantry's default gun (`squad` in the soldier's mount), whose copies keep independent firing cycles and have no reload pause. Its readout stays loaded while any surviving carrier is loaded. Suppression's tier widens launch scatter as well as slowing cycle progress; those penalties compose with movement and cover.

A replacement soldier brings a fresh default gun; a transferable special weapon keeps its identity, rounds and reload progress across handoffs. Living original carriers operate their own guns, survivors retain recovered assignments, and free soldiers take up remaining stocked weapons. One soldier operates one special weapon at a time. Additional recovered weapons remain spares: their reload pauses until an operator is available, while movement still interrupts stationary weapons. An empty launcher's operator finishes guiding its last missile before taking up a stocked spare. Guidance belongs to the physical launcher, so an eligible survivor can continue an in-flight missile; loss of support, movement, Stop or lost sight still releases it.

## Adding a unit type (a tank variant, a vehicle, an infantry type)

Add one entry. A variant is an `extends` and what differs. Code learns nothing about the type, and changes only where a genuinely new behaviour appears, as a new component or capability. Adding types has gone wrong before in the ways below. Each rule has a guard; if you add a type that the guard doesn't cover, extend the guard.

- **Never branch on a type's id or role for a rule.** If a function asks "is this a tank?" to pick an offset, a size or a speed, that value belongs in the type's components. Scripts and the AI may select units by role; rules never do.
  - **Seen before:** the weapon muzzle came from a per-kind match that returned the cannon's point for every tank mount. The roof HMG then fired from the cannon's tip, and when aimed sideways, from mid-air.
- **Every mount is its own weapon.** It has its own lock, bearing, pivot and muzzle. A mount on a turret pivots with the turret; a mount on its own ring (a roof HMG) turns on its own bearing about its pivot. Two mounts never share a muzzle point. A vehicle model declares which of its rigs draws each mount in its `assets/catalog.json` entry (`"mounts": { "cannon": "gun", "HMG": "hmg" }`); a mount's name decides nothing.
- **The simulation's numbers are the authority, and art is fitted to them.** Hull extents, eye heights, mount pivots and muzzles, the canopy and the footprint are set in data. Each type's own model is fit-checked against its own resolved numbers (the `fit.*` checks in `packages/scene-assets/src/validate.ts`). A variant with a new model inherits its parent's mount geometry, so the new model must put its turret and gun where the inherited mounts say, or the variant overrides those numbers. When you add a number with a drawn counterpart, add its `fit.*` check in the same change.
- **Presentation anchors to what is drawn, not to the simulation's points.**
  - Effects that belong to a model part (muzzle flashes, exhaust, dust at the tracks) sit on the drawn part's socket, and fall back to the published point only when nothing is drawn.
  - Ground markers are sized from the type's footprint (its hull), so they peek out from under the model.
  - The renderer skill has the mechanics (`.agents/skills/renderer/SKILL.md`).
- **Culling bounds cover every reachable pose:** a traversed turret, a deployed spade, a raised gun. Size them from the articulation's extremes, not the rest pose.
- **A vehicle needs its whole life cycle:** its `mobility`, its weight and push class (what it pushes, and what it's worth as cover, live or wrecked), its `wreck` (a prop row with hit points and a destroyed state, which may chain into a lighter wreck), and its `sound`. Missing one shows up late, as a vehicle that can't be pushed, or a wreck that never burns out.
- **Infantry** share one body frame: the soldier's radius, height, eye and muzzle heights, and the cover rules, are `physics` numbers here, not per soldier kind. A squad's soldier kinds share one skeleton, so their clips agree.

**The guards scale with the catalog.** Every resolved type runs the same generated checks with no test of its own: its components are complete and in range, it sets up, fires each mount and moves (`crates/sim/tests/catalog.rs`), its appearance exists and fits its numbers, and its icons exist. Then add the type to the lab or scene that exercises it: the workbench sheet for the model, and a movement scenario (`crates/sim/tests/movement_scenarios.rs`) for its movement and cover. Run the checks for whatever you changed (see [`AGENTS.md`](../AGENTS.md)).

## The prop catalog

A prop type (a house, a wall, a tree, a wreck, rubble) is **one entry** of a `props` section, in `props/<faction>/<family>.json`, addressed by its string id. No code lists prop types, and no rule asks which one a prop is. A map's props, a forest's trees (`forests.tree` in `village.json`), a bridge's deck (`deck` on each map bridge) and a vehicle's wreck all name types by id; the world layout and the publication send the id list (`propKinds`) and each prop's index into it.

- **A type is its body row, its destroyed state and its appearance binding:**
  - `body`: `blocks` (per mover class), `stops_rounds`, `occludes`, `weight_class`, `cover_tier`, `lifetime_s` (a transient body, like smoke), `conceals` (foliage), `hp` and `armor` (integrity), `topples` (falls rather than slides: a tree) and `garrison` (a squad can hold it from inside). Each column has its own readers; a rule reads columns, never the id.
  - `destroyed`, with `hp` and only with it: `"removed"`, `"cleared"` (open ground, for a toppling body) or `{ "into": { "prop": <id>, "height_m": h } }`, remains on the same plan. Chains end: a tank wreck burns down to a truck's and then a jeep's.
  - `appearance`: what draws it. `drawn_by` names the asset catalog's scenery kind whose appearances are fitted to its box (`building` for the building appearances, `forest` for the trees a forest draws itself); `modular` repeats a module along the box instead of stretching it; `map_only` marks a type a battle never leaves or places, so only the appearances a map uses load; `remains_state` draws remains as the body they replace, in that state (a building's ruin).
- **A variant is `extends` plus overrides,** as for units: `wrecks.json` shares one abstract `wreck` frame.
- **Resolution happens once, in the simulation,** with the units: unknown or looping destroyed states, `hp` without `destroyed`, a cleared state on a body that doesn't topple, and a unit's wreck that names no prop type fail at load, naming the entry. Regenerate `catalog.json` after editing, as for the units above.

## Adding a prop type (an obstacle, a vehicle's wreck, a city's building)

Add one entry. A mechanic comes from the body's columns, so a new obstacle needs no code: dragon's teeth are small heavy bodies that block vehicles, and a squad takes cover behind each one.

- **Never branch on a prop's id.** If code asks "is this a building?" or "is this a tree?", the answer belongs in a column (`garrison`, `topples`) or in the appearance binding (`drawn_by`, `remains_state`).
- **A unit type's wreck is its own prop type.** At hundreds of vehicles, give each family's wreck an entry extending `wreck`, with the vehicle's weight class and cover tier (a wreck keeps its vehicle's tier, checked at load), `hp`, and a destroyed state that chains into a lighter wreck or goes.
- **Every type needs an appearance that draws it:** a scenery kind in `packages/scene-assets` with a catalog appearance authored to a box the simulation places (the catalog footprint test holds every drawn kind to one), or `forest`. The renderer reads the binding from the world layout; it lists no prop types.
- Then add it to the map or scenario that exercises it, and run the checks for what you changed (see [`AGENTS.md`](../AGENTS.md)).

## Surface speeds

Every mover states its own two top speeds in its `mobility` row, in km/h: `offroad_kmh` on open ground and `road_kmh` on a full road (at most 130, and never below the off-road speed). `village.json`'s `surfaces` table has one row per surface kind a map may pave (`road`, `country_road`, `dirt_track`, `sidewalk`). A row's `speed_factor` scales each unit type's own road speed on that surface, never below its off-road speed: 1 is a full road, 0 is no road at all. A new surface kind is a new row plus its variant in `contract::map::SurfaceKind`.

## Parity oracles

`parity/` holds frozen inputs and expected outputs that the native tests and the web tests both read, so the Rust simulation and its WebAssembly build are held to the same answer: building aggregates, fog delivery, ground learning and transport, the map compiler, physical templates (including the rejected descriptors that must keep failing) and terrain queries. A file changes only with a named behaviour change, and every test that reads it changes in the same commit.
