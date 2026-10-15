# 04 — Eastern general-purpose RPG and roster

## Contract unlocked

The Eastern base rifle squad gets a three-round, general-purpose RPG in place of its grenade launcher. US and Europe remain unchanged. Existing RPG-team rows become clearly named dedicated AT weapons.

## Initial authored tuning

- range: 250–300 m
- blast radius: 5–6 m
- penetration: 70–90 starting band
- direct damage: 45–55 starting band
- structural damage: 25–40 starting band
- reload: about 8 s
- `partial_from: 0.08`
- ground classes: `infantry`, `vehicle`

Tune against deterministic side hits on a representative heavy tank so three good hits are sufficient, then record the battle report/digest. The grenade keeps its penetration and receives explicit `partial_from: 0.08`, so it can chip light vehicles and MBTs for very small damage.

## Data migration

Rename internal dedicated rows to clear AT names (for example `rpg_at_light`, `rpg_at_heavy`) and update profiles, service costs, appearances, tests, and derived catalogs. Do not preserve old IDs.

## Tests and gates

- Eastern squad has RPG, three rounds, and no grenade launcher.
- US/Europe squad loadouts remain grenade-based.
- RPG damages infantry splash, light vehicles, and chips heavy armour.
- Grenade can chip MBT armour.
- Three good RPG side hits meet the initial heavy-tank goal; report the exact armour/damage assumptions.
- Run narrow simulation/catalog checks before balance retuning.
