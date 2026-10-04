# 02 One paved kind for hard ground that is no way through

**Unlocks:** courts and aprons as one kind the renderer draws and the simulation reads, without widening the terrain shader.

**Seam (contract change):**
- `contract::map::SurfaceKind::Sidewalk` is renamed `Paving` (serde `paving`); `is_road()` becomes an explicit match on `Road | CountryRoad | DirtTrack`.
- The parcel pass's aprons (`parcels/lots.rs`, today `SurfaceKind::Road` polygons) are `Paving`.
- The rules' `surfaces` row (`fixtures/game.json`), the sim's surface mapping (`crates/sim/src/world/mod.rs`, `export.rs`), the renderer's `SURFACE_AREA_KINDS` and `checkSurfaceKinds` (`terrain/surfaces.ts`), the biome rows (`fixtures/biomes/*.json`: the `walk` and `area` references) and `mapgen inspect` colours follow the rename. Sweep every `sidewalk` string.
- Movement on `paving` is a rule: decide its `speed_factor` with [tweak-mechanics](../../../.agents/skills/tweak-mechanics/SKILL.md). Aprons were road-speed; a hard yard is hard ground. Record the call.

**Verify (tests first):** a paving polygon is not a road to the parcel pass's network, to street furniture, or to the simulation; where a road and paving overlap the road wins (`surfaceField` test); the presets revision bumps (`layout-presets-14`); parity is re-blessed once here. Shots: an industrial district's aprons at a 65 m station, before and after, per region; screenshot-critique; compare-screenshots against the before shot.

**Stays green:** every other test; vehicle routes along roads.

**Delegated:** the paving row's default look until slice 04.
