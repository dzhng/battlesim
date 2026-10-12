# Presentation

The per-frame presentation every client shares: one side's publications in,
a `Frame` of flat records out ([the presentation API](../../specs/native-rendering-parity/slices/18b-presentation-api.md)).
The browser runs it through [its WebAssembly binding](../presentation-wasm/README.md);
native adopts it when it moves to `wgpu`. Rust is as smart as it can be and a
renderer as dumb: a renderer draws the records it is given and decides nothing
per entity.

It reads only the side's publication, so it never sees past fog of war, and it
never touches the simulation, so tuning it never moves a digest or a replay.
`frame()` is a pure function of the world it loaded, the publications pushed and
the frames asked for: no wall clock, no unseeded randomness. Its seeds are the
browser's (Mulberry32 over FNV-1a of keys that embed JavaScript's number text,
[`seed.rs`](src/seed.rs)), so a dust puff or a tracer's chance falls the same
wherever it runs.

## What it owns

- [Records](src/records.rs): every record a renderer reads, defined once;
  `records_are_current` generates `packages/battle-renderer/src/records.gen.ts`.
- [Units](src/units/): interpolation between ticks, the pose feed and driver,
  articulation, joint palettes (and each skinned row's rest palette: a corpse
  as it lies, a body in its far pose, which a renderer poses its mesh by once),
  and where each drawn model's muzzles and rotors are.
- [Effects](src/effects/): the effect publication, launches (the one derivation
  the flashes, the firing soldiers and the gunfire sounds read), the effect frame,
  the cast lights it chooses for the view, and cook-offs.
- [The view](src/view.rs): what the camera means for culling and detail, the one
  tier rule every drawn model takes, and the frame's `View` record with the
  sky's corner rays.
- [The light](src/light.rs): the fixture's light resolved into the sky's,
  bloom's and grade's uniform records, as the browser's sky model resolves it.

## How a battle looks

Soldier poses follow actual published soldiers rather than formation slots. A
soldier down in a posture, or shooting, keeps his weapon toward the threat as he
moves and steps, crouches or crawls the way he goes, with a clip for every way;
otherwise, and at a run, he turns to face where he is going. No posture slides
along the ground. His gait reads his speed, ground over time, so while the clock
stands still (paused, or between ticks) he holds mid-stride and resumes the same
step. A soldier dies from the posture he is in, never getting up to
fall: standing, most are thrown onto their backs and the rest pitch onto their
faces; kneeling or lying, they fall forward. His corpse lies as his fall ended,
and past the corpse cap the oldest sinks away.

An effect on a barrel follows the drawn muzzle; its physical launch remains the
simulation's event. A shot kicks its shooter as a war film shows it, harder
than life: a heavy gun throws its hull back on its springs, away from where the
round went, to rock past rest and settle without moving off its spot, and shakes
dust off the whole hull and off the ground ahead of the muzzle; each of a
soldier's own shots snaps his upper body back about his waist, his muzzle
climbing. How hard each round kicks is its row in `presentation.pose`
(`hull_kick`, `soldier_kick`) and its flash's `dust`. The whole-model motion
carries the drawn muzzle. A flying aircraft's rotor is drawn by the effects from the
same drawn pose, as a film camera's shutter sees it: each blade smeared over the
angle it turned while the shutter was open, so a fast rotor blurs at any frame
rate and a paused one shows its blades.

A hull the side watched die cooks off: fireballs from its ring, its hull jolted
and its turret thrown, both pieces of its own unit's wreck, until they lie where
the wreck has them and the whole wreck takes over without a seam. A wreck without
detachable pieces jolts as one body. What the blast throws clear lies still for a
while, then sinks into the ground and is gone; a wreck found later has none.
Every watched vehicle keeps its last drawn look until the blast and follows its
roll to the resting wreck.

## Checks

`tests/poses.rs` and `tests/effects.rs` hold it to the browser's recorded vectors
(native's fixture capture and fixed effect causes); the behaviour tests beside
them cover what those do not reach.
