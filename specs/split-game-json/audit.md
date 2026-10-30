# Combined fixture audit

Taken 2026-10-29 on the shared renderer branch (PR #16). An inventory of all
2,835 leaves (readers by name, value history over 323 commits) targeted five
per-section traces of the code; claims marked (verified) were rechecked by hand.
File and line references are as of that commit and will drift.

## Caused by this PR (the TypeScript renderer's retirement)

- 162 `presentation.terrain` and 23 `presentation.grass` numbers are dead: the
  ported WGSL holds them as literals, serde ignores the keys. None ever changed
  value since entering the file on 2026-10-11. `tuning.rs`'s doc is stale.
- Five numbers have two owners: Rust tables read the fixture, the shader holds
  a literal copy. Editing one desyncs CPU reach from GPU drawing.
  `grass.edge_m` (tables.rs:65 / passes_models.wgsl:64,77),
  `terrain.forest_verge.feather` (tables.rs:76 / scenery.wgsl:74,200),
  `terrain.bank.ease_m` (tables.rs:78 / models_material.wgsl:99),
  `terrain.marks.clear_m` (tables.rs:70 / models_material.wgsl:209),
  `terrain.scars.depth_cap` (tables.rs:128 / terrain_scars.wgsl:93-94, fog_sight.wgsl:327).
- `presentation.glass` (turn 0.5, glint 1) is read by nothing; literals at
  passes_models.wgsl:237-238 (verified).
- `presentation.fog_geometry.tile_px` / `radial_bins` frozen at 16 / 64 in
  passes_fog.wgsl:206-207,331,363 (verified), while `sight.rs:96-106` still
  accepts 1..16 and any ≥ 2 and the CPU dispatches from them: a retune breaks
  culling or overruns the shader's arrays silently.
- Live terrain: `distant.plot_pixels`, `shore.*` (11), `water.*` (16).

## Live bugs already on staging

- Tank guns never rock when firing (verified): `kick_of` (driver.rs:177) wants
  an exact weapon id; only `tank_ap`/`tank_he` kick and no unit fires them
  directly; `default` is 0. Same fault: soldier kick for atgm/rpg/igla/fn_6, and
  `impacts.*.round_scale` (effects/frame/pool.rs:221): tank-round ground impacts
  3x and 1x lifetime instead of 5x/5x. Only tracers, flashes, impact_scale,
  blast_scale inherit along `extends` (fielded/effects.rs:16).
- RPGs about 11 dB quieter since 2026-10-16 (verified): b4410901c renamed
  `rpg_light/heavy` → `rpg_at_light/heavy` (+ `rpg_general`) but audio
  `shots`/`blasts`/`impact_scale` rows kept the old keys, so all three RPGs
  fall back to `grenade`'s rows.
- A bad weapon row in an encounter override or uploaded bundle panics the battle
  instead of being refused: `sim::weapons::check_rules` runs only from
  `fixtures::admit` and `Arsenal::new` (.expect) (verified callers). Same split
  for cover, ground, movement.drive, ricochet, guided, supply (asserted in
  `Battle::new`). Unchecked: `infantry_movement` (`wander_period_s: 0` → NaN
  positions), `bail_out` (negative `hp_lost` heals), garrison, pushing, launch.
- Shipped UI reads `tick_hz` (captions.tsx:17, battleStatus.tsx:11,
  MatchResultOverlay.tsx:34, battleOverlay.ts:355) and `air`
  (battleOverlay.ts:261, `ghostAloft` re-implements `AirRules::cruise_agl_m`)
  from the raw file, not the battle's resolved rules. Latent: no override
  touches them today.
- Presentation resolves weapon `extends` from the raw file
  (effects.rs:10-14, sound/tables.rs:386-399, presentedFrame.ts:129, soundFeed.ts:27):
  a weapon added by an encounter looks and sounds `default`. `presentationRules`
  (compose.ts:89-93) lays shipped rows over the scenario's. Latent.

## Why they hid: unknown keys and table keys are never checked

- Ground `Tuning`/`TerrainTuning`/`GrassTuning`, `LightSettings` lack
  `deny_unknown_fields`; TS readers cast raw JSON. (`fog`, `fog_geometry`,
  audio structs, rules do refuse unknown fields.)
- Weapon-keyed presentation tables (audio shots/blasts/motors/impact_scale,
  effects tables, kick rounds) never check keys against `weapons`.
- Light is validated only in TS (`validateLight`, sceneLight.ts:168); native and
  WASM deserialize unchecked; `shadow.rs:170` clamps `cascades.count` silently.
- Audio sound names unchecked: an unknown vehicle/ambience sound panics in
  `bank.prepare`; an unknown blast fails only when it plays.

## Dead, duplicated, or in the wrong home

Weapons (49 rows, every row used):
- `description` (39 rows) read by nothing. `decoyable` on 9 ground-only rows
  does nothing. About 69 child values equal the parent's.
- Row `range_m` equals the largest layer range on all 49 rows: derive it.
- `gravity_scale`'s doc calls it `(speed/real speed)^2`, but the values are
  hand-tuned arcs (they imply a 301 m/s tank round, a 43 m/s grenade); the
  doc is wrong, not the data. RPG rows copy the tank's 1.7651.
- `helicopter_crash` extends `tank_he` and inherits fields a crash never reads;
  nothing checks at load that it exists.
- `name`/`icon`/`description` are card presentation inside the hashed rules.
- No check that a guided missile's lifetime covers its range (air_missile: 3000
  m in exactly 11 s for a 3000 m range).

