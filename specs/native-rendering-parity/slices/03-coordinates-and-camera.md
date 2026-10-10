# 03 — Coordinates and camera

**Contract:** native draws each recorded frame from the browser's eye, in the same unmirrored world. This is where slices 01 and 02 join: the first real browser/native pair.

**Seam:**
- **Coordinates:** `core/coords.gd` holds `sim_to_godot(v) = (x, z, −y)` and a yaw-to-basis helper. Every placement in `legacy_map_layer.gd`, the sun and ring meshes goes through it:
  - the road angle sign (`reel.gd:333`) flips;
  - the `rotation.y = yaw` sites become correct as they are;
  - ring meshes get normalised winding and up normals.
- **Camera:** `reel/camera_rig.gd` provides `apply(record, lens)`.
  - It calls `look_at_from_position(eye, target, up)` after conversion.
  - `fov = rad_to_deg(lens.fovY)` with `KEEP_HEIGHT`, and `near = lens.near`.
  - Godot can't use an infinite far plane, so `far` is the map's diagonal extent plus `backdrop.reach_m`, recorded in the report as a budget.

**Runnable artifact:** paired plates for every plate frame of paris-corner and market-town, with the browser and native plates side by side on a contact sheet.

**Visual variable:** framing only. The same shot means the same subject, the same side of frame and the horizon in the same place, by eye.

**Focused gates:**
- A headless test converts asymmetric known points; its determinant is +1, and the result matches the browser's handedness. It is a regression test, not a pixel budget.
- Every plate is exactly 1280×720.
- Digest and replay are unchanged.
- Run [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md) on each pair, then unprimed [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md) on the native plates as the last check.
- Open the contact sheet with [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md) for a non-blocking review. Wait about 5 minutes; if nobody answers, decide on the evidence, record it, close the shots and continue.

**Out of scope:** colour, light, terrain look, grass, models and effects.

**Delegated:** the contact-sheet layout.

**Feedback that would change it:** a shot that reads differently even though the framing matches, which would point at the lens (FOV or aspect) rather than the pose.
