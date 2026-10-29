# Air-defence radar networks

This idea is closed as a superseded, deferred follow-up. The shipped [air-war
record](../air-war/README.md) owns radar emission, SEAD contacts, layered air
defence, interception, and persistent aircraft. The separate network proposed
here—launchers that are blind at long range until a distinct radar vehicle
finds the target—was not implemented and is intentionally deferred because it
would add another battery-coordination mechanic.

## What the shipped contract covers

Radar is an authored capability on a unit. `capabilities.radar_emitter` makes
it a radar signature that a radar receiver, such as a SEAD aircraft, can
detect; its high-air reach is its own authored `sensors.high_air_m`, which
emitting does not change ([unit concealment](../unit-concealment/README.md)). The receiver gets an approximate
bearing and range; it does not identify the emitter. Side knowledge remains the
only route from a sighting to a legal shot.

The current owners are:

- `crates/contract/src/catalog.rs` for `radar_emitter` and `RadarReceiver`;
- `crates/sim/src/knowledge.rs::update_radar` for radar contacts;
- `crates/sim/src/sight.rs` and `crates/sim/src/weapons.rs` for layer sight and
  legal engagement;
- `crates/sim/tests/radar.rs`, `air.rs`, and `air_weapons.rs` for deterministic
  proof;
- [air-war](../air-war/README.md) and [air-war-flight](../air-war-flight/README.md)
  for the player-facing aircraft and air-defence rationale.

## The deliberate boundary

Radar-emitting SAMs author their own radar emission and high-air reach; other
AA launchers may use optical or shorter authored air sight. There is no
separate radar vehicle that supplies a launcher's sight, no launcher-blindness
rule, and no battery purchase or loss behavior. That is the important
divergence from the original idea. If the game later needs that counterplay,
it should start as a new mechanics decision and read the shipped radar and
side-knowledge owners rather than revive this plan.

There is no visual provenance for the deferred network concept; the air-war
record contains the shipped radar and SEAD evidence.
