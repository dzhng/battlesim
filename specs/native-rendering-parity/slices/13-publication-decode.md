# 13 — Full publication decode

**Contract:** native decodes every group and section of the capture's publication, exactly as the browser's observation decoder (`web/src/battle/sim/observation.ts`) does. Today it keeps only the `own` unit group.

**Seam:**
- `native/godot/capture/presentation_capture.gd`'s `decode_publication` reads every group in the layout (`crates/sim/src/publication.rs` `layout_json`):
  - `own` and `identified`, with their `members`, `memberIds`, `memberLeans`, `memberHp`, `weaponPoses` and velocity sections;
  - `projectiles` (with `path` and `ricochets`), `blasts`, `crashes`, `corpses`, `knownProps`, `fallenBodies` and `audible`;
  - the ground runs, including the `cleared` byte that `learned_ground.gd:44` masks today.
- The output is one decoded observation per sample, shaped like the browser's. The publication layout's own names are the field names.

**Runnable artifact:** none visual. A decode dump of each sample beside the browser's decode of the same sample.

**Focused gates:**
- A fixture test compares native's decode of every sample in the paris-corner fixture capture against the browser's `observation.ts` decode of the same words, exported as JSON by a vitest. Every group and field must be equal.
- A layout with an unknown group, a short payload or a bad encoding is a named blocker.
- Digest and replay unchanged.

**Out of scope:** drawing anything new.

**Delegated:** native's internal record shapes.
