# 17 — Skirmish AI and jets

**Delivers:** Q14 (the user said yes).

## Seam
In `crates/sim/src/skirmish_ai/mod.rs`:
- A high-air demand that mirrors `low_air_demand`: when the AI sees enemy `HighAir`, it buys a unit that can target `HighAir`.
- Jets join the reinforcement rotation and are ordered with attack-move. The jet rules (slices 08–11) do the rest.

## Tests
- `the_ai_buys_high_air_defence_when_it_sees_enemy_jets`
- `the_ai_sends_bought_jets_on_attack_move`
