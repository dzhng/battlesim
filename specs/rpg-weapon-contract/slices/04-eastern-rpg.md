# 04 — Eastern general-purpose RPG and roster

## Contract unlocked

The Eastern base rifle squad gets a three-round, general-purpose RPG in place of its grenade launcher. US and Europe remain unchanged. Existing RPG-team rows become clearly named dedicated AT weapons.

## Initial authored tuning

- range: 250–300 m
- blast radius: 5–6 m
- penetration: 70–90 starting band
- direct damage: 45–55 starting band
- structural damage: 25–40 starting band
- reload: 5 s at most (game-wide cap, D50)
- `partial_from: 0.08`
- ground classes: `infantry`, `vehicle`

Tune against deterministic side hits on a representative heavy tank so three good hits are sufficient. That is a tuning sample judged in a sample run, not a test. The grenade keeps its penetration and receives explicit `partial_from: 0.08`, so it can chip light vehicles and MBTs for very small damage.

## Data migration

Rename internal dedicated rows to clear AT names (for example `rpg_at_light`, `rpg_at_heavy`) and update profiles, service costs, appearances, tests, and derived catalogs. Do not preserve old IDs.

## Tests and gates

- On `test_*` units with numbers written in the test: a multipurpose explosive row splashes infantry, damages a light vehicle and chips a heavy hull through `partial_from`; a grenade row with penetration chips a heavy hull.
- The roster's loadouts (Eastern squad RPG with three rounds, US/Europe grenades) are content: check them by loading the catalog and in the sample run, not with a test that names roster rows.
- Report the three-hit calibration (armour and damage assumptions) from a sample run.
- Run narrow simulation/catalog checks before balance retuning.
