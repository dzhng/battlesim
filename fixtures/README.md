# Fixtures

`village.json` is the one owner of the game's rules and look numbers. Labs and scenes reuse it. `biomes/` holds the terrain palettes. A number that changes how the battle plays or looks belongs here, validated by the module that reads it, never as a constant in code.

## Adding a unit kind (a tank variant, a vehicle, an infantry type)

A unit kind is **data first**. Most of it is rows in this fixture: its body row, its physics and sensor numbers, its mounts, and its weapons. Its appearance is an entry in the asset catalog. Code learns the kind only where a genuinely new behaviour appears. Adding a kind has gone wrong before in the ways below. Each rule has a guard; if you add a kind that the guard doesn't cover, extend the guard.

- **Never branch on the kind's name for a number.** If a function matches on `UnitKind::Tank` to pick an offset, a size or a speed, that value belongs in the kind's row. The same test applies to reviews: a new `match` arm on a kind is a smell.
  - **Seen before:** the weapon muzzle came from a per-kind match that returned the cannon's point for every tank mount. The roof HMG then fired from the cannon's tip, and when aimed sideways, from mid-air.
- **Every mount is its own weapon.** It has its own lock, bearing, pivot and muzzle. A mount on a turret pivots with the turret; a mount on its own ring (a roof HMG) turns on its own bearing about its pivot. Two mounts never share a muzzle point. Mounts are listed per kind here, each row with its carrier (`on`, an earlier turret mount; absent, the hull), its `pivot_m` in the carrier's frame and its `muzzle_m` along its own bearing; the weapons they fire are rows of their own.
- **The simulation's numbers are the authority, and art is fitted to them.** Hull extents, eye heights, the muzzle arc, the canopy and the footprint are all set here. The appearance validator's `fit.*` checks (in `packages/scene-assets/src/validate.ts`) hold every model to them within the catalog's tolerances. When you add a number with a drawn counterpart, such as a new mount's muzzle, add its `fit.*` check in the same change, or model and simulation will drift apart silently.
- **Presentation anchors to what is drawn, not to the simulation's points.**
  - Effects that belong to a model part (muzzle flashes, exhaust, dust at the tracks) sit on the drawn part's socket, and fall back to the published point only when nothing is drawn.
  - Ground markers are sized from the kind's footprint, so they peek out from under the model, which covers everything from a jeep to a heavy tank.
  - The renderer skill has the mechanics (`.agents/skills/renderer/SKILL.md`).
- **Culling bounds cover every reachable pose:** a traversed turret, a deployed spade, a raised gun. Size them from the articulation's extremes, not the rest pose.
- **A vehicle kind needs its whole life cycle:**
  - how it drives (tracked or wheeled, turning, reverse speed);
  - its weight class (what it pushes, and what it's worth as cover, live or wrecked);
  - its wreck, a prop row with hit points and a destroyed state, which may chain into a lighter wreck;
  - its sound and loudness.
  
  Missing one shows up late, as a vehicle that can't be pushed, or a wreck that never burns out.
- **Infantry kinds** share one skeleton per unit so variants can mix (`id mod n`). Their sight and muzzle heights, and the cover rules, are the infantry numbers here, not per model.

Then add the kind to the lab or scene that exercises it: the workbench sheet for the model, and a slice-30 movement scenario for its movement and cover. Run the checks for whatever you changed (see [`AGENTS.md`](../AGENTS.md)).