Rules:
- `service.round_costs` is per-weapon (move to the weapon row as `round_cost`);
  `air_gun` has unlimited ammo but a price; about 20 entries repeat the parent's.
- `buildings.cover_strength` = 1 is a no-op blend weight; `capacity_soldiers`
  silently capped by `.min(32)` in garrison.rs:150.
- `forests` logs and boulders: `*_per_ha` = 0 since a same-day revert on
  2026-10-02; their tables affect no battle. Ship or delete.
- `hull_limits`, `air_speed_limits_kmh`: catalog-admission limits stored as
  overridable battle rules; `hull_limits.rotor.half_length_m` doubles as the
  helicopter routing margin.
- `controls` is browser input; `seed` is the lobby default (20260925 also
  copied in authority.tsx:15, ballistics.tsx:34, streetScenario.ts:12).
- `shouts.talk_s`: 17 values, all 2.5.
- Gravity 9.81 hard-coded in cook_off.rs:19 and units/driver.rs:32.

Audio (691 leaves; about 275 removable):
- `shots.near/far` (94 leaves) never play: `sounds.json` has a `default` choice,
  so `shot()` (tables.rs:512-532) keeps only `gain`/`far_m`. Two owners of firing.
- 66 leaves repeat what inheritance gives; about 98 repeat the same baseline
  name per row (`explosion`, `motor`) that `sounds.json` replaces anyway.
- `audio.impact_scale` (loudness) vs `effects.impact_scale` (size): same idea,
  hand-tuned apart (grenade 2.0 vs 2.5).
- Vehicle rows for classes no unit has (air_medium, air_heavy, high_air_*).

Effects, pose, stand_ins (all live except glass):
- `stand_ins.tints` (38) is a colour per prop kind: belongs on prop rows, read
  twice (files.rs:187, propAppearance.ts:119).
- `pose.gauge` only splits tracked/wheeled; unknown class silently 1.0.
- `effects.wake.soldier_radius_m` 0.15 vs `physics.soldier_radius_m` 0.3: rename.
- Zero `fade_s`/`out_s` values divide by zero; no range checks in pose.
- `buildings` validated only by leftover TS in the retired package
  (buildingReferences.ts:41-49).

View (fog, light, camera, overlay, hud, contacts, map_border):
- All live; overlay/hud/contacts/camera are TS-owned UI with their own validators.
- `overlay.xray.ally` = `hud.ally`; `overlay.ruler.reach` = `hud.accent`.
- `light.cascades.count` can only be 4; `light.backdrop.reach_m` is ground extent.
- `camera.default.target` is one map's opening framing.
- TS picks the fog style itself (gameFog.ts:24), duplicating `FogPresentation::selected`.
- `fog/mod.rs:18` cites a removed `FOG_DISTANCE_CAP_PX`; cap is literal 16 in fog_masks.wgsl.
