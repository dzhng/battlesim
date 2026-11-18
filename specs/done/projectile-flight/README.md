# Projectile flight and visibility

Projectiles keep one engagement and visibility band from launch through their
explosion. Missiles get one homing attempt and remain physical rounds after it
ends. These rules prevent a missed missile from orbiting its last aim point and
keep an airborne weapon's explosion visible to the same kind of sensing as its
flight.

## Why a fixed band

The band describes the engagement, not the projectile's instantaneous height.
Crossing height thresholds would make a missile change its defensive matchups
and visibility repeatedly during one shot. A ground-launched missile engaging a
jet therefore belongs to HighAir from launch; a jet's ground attack also stays
HighAir through impact. Deliberately indirect and top-attack ground fire belongs
to LowAir. Low-arc grenades, RPGs and ordinary bullet drop stay Ground by the
user's explicit choice.

The rule uses existing physical flight authoring rather than weapon names or a
new lob category. Interceptability remains a weapon property; active protection
is a capability of the defending unit. Assigning an air band does not make every
shell interceptable.

## Why one attempt

A missile that passes its aim or loses tracking cannot start another chase.
Erasing it would also erase a close physical hit, so loss of homing leaves
ordinary motion, collisions, fuses and lifetime active. Existing release
coast/descent supplies the miss behavior without another timeout or tuning law.
Autonomous seekers remain independent of launcher movement and sight.

A top-attack climb is an authored path. Its tracking cone follows the planned
climb/dive aim, respecting the authored climb rather than rejecting the tank
beneath it. Target motion still supplies the lead used for steering. The tradeoff
is recorded first in the [decision ledger](decisions.md).

## Knowledge must follow the projectile

Air sight can reveal a path and its explosion over unseen ground without
clearing that ground or identifying launchers, casualties or hidden units.
Existing tracer kind and shooter-member metadata remain unchanged. The assigned
band must survive even when a projectile is consumed on its first advancement,
before observation. Ground-band fog and aircraft-crash explosions retain their
existing rules. Expiry creates no new blast.

An impact may round just inside its collision surface. Visibility samples the
existing physical blast origin outside that surface while publishing the actual
impact coordinate. This shares damage's geometry convention instead of adding a
sensor exception or a height threshold.

## Owners and regression proof

- [Launch and tracking](../../../crates/sim/src/battle/fire.rs) owns fixed bands
  and target ownership. [Flight](../../../crates/sim/src/flight/) owns physical
  advancement and the irreversible end of steering.
- [Flown evidence](../../../crates/sim/src/battle/flown.rs) retains band and burst
  samples; [observation](../../../crates/sim/src/battle/observation.rs) decides
  what the side receives. The public observation and wire layouts are unchanged.
- [Guidance regressions](../../../crates/sim/src/flight/guidance_tests.rs) and
  [battle interception regressions](../../../crates/sim/src/battle/interception_tests.rs)
  pin no turnback, no reacquisition, autonomous loft and live close impacts.
- [Visibility regressions](../../../crates/sim/src/battle/projectile_visibility_tests.rs)
  pin blind-band isolation, warnings, same-tick bursts and no ground reveal.
  The [browser scene](../../../web/scenes/air-survivability.mjs) drives the real
  battle view with an enemy bomb over unseen ground.

The deterministic regressions exercise the stale-point mechanism that allowed
circling and pin its corrected behavior. They do not identify the exact missile
or replay the original reported battle.
A still frame cannot prove an entire flight path.

## Visual provenance

The [original reporter screenshot](reference/reported-circling.png) was supplied
with the circling-AA-missile report. It establishes the player's strategic
framing and observed symptom, not a replay or a known weapon identity. Review
used matched before/after captures at fixed ticks, both wide and at a comparable
strategic angle and zoom; the map and event are different. Generated captures,
comparison measurements and logs are disposable evidence in `throwaway/`.
