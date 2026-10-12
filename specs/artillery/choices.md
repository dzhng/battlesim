# Artillery choices ledger

The user-set decisions are recorded in the feature README. This ledger holds
implementation choices made where the plan left the mechanism open.

## Sound

### Static artillery source meshes use fixed hull mounts with explicit visual overhang tolerance

The artillery GLBs contain complete static vehicles but do not contain the
renderer’s generic `turret`/`gun`/`muzzle` articulation chain. The physical rows
therefore keep a real muzzle for simulation and mark the mount fixed for asset
fit, while the appearance entries allow the source model’s long barrel or
launcher to overhang the compact collision hull. A fabricated rig would claim
the model can animate when it cannot; widening the collision hull would make
barrels block movement. The asset fit gate still judges every source and wreck.

Gap: the physical roster slice delegated source-specific fit decisions.

Reach: future artillery art can add a real rig and remove the tolerance without
changing weapon, command or AI data.

Verdict: sound. Confidence: high.

### AI artillery guesses are deterministic, bounded area-biased ground orders

When an artillery unit has no suitable identified target, the policy chooses the
nearest fresh ground contact that lies between the unit’s indirect weapon
minimum and maximum range. It extrapolates a matching observed velocity for at
most three seconds, then adds a stable offset inside the contact’s reported
area. The command is ordinary `Attack` with `TargetRef::Ground`, so the battle
authority still resolves terrain, collision, minimum range and hold-fire.

Gap: the plan delegated evidence scoring, horizon and seed salts.

Reach: changing the guess personality changes only this pure policy helper; it
does not widen observations or introduce hidden-state access.

Verdict: sound. Confidence: high.

### Attack Ground emphasis is derived from weapon trajectory metadata

The command bar adds an inset accent border only when a selected unit’s mounted
weapon row says `trajectory: "indirect"`. Mixed selections keep the existing
reach rule and the same command, shortcut and serialization. A role or unit ID
would drift from the catalog, so the UI reads the session weapon rows instead.

Gap: the visual slice delegated the exact existing focus treatment.

Reach: new indirect weapons automatically receive the cue; no new command or
range visualization is needed.

Verdict: sound. Confidence: high.

### The artillery lab reuses the production flight bench

The focused artillery route points at the existing ballistics lab and swaps its
synthetic indirect probe for the shipped mortar row. This keeps the lab’s ridge,
collision, spread and impact assertions on the same WASM flight owner instead
of creating a second artillery scene harness.

Gap: the plan required a focused lab but did not require a new renderer scene.

Reach: future projectile presentation tests can extend one bench and keep its
deterministic evidence in one place.

Verdict: sound. Confidence: medium.

