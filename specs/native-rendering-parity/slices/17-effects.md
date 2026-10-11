# 17 — Effects

**Contract:** native draws every effect the browser draws, from the same published causes, on the same clock and with the same seeds. That covers tracers, impacts and ricochet sparks, muzzle flashes, blasts, dust, aircraft smoke trails, wreck and ruin fire and smoke, cook-offs and rotor blur.

**Seam:**
- The capture already carries every cause (slice 13's decode), so no new data path is needed.
- Port the browser's derivations:
  - `apps/battle-lab/src/effectFeed.ts`;
  - `packages/battle-renderer/src/effects/effectFrame.ts`, `effectPass.ts` and `launches.ts`;
  - `apps/battle-lab/src/cookOffs.ts` and `cookOff.ts`.
- **Determinism:**
  - Per-tick bursts use `mulberry32((tick·2654435761)>>>0)`, consumed in publication order.
  - Dust, smoke trails and smoke sources use their `hashString` seeds.
  - Time is the frame record's `simTime`.
- The manifest gains `presentation.effects` and the catalog rows effects read: muzzles, wrecks and `buildingRemains`.
- Muzzle flashes sit on the posed muzzles from slice 14.

**Runnable artifact:** plate pairs across the reel, including the explosion-filled paris-corner plate that slice 03 couldn't judge.

**Focused gates:**
- **Counts.** Per plate, live effects of each kind equal the browser's, which the browser exports from `effectFrame` in the plate's frame.
- **Seeds.** A test checks seeded particle positions for fixed causes against TS vectors.
- **Visual.** `compare-screenshots`, then an unprimed `screenshot-critique`, then `preview-shots`.
- **Cost.** Pass cost reported, and slice 12's measurement re-run with native at or below the browser.

**Visual variable:** effects.

**Delegated:** the particle drawing technique, provided counts and seeded positions match.

## Findings (2026-10-11)

**Contract as built.**
- The manifest gains `effects`: `presentation` (`presentation.effects` as `gameEffects` resolves it, each weapon's rows inherited), `types` (per fielded type, its hull's half extents and each mount's muzzle model), `smokes` (the prop kinds that smoke once known, with their look: ruins, gutted shells, wrecks) and `wrecks` (per wreck appearance with a whole state, what a cook-off throws: the turret piece's centre and underside, the debris's top; one owner, `cookOffs.ts` `wreckPieces`). Units gain each body's `muzzle` socket, each rig's `muzzles` by role and its rotors' blade facts. `effectCounts` holds, per plate, the effects running at its clock by kind, the instances by shape and the cast lights (`EffectFrame.counts`). Strict readers on both sides; a stale manifest is refused.
- `native/godot/effects/` ports `effectFeed.ts` (`effect_feed.gd`), `launches.ts` (`launches.gd`, moved from `units/`: the pose feed reads the same tracker), `CookOffWatch` and `effectCookOff` (`cook_offs.gd`), `EffectFrame` (`effect_frame.gd`), `DrawnModels`' muzzles and rotors (`drawn_models.gd`) and `effectPass.ts` (`effect_pass.gd`, a raster pass between the haze and the post chain). `castLights.ts` is `cast_lights.gd`: the lights in view (the strongest past 24) reach every world material through shader globals and the smoke through the pass.
- `core/js_text.gd`: `hashString`, JavaScript's `String(x)` for a float32 (shortest round trip, ties to even) for the hashed keys, and a decimal read to the nearest double. The capture parser reads each record's `simTime` with it.

**Counts gate.** Control `2026-10-11T15-26-04-627Z`. All 24 plates (6 paris-corner, 18 market-town) equal the browser's in every kind, shape and light count, and the run's `effects` blocker is clear. Two causes had to be found on the way:
- Godot's JSON reader puts `18.066666666666666` an ulp below the browser's double. Market-town plate 1335's clock sits exactly on tick 543's start, so native left 18 of its tracers unstarted. Reading `simTime` exactly fixed it.
- The cook-off at tick 87 lands its turret's dust 8 puffs into plate 380: the landing point is read from the wreck the props fit (`prop_layer.gd` `fit`) and `effects.wrecks`.
- A TypeScript replay of the capture through the browser's own `EffectFrame` equals the live browser on 23 of 24 plates without landing points and 24 of 24 with them, so the capture is a sufficient record of every cause.

**Seeds gate.** `web/tests/nativeEffectVectors.test.ts` writes the browser's instances and cast lights for fixed causes over five publications at seven clocks: a squad's chance-chosen tracers and their flashes, a tank's hull shot off its muzzle, a ricochet, impacts, a blast, a cook-off with a landing, dust, a smoking airframe, a burning wreck, a grenade's trail and a rotor, every shape and kind. `tests/effect_vectors_test.gd` matches every float within 2e-4 (relative), every light and every count. Changing the per-tick multiplier by 2 turns it red. `tests/js_text_test.gd` pins the text against Node's (ties, small numbers, the boundary clock).

**Plates.** `compare-screenshots` against the control: parity distance 0.005–0.042 on paris-corner (the rest is the railings' dither, not effects) and 0.002–0.015 on market-town; tracers, flashes, fireballs, impact puffs, sparks, dust, smoke columns, the ATGM trail and the cast-light pools sit in the same place at the same size and brightness. The unprimed critique found one effects difference: on plate 1206, where the camera sits inside a burst, native drew bright smoke over the right third where the browser shows the darker fire. Cause: smoke sprites nearer than the near plane, which the browser's reverse-Z projection clips. Native now clips them the same way, and plate 1206's distance fell from 0.087 to 0.005. The critique's other notes are shared by both clients (the ATGM trail reads as beads; a near smoke sprite's straight edge) or are not effects (market-town window checkerboards, Paris railing dither, paler wheat).

**Cost.** Release export, market-town records 1000–1299, plugged in, minimised, three runs each, interleaved, with and without `effects` in the layers (medians, range):

| | avg FPS | p50 ms | p95 ms | p99 ms | CPU p50 ms |
| --- | --- | --- | --- | --- | --- |
| without effects | 48.1 (46.2–48.6) | 23.76 (23.64–24.18) | 26.64 (26.59–27.85) | 27.89 (27.71–28.69) | 9.33 |
| with effects | 44.3 (42.3–45.6) | 25.02 (24.50–25.92) | 29.55 (28.36–30.69) | 30.77 (29.75–32.02) | 10.69 |

- The effects cost about 1.3 ms at p50 and 2.9 ms at p95, of which about 1.4 ms is CPU (the per-frame build and the publications noted, in GDScript).
- The absolute numbers carry slice 15's open market-town cost (the static world's props), which its agent is fixing. The combined bar is re-checked after that lands; the effects' share is the difference above.

**Not ported, reported per run.**
- A cook-off's moving wreck (the hull rolling on and jolting, the turret thrown) and its thrown debris: `cookOffModels` and `debrisModel` draw models, not effects, through the props' and units' layers. The static wreck shows throughout; the run reports the cook-offs as `effects.cookOffs`. Market-town's three cook-offs leave debris on about six plates for 24 s each, which no count covers.
- Lit smoke takes the sky averaged over each hemisphere, cosine-weighted, where the browser reads its prefiltered environment at full roughness (`effects.sky`); cast light on a model faces its mesh normal, not its normal map's (`effects.cast`).
- Rotor blur draws from the frame record posed before it; a plate run skips records, so the angle a rotor turned is from the last drawn record. No reel map has aircraft.

**Coverage of `crashes` and `guided`.** Not added. Neither control publishes them (`guided` is 0 over the whole market-town capture). A WASM stream needs a helicopter that is shot down: the lab scenario fields a `test_heli` and a `test_gun_jeep`, but the jeep never brought it down at full health and patching its hit points through the test rules did not take within the time box. The effects' code for a falling airframe (damage smoke, rotors) is covered by the fixed-cause vectors instead; the decode of both groups remains proven only by shared row code (slice 13).

**Scratch:** `throwaway/slice17/` holds the control logs, native runs, comparison artifacts, crops and the benches.
