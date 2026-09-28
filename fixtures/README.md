# Fixtures

`village.json` is the one owner of the game's rules and look numbers. Labs and scenes reuse it. `units/` is the unit catalog: every unit type, soldier kind, role and upgrade part. `biomes/` holds the terrain palettes. A number that changes how the battle plays or looks belongs here, validated by the module that reads it, never as a constant in code.

## The unit catalog

A unit type is **one catalog entry**, addressed by its string id (`"tank"`, later `"m1a2_sepv3"`). No code lists unit types. Scenarios, spawn rows and commands name types by id; the publication sends the id list (`unitKinds`) and each unit's index into it.

- **Files:** `units/<faction>/<family>.json`, one family per file, plus `units/roles.json` (the role registry) and, when parts exist, a parts file. Each file is an object with any of four sections: `roles`, `parts`, `soldiers` and `units`. An id is defined once across all files.
- **A type is its components.** Behaviour comes from them, never from the id:
  - `body`: `{ "squad": { "slots": [soldier kinds] } }` or `{ "hull": { half_extents_m, eye_m, hp, armor, weight_class, push_class, wreck } }`;
  - `mobility`: `foot`, `tracked` or `wheeled`, each with its own speeds and turning;
  - `sensors`: the one sight (`ground_m`, `sight_shape`, and `on`, the turret mount the optics turn with);
  - `mounts`: a hull's weapons, each row with its carrier (`on`), `pivot_m` and `muzzle_m`. A squad's come from its soldiers;
  - `capabilities`: optional abilities such as `deploy` and `supply`;
  - `roles` (what scripts and the AI select by), `cost`, `sound`, `name`, `description`, `faction`, `family`, and a hull's `appearance`.
- **Soldier kinds** are a catalog of their own: `hp`, the `appearance` set a soldier of the kind wears (one picked per soldier), and the `mounts` he carries. A `special` mount passes to the next living soldier when its carrier falls; any other is lost with him. A squad's slots name soldier kinds, so hundreds of squads reuse a few kinds.
- **A variant is `extends` plus overrides.** `"m1a1": { "extends": "m1", "body": { "hull": { "armor": { "front": 180 } } } }` inherits everything else. Objects merge key by key, a list of named objects (mounts) merges by name, and anything else is replaced. An `abstract` entry only exists to be extended. Weapon rows in `village.json` extend the same way.
- **Parts are upgrades:** `"parts": ["trophy_aps"]` merges each part's `patch` into the type after inheritance. A part that needs a capability the simulation doesn't build yet is refused at load, because the type no longer parses. A part names the model nodes that show its hardware (`nodes`), and the type's model must draw them.
- **Resolution happens once, in the simulation** (`contract::catalog`): cycles, unknown parents, roles, soldiers or parts, and incomplete types fail at load with an error naming the entry. The browser reads the resolved view, `unit-catalog.json`. After editing the catalog, regenerate it: `BLESS_CATALOG=1 cargo test -p sim --test sim catalog::` (the test fails while it is stale). Then regenerate the icons (each type's silhouette is rendered from its baked model): `bun run --cwd web asset -- icons`.

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

**The guards scale with the catalog.** Every resolved type runs the same generated checks with no test of its own: its components are complete and in range, it sets up, fires each mount and moves (`crates/sim/tests/catalog.rs`), its appearance exists and fits its numbers, and its icons exist. Then add the type to the lab or scene that exercises it: the workbench sheet for the model, and a slice-30 movement scenario for its movement and cover. Run the checks for whatever you changed (see [`AGENTS.md`](../AGENTS.md)).
