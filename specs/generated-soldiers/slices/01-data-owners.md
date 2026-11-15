# 01 One owner each for pins, army data and source paths

Byte-identical refactor. Nothing anyone sees changes.

## Contract

- **Every generated skin is read through one verifier** (`skin_pins.py`), and
  every receipt names the skins its export wore. Today two copies do this:
  `aircraft_skin.pinned` and `WORN` (`aircraft_skin.py:58-74`), and
  `generated_body.pin_of` and `_verified` (`generated_body.py:65-80`). The
  infantry receipt still calls every look "Original procedural kit"
  (`roster/infantry_equipment.py`, `receipt`).
- **Army data lives in `roster/armies.json`**, read by Python now and by the
  Node generation tool in slice 04.
- **A kit look's source path is named by army id, not faction.** Today
  `source_path` prefixes a non-first army with its faction, so two Europe or
  two Eastern armies would write the same file (`infantry_equipment.py:537-542`).

## Seam

- **`packages/scene-assets/blender/skin_pins.py`** (new) exposes:
  - `pins(family) -> list[dict]`;
  - `pin_where(family, **key) -> dict | None`;
  - `verified(pin) -> str`, the absolute path. It refuses LFS pointers and
    hash mismatches, and records `WORN[glb] = sha256`.
  - `aircraft_skin.pinned`, `generated_body.pin_of` and
    `generated_body._verified` are deleted; their callers use it.
  - `vehicle_export._receipt` and the infantry `receipt` both read `WORN`.
  - The infantry receipt's `authoring` text names the generated bodies it
    wore.
- **`packages/scene-assets/blender/roster/armies.json`** (new) has this shape:
  ```json
  {
    "kinds": {"rifle": {"cue": "…"}, "recon": {"cue": "…"}},
    "studio": "…the fixed studio frame clause…",
    "armies": {
      "us_army_ocp": {
        "faction": "us", "rifle": "m4a1", "lmg": "m249",
        "gloves": [0.06, 0.05, 0.036],
        "cloth_mean": [0.13…, 0.11…, 0.07…],
        "prompt": "…base description…",
        "looks": {"a": "…", "b": "…", "c": "…"}
      }
    }
  }
  ```
  - `cloth_mean` is the exact float `repr` of today's
    `textures.baked(print).mean()`. `generated_body._surface` reads it in place
    of the recipe, so the regrade no longer depends on the print recipes
    slice 08 deletes.
  - `prompt`, `looks` and the kind cues are written here as text. The look
    clauses come from today's `ARMIES` variants (README, the look scheme).
    Slice 04 composes them.
  - Python `ARMIES` keeps only what the procedural shells still read, renamed
    `PROCEDURAL_LOOKS` so it reads as the legacy owner slice 08 deletes. `KITS`
    stays in Python; it is the exporter's own matrix.
- **`packages/scene-assets/blender/roster/infantry_roster.py`** (new, no bpy, so
  plain `python3` can import it) exposes `armies()`, `kit(name)` and
  `source_path(kit, look, mode, army)`, prefixing by army id.
  `infantry_equipment.source_path` is deleted and its callers use this.
- **Renames:** non-first-army files move with `git mv` to the army-prefixed
  name, and `assets/catalog.json`'s `source` entries are edited to match.
  Appearance ids don't change.

## What you can run

`python3 -c "import infantry_roster as r; print(r.source_path('rifle_squad','a','active','eastern_emr'))"`
from the blender folder prints the army-prefixed path.

## Verification

- Prove the pilot and one airframe export byte for byte the same as **fresh
  exports from the pre-change scripts**: `rifle_squad a active` and one jet
  with a skin. Don't compare against committed GLBs, which can drift (see
  `rifle_squad/active_b.glb`).
- `node web/asset.mjs bake` resolves every renamed source, and `asset check`
  passes.
- `node web/asset.mjs validate` on the pilot has no findings.
- The size ratchet passes on every touched file. `infantry_equipment.py` has
  to shrink, and moving the army data out does that.

There are no pictures, so no visual gate.

## Decisions delegated

The JSON field order, function names inside `infantry_roster.py`, and how
`WORN` is shared between the two receipt writers.

## Must stay green

Pilot bytes, airframe bytes, `asset check`.

## Feedback that would change it

None expected. This slice is plumbing.
