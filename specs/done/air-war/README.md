# Persistent aircraft and layered air defence

Aircraft and air defence now share one persistent battlefield contract. Fixed-wing
units occupy `HighAir` at the authored cruise height; helicopters remain in
`LowAir`. Aircraft enter from a friendly map edge, circle, attack, dogfight,
burn fuel and withdraw. Ground AA is layered into short ballistic, medium
missile and long missile roles, with interception and anti-radiation authored on
weapon rows.

The feature exists because aircraft and AA are one interaction loop: a SEAD
missile exposes whether radar emitters can survive their first exchange, while
fighters, flares, fuel and dogfights decide what happens after that. Keeping
these mechanics in separate sortie and defence systems would make fog,
interception, reloads and targeting disagree. The hard cutover therefore removed
the old sortie compatibility path and made simulation, publication and browser
consumers read the same current state.

## Principles that must hold

- `HighAir` aircraft never reveal or clear Ground fog. They can sense airborne
  layers; ground AA can sense HighAir through authored target ranges but must
  have physical LOS through terrain, buildings and foliage to fire.
- Radar is an always-emitting, unobstructed 360° channel. Only SEAD aircraft
  detect signatures. Contacts stay approximate and carry bearing/range text;
  they do not identify or reveal ground units.
- Every faction keeps short, medium and long AA plus a dedicated SEAD platform.
  Interception is a property of a weapon row. Short guided Stinger/Igla rows
  remain non-interceptors; the ballistic short row may intercept incoming
  missiles.
- Seekers obey authored FOV, turn rate, lifetime and decoyability. Flares are
  finite physical decoys, and splash still reaches an aircraft near a diverted
  missile. Interceptor rounds meet incoming rounds through a small physical
  proximity envelope rather than the launcher's full engagement range.
- Aircraft auto-target by legal range and value, prioritising air when they have
  AA missiles. Explicit move/attack orders break dogfights; attack-move leaves
  the automatic engagement policy in place. Afterburner is a persistent player
  toggle trading speed and fuel for ground-attack accuracy.
- Aircraft fuel starts on map entry. Near-empty aircraft return to the friendly
  edge and receive the health/ammunition/fuel-weighted withdrawal refund.
- Aircraft have no ground circles. Their state lives in the existing floating
  cards; order ghosts carry compact cards while Space is held. The first
  HighAir episode raises one side-local alarm and the jet loop is positional.
- Downloaded audio sources remain provenance and audition material until their
  rights are accepted. Runtime assignments use safe generated placeholders.

## Code pointers

The contract types and admission rules live in
[`crates/contract/src/catalog.rs`](../../../crates/contract/src/catalog.rs) and
[`crates/contract/src/weapons.rs`](../../../crates/contract/src/weapons.rs).
Simulation ownership is split between `crates/sim/src/movement/air.rs`,
`crates/sim/src/sight.rs`, `crates/sim/src/knowledge.rs`,
`crates/sim/src/weapons.rs`, `crates/sim/src/flight/`, `crates/sim/src/damage.rs`
and the battle tick in `crates/sim/src/battle.rs`. Focused deterministic proof
for sensing, weapons, radar, guidance and protection is in
`crates/sim/tests/air.rs`, `air_weapons.rs`, `radar.rs`, `guidance.rs` and
`protection.rs`; catalog and weapon admission are covered by the contract tests.
Dogfight joining/breaking and afterburner speed, fuel and accuracy are
implemented but do not yet have dedicated behavioral tests.

Publication and side filtering remain in `crates/sim/src/publication.rs` and
`crates/presentation/src/presentation.rs`. The browser card, ghost and radar
surfaces are owned by `web/src/battle/present/`, `apps/battle-lab/src/unitGhosts.ts`
and `packages/battle-renderer/src/orderOverlay.ts`. Sound causes and side-local
alarm state are in `crates/presentation/src/sound/` and
`packages/battle-audio/`; source receipts are kept in `fixtures/sounds.json` and
`assets/third-party/audio/`.

The air checkpoint URLs are registered in
`apps/battle-lab/src/fixtures.json` and share the production air scene through
`apps/battle-lab/src/routes/airWar.tsx`. No approved screenshot baseline was
provided for this feature. The browser preview loaded the routes and WASM but
could not render WebGPU in the closeout environment (`No available adapters`),
so native deterministic tests remain the visual seam's available proof until a
GPU-capable runner captures those checkpoints.

## Deliberate dead ends

The original immediate fixed-entry sortie model was not kept as a compatibility
layer. Radar was not folded into ordinary fog because that would reveal too much
or make SEAD impossible to reason about. Dogfights were not represented as a
special group object: pair links naturally admit additional aircraft and break
when range or explicit orders change. Unlicensed downloaded clips were not
assigned to runtime recipes.
