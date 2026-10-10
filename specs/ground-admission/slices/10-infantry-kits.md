# 10 — Javelin and Akeron infantry kits

**Unlocks:** the full equipment set the two AT teams need: every look (a/b/c), both modes
(active, carried), and receipts, matching the live TOW and Kornet teams.

**Slice variable:** equipment completeness and identity (does it read as a Javelin CLU and
tube, an Akeron MP firing post?). Soldier rig and uniform shape are not in scope.

## Seam

- `packages/scene-assets/blender/roster/infantry_equipment.py` already builds every look
  and mode for the `javelin` and `akeron` kits (`KITS`). Today only Javelin `active_a`,
  `carried_a` and Akeron `active_a` are exported.
- Export Javelin `active_b/c`, `carried_b/c`; Akeron `carried_a`, `active_b/c`,
  `carried_b/c`; add the Akeron receipt; add guard looks as the live teams have.
- The Javelin card is fielded by US and Europe but its kit lists only the US Army look. Add a
  European look (Flecktarn) and its reference variant.
- The Akeron kit names `german_flecktarn` for a French weapon. Use the French look if one
  exists in `infantry-appearances.json`; otherwise flag it in [choices.md](../choices.md)
  and keep it.

## Verification

- `asset validate`, `bake`, `check` for each new GLB.
- Sheets beside the TOW, Kornet and RPG teams at `battle-near` and `battle-mid`.
  [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) against those
  teams, then an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
  as the last check.

## Delegated

Which European look the Javelin wears; guard poses.

## Stays green

Existing infantry exports are unchanged (byte-compare a live kit after any shared helper edit).
