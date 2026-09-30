# Independent fire and magazines

Infantry fire should read as soldiers fighting individually, not a synchronized volley. Target selection remains a squad-level decision; firing cadence belongs to each physical gun. This keeps the existing command and observation boundaries while letting each rifle fire and reload independently.

## Invariants

- Each burst samples its own bounded aim delay from the replay-seeded simulation RNG. Re-aiming spreads firing and reload phases over time without a squad firing schedule. This models sustained, overlapping fire, not a guarantee that every instant has a shot.
- Simulation launches drive projectiles, flashes, recoil and sound. Presentation never invents delayed shots.
- A rifle's cycle follows its soldier ID. Casualties do not reset other rifles, replacements get fresh guns, and a transferable special weapon keeps its cycle on handoff.
- Magazine capacity is separate from reserve ammunition. Positive reload times require magazine changes even with unlimited reserves; infantry rifles deliberately use zero-time refill so the shared readout never cycles between individual soldiers; finite ammunition counts actual rounds launched. A blocked gun spends nothing and accumulates no catch-up volley.
- Orders preserve remaining magazine contents. Existing stop and stationary-movement rules still interrupt unfinished reloads. Timing completes on simulation ticks, with at most one launch per gun per tick.
- Suppression widens physical launch dispersion as well as slowing cycle progress. Movement and target cover compose with it once. A soldier's own cover remains protected whether it is a prop or a vehicle hull.
- A single mount readout describes the group: loaded while any gun has a loaded magazine, earliest refill progress only while all are empty. Short inter-shot cycling is not a magazine-reload ring.

## Owners

[`WeaponDefinition`](../../../crates/contract/src/weapons.rs) owns the optional magazine contract; [`SuppressionTierRules`](../../../crates/contract/src/scenario.rs) owns the scatter multiplier. Values belong to [`village.json`](../../../fixtures/village.json). [`Cycle`](../../../crates/sim/src/weapons/cycle.rs) owns physical weapon timing; [`weapons`](../../../crates/sim/src/weapons.rs) owns shared targeting and launch eligibility. The packed observation adds a `primaryLabel` flag per contact so presentation shows one report label per enemy, preferring visual memory to hearing without exporting hidden identity. Replay/digest state includes each physical cycle.

Tracer world widths and pixel floors are distinct controls. Each core/glow layer owns its floor, so thinning rifles need not change HMGs or other effects. Matched native tactical/close captures and a crowded gameplay filmstrip judge readability; enlargements expose residual diagonal raster aliasing without confusing it with native visibility.

[`choices.md`](choices.md) records scope calls, validation and battle-outcome changes. The [user's report](assets/reference/user-rifle-volley.png) is a native game-viewport crop of the September 29, 21:15 screenshot, with browser chrome removed. It records the problem, not a target to match, and never ships in the game. The opt-in `cadence` tour in [`village-watch`](../../../web/scenes/village-watch.mjs) reproduces the gameplay review surface.
