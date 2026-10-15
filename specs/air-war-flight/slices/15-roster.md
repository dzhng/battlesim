# 15 — Roster: cleanup, loadouts, enable every jet

**Delivers:** D22′, D24, D25, L1.

## Passes
1. **Cleanup.** Delete the generic `*_fighter_aircraft` rows and the duplicate SEAD rows from `fixtures/units/air/air-war.json`. Repoint everything that names them:
   - the encounters under `fixtures/maps/air/encounters/`;
   - `fixtures/voices.json` and the icons;
   - the parity captures, the fog vectors and the scenes.

   Keep the family limit (`catalog.rs` ~1260) at 10 or fewer.
2. **Loadouts.** Author L1 ([map](../map.md)) on all 25 jets. The Su-34 becomes the iron bomber and the Su-30SM2 the SEAD jet.
3. **Enable, one faction at a time:** US, then Europe, then Eastern. For each of the 19 rows:
   - replace the `planned` block with a full row;
   - remove its model-manifest entry;
   - add an `assets/catalog.json` appearance, a voice and a baked icon.

   A faction's pass runs once slice 16 has accepted its models.

## You see
- `/labs/air-roster`: every jet in the lineup.
- The skirmish purchase cards.

## Tests
- `every_faction_fields_a_ground_missile_jet_and_a_bomb_jet`
- `no_jet_carries_bombs_and_ground_missiles`
- `only_drones_are_planned`
- The voice and asset checks.

## Delegated
Voice line choices. Prices stay as they are until slice 19.
