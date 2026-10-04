# 05 Court amenities as bodies, with stand-in art

**Unlocks:** courts that fight: cover, sight and movement change inside the blocks.

**Seam:**
- Catalog rows (`fixtures/props/city/courts.json`, new) for the shared core (`bike_rack`, `playground_frame`, `swing`, reuse `bench`, `bins`) and each region's signature kinds (China `bike_shed`, `pingpong_table`, `outdoor_gym`, `laundry_poles`; New York `chainlink_fence`, `basketball_hoop`, `garage_row`, `dumpster`; Paris `kiosk`, `petanque_pitch`, plane trees as `street_tree`). Pitches and court floors are walk-on bodies that block nothing. Each row decided with [tweak-mechanics](../../../.agents/skills/tweak-mechanics/SKILL.md).
- Presets: `street_props.bodies` boxes for each kind (fixed from real dimensions now, so art never moves them); named groups (`{size_m, pieces:[{kind, at, yaw_deg}], fence?, gate_m?}`); `props.courts` per district with weighted groups, and per-family group tables. Presets validation refuses a group naming an unlisted family or kind.
- A consumer `street_props/courts.rs`: anchors over each court on a stable grid with jitter from `street-props/<court>/court`, a group drawn by weight from the shared and the map family's tables, placed whole or not at all through `field::legal`; fenced groups use the fence routine and keep a gate.

**Verify (tests first, `tests/street_props.rs`):**
- every piece lies inside its court and is legal; groups are whole;
- a map of one family never places another family's signature kind;
- **reachability:** every court keeps a way in from a street for infantry and for the widest vehicle class, checked with the simulation's navigation; every door stays reachable (extend `routes_survive_the_furniture_on_every_type_and_size_of_map`);
- deterministic placement; largest map within `limits.max_authored_parts` (`fixtures/generated-battle.json`);
- a quick battle sample on one town map per region (tweak-mechanics' sample, not the full balance report).

**Run/see:** stand-in boxes in the courts at the town stations per region; screenshot-critique.

**Review checkpoint (non-blocking):** density and mix; the cover each kind gives.

**Delegated:** group recipes, densities, grid spacing.
