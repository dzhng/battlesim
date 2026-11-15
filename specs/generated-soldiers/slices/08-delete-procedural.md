# 08 The procedural path deleted; test soldiers on pins

## Contract

The generated path is the only path. A missing pin is an error. The
procedural clothing, kit builders and look tables are gone. The 12 test
soldiers wear existing pins.

## Seam

**Deleted:**
- From `infantry_kit.py`, the shell and kit builders:
  - clothing and kit: `uniform`, `boots`, `soles`, `plate_carrier`,
    `pack_straps`, `belt`, `on_vest`, `on_belt`, `pouch`, `belt_pouch`,
    `chest_rig`, `molle`, `markings`, `hip_kit`, `hydration`, `pack`;
  - head: `head_gear`, `ring_on`, `band`, `helmet_band`, `brim`, `balaclava`,
    `armour_collar_and_groin`, `scarf`, `skin`;
  - their shape helpers: `tubify`, `fill_legs` and `LEG_FILL`, `bump`, the
    local `smoothstep`, `add_rim`, `convex_hull_2d`, `bvh_of`;
  - the look tables: `LOOKS`, `KIND_LOOKS`, `ARMY_DEFAULTS`, `look_of`,
    `grime`, and the palettes the gloves don't use.
- From `roster/infantry_equipment.py`: the procedural branch, the grenades
  block (dead since generated bodies), and `PROCEDURAL_LOOKS`.
- The `None` path of `skin_pins.pin_where` for infantry: the exporter calls a
  `required` lookup that fails, naming the key and the `skin-generate` command
  to run.
- `textures_uniforms.py`, its import in `textures.py`, and every recipe nothing
  reads any more, found by grep. `cloth_mean` lives in `armies.json` since
  slice 01, so the regrade no longer needs the print recipes.

**Kept, in `Kit`:**
- materials for gloves, weapon, lens, launcher, webbing and marking;
- `slim_body` (the weight source) and `shell` (gloves);
- `rigid`, `weapon`, `eye`, `paint_all`, `join`, `bake_ao`, `back_y`;
- `bone_of` and `fold_leaf_weights`.

**One soldier export:** a single `export_soldier(rig, kit, path)` is shared by
the roster and the test build, so tiers, UVs, sockets and export have one
path.

**Test soldiers:** a constant in `infantry_kit.py` maps each to a pin.

| Test soldier | Pin |
|---|---|
| `rifle`, `at`, `at_carried` | `us_army_ocp` rifle, looks `a`/`b`/`c` |
| `recon` | `us_army_ocp` recon, looks `a`/`b`/`c` |

They keep their test weapons and holds, and buy no new bodies.
`build_sources.sh` builds all 12; today it rebuilds only `at_carried`. The test
jeep reads `assets/source/test/rifle.glb` (`jeep.py:281`) and is re-exported
too.

**Docs:** the Blender README's infantry section becomes generated bodies only,
dropping "can instead wear" and the cloth-shell method. The method docstring
in `infantry_kit.py` goes with the code.

## What you can run

`grep` finds no import of a deleted name anywhere. Exporting any kit look
with no pin fails with the message that names its key.

## Verification

- `asset validate` has no findings on all 12 test soldiers and the test jeep.
- The Blender `*_test.py` scripts, the scene-assets tests and `asset check`
  pass.
- **One test-launcher shot:** the test `at` and `at_carried` sheets,
  compare-screenshots against their current review baselines
  (`assets/review/at_carried*`), then screenshot-critique.
- The size ratchet passes; `infantry_kit.py` shrinks a lot.

## Decisions delegated

How the kept helpers are arranged, and whether `infantry_kit.py` is renamed
for what it now holds (for example `soldier_assembly.py`). If it is renamed,
every import moves in this slice, with no shim.

## Must stay green

Every slice 07 probe export, now built by the cut-down code, is byte-identical
to the same export before the deletion.

## Feedback that would change it

None expected. The user decided the deletion.
