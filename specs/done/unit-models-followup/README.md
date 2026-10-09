# Unit models follow-up

Work the user decided on 2026-10-08, after [unit models](../unit-models/README.md)
closed. Every invariant of that spec still holds; its
[choices](../unit-models/choices.md) record each decision below under
"Follow-up 01–04", "Abrams glacis" and "Names and weapon icons".

## What and why

- **Hulls follow their real faces.** Parts bolted to a leaning hull face lie on
  it (`vehicle_parts.on_side`); the Stryker has its chine and leaning upper
  sides, the Abrams a long flat glacis with flush skirts and blunt turret
  cheeks, the M10 its tall blunt turret. The user saw flat panels hanging off
  sloped hulls and an Abrams front that dipped.
- **Wreck debris is presentation, never cover.** A wreck's thrown pieces are
  their own `debris` state (`wreckage.scatter`, `<appearance>_wreck_debris.glb`),
  held to a loose scatter allowance instead of the hull footprint, drawn only
  for a death the side watched, and sunk after a hold
  (`effects/cookOff.ts` `debrisSink`, `presentation.effects.cook_off.debris`).
  The simulation's wreck and its cover are unchanged. The user: debris may
  scatter wider "and just disappear".
- **Disabled cards meet the roster bar.** They will be made playable next, so
  every one (aircraft, helicopters, artillery, air defence, drones, ground,
  infantry) is built to the same detail as roster units, judged side by side
  against a roster exemplar.
- **Labels are authored, never derived.** Units, families, mounts, weapons,
  protection and planned weapons carry concise player-facing names apart from
  their ids, held by one rule (`contract::labels`), and every weapon a card
  shows names an icon drawn in the generated set; either missing fails catalog
  load. No UI truncation, underscore replacement or fallback icon.
