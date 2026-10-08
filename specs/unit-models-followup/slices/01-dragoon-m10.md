# 01 Dragoon hull and M10 turret

**Unlocks:** the Stryker family reads as a Stryker from the side, and the M10
Booker stops reading as a small Abrams.

- **Stryker (all four variants, and the support cards on its hull):** the
  photos show a pronounced chine: a lower hull that flares out over the
  wheels to a sharp knuckle, then upper sides leaning well in to the roof, and
  a nose of bolted applique plates. The 2026-10-08 sloped-panel pass gave the
  upper side a 16° lean that doesn't read at sheet distance because the
  leaning band is only ~0.5 m tall. Rebuild the hull section from the
  references so the chine and the lean read at a glance (the knuckle lower and
  stronger, the lean taller), inside the same frame (±0.1 m fit, dressing
  rules), with the bolted nose plates and side tiles lying on their faces
  through `vehicle_parts.on_side`.
- **M10 Booker:** its photos show a taller, blunter turret than the model's
  Abrams-like wedge: rebuild the turret shape from the references, keeping
  its mounts.

**Verify:** re-export twice, byte-identical; `asset validate`; bake, icons,
check; `asset sheet --references` before/after for the Dragoon, M1126 and
M10; compare-screenshots against the references; adversarial critique last.

**Delegated:** part sizes within the references and frame.
