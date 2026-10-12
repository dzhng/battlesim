> **Dropped (2026-10-11).** Phase 5 replaces Godot, so native's remaining GDScript is deleted with it rather than ported. See the README's phase 4 note.

# 21 — The rest of native's CPU side in Rust

**Contract:** no GDScript owns logic. What is left after 18–20 moves into the native core:
- the learned ground and scar texture;
- the frame stepper and camera rig;
- the environment's light derivations (sun colour, cascades, sky LUT bake);
- grass and terrain table uploads;
- the plate writer;
- the release script's run modes.

GDScript keeps only the entry scene.

**Focused gates:**
- `find native/godot -name '*.gd'` lists only the entry scene's script and display-test drivers that can't be Rust.
- Plates are byte-identical within the noise floor, and every counts gate is unchanged.
- Slice 12's bar holds on both scenes.
- CPU per frame is reported against the GDScript build's.

**Delegated:** whether display-test drivers stay as GDScript or move into Rust integration tests.
