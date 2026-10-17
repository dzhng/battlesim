# Choices ledger

## Sound

### 1. The general-purpose squad weapon is a new `rpg_general` row — high confidence

The implementation keeps dedicated anti-armour team ammunition separate as `rpg_at_light` and `rpg_at_heavy`, then adds `rpg_general` for the Eastern rifle squad. This gives the general-purpose round its own blast, penetration, ammunition, and targeting contract instead of weakening or overloading the specialist rows.

### 2. Eastern infantry gets a separate `eastern_rifle_squad` catalog card — high confidence

The shared `rifle_squad` remains the US/Europe version with a grenadier. Eastern receives a faction-specific card whose explosive specialist carries the general RPG. This keeps faction loadouts data-owned and avoids a faction conditional inside the shared squad.

### 3. The initial RPG tuning is 280 m, 80 penetration, 50 damage, 35 structural damage, three rounds, and 4 s reload — medium confidence

These values sit inside the planned tuning band. The implementer checked the authored 100-armour test tank side value: one good hit deals about 39 damage, so three good hits exceed the hull's 100 HP. This is a starting balance sample, not a permanent test constant.

### 4. Existing rifle-squad appearance art is reused for the RPG gunner — medium confidence

The new Eastern RPG gunner references the existing rifle-squad active appearance because no dedicated launcher art was required by the spec. This preserves the current asset contract and leaves replacement art reversible.

### 5. The blocked-shot icon keeps the existing warning/readout contract — high confidence

The icon was redrawn in place. `BlockedTrajectory`, full/compact rows, warning color, and ammunition behavior remain unchanged; only the SVG artwork changed to make the path and barrier read at small sizes.
