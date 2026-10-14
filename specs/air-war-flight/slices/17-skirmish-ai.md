# 17 — Skirmish AI and jets

**Delivers:** Q14 (the user said yes), plus the user's follow-up: AA is part of every standard opening.

## Why
Jets are tuned so that a player who forgets AA takes heavy damage before they recover. Every side therefore needs at least one AA unit as insurance, and the AI must model that habit rather than wait to see a jet.

## Seam
In `crates/sim/src/skirmish_ai/mod.rs`:
- **Opening:** every opening buys at least one unit that can target `HighAir`. It is chosen by that capability, never by a unit id, and the opening drops a lower-priority role if credits or slots run short.
- **Reaction:** a high-air demand that mirrors `low_air_demand`: when the AI sees enemy `HighAir`, it buys more high-air-capable units.
- **Jets:** jets join the reinforcement rotation and are sent out on attack-move. The jet rules (slices 08–11) do the rest.

## Tests
- `every_ai_opening_includes_a_high_air_defender`
- `the_ai_buys_more_high_air_defence_when_it_sees_enemy_jets`
- `the_ai_sends_bought_jets_on_attack_move`
