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

## Added 2026-10-08: the Abrams

User: "the tank used for the main menu video also have a front that seemed to
be sloped to a point too much, and the side panels also hangs off". The menu's
blue tank wears `us_m1_abrams_sep_v2`. Against the references
(`assets/references/abrams`), the model's turret front closes to a sharp
wedge where the real turret has a broad, blunt face with flat angled cheeks,
and the side skirts stand off the hull instead of sitting on it. Fix all three
Abrams appearances (SEPv2, SEPv2 Trophy, SEPv3 Trophy) and their wrecks: the
turret's front and cheeks from the photos, skirts flush on the hull sides
(through `on_side` where a face leans). Same frame and mounts. Then check
the menu reel's frames (the menu's tank keeps the test hull, slice 08
choices): the exact-shot test must still pass, since art doesn't move events.
