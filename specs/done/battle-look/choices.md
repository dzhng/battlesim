# Choices ledger

These are the decisions the build made where the spec, the slices or the user were silent. It is the review surface for everything decided on your behalf, so you don't have to read the diff. Every entry was re-audited against the shipped code (main after the unit and prop catalogs and the post-close review fixes, 2026-09-28), per [audit-choices](../../../.agents/skills/audit-choices/SKILL.md):
- An entry a later pass changed is written as its end state.
- An entry a later pass reverted or superseded is gone. That covers trenches, the overlay-then-paint history and the per-kind muzzle.
- Duplicates are merged.
- Gate results, measurements and evidence live in [`decisions.md`](decisions.md) and [`frame-cost.md`](frame-cost.md), not here.

**How to read it.** Entries are grouped by verdict, least confident first:
- **needs your call**: yours alone to settle;
- **unsound**: to be redone from the corrected decision;
- **sound**: the architecture you now own.

Each entry ends in:
- a **verdict**;
- a **confidence**: how sure the audit is that you'd have made the same call;
- **provisional**, where the entry is a number or rule running on a recommended call you may still override;
- where it came from.

Each entry walks one concrete case: what happens in the game today, and what the alternative would have done.

## Review these first

- [An idle turret keeps its last bearing instead of returning to the front](#an-idle-turret-keeps-its-last-bearing-instead-of-returning-to-the-front) (needs-user, confidence low)
- [Village AP never glances off a tank](#village-ap-never-glances-off-a-tank) (needs-user, confidence low)
- [Wreck and variant model builds are not byte-reproducible; the committed GLB files are the source of truth](#wreck-and-variant-model-builds-are-not-byte-reproducible-the-committed-glb-files-are-the-source-of-truth) (needs-user, confidence low)
- [The ordinary ambush now costs blue more tanks; the balance spec decides whether that stands](#the-ordinary-ambush-now-costs-blue-more-tanks-the-balance-spec-decides-whether-that-stands) (needs-user, confidence low)

## Provisional calls you may still override

Each runs today as the recommended call and is reversible; the entry says how.

- [An idle turret keeps its last bearing instead of returning to the front](#an-idle-turret-keeps-its-last-bearing-instead-of-returning-to-the-front) (needs-user, confidence low)
- [Village AP never glances off a tank](#village-ap-never-glances-off-a-tank) (needs-user, confidence low)
- [The village capture target is left unbalanced; balance gets its own spec](#the-village-capture-target-is-left-unbalanced-balance-gets-its-own-spec) (needs-user, confidence low)
- [The ordinary ambush now costs blue more tanks; the balance spec decides whether that stands](#the-ordinary-ambush-now-costs-blue-more-tanks-the-balance-spec-decides-whether-that-stands) (needs-user, confidence low)
- [Wreck and variant model builds are not byte-reproducible; the committed GLB files are the source of truth](#wreck-and-variant-model-builds-are-not-byte-reproducible-the-committed-glb-files-are-the-source-of-truth) (needs-user, confidence low)
- [A tank's long gun can poke into a wall; nothing guards against it](#a-tanks-long-gun-can-poke-into-a-wall-nothing-guards-against-it) (needs-user, confidence medium)
- [A soldier's round starts at his body, not at his rifle](#a-soldiers-round-starts-at-his-body-not-at-his-rifle) (needs-user, confidence medium)
- [A garrison sees, and spots, from one eye per facade it holds](#a-garrison-sees-and-spots-from-one-eye-per-facade-it-holds) (needs-user, confidence medium)
- [The far view stays at the default's 49° tilt instead of WARNO's horizon vista](#the-far-view-stays-at-the-defaults-49-tilt-instead-of-warnos-horizon-vista) (needs-user, confidence medium)
- [No trees stand inside the map outside the simulation's forests; hedgerows and copses stand only past the edge](#no-trees-stand-inside-the-map-outside-the-simulations-forests-hedgerows-and-copses-stand-only-past-the-edge) (needs-user, confidence medium)
- [Scar colours and strengths live in the biome file](#scar-colours-and-strengths-live-in-the-biome-file) (needs-user, confidence medium)
- [Grass over painted ground glows only near its root](#grass-over-painted-ground-glows-only-near-its-root) (needs-user, confidence medium)
- [The fog's ruled lines are fixed to the screen, not the ground](#the-fogs-ruled-lines-are-fixed-to-the-screen-not-the-ground) (needs-user, confidence medium)
- [Fog's face rule judges an upward face by the air above it, so low props can show lit ledges on an unseen side](#fogs-face-rule-judges-an-upward-face-by-the-air-above-it-so-low-props-can-show-lit-ledges-on-an-unseen-side) (needs-user, confidence medium)
- [Model surfaces are vertex-colour paint, with baked textures on vehicles, soldiers and small props](#model-surfaces-are-vertex-colour-paint-with-baked-textures-on-vehicles-soldiers-and-small-props) (needs-user, confidence medium)
- [Effect curves and sizes are fixture tuning](#effect-curves-and-sizes-are-fixture-tuning) (needs-user, confidence medium)
- [A wreck burns from when the side first knows it: 60 s of fire, 150 s of smouldering](#a-wreck-burns-from-when-the-side-first-knows-it-60-s-of-fire-150-s-of-smouldering) (needs-user, confidence medium)
- [Smoke, flame, dust and blast-plume looks are fixture tuning](#smoke-flame-dust-and-blast-plume-looks-are-fixture-tuning) (needs-user, confidence medium)
- [Tracers are styled by round kind, not by side](#tracers-are-styled-by-round-kind-not-by-side) (needs-user, confidence medium)
- [Cover tier is a filled pip inside the soldier's marker: yellow, light green, strong green](#cover-tier-is-a-filled-pip-inside-the-soldiers-marker-yellow-light-green-strong-green) (needs-user, confidence medium)
- [Prop hit points and the wreck chain are provisional numbers](#prop-hit-points-and-the-wreck-chain-are-provisional-numbers) (needs-user, confidence medium)
- [Soldier kinds carry hit points, looks and weapons; body size stays one shared frame in `physics`](#soldier-kinds-carry-hit-points-looks-and-weapons-body-size-stays-one-shared-frame-in-physics) (needs-user, confidence medium)
- [HE shells hurt a tank a little even when they don't penetrate](#he-shells-hurt-a-tank-a-little-even-when-they-dont-penetrate) (sound, confidence medium)
- [A fallen soldier faces the squad's heading at the moment he fell](#a-fallen-soldier-faces-the-squads-heading-at-the-moment-he-fell) (sound, confidence medium)
- [A glancing round bounces near the mirror angle, slowed and weakened](#a-glancing-round-bounces-near-the-mirror-angle-slowed-and-weakened) (sound, confidence medium)
- [Crater and scorch size scale with the weapon's blast radius](#crater-and-scorch-size-scale-with-the-weapons-blast-radius) (sound, confidence medium)
- [Cover tiers are three spread multipliers; each body's tier comes from its catalog row or its weight class](#cover-tiers-are-three-spread-multipliers-each-bodys-tier-comes-from-its-catalog-row-or-its-weight-class) (sound, confidence medium)
- [Weight and push classes are ranks; a shove slows the pusher by the ratio](#weight-and-push-classes-are-ranks-a-shove-slows-the-pusher-by-the-ratio) (sound, confidence medium)
- [Vehicles see 100 % ahead, 50 % abeam and 30 % astern](#vehicles-see-100--ahead-50--abeam-and-30--astern) (sound, confidence medium)
- [Forest densities: light, medium, dense; the village's west wood is medium, the orchard light](#forest-densities-light-medium-dense-the-villages-west-wood-is-medium-the-orchard-light) (sound, confidence medium)
- [Benchmark memory is the frame's GPU allocations plus the main-thread JS heap](#benchmark-memory-is-the-frames-gpu-allocations-plus-the-main-thread-js-heap) (sound, confidence medium)
- [Fog tracing budgets live in the fixture as provisional numbers, and a crowded screen tile drops eyes past its cap](#fog-tracing-budgets-live-in-the-fixture-as-provisional-numbers-and-a-crowded-screen-tile-drops-eyes-past-its-cap) (sound, confidence medium)
- [Model detail tiers are chosen by projected height; casters draw one tier coarser](#model-detail-tiers-are-chosen-by-projected-height-casters-draw-one-tier-coarser) (sound, confidence medium)
- [Vehicle mounts ease elevation and recoil but draw the turret bearing as published](#vehicle-mounts-ease-elevation-and-recoil-but-draw-the-turret-bearing-as-published) (sound, confidence medium)
- [Impostor cards are baked by our own renderer at install time, 8 headings × 2 pitches, not shipped in bundles](#impostor-cards-are-baked-by-our-own-renderer-at-install-time-8-headings--2-pitches-not-shipped-in-bundles) (sound, confidence medium)
- [Trees draw in their own scenery layer with per-tree detail tiers; props draw in the models layer](#trees-draw-in-their-own-scenery-layer-with-per-tree-detail-tiers-props-draw-in-the-models-layer) (sound, confidence medium)
- [Water is its own world layer, shaded by the terrain material, with a pale bare shore](#water-is-its-own-world-layer-shaded-by-the-terrain-material-with-a-pale-bare-shore) (sound, confidence medium)
- [The fog sight map is 4096 rays by 64 range bins per eye](#the-fog-sight-map-is-4096-rays-by-64-range-bins-per-eye) (sound, confidence medium)
- [The run is the library jog with its upper-body twist damped](#the-run-is-the-library-jog-with-its-upper-body-twist-damped) (sound, confidence medium)
- [Gun and HMG pitch are clamped by presentation constants](#gun-and-hmg-pitch-are-clamped-by-presentation-constants) (sound, confidence medium)
- [The side's colour is a tint mask on materials, with the side colours in the asset catalog](#the-sides-colour-is-a-tint-mask-on-materials-with-the-side-colours-in-the-asset-catalog) (sound, confidence medium)
- [Detail tiers are authored per part, not decimated](#detail-tiers-are-authored-per-part-not-decimated) (sound, confidence medium)
- [Some props deliberately overhang their box, with per-appearance tolerances](#some-props-deliberately-overhang-their-box-with-per-appearance-tolerances) (sound, confidence medium)
- [Wreck models are dense: about 143k triangles for the tank at the finest level of detail](#wreck-models-are-dense-about-143k-triangles-for-the-tank-at-the-finest-level-of-detail) (sound, confidence medium)
- [Every sound is synthesised in code; there are no audio files](#every-sound-is-synthesised-in-code-there-are-no-audio-files) (sound, confidence medium)
- [The mix: four buses into a limiter, ambience kept well under the fight](#the-mix-four-buses-into-a-limiter-ambience-kept-well-under-the-fight) (sound, confidence medium)
- [The jeep is a light, fast recon vehicle with a pedestal HMG](#the-jeep-is-a-light-fast-recon-vehicle-with-a-pedestal-hmg) (sound, confidence medium)
- [Vehicle drive is a catalog `mobility` variant: tracked or wheeled](#vehicle-drive-is-a-catalog-mobility-variant-tracked-or-wheeled) (sound, confidence medium)

## Needs your call (22)

Taste, product direction or external cost: only you can settle these. Each carries the provisional call the game runs on today.

### An idle turret keeps its last bearing instead of returning to the front

***needs-user** · confidence **low** · **provisional** · Simulation rules · from Slice 04; Orchestrator, after slice 04*

**The choice.** A tank fights to its left, then drives off east. Its turret only turns while it has a target (`weapons.rs`), so it stays pointed left, and because sight follows the turret, the tank sees its own route at side reach (50 %). The alternative, turning an idle turret back over the hull's front, would be a new weapons rule.

**The gap.** Sight along the turret made the old "turret stays put" behaviour matter.

**The reach.** Tanks on the move after a fight are half-blind ahead.

**Verdict.** needs-user. Provisional: keep the last bearing. To reverse, in the weapons step, slew a turret with no resolved target toward the hull's yaw at its traverse rate.

### Village AP never glances off a tank

***needs-user** · confidence **low** · **provisional** · Simulation rules · from Orchestrator, after slice 06; Slice 06*

**The choice.** The rule order is "fails to penetrate, then rolls the face's ricochet chance". AP penetration is 180 and the tank's front armour 140, so AP pierces every face and never ricochets; only stray rifle and HMG rounds glance off tanks. The user's wording ("per-face probability of ricochet, to simulate the angle") could also support rolling ricochet before penetration on every face, as an angle stand-in. Or AP penetration could come down, or armour go up.

**The gap.** Q9's order with today's numbers makes tank-on-tank ricochets impossible.

**The reach.** Whether tank duels ever see a bounce.

**Verdict.** needs-user. Provisional: keep the order. To reverse, roll the ricochet chance before the penetration check in `damage::meet_hull`, or lower `tank_ap.penetration` below 140.

### The village capture target is left unbalanced; balance gets its own spec

***needs-user** · confidence **low** · **provisional** · Simulation rules · from Slice 04; Slice 07; Orchestrator, after slice 04; Orchestrator, after slice 07; Orchestrator, after slices 21 and 22*

**The choice.** The scripted village battle is a supported blue attack (infantry, recon and tanks together) on red's village. `encounter.md` sets targets for it, the main one being "the supported attack captures the village in at least 7 of 10 seeds". Many rules the user asked for move that rate up and down: directional sight, crater cover, per-mount muzzles, rounds passing fences, rifles and missiles firing into trunks. No fixture number is retuned to chase the target, because retuning would undo or blunt rules the user chose. The user decided (2026-09-27) that the rebalance stops where it stood (7/10) and that balance gets its own future spec. Every rule change since has been measured and recorded but not retuned. The last full measurement (ten seeds, 900 s) captured 6/10, missing the target by one seed; quick three-seed runs since have moved again. The unbuilt alternative was to tune the village fixture (placements, forest densities, cover tiers) after every rule change until the target held.

**The gap.** The slices said "retune if targets move a lot", which conflicted with the new rules, and nobody in this spec owned the outcome target once the user moved balance out.

**The reach.** The village report's capture numbers are measurements, not a guarantee. The village may be slightly easier to defend than designed. The balance spec owns the capture target, the prompt-versus-delayed retreat check, ruin-hugging survivors and the ambush tuning.

**Verdict.** needs-user (deferred by the user). Provisional: accept the current rate and leave rebalancing to the balance spec. To reverse, run the balance spec against `encounter.md`'s targets, tuning the village fixture's placements, densities and cover numbers through `village_report`.

### The ordinary ambush now costs blue more tanks; the balance spec decides whether that stands

***needs-user** · confidence **low** · **provisional** · Simulation rules · from Post-close review (sim)*

**The choice.** In the ordinary ambush script, blue's tanks drive toward the village, red's AT teams wait in the west wood, and blue retreats 0.75 s after the first missile. A retreating tank now takes a second missile. On the quick report (`village_report -- --quick --compare main`: three seeds, 600 s), blue lost 3 tanks where it lost 1, and 672 cost where it lost 280; seeds 2 and 3 each lose a tank they kept. A probe of the operator-muzzle fix alone reproduces it: red's AT teams launch 2 and 4 missiles in seeds 2 and 3, where they launched 1. From the team's middle, the follow-up shot at the retreating tank never cleared. From the gunner's own muzzle, up to 5 m off that middle, or out on his lean, it does. So this is the rule working the way a real ambush would, not a bug. The flank script still captures 2 of 3. Nothing was retuned. The unbuilt alternative was to retune the ambush (the AT teams' placement or the retreat delay) until the old cost returned.

**The gap.** The fix corrected a rule. The user had already moved balance to its own spec (`decisions.md`, 2026-09-27), so whether the new cost is acceptable is that spec's call.

**The reach.** The encounter's question is whether "an ordinary ambush still permits a narrow escape through prompt response" ([`encounter.md`](../../battle-foundation/encounter.md)); its target is a tank surviving in at least 7 of 10 ordinary ambushes. That may no longer hold. Only three seeds were run, not the full report (every script, ten seeds). The balance spec should start from a full report on main.

**Verdict.** needs-user, for the balance spec. Provisional: keep the rule and record the shift, which is in `decisions.md` (tuning log, "Post-close review fixes (sim)"). To reverse the balance effect without undoing the rule, retune the ambush in the balance spec.

### Wreck and variant model builds are not byte-reproducible; the committed GLB files are the source of truth

***needs-user** · confidence **low** · **provisional** · Models and the asset pipeline · from Slice 22b*

**The choice.** Rebuilding the live tank gives identical bytes, but rebuilding a wreck or an infantry variant does not (the infantry kit's Cycles ambient-occlusion bake was already non-deterministic; the wreck path's cause was not found in a time-box). So the committed model files, not the scripts, define what ships. A rebuild changes hashes and the asset manifest without changing the look.

**The gap.** The spec assumed reproducible asset builds.

**The reach.** Any rebuild produces a noisy diff; a manifest hash check cannot tell a real change from noise.

**Verdict.** needs-user — provisional call: keep committed GLBs authoritative; reverse by funding a determinism hunt in the wreck path and seeding the AO bake.

### A tank's long gun can poke into a wall; nothing guards against it

***needs-user** · confidence **medium** · **provisional** · Simulation rules · from Slice 24; Orchestrator, after slices 21 and 22*

**The choice.** A tank parks hard against a house and turns its turret. Its 5.9 m gun reaches into the wall. The simulation launches rounds from that muzzle (each mount's `muzzle_m`), so the drawing is left as is: hiding or shortening the gun would draw something other than what fires. The alternative is a simulation rule, such as limiting traverse or requiring muzzle clearance, which is a named rule change.

**The gap.** Neither the simulation nor presentation covered a muzzle inside a body.

**The reach.** A round can start inside a building's box; how that round meets the wall depends on flight rules, not on this.

**Verdict.** needs-user. Provisional: leave it unguarded and draw the true gun. To reverse, add a sim rule (traverse or muzzle clearance against bodies) in the weapons module; the renderer needs no change.

### A soldier's round starts at his body, not at his rifle

***needs-user** · confidence **medium** · **provisional** · Simulation rules · from Slice 27 (muzzle flash); Slice 27 (per-mount muzzles)*

**The choice.** A rifleman fires. The simulation starts the round at his position plus `physics.infantry_muzzle_m`, which is straight up from the centre of his body (`weapons::muzzle`, when the mount row has no `muzzle_m`). The drawn rifle's muzzle is about 0.8 m forward of that. The flash is drawn on the rifle, so the tracer begins 0.8 m behind the flash. For vehicles the user's rule is "each mount fires from its own muzzle", and every vehicle mount now does. Hand weapons were left body-centred. The alternative is a hand-weapon muzzle offset along the soldier's facing, which would also move where lean and line-of-fire checks start.

**The gap.** The per-mount muzzle rule was stated for vehicle mounts. Nothing said whether a soldier's weapon counts as a mount with its own muzzle.

**The reach.** Line-of-fire tests, friendly-in-line checks and the lean rule all start from this point. Changing it moves every digest.

**Verdict.** needs-user. Provisional: keep the body-centred start, because 0.8 m is below what the rules resolve and it keeps lean and cover simple. To reverse it, give soldier weapon rows a `muzzle_m` turned by the soldier's facing and let `weapons::muzzle` use it.

### A garrison sees, and spots, from one eye per facade it holds

***needs-user** · confidence **medium** · **provisional** · Sight, sensing and the simulation's fog · from decisions.md 27 perf (named decisions), decision 2; decisions.md 2026-09-27 provisional calls*

**The choice.** Eight soldiers hold a village house, two per wall. The squad sees from at most four eyes, one per facade (wall face) where a living soldier holds a slot. Each eye sits at the middle of that facade's slots at infantry eye height (`garrison::facade_eyes`). One list, `sensing::eyes`, feeds three things: the fog sweep, identification (spotting enemies), and the published `UnitSight::eyes` the renderer draws fog from. The drawn fog and what the garrison can identify always agree.
- The fog part was a performance cut the user approved. It cut village battles' cost by about 20% and changed no village digest.
- Using the same eyes for identification is a provisional call made while the user was away. The alternative, per-soldier eyes for identification only, would let a soldier at a wall's end spot an enemy round a corner that the drawn fog hides.

**The gap.** The user approved facade eyes for the fog sweep. Whether spotting follows was not asked.

**The reach.** A garrison sees round its building's corners only what the middle of the facade sees. A future "corner window" or per-soldier firing port would need its own eye.

**Verdict.** needs-user. Provisional: spotting uses the facade eyes, so the fog never lies. To reverse it, have `sensing::evaluate` read per-soldier eyes for a garrison while the fog sweep keeps `facade_eyes`. The drawn fog would then under-show what the garrison can spot.

### The far view stays at the default's 49° tilt instead of WARNO's horizon vista

***needs-user** · confidence **medium** · **provisional** · Camera and controls · from Slice 09*

**The choice.** The pitch curve today is `[25 m, 0.22 rad]`, `[65 m, 0.85]`, `[2000 m, 0.85]`. Zooming in from the default (65 m) lowers the camera toward the ground: at 25 m it looks almost level, with the horizon about where Broken Arrow puts it but from about 5.5 m up (Broken Arrow is about 20 m), so near units look bigger. Zooming out from the default keeps the same 0.85 rad (about 49°) tilt all the way to 2 km, so the strategic view looks down on the map rather than across to a horizon. WARNO's far view is flatter (about 0.4 rad). Getting there would need pitch to fall again as you zoom out, which the monotonic-curve rule forbids. The player can still middle-drag to tilt to a vista.

**The gap.** The contract asks for a monotonic curve, and the two references (Defilade's default, WARNO's far view) can't both sit on one.

**The reach.** What the strategic view looks like by default.

**Verdict.** needs-user — a taste call. Provisional: plateau at 0.85. To reverse, allow a non-monotonic curve in `CameraController` and add a flatter far point to `pitch_curve`.

### No trees stand inside the map outside the simulation's forests; hedgerows and copses stand only past the edge

***needs-user** · confidence **medium** · **provisional** · Light, terrain, grass and trees · from Slice 19*

**The choice.** WARNO-style maps have tree lines along roads and hedgerows between fields. Here, a drawn tree line inside the map that the simulation does not know would hide units the simulation says are visible. So today no tree stands inside the map except in the simulation's forests. Hedgerows (shrubs 4.2 m apart with trees 6–22 m apart in them) and copses stand only past the map, from `trees.backdrop.clear_m` (12 m) off it out to `reach_m` (1,500 m), on the patchwork's plot edges. The alternative is authoring in-map tree lines as real simulation forests (rect volumes in `village.json`), which changes sensing and balance.

**The gap.** The plan asked for WARNO's tree lines without deciding whether they are simulated.

**The reach.** The map's interior looks more open than WARNO's; in-map hedgerows would be a simulation and balance change.

**Verdict.** needs-user — provisional: none inside the map. To reverse, author narrow forest rects along roads and field edges in the fixture; drawn trees then follow automatically.

### Scar colours and strengths live in the biome file

***needs-user** · confidence **medium** · **provisional** · Light, terrain, grass and trees · from Slice 17; Slice 17b*

**The choice.** `fixtures/biomes/summer.json` `scars {crater {palette, full, strength, relief_m, rim, ejecta_palette, ejecta}, scorch, tracks, trampled, grass {thin, tracks_thin, flatten}}` and palettes `crater_soil` [0.3, 0.24, 0.17], `crater_ejecta`, `scorch` [0.11, 0.105, 0.1], `track_soil` [0.3, 0.25, 0.18] (strength 0.85: a rut is bare edged soil, not darker green), `trampled` (strength 0.15; the grass lying over carries it). `validateBiome` checks every field; a winter biome would be another file on the same schema. Technique constants stay in code, named beside `groundScars` (lip, ring, ash scale, depth cap 1.6 fulls, rim reach 1.5 cells, floor 0.7).

**The gap.** Material contrast was delegated.

**The reach.** A new biome retunes scars by data.

**Verdict.** needs-user — provisional: tuned on critique frames; reverse by editing `summer.json`.

### Grass over painted ground glows only near its root

***needs-user** · confidence **medium** · **provisional** · Light, terrain, grass and trees · from decisions.md 2026-09-27 provisional calls; Slice 27e (follow-ups); Slice 27 (muzzle flash) — the paint is a light; grass fringe fix*

**The choice.** A painted amber selection circle (or a painted blocked route) lies under long grass. A grass blade keeps its own colour and takes an additive glow in the paint's colour, strongest at the root: `presentation.overlay.paint.grass_glow` 0.8, falling off over `paint.grass_falloff_m` 0.06 m, so only the bottom few centimetres light up. A blade fragment that stands over the painted stroke on screen carries the stroke's glow at full strength, so the line runs unbroken through the grass without spreading past its edges. The alternative, a longer falloff (0.3 m, or a blade's full height), lit whole blades and grew a glowing fringe of blade tips out of every stroke, so marks read as fuzzy light columns in tall grass.

**The gap.** The user asked for paint on the grass and for grass to glow a little; how far up a blade the glow goes was not specified. Marked provisional while the user was away.

**The reach.** Look only; two fixture numbers. With order marks on the overlay layer under `yellow-orders`, this mainly affects the amber selection paint and other painted ground marks.

**Verdict.** needs-user. Provisional: short root glow with clean stroke edges. To reverse, raise `presentation.overlay.paint.grass_falloff_m` in `fixtures/village.json` (for example to a blade's height) and accept a fringe.

### The fog's ruled lines are fixed to the screen, not the ground

***needs-user** · confidence **medium** · **provisional** · How fog looks · from Slice 15*

**The choice.** The player pans the camera across fogged ground. The fine diagonal lines that mark unseen stay still on screen while the world slides under them, like a film on the glass. Screen anchoring keeps one spacing at every zoom and never shimmers or aliases. The alternative, lines painted onto the ground, would move with the world but thin out, crowd or alias as the camera zooms. A critique did describe the current lines as a film on the glass.

**The gap.** The look board named a texture, not what it is anchored to.

**The reach.** Every frame with fog; changing it is one more `FogStyle` field plus a world-space line in `fogLook`.

**Verdict.** needs-user — a taste call; provisional: keep screen-anchored. To reverse, add a world-anchored option to `FogStyle` and select it per style.

### Fog's face rule judges an upward face by the air above it, so low props can show lit ledges on an unseen side

***needs-user** · confidence **medium** · **provisional** · How fog looks · from Slice 37; Slice 37b (fog)*

**The choice.** Sandbags straddle the fog edge. Their far side is unseen, yet under the black fog style the top of each course shows a lit stripe 1–2 pixels wide. The cause: `fogTerm` decides whether a face is seen by probing 0.1 m along its normal. An upward ledge probes the air above it, which is seen, even though the bag courses above hide the ledge. The sight model doesn't know the sandbag is there, because a low body is not a sight occluder. The same gap lets a thin lintel on an unseen house wall count as a roof (the roof rule, `roof_reach_m`). This is left unfixed, because every sound fix changes a contract:
- a per-model hull, or a baked "bent normal" for fog (the average open direction, so a ledge under the next course points outward), needs an asset-pipeline change;
- low props as fog occluders would disagree with sim sight;
- limiting the roof rule to a body's top needs the fragment to know its occluder.

A screen-space cleanup would also erase true thin seen features. The fog-look check keeps its 2 px margin, and its comment names this cause.

**The gap.** The fog rule has no self-occlusion.

**The reach.** Ruins show the same artefact (their floor fogged while near wall tops read seen). The fix belongs with the model pass or a fog rework.

**Verdict.** needs-user. The provisional call is to leave the rule as it is until the model pass can bake a bent normal for fog. To reverse, add self-occlusion (a baked fog normal per model) when the asset bake is reopened.

### Model surfaces are vertex-colour paint, with baked textures on vehicles, soldiers and small props

***needs-user** · confidence **medium** · **provisional** · Models and the asset pipeline · from Slice 21; Slice 22; Orchestrator, after slices 21 and 22*

**The choice.** A house wall needs grime near the ground and lichen higher up; a tank needs printed camouflage, chipped paint and stencilled markings. Every model material is first a "paint": a function of world position, normal and edge sharpness, plus ray-cast ambient occlusion against the model and a ground plane, baked into vertex colour. Materials carry base colour 0.5 and the vertex colour holds albedo ÷ 0.5, so dark paints keep precision in 8 bits. Large faces are split so the paint has vertices to live on (0.28 m at LOD0 for vehicles, coarser on textured faces and for houses and the bridge). Houses use this paint alone and passed the look gate that way. Paint at 0.28 m spacing cannot draw chips, weave, mud or markings, and the unprimed critique called the soldiers, tank, truck and small props "toy-like" in every round. The user had chosen a real modelling budget with the critique as pass/fail, so the gate was not waived: those models also carry baked textures (albedo, normal map, ORM, with a side-tint mask in the ORM alpha; see the bundle format entry), and the vertex colour becomes macro variation over them. The alternatives were accepting the untextured vertex-colour look everywhere, or textures everywhere.

**The gap.** The slices' contract kept textures out of bundles, and the spike's wear and camouflage were Cycles-only nodes; passing the look gate needed a texture channel the plan did not have.

**The reach.** Every new vehicle, soldier or small prop is expected to be textured to pass the look gate. Texture memory is a budget of its own. Paint edges are soft at vertex spacing; anything finer needs the texture channel.

**Verdict.** needs-user (still listed as an open call). Provisional: textured appearances ship; houses stay paint-only. To reverse, drop the texture sets from the Blender scripts and rebake; the renderer already draws materials without textures.

### Effect curves and sizes are fixture tuning

***needs-user** · confidence **medium** · **provisional** · Effects and sound · from Slice 25*

**The choice.** Every look number lives in `presentation.effects` in `fixtures/village.json`: tracers (rifle 14 m × 0.07 m, HMG 18 × 0.10, AP 30 × 0.15, HE 20 × 0.14, grenade 2.5 × 0.08 dim, ATGM 3 × 0.35), flashes (rifle 0.25 m for 0.05 s; cannon 1.8 m with a 3.5 m fireball over 0.35 s; no star on muzzle glows, the tongue along the bore gives direction), impact puffs scaled by round kind, ten sparks per ricochet, and a blast fireball 0.9 × its radius (at least 3 m) over 1.6 s with a short compact flash and 32 sparks. Colours are HDR multipliers tuned under this lighting.

**The gap.** All curves were delegated.

**The reach.** Changing light or tone mapping means retuning these.

**Verdict.** needs-user — provisional: the numbers tuned on the gate frames; reverse by editing `presentation.effects`. Critiques left open: no scorch at the burst, sparks reading as zig-zags, tracers as rods from low cameras.

### A wreck burns from when the side first knows it: 60 s of fire, 150 s of smouldering

***needs-user** · confidence **medium** · **provisional** · Effects and sound · from Slice 26*

**The choice.** A tank is destroyed. When your side first learns its wreck, it draws flames and thick smoke for `burn_s` 60 s, then thin smoke for `smoulder_s` 150 s, then nothing. Knowledge carries no death time, so a wreck first seen long after it died (or known when a view starts) burns from then. A wreck no longer published stops making smoke and its puffs live out.

**The gap.** How long a wreck burns, and from when.

**The reach.** Adding a death tick to prop knowledge would let late-seen wrecks show their true age.

**Verdict.** needs-user — provisional: the durations are `presentation.effects.smoke.wreck` tuning; reverse there, or add a death time to the publication if late wrecks burning fresh looks wrong.

### Smoke, flame, dust and blast-plume looks are fixture tuning

***needs-user** · confidence **medium** · **provisional** · Effects and sound · from Slice 26*

**The choice.** In `presentation.effects`: wreck smoke sooty grey (albedo 0.16, opacity 0.7, 1.2 → 6 m over 10 s, rising 3 m/s), each wreck's column 0.75–1.25× in size by its own seed; smoulder thinner; flames 1.3 m, 14 a second with a flickering 3.5 m light; dust sandy (opacity 0.4, 0.8 → 3 m over 2.4 s); a blast's plume of 8 dirt puffs and 4 smoke puffs lasting 9 s; wind 0.6, 0.3 m/s (presentation only).

**The gap.** Looks were delegated.

**The reach.** If smoke hides units too much in play, opacity is the knob.

**Verdict.** needs-user — provisional: the gate-frame tuning; reverse by editing `presentation.effects`.

### Tracers are styled by round kind, not by side

***needs-user** · confidence **medium** · **provisional** · Effects and sound · from Slice 25*

**The choice.** A rifle tracer looks like a rifle tracer whether it is yours or the enemy's. `presentation.effects.tracers.<kind>` (rifle, hmg, tank_ap, tank_he, grenade, atgm) sets each round kind's colour and size, and the battle view draws no side colour on fire at all. The alternative keeps side colours (for example red for enemy, white for own) so a player can tell who is shooting from the fire alone.

**The gap.** The spec didn't say whether fire must show sides.

**The reach.** Players read side from unit markers and positions, not tracers.

**Verdict.** needs-user — provisional: kind-styled tracers (planning decisions called showing the enemy's round kind good). To reverse, add a side tint per tracer in `EffectFrame` or split the style table by side.

### Cover tier is a filled pip inside the soldier's marker: yellow, light green, strong green

***needs-user** · confidence **medium** · **provisional** · In-world UI and HUD · from Slice 27 (muzzle flash) — light cover cyan; cover tiers ramp; cover icons; cover icons centred*

**The choice.** Hold Space. Each soldier's marker, and each destination spot, shows a filled pip in its middle in its cover tier's colour: light is the orders' yellow [1.0, 0.9, 0.3], medium a clear light green [0.55, 0.95, 0.45], heavy a strong green [0.1, 0.8, 0.25] (`overlay.orders.cover`). With no cover, the marker's middle is empty. The pip is 0.3 m (`cover_pip_m`) inside a 0.45 m marker, clear of the marker's line. The user asked for 1.6×, which would have filled the circle. The pip is built into the same mesh as its marker, at the marker's own height, so it shares the marker's layer and cannot drift off-centre. `cover_glow` is 1, because the pips are overlay in `yellow-orders` and any extra glow only clamps toward white. Scene checks find pips by position and shape, since light cover shares the orders' yellow.

**The gap.** The user picked the colours reversibly. The pip size compromise and "light cover = order yellow" were consequences.

**The reach.** A light-cover pip is the same colour as the order marks around it. It is told apart only by being a filled dot inside a marker.

**Verdict.** needs-user. Provisional: the user's colours, pip at 0.3 m. To reverse it, change `overlay.orders.cover.light` to a hue distinct from the order colour.

### Prop hit points and the wreck chain are provisional numbers

***needs-user** · confidence **medium** · **provisional** · The catalog · from Slice 34c; Slice 27c*

**The choice.** The body rows in `fixtures/props/generic/` set: crate and fence 20, sandbags 150 (into rubble at 0.3 m), wall 300 (into rubble at 0.4 m), building 400 (into ruin at 2 m), and trunk 100, which the user set (see the weapons entry). Wrecks drop one weight class each time they are destroyed: a tank wreck (hp 400, heavy cover) becomes a supply wreck at 1.2 m (250, medium), which becomes a jeep wreck at 0.8 m (150, light), which is removed. Every wreck has `armor` 0.5, so AP mostly passes through a hulk and blast is what breaks it. Tooth, ruin, bridge deck and rubble have no hp. In rough terms: a crate or fence falls to one HE shell, sandbags to two, a wall to three, a building to four.

**The gap.** The numbers were delegated.

**The reach.** They set how long cover lasts under fire, which is the user's stated goal ("cover doesn't last forever").

**Verdict.** needs-user. The provisional call is to keep these values. They are checked as plausible, and balance has its own future spec. To reverse, edit the rows in `fixtures/props/generic/*.json`.

### Soldier kinds carry hit points, looks and weapons; body size stays one shared frame in `physics`

***needs-user** · confidence **medium** · **provisional** · The catalog · from Slice 27f; Sim lane (review fixes, props catalog)*

**The choice.** A rifleman and an AT gunner are different soldier kinds. Each has its own `hp`, appearance and mounts. But every soldier has the same radius (0.3 m), height, eye height, aim height (`infantry_aim_m` 1.0), centre height (`infantry_center_m` 0.9) and muzzle height, all in `physics`. Two things force this. Navigation's clearance grid is built for one soldier radius, and cover and the fit authority read one body frame. The aim and centre heights are their own rows, not derived from `soldier_height_m`: deriving them would have moved both values and every digest.

**The gap.** The user said each soldier kind has its own body numbers. The existing grid and cover code support only one.

**The reach.** A per-kind radius or height, such as a heavy weapons soldier, needs the navigation grid and cover to handle several frames.

**Verdict.** needs-user. This was constrained to what exists. Provisional: one shared frame. To reverse it, move the body numbers onto soldier kinds and build clearance and cover per radius.

## Unsound (0)

Working code resting on a decision that should be redone; each would name the corrected decision. None is open.

## Sound, confidence medium (167)

The architecture you now own. Within a confidence, provisional numbers come first, then the rest by area.

### HE shells hurt a tank a little even when they don't penetrate

***sound** · confidence **medium** · **provisional** · Simulation rules · from Slice 00*

**The choice.** A tank fires HE (high-explosive) at another tank. HE's penetration (30) is far below any tank face's armour, so it never pierces. Today it still does damage: the weapon row's `armor_fraction` (0.15) times its damage (80), so 12 hit points a hit, against AP's 40 when AP pierces. The row is `weapons.tank_he.armor_fraction` in `fixtures/village.json`, read in `damage.rs`. The alternative, a fraction of 0, would make a tank that ran out of AP harmless to other tanks.

**The gap.** The user said HE on armour is "partial, not nearly as effective as AP" but gave no number.

**The reach.** How long a tank with only HE left takes to kill another tank. The same field is the "failed penetration" damage a glancing round deals (see the ricochet damage entry).

**Verdict.** sound — matches the user's words. Provisional number: change `armor_fraction` on the HE row to retune; 0 turns partial damage off.

### A fallen soldier faces the squad's heading at the moment he fell

***sound** · confidence **medium** · **provisional** · Simulation rules · from Slice 05*

**The choice.** A soldier dies. The simulation records `Fallen {at, yaw}` with `yaw` = the squad's heading then, and it enters `Battle::digest`. Authored starting casualties take the setup yaw. The alternative, facing away from the killing round, would need the impact direction recorded at the fall.

**The gap.** Soldiers have no facing of their own in the simulation.

**The reach.** Every death pose and corpse orientation.

**Verdict.** sound, provisional — a death-direction yaw can be added later as its own field.

### A glancing round bounces near the mirror angle, slowed and weakened

***sound** · confidence **medium** · **provisional** · Simulation rules · from Slice 06*

**The choice.** A kinetic round that fails to pierce rolls the face's ricochet chance (tank `armor.ricochet {front 0.5, side 0.35, rear 0.2, roof 0.6}`, truck `{0.1, 0.1, 0.05, 0.15}`, jeep lower). On a bounce it flies on in the same tick along the mirror reflection turned randomly within `ricochet.scatter_deg` (12°), never within about 1° of the plate, keeping `speed_kept` (0.6) of its speed and `penetration_kept` (0.5) of its penetration, for up to `max_bounces` (2). Many glancing rounds dive into the ground nearby; some leave the map.

**The gap.** Delegated within a provisional range.

**The reach.** Where stray rounds go after hitting armour.

**Verdict.** sound, provisional — every number is in `village.json` (`ricochet`) and each hull's `armor.ricochet`.

### Crater and scorch size scale with the weapon's blast radius

***sound** · confidence **medium** · **provisional** · Simulation rules · from Slice 07*

**The choice.** A round bursts on open ground. Today its crater radius is blast radius × `ground.crater_radius_fraction` (0.15) and its scorch radius blast radius × `scorch_radius_fraction` (0.4). Crater depth at the centre is `crater_depth_per_m` (90) × crater radius, falling linearly to the rim, full at `crater_full_depth` (160). So a 12 m blast digs a 1.8 m crater, full at its centre; a 6 m blast a 0.9 m crater half full (not deep enough to be cover by itself); a 4 m blast a 0.6 m shallow one. Repeated bursts add up to full.

**The gap.** "Crater radius per weapon, from blast radius"; the accumulation curve was delegated.

**The reach.** Every blast in every battle; with the 0.5 cover threshold, it decides which weapons make cover in one hit.

**Verdict.** sound, provisional — tunable in the fixture's `ground` block.

### Cover tiers are three spread multipliers; each body's tier comes from its catalog row or its weight class

***sound** · confidence **medium** · **provisional** · Simulation rules · from Slice 33; Slice 34*

**The choice.** `cover.tiers` in `fixtures/village.json` holds light 1.4, medium 1.8, heavy 2.4: a round aimed at a soldier behind heavy cover scatters 2.4× as wide. Heavy stays under a garrisoned building's 3×. A prop's tier is its catalog body row's `cover_tier` (`fixtures/props/`: crate and fence light; trunk, sandbags and tooth medium; wall, ruin and building heavy; rubble light; bridge deck none). A vehicle, live or wrecked, takes its tier from its weight class (light/medium/heavy). A crater at least half full is light. `cover::validate` refuses tiers that do not widen the spread, heavier tiers that are narrower, and a wreck row whose tier differs from its vehicle's.

**The gap.** The spec named tiers, not numbers or where each body's tier lives.

**The reach.** Every new prop or vehicle gets cover by filling one column; balance tuning is three numbers.

**Verdict.** sound shape; the numbers are provisional for the balance spec (the user moved balance to its own spec). Reverse by editing `cover.tiers` or a row's `cover_tier`.

### Weight and push classes are ranks; a shove slows the pusher by the ratio

***sound** · confidence **medium** · **provisional** · Simulation rules · from Slice 34*

**The choice.** Weight ranks: light 1, medium 2, heavy 3, immovable beyond all. Push ranks: none 0, light 1, medium 2, heavy 3, super-heavy 4. A vehicle shoves a body only when the body's weight rank is strictly below its push rank, so a jeep (push light) shoves nothing. While shoving, the vehicle keeps `1 − weight/push` of its speed (`PushClass::shove_speed`): a tank against a light body keeps 2/3, a medium one 1/3; a truck against a light one 1/2. Fixture numbers: `pushing.turn_deg_per_m` 8, `pushing.relearn_m` 1.

**The gap.** Q2 asked for a class-ratio slowdown without a formula.

**The reach.** Every future push interaction is these two ranks; no per-pair table.

**Verdict.** sound rule; the pushing numbers are provisional for the balance spec. Reverse in the `pushing` fixture section.

### Vehicles see 100 % ahead, 50 % abeam and 30 % astern

***sound** · confidence **medium** · **provisional** · Sight, sensing and the simulation's fog · from Slice 04*

**The choice.** The tank and supply truck carry `sensors.sight_shape {front 1.0, side 0.5, rear 0.3}` in the unit catalog (`fixtures/units/generic/`). The jeep and infantry are even all round (`{1, 1, 1}`). Load refuses a shape that isn't `0 < rear ≤ side ≤ front`.

**The gap.** The user said "sides about 50 %, rear about 30 %" but left the values to the build.

**The reach.** How easily a tank is flanked.

**Verdict.** sound, provisional — edit each type's `sight_shape`.

### Forest densities: light, medium, dense; the village's west wood is medium, the orchard light

***sound** · confidence **medium** · **provisional** · Sight, sensing and the simulation's fog · from Slice 34b; Orchestrator, after slice 34b*

**The choice.** light: spacing 14 m, jitter 0.3, concealment infantry 0.7 / vehicle 0.9, 0.007/m, crown 8 m. medium: 9 m, 0.3, 0.45/0.7, 0.011/m, 6.5 m. dense: 6 m, 0.25, 0.3/0.5, 0.02/m, 5 m. Trunks are weight medium, so a heavy pusher (tank) knocks them; the truck and jeep cannot. The village's west wood (the ambush edge) is medium, its orchard light. Swapped, the flank captured more but the AT ambush in light forest died before sighting a tank.

**The gap.** Numbers and village assignment were delegated.

**The reach.** Strong balance lever: medium woods hide defenders well.

**Verdict.** sound; provisional for the balance spec. Reverse in `forests.densities` and each village forest's `density`.

### Benchmark memory is the frame's GPU allocations plus the main-thread JS heap

***sound** · confidence **medium** · **provisional** · Menu and benchmark · from Slice 10*

**The choice.** The benchmark reports memory as the frame's sized GPU buffers and textures (from the resource registry) and Chromium's `performance.memory` JS heap, read with each GPU reading, as the end value and the peak. The worker's WebAssembly memory (the simulation) is not reported. The heap peak tends to land just after warm-up garbage.

**The gap.** "Memory" was unspecified.

**The reach.** A simulation memory leak would not show in the benchmark.

**Verdict.** sound, provisional — add the worker's WebAssembly memory size to the row if simulation memory becomes a concern.

### Fog tracing budgets live in the fixture as provisional numbers, and a crowded screen tile drops eyes past its cap

***sound** · confidence **medium** · **provisional** · Renderer frame · from Slice 14*

**The choice.** The GPU fog builds a small sight map per eye, and each 16-pixel screen tile keeps a list of which eyes might see it. Five of the numbers that bound this work are fixture keys in `presentation.fog_geometry`: `first_bin_m` 1 (the first distance ring), `terrain_step_m` [0.5, 4] and `terrain_step_fraction` 0.01 (how finely the terrain is marched), `tile_eyes_max` 128 (eyes listed per tile), `rebuild_eyes_per_frame` 48 (sight maps rebuilt per frame). If a tile ever has more than 128 eyes that could see it, the extras are dropped and that tile's fog could be wrong; the lab's `tileCounts` probe would show it. The spike measured at most 72 eyes in a tile. At 1080p the tile lists cost about 4.2 MiB.

**The gap.** The plan said every provisional number goes in the fixture, and the spike named these budgets without keys.

**The reach.** A battle much denser than 100 units a side, or a smaller tile size, could hit the cap silently.

**Verdict.** sound, provisional — the numbers are tunable; to reverse, raise `tile_eyes_max` or `rebuild_eyes_per_frame` in `fixtures/village.json`.

### Model detail tiers are chosen by projected height; casters draw one tier coarser

***sound** · confidence **medium** · **provisional** · Renderer frame · from Slice 23; Slice 24*

**The choice.** A soldier 65 m away is about 35 pixels tall. Tiers come from projected height (`presentation.models {lod_px [150, 60, 24], impostor_px 10}`, `models/modelDetail.ts`), measured on each appearance's far-pose height (a corpse's from its length; a tank's from its swept bounds with the gun). At the default framing soldiers draw tier 2, at the closest zoom tier 1, and cards from about 230 m. Models more than 3 m (their shadow's reach) outside the view are skipped. Props enter with their size grown by their largest fitted scale. Every model's shadow caster draws one tier coarser (`CASTER_COARSER`), as the forest's do. Only skinned bodies have impostor cards; vehicles and props past the coarsest tier keep drawing it. A `tier` on a model still forces it (the workbench).

**The gap.** Delegated: LOD distances.

**The reach.** One detail rule for units and props; the tree layer uses the same `DetailView`.

**Verdict.** sound method; the pixel thresholds are provisional. Edit `presentation.models` in the fixture to change them.

### Vehicle mounts ease elevation and recoil but draw the turret bearing as published

***sound** · confidence **medium** · **provisional** · Renderer frame · from Slice 24*

**The choice.** A tank fires. A mount's published elevation is its last round's, so it changes only on a shot; the gun eases to it at `presentation.pose.mount.gun_elevation_rad_s` 0.6 (HMG 2). Each rise of the cannon's shot counter runs the gun back `recoil_m` 0.45 m, returning over `recoil_return_s` 0.9 s (quadratic ease). A vehicle first seen with rounds already fired does not recoil. HMG bursts move nothing. The turret bearing is not eased: it is the simulation's own traversed turret at 30 Hz, drawn as published. There is no suspension motion: the rig has no hull input, and rocking the hull needs the running gear split from the hull node. Identified enemy vehicles are drawn packed, since their deployment is unknown.

**The gap.** Delegated: mount feel.

**The reach.** Recoil and elevation are look numbers in the fixture. Suspension needs art and rig changes.

**Verdict.** sound; the feel numbers are provisional. Edit `presentation.pose.mount`.

### Impostor cards are baked by our own renderer at install time, 8 headings × 2 pitches, not shipped in bundles

***sound** · confidence **medium** · **provisional** · Renderer frame · from Slice 20; Slice 23*

**The choice.** A soldier far away is a few pixels, so he is drawn as an impostor: a flat card showing the model from the nearest angle. Our own renderer bakes the cards (`models/impostor.ts`): 8 yaws × 2 pitches (0.85 rad, the camera's far plateau, and 0.5), rendered at 2× and box-filtered on the CPU, storing unlit albedo, model-space normals and a side-tint mask, so the battle can relight cards and one atlas serves both sides. One orthographic frame fitted to the far pose's bounding sphere serves every cell, and the same inputs give the same bytes. The battle frame bakes each skinned appearance's far pose and corpse when appearances are installed (64 px cells, `CARD_SPEC`; about 56 ms for the three infantry kinds); the workbench bakes 128 px cells (`IMPOSTOR_SPEC`) for its sheets. A card uses the cell nearest the view direction, alpha-tested; it writes depth in the colour pass and casts no shadow. The rejected alternative was atlases shipped in bundles (a format change, LFS churn, and a GPU in the bake tool).

**The gap.** "LODs and impostors come from the port"; the layout and where atlases are made were left open.

**The reach.** Install time grows with appearances; atlases never go stale against the art. A card can jump between cells as the camera turns (under two pixels at 10 px).

**Verdict.** sound; provisional. To ship atlases in bundles instead, move the bake into the asset CLI and add an atlas view to the bundle format.

### Trees draw in their own scenery layer with per-tree detail tiers; props draw in the models layer

***sound** · confidence **medium** · **provisional** · Renderer frame · from Slice 19; Slice 24*

**The choice.** Thousands of trees, most far away, plus a few hundred props. Two layers draw static meshes. Trees draw in `frame/sceneryLayer.ts`: it reads the same bundles as models but scales and tints each instance and picks a detail tier per tree by projected height (`trees.lod_px` [260, 90, 26]). 128 m chunks too far for tier 2 draw whole at tier 3 from a static buffer; the backdrop scenery is culled to the view's sides, the forest never (it casts shadows into view). Shadow casters draw one tier coarser than the view (`CASTER_COARSER`), and leaf-clump noise is skipped where it has faded (about 0.5 ms saved). Its crowns take fog whole at their heart. It draws about 44,000 instances, which cost about 0.17 ms; thinning backdrop scenery was rejected because it is what breaks the horizon. Props joined the models layer instead, which already draws static bundles with materials, detail tiers, culling and cards, and gained a per-axis scale. Both layers choose tiers from one `DetailView`. The alternative was converging the two now.

**The gap.** The models layer's placement had no scale or tint and one tier per call; the spec said "consider converging".

**The reach.** Two owners of instanced static meshes. Folding the scenery's chunked population into the models layer is the step to one owner.

**Verdict.** sound for now, split by genuine need and measured; provisional. The reversal is that fold, when a third static population appears.

### Water is its own world layer, shaded by the terrain material, with a pale bare shore

***sound** · confidence **medium** · **provisional** · Light, terrain, grass and trees · from Slice 22b*

**The choice.** The world's water layer is drawn from the terrain material's `waterSurface` and `waterNormal`, one owner with the bed and the shore. Colour is the biome's `water` palette over deep water, with the bed showing at the edge; two ripple octaves fade with distance; low roughness reflects sky and sun; shadows keep 55% of the light. The quad extends 6 m past its rectangle to meet banks the 4 m height grid slopes, and the depth test hides it under higher ground. A biome `shore` (palette, `width_m` 2) paints pale bare earth along a ragged line round every water rect, with no grass. A dark wet-soil shore read as a shadow rim.

**The gap.** "Coordinate with the terrain's water, don't fork."

**The reach.** Water look is one material; rectangular water still reads as a canal (map authoring, not this).

**Verdict.** sound shape; the shore colour is a provisional look. Reverse by changing the biome's `shore` palette.

### The fog sight map is 4096 rays by 64 range bins per eye

***sound** · confidence **medium** · **provisional** · How fog looks · from Spike 02*

**The choice.** The drawn fog asks, for each pixel, whether any friendly eye (a unit or soldier that sees) sees that spot. Each eye has a sight map: `presentation.fog_geometry.azimuth_bins` 4096 directions by `radial_bins` 64 range steps, with terrain marched at 512 directions. That is about 1 MiB per eye. At 4096, the edge of a building's sight shadow stays within 1.4 px of the building corner at ground framing; 2048 would halve memory at about 5 px.

**The gap.** Resolution was left open.

**The reach.** GPU memory scales with eye count; a very large battle may need 2048.

**Verdict.** sound, provisional — lower `azimuth_bins` in the fixture to trade edge accuracy for memory.

### The run is the library jog with its upper-body twist damped

***sound** · confidence **medium** · **provisional** · Models and the asset pipeline · from Spike 03*

**The choice.** No rifle-run clip exists. Today the run is the library's jog with the spine and neck twist damped to 30 % of the original (`RUN_TWIST` in `clips_infantry.py`), so the weapon stays pointed forward.

**The gap.** No clip to use.

**The reach.** How running soldiers read.

**Verdict.** sound, provisional — change `RUN_TWIST` or author a dedicated run clip.

### Gun and HMG pitch are clamped by presentation constants

***sound** · confidence **medium** · **provisional** · Models and the asset pipeline · from Slice 20*

**The choice.** The simulation aims a tank gun at a target on a hill. Today the drawn gun pitch is clamped to −10°..+20°, the HMG to −10°..+45° (`PITCH_LIMITS` in `scene-assets/src/articulation.ts`), and the culling bounds sweep those limits. These are presentation constants, not simulation rules: the simulation's fire is not limited by them.

**The gap.** The spike's test strip went −8..+20 without saying who owns the limit.

**The reach.** If the simulation ever fires at steeper angles than the model can show, the gun will visibly point off its target.

**Verdict.** sound, provisional — to reverse, move the limits into per-type catalog data or a rule the simulation also obeys.

### The side's colour is a tint mask on materials, with the side colours in the asset catalog

***sound** · confidence **medium** · **provisional** · Models and the asset pipeline · from Slice 21; Slice 24*

**The choice.** A red tank and a blue tank use the same mesh. Each material carries a `tint` value from 0 to 1 (glTF material extras): how much of the side's colour it takes. Where a material has an ORM texture (occlusion, roughness, metalness), that texture's alpha multiplies the mask. Soldiers' uniforms and helmets take 1.0, gear and webbing 0.6, skin, boots and weapons 0. The tank's `tank_camo` and the truck's `truck_paint` take 1.0; wrecks take none. The side colours live in the asset catalog's `sides`: blue `[1, 1, 1]` (neutral, so own vehicles keep their camouflage) and red `[1.18, 1.0, 0.78]` (a shift toward sand). Each drawn model carries its tint in its instance record. Impostor cards are baked untinted, with the mask baked beside them, so one atlas serves both sides. Telling sides apart at a glance is the HUD's job, not the paint's. The alternative was separate meshes or textures per side.

**The gap.** The spec said "a tint mask" with no form, and not where side colours live.

**The reach.** Every new appearance must set `tint` on its materials, or it draws the same on both sides.

**Verdict.** sound for the mechanism; the red value and the neutral blue are provisional presentation. To change them, edit `sides` in `assets/catalog.json` and rebuild the runtime catalog.

### Detail tiers are authored per part, not decimated

***sound** · confidence **medium** · **provisional** · Models and the asset pipeline · from Slice 22*

**The choice.** The tank far away should drop its tow hooks. Each part declares which tiers it appears in (small detail drops out by LOD1–2), segment counts fall per tier (×1, 0.6, 0.36, 0.2), and bevels exist only on LOD0–1. The tank is about 74.5k / 20.4k / 3.7k / 1.0k triangles, the truck 35.3k / 10.4k / 1.9k / 280, houses 68–110k at LOD0 (the roof courses). The alternative was automatic decimation from one mesh.

**The gap.** "Four tiers" with no triangle budget.

**The reach.** The tank's LOD0 is about 2.3× the spike's, mostly paint splits. Memory in the battle is dominated by houses' LOD0.

**Verdict.** sound method; the triangle budget is provisional. To tighten it, lower the per-tier segment factors or the paint split size in `parts.py` and rebuild.

### Some props deliberately overhang their box, with per-appearance tolerances

***sound** · confidence **medium** · **provisional** · Models and the asset pipeline · from Slice 22*

**The choice.** A tank wreck's thrown track and askew turret reach past its hull box. Per-appearance footprint tolerances (catalog `tolerances.footprint_m`) allow it: houses and the ruin 0.5 m (eaves, chimneys, rubble), the tank wreck 3.0, the truck wreck 2.0, the jeep wreck 1.0, the field wall 0.15, the bridge 1.2 (a railing above the deck box). The alternative was making the art fit the box exactly, or growing the simulation boxes.

**The gap.** The spec did not cover art that overhangs a rule box on purpose.

**The reach.** A round or a sight line can pass through a drawn overhang (a wreck's thrown track) that the simulation doesn't have.

**Verdict.** sound as a shape; the values are provisional. To tighten, edit each appearance's `tolerances.footprint_m` and fix the art the validator then flags.

### Wreck models are dense: about 143k triangles for the tank at the finest level of detail

***sound** · confidence **medium** · **provisional** · Models and the asset pipeline · from Slice 22b*

**The choice.** The deformation needs densified plates, so the tank wreck's finest tier (LOD0) is about 143k triangles and the truck's 100k. A wreck draws that tier only up close.

**The gap.** No triangle budget for wrecks.

**The reach.** Frame cost of a battlefield strewn with close-up wrecks.

**Verdict.** sound for now; provisional. The lever is a coarser `EDGE_M` in the wreck script if the frame budget bites.

### Every sound is synthesised in code; there are no audio files

***sound** · confidence **medium** · **provisional** · Effects and sound · from Slice 40*

**The choice.** `packages/battle-audio/src/synth.ts` makes the whole bank (29 sounds: rifle, HMG and cannon near and far, grenade thump, missile launch and motor, impacts by kind, ricochet, explosions, footsteps, two engines, tracks, wheels, turret servo, reverse whine, fire, countryside ambience, four vague cue sounds) from seeded noise, sines, filters and envelopes, deterministic per name and sample rate. So there are no third-party audio files, no LFS audio, no file catalog and no licence to accept. The spec's "small Ogg/Opus files through a catalog" waits until a recorded sound replaces a synthesised one (one row of `SOUNDS`).

**The gap.** The spec assumed files; synthesis sidestepped sourcing and licensing.

**The reach.** Swapping in recordings later is a per-sound change.

**Verdict.** sound — provisional on the user's ear: reverse by adding recorded files for the sounds that fall short.

### The mix: four buses into a limiter, ambience kept well under the fight

***sound** · confidence **medium** · **provisional** · Effects and sound · from Slice 40*

**The choice.** `presentation.audio.buses`: master 0.9, units 0.7, effects 1.0, ambience 0.4, into a limiter (−6 dB threshold, ratio 20). The countryside bed is at gain 0.2; at 0.6 it sat within 2 dB of the effects' peaks.

**The gap.** Mix levels were unspecified.

**The reach.** Every new sound is judged against this mix.

**Verdict.** sound — provisional on the user's listen; reverse in `presentation.audio`.

### The jeep is a light, fast recon vehicle with a pedestal HMG

***sound** · confidence **medium** · **provisional** · The catalog · from Slice 34; Slice 36*

**The choice.** `fixtures/units/generic/vehicles.json` `jeep`: 9 m/s off road, 18 on it; hull half extents 2.2 × 1.0 × 0.95 m; eye 2.1 m; 40 hp; armour front 8, side 7, rear 6, roof 6 (the rifle's 5 stops, every heavier round gets through: a test pins that rule, not the numbers); 450 m all-round sight; 500 m loudness; cost 70; weight light, push light, leaves `jeep_wreck`; wheeled, 60°/s, 6 m radius, reverse 0.4×. Its HMG mount pivots on the hull origin at 1.68 m with the muzzle 1.43 m ahead (2.0 m up): the pedestal stands between the front seats, so a turret muzzle swung about the origin points forward. The art is a 4.4 m open-topped 4×4 with a folded windscreen so the HMG sweeps clear.

**The gap.** The jeep's stats and mount placement were delegated.

**The reach.** First light vehicle; its numbers belong to the balance spec.

**Verdict.** sound shape; numbers provisional for the balance spec. Reverse in the jeep's catalog row.

### Vehicle drive is a catalog `mobility` variant: tracked or wheeled

***sound** · confidence **medium** · **provisional** · The catalog · from Slice 39*

**The choice.** A vehicle's `mobility` is `tracked {mps, road_mps, turn_deg_s, reverse_fraction}` or `wheeled {…, turning_radius_m, …}` (`contract::catalog::Mobility`); only wheels have a radius. It is read once into the vehicle's drive state. Route planning never reads it. The follower's feel numbers are the fixture's `movement.drive` (pivot beyond 60°, abeam 1.5 m, and so on). Numbers: tank tracked 45°/s, 0.4× reverse; truck wheeled 40°/s, 9 m, 0.35×; jeep wheeled 60°/s, 6 m, 0.4×.

**The gap.** The spec named no turn rates for wheeled vehicles.

**The reach.** New vehicle kinds pick a variant; a rotor or air variant can join later.

**Verdict.** sound shape; numbers provisional for the balance spec. Reverse in each unit's `mobility` row.

### A glancing round deals the weapon's failed-penetration damage

***sound** · confidence **medium** · Simulation rules · from Slice 06*

**The choice.** A ricochet deals the weapon's `armor_fraction` of its damage and grants return fire only if that is above zero. Every kinetic row has fraction 0, so a ricochet hurts nothing today. A ricochet doesn't suppress by itself; the round's near misses on each leg do.

**The gap.** What a ricochet does to the hull.

**The reach.** Giving a kinetic row an `armor_fraction` would make glancing hits hurt.

**Verdict.** sound.

### A burst marks the ground only if it bursts near the ground, never on water, and after its damage is resolved

***sound** · confidence **medium** · Simulation rules · from Slice 07*

**The choice.** An HE shell hits a house roof, or a tank's turret. Today a burst digs only if it is within its crater radius of the terrain below (`GroundLayer::burst`), never on water, and only after the burst's damage is applied. So roof and high hull hits leave no crater, and a crater never shelters anyone from the round that dug it.

**The gap.** HE hitting a roof, a hull or a soldier was unaddressed.

**The reach.** Crater fields reflect where rounds landed, not everything they hit.

**Verdict.** sound — physical and order-safe.

### "Between" means the soldier's disc swept toward the shooter touches the body, not the bare line

***sound** · confidence **medium** · Simulation rules · from Slice 33*

**The choice.** A soldier peers past the edge of a tree trunk: his line of fire is clear, yet the trunk still counts as between him and the shooter, because his 0.3 m disc swept `reach_m` toward the shooter meets it. A soldier standing beside the end of a wall, whose disc sweep misses it, gets nothing. With the bare line, every covered soldier would also have his own line blocked by his cover and could never fire from it.

**The gap.** Q20 said "between" without a geometry.

**The reach.** Lets "in cover" and "can fire" hold at once, which the whole lean/step-out design relies on.

**Verdict.** sound — the disc radius is the soldier's, a rule not a tuning; medium only because a wider sweep would also be defensible.

### Cover applies to aimed fire at a seen soldier in the open; area fire and fire at vehicles get none

***sound** · confidence **medium** · Simulation rules · from Slice 33*

**The choice.** A rifleman fires at a seen enemy soldier: the aim point is that soldier, the shooter is his muzzle, and every tiered prop, every live hull of either side and the crater under the target count (`weapons.rs`, `cover::at`). A garrisoned squad gets its building shelter instead, never both. Area fire at a *contact* (a vague sound or sighting, not an identified unit) and fire at a ground point aim at no soldier, so no cover applies. A vehicle target never has cover.

**The gap.** Q20 said "a round aimed at a soldier"; area fire was unstated.

**The reach.** Suppressing a contact is not weakened by cover the shooter cannot see; blast damage still reaches covered men.

**Verdict.** sound — medium because giving area fire cover too would also be defensible.

### A squad's threat is the enemy it engages, else the nearest identified, else the last seen, else the facing or way it was sent

***sound** · confidence **medium** · Simulation rules · from Slice 33*

**The choice.** Cover is chosen *against* a threat point. The threat is, in order: the enemy one of the squad's weapons is engaging (an identified unit or a ground point); else the nearest enemy the side identified at its last sensing (movement runs before this tick's sensing); else the one last seen; else, at an order, 200 m off toward the right-drag facing or along the way the squad was sent (`cover::threat`, `take_cover::order_threat`). Area fire at a contact is not a threat point, since its centre is only an estimate. Against a seen enemy, a place counts only if the soldier there could engage one of the enemy's seen soldiers, straight or by leaning, within range.

**The gap.** Q7/Q9 named a threat without the fallback chain; D3 named step-outs without saying how they meet seeking.

**The reach.** Without the "can engage" filter, a squad behind a tall wall lost sight, fired area fire, and walked back behind the wall on the next re-resolve.

**Verdict.** sound.

### A body that blocks no infantry but has a tier is cover to whoever stands inside it

***sound** · confidence **medium** · Simulation rules · from Slice 34*

**The choice.** Rubble left by destroyed sandbags or walls blocks nobody but has a cover tier (light). A soldier standing inside its footprint is covered from every direction, as in a crater (`cover::Body::ground`, derived from `blocks.infantry`, no extra column). Cover spots for it run down its long middle. The alternative, rubble as a body you hide behind, would not match a low heap you lie in.

**The gap.** The spec named ground cover's tier, not how it differs from a body's.

**The reach.** Any future ground-type cover (a ditch, a shell scrape) is just a non-blocking tiered row.

**Verdict.** sound.

### A shove moves the body out along its least-overlap axis and turns it; no chain shoves; movement never writes the world

***sound** · confidence **medium** · Simulation rules · from Slice 34*

**The choice.** A tank meets a wreck. Box against box (`Obb2::separation`), the wreck slides out along the axis of least overlap, 1 cm clear, and turns by `turn_deg_per_m` × depth × a lever arm (where along the body the contact is), in three passes. A shove that would drive it into another vehicle-stopping body or a live hull, or off the map, fails and the vehicle stops as at a solid. `movement::advance` returns the tick's `Shove`s and `Battle` applies them after the movement pass, so later units in the same tick see the old pose. Soldiers the moved body now covers step out unhurt (`movement::clear_of`). A body that `topples` (a tree) is knocked down instead of sliding.

**The gap.** Q2/L8 asked for pushing without the mechanics.

**The reach.** Keeps movement a pure read of the world; chain-pushing is a future rule if ever wanted.

**Verdict.** sound.

### A tank carving through forest goes at forest speed; only the lane behind it becomes open ground

***sound** · confidence **medium** · Simulation rules · from Slice 34b*

**The choice.** Forest speed applies on *forest ground*: inside an authored forest rect, less cleared cells. Each tick, every vehicle able to knock trunks that moved clears the forest ground its hull has left behind, widened `ground.lane_margin_m` (0.5 m) each side, on 1 m cells; not the ground under the hull now. A knocked tree takes its foliage but not its ground, so the carving tank moves at forest speed; the jeep following uses the lane at open speed. Two stores are written deliberately: the world's cleared mask is authority for speed, sensing, fog and cover; the ground layer's `cleared` channel (255) is each side's learned knowledge of it, drawn as crushed ground.

**The gap.** Q16 said the lane stops being forest, not the carving tank's own speed.

**The reach.** Fire-felled trees use the same two writes.

**Verdict.** sound — medium because two stores of one fact is a deliberate duplication.

### What a soldier's cover markers mean: the side's plan at his spot, the true world where he stands

***sound** · confidence **medium** · Simulation rules · from Slice 35*

**The choice.** A squad is ordered behind a wall with an enemy seen to the east. Each soldier has two markers. The final marker sits at his spot while he moves, at his post while he holds, and otherwise where he stands. Its tier (`cover_there`) is `Soldier.cover`, the tier his place was given when the squad worked out its spots, so it reflects what the side knew then. The current marker's tier (`cover_now`) is `cover::at` read against the true world, meaning what incoming rounds would actually meet. It is measured from the squad's current threat direction (`Watch.threat`). With no threat yet, or while garrisoned in a building, it shows none. The alternative was to judge both against the side's own knowledge, or both against the true world.

**The gap.** The spec said "cover he has now" but named no threat to measure it from.

**The reach.** A player reading "no cover now" trusts the true world. Anything that later shows cover in advance has to choose which of the two truths it uses.

**Verdict.** sound. The planned spot reads the plan, and "now" reads what bullets would meet.

### How props take damage: direct hits scaled by armour, blast by distance, unshielded and unarmoured

***sound** · confidence **medium** · Simulation rules · from Slice 34c*

**The choice.** An HE shell bursts 3 m from a house. Only weapons with `structural_damage` wear props. The body a round strikes takes `structural_damage × armor`. On a burst, every destroyable body within the blast radius takes `structural_damage × (1 − r/R)`, where `r` is the distance from the burst to its footprint (`damage::blast_props`). Blast is not shielded by anything in between and not scaled by armour. The struck body takes the direct hit instead of blast. So a building near an HE burst wears too. The lab's `burst` event applies the same blast to props, but never to units. The alternative was blast that other bodies shield, or that armour reduces.

**The gap.** The seam gave the damage sources, but not whether blast is shielded or reads armour.

**The reach.** Wrecks (armour 0.5) fall mostly to blast rather than to AP. Near misses count against houses.

**Verdict.** sound. It is simple and reads right on screen. Shielded blast would be a costlier refinement.

### A felled tree clears only its own share of the forest floor

***sound** · confidence **medium** · Simulation rules · from Slice 34c*

**The choice.** A barrage fells nine trees in a wood. Each felled tree is knocked down, and its spot of forest ground is cleared (`WorldGeometry::clear_spot`). The spot is the forest ground within the density's spacing that lies nearer this trunk than any standing trunk. The clearing is written to the world mask and to the ground layer's `cleared` channel. So a shelled patch becomes open ground trunk to trunk, and one felled tree opens only its own share. The first idea, a half-spacing square per tree, left 26% of the patch as forest between jittered trunks.

**The gap.** "Its spot" had no size.

**The reach.** Foliage, concealment and the drawn fog all follow cleared ground.

**Verdict.** sound. Nearest-trunk partition is the general rule and leaves no leftovers.

### The village's works: a road block, sandbags by two houses, fences in 6 m panels, placed off the tanks' firing lines

***sound** · confidence **medium** · Simulation rules · from Slice 37; Slice 27a*

**The choice.** `fixtures/village.json` `map.props` places, after the three houses (so building indices don't move):
- A road block of 20 dragon's teeth across the main road at the village's west entrance (x 895, y 767–813, 2.4 m pitch, one-man gaps). Vehicles leave the road round either end, and squads thread through.
- Sandbags on the north house's west face (three sections and a return arm, x 947, y 737–752).
- Sandbags on the square facing the road's end (two sections and an arm, x 1012, y 790–800). The sections abut, so the corners close.
- Fences in 6 m panels, each its own body 0.2 m apart, so a shove moves one panel, not a 50 m line. There is a field fence across the approach to the west wood (x 600, y 872–922, with a gate), a garden fence behind the south house (y 905, with a gate), and a farm fence at blue's start, clear of both roads.

The works stand beside the houses because the defenders start garrisoned. They use the works once a house falls and they spill out. Every work sits off the lines from the flank script's tank position (`BOMBARD`) to the houses, because works on those lines stopped the shelling.

**The gap.** Placement was delegated.

**The reach.** The flank script's result depends on these lines. Moving a work onto a bombard line changes the balance.

**Verdict.** sound. It is reversible tuning; balance has its own future spec.

### The west wood stays medium density, with its north edge 60 m south so its AT team can be found

***sound** · confidence **medium** · Simulation rules · from Slice 37; Slice 27a*

**The choice.** The west wood is medium density (rect `[700, 880, 180, 160]`) and the orchard is light. Red's ambushing AT team spawns 6 m inside the west wood's edge (760, 886). With the team 60 m deep, blue's recon and tanks never identified it and it stalled the push. At 6 m in, blue finds and kills it in about 35–105 s. The alternative was to make the wood light density, which killed the ambush at once and changed the wood the user sees. Deeper spots were measured and gave worse capture rates.

**The gap.** The slice asked for a density per forest. The ambush depth fell out of balancing.

**The reach.** The AT team's position also decides the prompt-against-delayed retreat test (open, in the balance spec).

**Verdict.** sound. It is reversible tuning.

### Each side gets a jeep: blue's south of its start, red's in rear reserve

***sound** · confidence **medium** · Simulation rules · from Slice 37; Slice 27a*

**The choice.** The slice asked for jeeps, but encounter.md names five unit kinds a side. Blue's jeep is appended to its spawn at (125, 905), south of the start. Red's is appended at (1175, 800), a rear reserve at the flank road's end, out of the objective zone. Appending keeps every existing spawn index (the variants' disabled indices and garrison pairs), but red's unit ids each move up one. Red's jeep at the road block pinned blue's staging area with its HMG from minute one. Blue's jeep at the road junction drew fire across the supply truck's post and killed the truck in 9 of 10 seeds. Both jeeps fire at will. Blue's scripts leave its jeep idle, except the flank script, where it joins the push. Holding red's jeep's fire cost less but captured the same, so it wasn't done.

**The gap.** The roster wasn't the slice's to change, and the fixture couldn't then set a blue unit's engagement.

**The reach.** A red reserve jeep can duel a halted push from 500 m (an open item for the balance spec).

**Verdict.** sound. Adding kinds to the encounter is a user-visible change, but the slice asked for jeeps. To reverse, remove the two spawn rows.

### "Can break" counts the rounds left, so a hold never turns into wasted fire

***sound** · confidence **medium** · Simulation rules · from Slice 27c*

**The choice.** A tank aims AP at an enemy tank behind a tank wreck. AP does 10 × armour 0.5 = 5 per round, and it has 20 rounds, so 100 against the wreck's 400 hp. It holds. The test is `structural_damage × armor × rounds_left ≥ hp_left`, and unlimited ammunition always passes. Without it, the tank would empty its AP into the hulk, and a squad would throw its eight grenades into sandbags. Only direct hits count, not blast, which errs toward holding. Test: `a_gun_holds_fire_when_its_rounds_left_cannot_break_the_blocker`.

**The gap.** Added by the agent. The brief didn't mention ammunition.

**The reach.** The rule is honest only while ammunition is the scarce thing. With unlimited ammo (MGs, rifles), every breakable non-occluder gets fired into.

**Verdict.** sound. It is first-principled and one line.

### A body the ordered ground point lies in is the target, not an obstacle

***sound** · confidence **medium** · Simulation rules · from Slice 27c*

**The choice.** The flank script orders a ground attack at a house's centre. The arc meets the house wall first. Because the ordered point lies inside that body, the gun fires into it, whatever its row, occluder or not. This applies to any body the point lies in, so a ground attack on a ruin now fires into the ruin instead of holding. It applies only to an ordered ground point: a unit target standing inside a ruin is unchanged.

**The gap.** Ground attacks at bodies weren't specified once works in front had fallen.

**The reach.** HE fired at a ruin now blasts any squad hugging it.

**Verdict.** sound.

### While he leans out, his body is at the lean point, so the enemy can hit him there

***sound** · confidence **medium** · Simulation rules · from Slice 27d*

**The choice.** A soldier leans out past a trunk to fire. `Soldier::exposed(tick)` returns the lean point while he is out, and that is where incoming rounds test his body and where enemies aim. Everything else (movement, sight, his published position) stays where he tucks in. So leaning costs something: he is exposed while firing, as in a war film. The alternative, a lean that fires without moving the target body, would make leaning free.

**The gap.** The slice didn't say whether leaning moves the body the enemy shoots at.

**The reach.** Balance: lean fights trade exposure for fire. A balance pass may tune it.

**Verdict.** sound — the realistic trade; medium because its balance effect is unmeasured.

### A mount's aim bearing is still measured from the hull origin, not from its pivot

***sound** · confidence **medium** · Simulation rules · from Slice 27 (per-mount muzzles)*

**The choice.** The tank's roof HMG sits 0.6 m off the hull's centre. When it turns to a target 80 m away, the bearing it aims along, which the traverse and the fire tolerance read, is measured from the hull origin, not from the HMG's own pivot. The error is under half a degree at 80 m. The round's arc is solved from the true muzzle, so it still reaches the target. The alternative, measuring from each pivot, is exact but would move more of the traverse code and every digest.

**The gap.** Per-mount muzzles raised the question of where "pointing" is measured from. The spec did not answer it.

**The reach.** A mount mounted far off-centre, such as a sponson gun several metres from the hull centre, would aim visibly wrong at short range. At that point the bearing should come from the pivot.

**Verdict.** sound for today's mounts. Revisit only when a far off-axis mount appears.

### A squad's weapons come from its soldiers, slot by slot, with an operator rule per mount

***sound** · confidence **medium** · Simulation rules · from Slice 27f; Post-close review (sim)*

**The choice.** A squad type lists soldier kinds by slot. The squad's mounts are the union of its soldiers' mounts: each mount name appears once, in slot order, with the slots that carry it. Two soldier kinds that carry different mounts under the same name are refused. Who fires a mount (`weapons::operator`):
- a `squad` mount: every living carrier fires;
- any other mount: its first living carrier, in whatever slot;
- a `special` mount (today the grenade launcher and the ATGM) falls back to the first living soldier only when every carrier has fallen, so it passes on until the squad is gone;
- a mount that is neither, with no living carrier, is lost with him and stands idle (`no_compatible_target`).

Where the operator fires it from is the operator-muzzle entry.

**The gap.** Moving weapons onto soldier kinds raised the question of who operates what. The spec did not say.

**The reach.** A crew-served weapon whose carrier dies goes quiet. A special weapon never goes quiet while anyone lives, which is generous and may want revisiting.

**Verdict.** sound. It is explicit, and a carrier keeps his own weapon.

### Tuning numbers live in the fixture; geometric tolerances stay in code

***sound** · confidence **medium** · Simulation rules · from Sim lane (review fixes, props catalog)*

**The choice.** A number that changes how the battle plays is a fixture row in `fixtures/village.json`: `cover.{lean_hold_s, lean_max_m, lean_clear_m, lean_apart_m, standoff_m, away_cos, search_slack_m}`; `movement.drive.*`, carried on each vehicle's drive; `ground.{track_gauge, lane_margin_m}`; `physics.{infantry_aim_m, infantry_center_m}`; `infantry_movement.path_clearance_m`. A number that is only a sampling step or float tolerance stays a code constant: `lean::GRAZE_M`, `lean::AT_PLACE_M`, `cover::STEP_RING_M`, `cover::FAR_M`, `drive::PROBE_M`, `take_cover::IN_PLACE_M`. A squad's footprint adds `soldier_radius_m`, and route clearance is its own `path_clearance_m`, rather than one half-width constant meaning both. The alternative was leaving behaviour numbers as code constants.

**The gap.** The review said "tuning belongs in the fixture". It did not draw the line.

**The reach.** Balance work can tune these without code changes. `AT_PLACE_M` sits in `Soldier::leaning`, which has no rules to hand, so moving it later means threading rules through every `exposed` caller.

**Verdict.** sound. The line is behaviour against numerics.

### A leaning soldier fires a burst, then tucks back in for a while

***sound** · confidence **medium** · Simulation rules · from Slice 27d*

**The choice.** Rifles have unlimited ammunition, so a leaning man who leaned out only while firing would stay out for the whole fight. A stretch out lasts at most `cover.lean_burst_s` (3 s). Then he stays in until `cover.lean_tuck_s` (2 s) has passed; tucked in, he fires only along a straight line. After his last round from the lean point he stays out `cover.lean_hold_s` (1.5 s), the firing pose's hold. The alternative, leaning out while he fires with no cap, left men out forever.

**The gap.** The slice's picture ("fires from there, and tucks back in") had no rhythm.

**The reach.** It cuts a leaner's fire by roughly 40%, unmeasured on balance. The numbers are fixture rules, validated positive.

**Verdict.** sound — gives the picture the slice asked for; medium on the numbers.

### A squad's single weapon fires from its operator's own muzzle, under the rifleman's rules

***sound** · confidence **medium** · Simulation rules · from Post-close review (sim)*

**The choice.** An AT team's gunner kneels 4 m left of his team's middle, behind a garden wall, and fires at a tank. His missile leaves from his own muzzle, or from his lean point when he is leaning out round cover (`fire_from`), both when the team chooses a target (`engage`) and when it fires (`fire`). Every squad shot, a single weapon or a rifle volley, goes through that one per-soldier path; only a hull fires from mount geometry. And like each rifleman already (`hides_behind`), a gunner in the open whose own cover is in the way of his target holds his round. Before, a single weapon left from the squad's middle, a point where nobody stands. One thing is still judged from the squad's middle: a squad weapon's first reach test, before the per-soldier check, which the finding didn't cover. Rejected: exempting a single weapon from the hold rule.

**The gap.** The per-soldier muzzle rule covered rifles. Single weapons were left on the old middle point.

**The reach.** A named digest change that shifted balance: a retreating tank now takes the ambush's second missile (see the needs-user entry on the ambush). Any future crew weapon fires from whoever operates it.

**Verdict.** sound. A round leaves from the man who fires it. Medium, because the first reach test still uses the middle.

### A roof or canopy top counts as seen when the air just in front of it toward an eye is seen

***sound** · confidence **medium** · Sight, sensing and the simulation's fog · from Slice 14; Slice 15*

**The choice.** A blue squad on the street looks at a two-storey house. Its roof faces the sky, above every eye, so a strict "is this face turned to an eye" test would always fog it. Today a surface facing up (normal z ≥ 0.7) and above an eye counts as seen by that eye if the point at the roof's own height, pulled up to `presentation.fog_geometry.roof_reach_m` (40 m; 0 turns it off) toward the eye, is visible from the eye with no facing test, and the roof is within the eye's range. So a roof reads like its building's near wall; a lower roof hidden behind a taller building stays unseen; a canopy top reads seen as far as sight into the foliage allows plus the reach. This lives in `fogSeenSurface`, which the fragment term and the lab probe share. The rejected alternative, "a roof is seen if the ground under it is", fails because that ground is inside the building's own blocker and never seen.

**The gap.** The spike handed roofs and canopy tops to the fog look without a rule.

**The reach.** Walls facing away from every eye stay unseen under a seen roof, so buildings can look half-fogged; this is geometry, not a bug.

**Verdict.** sound — a general rule from the eye's own sight line, not a roof special case.

### Each side remembers where it last saw each moved body; planning and drawing use that, local steering uses the true box

***sound** · confidence **medium** · Sight, sensing and the simulation's fog · from Slice 34*

**The choice.** A red tank shoves a crate out of blue's sight. `SideGeometry::seen` maps each body to its last-seen centre and yaw; an authored body with no entry is believed where the map put it. Before any shove, every side without an entry records the old pose, so blue keeps planning round the crate where it was. A side re-learns a body when it sees it more than `relearn_m` from its belief or at rest; each learning bumps the side's revision. The route grid and the publication read these beliefs. A soldier's local steering, spots and cover seeking read the true boxes of the bodies his side knows: a shoved body is one some unit touched or saw, and beliefs there would need a second prop index per side.

**The gap.** L1 named the store, not every reader.

**The reach.** Hidden shoves never leak; a soldier could in rare cases dodge a body at its new place before his side "knows" it moved.

**Verdict.** sound as a bounded stopgap — the true-box read is limited to bodies already known.

### Foliage is an 8 m cell grid built from the trunks whose crowns cover each cell

***sound** · confidence **medium** · Sight, sensing and the simulation's fog · from Slice 34b*

**The choice.** A cell's strength is `s = 1 − Π(1 − conceals)` over the standing trunks whose crown (`canopy_radius_m`) covers its centre. The densest forest's row there scales it: depth per metre `attenuation_per_m · s`; range multipliers `1 + (concealment_<class> − 1) · s`. Knocking a trunk refreshes the cells its crown reached. Light forest is therefore patchy (a cell no crown reaches is open), and a knocked lane opens the 8 m cells only its trees covered. Only trunks conceal today (a crate would need a crown).

**The gap.** Q21 gave the product, not how per-class numbers combine.

**The reach.** Foliage overhangs a forest's rectangle by up to a crown; lanes open in 8 m blocks.

**Verdict.** sound.

### A crater slows a vehicle only by the cell under its centre, and only its driving speed

***sound** · confidence **medium** · Movement, cover and pushing · from Slice 07*

**The choice.** A 7 m tank drives over a 1 m crater cell. Today the speed multiplier reads the cell under the hull's centre (`GroundLayer::vehicle_speed`: 1 − (1 − `crater_vehicle_mult` 0.85) × fill), and scales translation only; turning in place, turret slew and infantry are never slowed. A tank straddling a crater with its centre off it is not slowed. Navigation never reads craters, so a crater never replans anyone.

**The gap.** How a large hull samples small cells.

**The reach.** Slowdown is "slight" by design; sampling the whole footprint would be a later refinement.

**Verdict.** sound for "slightly".

### When a lane would run into a body, the soldier tries nearby offsets before falling back to the corridor

***sound** · confidence **medium** · Movement, cover and pushing · from Slice 32*

**The choice.** A soldier walking toward a line of concrete anti-tank teeth checks his lane ahead: from where he stands, through his steer point and on up to `lane_lookahead_m` (8 m) along the lane, his 0.3 m disc must cross no body his side knows about. If his wanted offset is blocked he tries, in order, the offset shifted by ±0.5, ±1, ±2, ±3 m, then half the offset, and finally the corridor itself (`movement::soldier::lane_offset`). The sideways shifts make each man pick his own gap between teeth, and let half a squad pass north of a crate stack while the rest pass south. The unbuilt alternative, simply narrowing every lane toward the corridor, funnels the whole squad through the corridor's one gap.

**The gap.** The mock lesson said only "the lane falls back toward the corridor".

**The reach.** Squads flow round small obstacles without replanning. The 3 m ladder is a code constant, not a fixture row.

**Verdict.** sound — a search reads naturally; the ±3 m bound is tuning living in code, which is the only doubt.

### The squad plans its corridor from the soldier nearest its middle, and each soldier starts at his current offset from it

***sound** · confidence **medium** · Movement, cover and pushing · from Slice 32*

**The choice.** A squad split by a wall has its geometric middle inside the wall. So the corridor is planned from the living soldier nearest the middle. The unit remembers where its corridor starts (`Unit.route_from`: the planning start, then the last waypoint every soldier has passed). On a new corridor each soldier's lane offset starts at his current side offset from it (`movement::soldier::join`), so nobody jumps onto the line. Stragglers behind the wall use the rejoin rule.

**The gap.** The spec did not say where a squad's route starts.

**The reach.** A badly split squad can get a corridor that suits only half of it; the rejoin rule absorbs that.

**Verdict.** sound — a real point on the ground beats a centroid; medium because a very split squad still leans on stragglers rejoining.

### A holding soldier walks to his post on his own route, and while he does the squad counts as moving

***sound** · confidence **medium** · Movement, cover and pushing · from Slice 33*

**The choice.** After a re-resolve, a soldier's new place is his *post* (`Soldier.post`). He walks there on his own fine route and stands on reaching it; if jammed (someone in the way) he replans round the soldiers about him. Posts survive a one-tick lapse of an attack-move's halt (the halt flickers today) and are cleared on arrival and every new plan. While a soldier walks to his post the squad's centre moves, so the squad counts as moving: moving spread, stationary weapons lose aim, supply sees it moving.

**The gap.** How re-resolved places are reached, and what that does to "stationary", was unspecified.

**The reach.** A squad shuffling into cover pays the moving penalties, which is realistic but can surprise a player whose squad is "idle".

**Verdict.** sound — medium because exempting short cover shuffles from the moving penalty is a defensible alternative.

### Vehicles weigh shoving a body against driving round it in the route search

***sound** · confidence **medium** · Movement, cover and pushing · from Slice 34*

**The choice.** Each vehicle route cell remembers the heaviest known body over it; a push class enters the cell only if it can shove that body. Clearance fields are built the first time a class plans and shared by classes that meet the same bodies. A shoved cell costs its step divided by the shove speed, so A* compares a shove with a detour: in open ground it detours, in a walled lane it shoves. A pusher replans when its side's knowledge changes and its remaining route crosses a body it would shove (`NavGrid::route_pushes`), as well as when the route no longer fits.

**The gap.** Q13/L7 asked for navigation classes, not the cost model.

**The reach.** Tanks do not plough through crates when a road round them exists.

**Verdict.** sound.

### Wheeled vehicles cut corners slightly and drop waypoints they pass abeam

***sound** · confidence **medium** · Movement, cover and pushing · from Slice 39*

**The choice.** A truck drops a waypoint within 0.5 m, one it passes abeam within `movement.drive.abeam_m` (1.5 m), and a corner early: at the fillet tangent `radius · tan(turn/2)`, capped at half a radius, so a corner is cut by at most about 0.1 radius. Route smoothing keeps its clearance, so the cut stays within that margin.

**The gap.** The spec delegated the arc construction.

**The reach.** Tight village corners are the risk: a truck could clip a wall corner the planner cleared.

**Verdict.** sound — medium; watch tight corners.

### A three-point turn is a reversing leg the follower starts, not a plan

***sound** · confidence **medium** · Movement, cover and pushing · from Slice 39*

**The choice.** A truck needs to turn round in a 10 m lane. When the next waypoint (over 1.5 m away) is inside its turning circle, or a turn over 20° would hit a solid within its next metre of arc, it starts a leg driven against the order's direction, turning the same way (`Unit::manoeuvre`, in the digest). The leg ends when the waypoint is outside the circle by 0.5 m and the whole forward arc is clear, after a quarter circle, or when its own next metre meets a solid. A tight lane takes as many points as it needs. Solids are the bodies that stop the hull, not live vehicles (traffic still waits). A forward turn counts as progress, so a U-turn is not a stall.

**The gap.** Q29 asked for three-point turns, not the mechanism.

**The reach.** No reverse planning; a narrow lane may take more than three points.

**Verdict.** sound.

### Final facing: the ordered facing if the unit can turn to it, otherwise the way it arrives; wheels never pivot

***sound** · confidence **medium** · Movement, cover and pushing · from Slice 35*

**The choice.** The player right-drags a jeep's order to face north, but the jeep drives in from the west. `movement::final_yaw` takes the ordered facing only for squads and tracked vehicles. Otherwise it uses the bearing of the route's last leg, plus 180° on a reverse move, and failing that the unit's current yaw. A wheeled vehicle can't turn on the spot, so it ignores a drag facing, and its marker shows the way it will actually come in (east). The alternative was to make wheeled vehicles do a turning manoeuvre at the goal, or to show a facing they can't reach.

**The gap.** The spec didn't say what wheeled vehicles do with an ordered facing.

**The reach.** If jeeps ever get a three-point turn, this is where their facing rule changes.

**Verdict.** sound. The marker never promises a facing the vehicle can't take (matches Q29: wheels steer, never pivot).

### Soldiers are assigned places in one greedy pass, not an optimal assignment

***sound** · confidence **medium** · Movement, cover and pushing · from Slice 27d*

**The choice.** Eight soldiers, a dozen candidate places, some with lean points. `cover::claim` lists every (soldier, place, lean) offer and sorts it: can he engage from it, then cover tier, then "he's already there", then distance, then a lean before none. It walks the list once and gives each soldier the first free offer. A place is `cover::Place { at, tier, direct, leans }`. A lean point is taken like a place, and none may be within `cover.lean_apart_m` (0.8 m) of another taken place or lean point. A soldier shoved outside the area is offered no "stay" and walks back in to the nearest free room. The alternative is an optimal assignment (e.g. Hungarian); conflicts are rare (two spots sharing a lean point), and the step-out rule (next entry) catches whoever is left unable to fire.

**The gap.** The slice named the goal, not the algorithm.

**The reach.** Cheap and deterministic; a later cover rule adds a sort key rather than a new solver.

**Verdict.** sound — medium only because greedy can leave a worse total than optimal in crowded spots.

### A soldier jammed against a squadmate re-plans his last metres around standing soldiers

***sound** · confidence **medium** · Movement, cover and pushing · from Sim lane (review fixes, props catalog); decisions.md Trenches removed*

**The choice.** A defender's post is 0.5 m off a house wall. A squadmate already stands at the corner, also 0.5 m off the wall, leaving a 0.2 m gap. The first man's own route ran through that gap, and he stood pinned for 35 s. Two changes fix it. The fine route (`movement::final_leg`, a 0.5 m grid over a small window) now treats standing soldiers as discs beside the bodies' boxes. A holding soldier who makes no progress toward a post more than `SETTLE_M` (1 m) away re-plans around the soldiers within 2 m, at most once a second. Two alternatives were rejected. Holding a waypoint until the next leg is clear did not help: the leg was clear, the man was not. Insetting corner posts did not help either, because the gap exists wherever two posts share a face.

**The gap.** A real movement bug, exposed when the trench was removed. How to fix it was open.

**The reach.** A man jammed by the crowd for a single tick pays for a search but changes nothing he does. The fine route now depends on where other soldiers stand.

**Verdict.** sound. It is a general crowd rule, not a corner special case.

### The route grid holds a 4×4 sub-cell mask per 2 m cell, so infantry routes find gaps narrower than a cell

***sound** · confidence **medium** · Movement, cover and pushing · from Slice 32*

**The choice.** Squad corridors are planned on a 2 m grid (`navigation::NavGrid`). Each 2 m cell also stores which of its sixteen 0.5 m sub-cells a 0.3 m soldier disc fits in. The cell is open to infantry when its free sub-cells form one connected gap; a step between two cells is open when free sub-cells touch across the shared edge (a diagonal step also needs both side steps open). A corridor step through a narrow gap is placed at the middle of the longest open run of the shared edge, so it runs down the gap's middle, not through a tooth. Next to untraversable ground each sub-cell reads the ground under its own centre. A cell split in two by a thin wall is simply closed rather than tracked as two nodes. Vehicles use the whole-cell grid with a clearance field; infantry planning ignores `Mobility.half_width_m`. The alternative was one resolution for both, closing gaps narrower than a cell to infantry.

**The gap.** Q27 asked for two resolutions without the data layout.

**The reach.** A line of teeth or a gap between wrecks stays open to infantry and shut to vehicles, from geometry alone (the user's first-principles rule).

**Verdict.** sound — the one known cost: closing a split cell can close a real gap where a thin wall runs down a cell's middle; worth a scenario if a map hits it.

### The renderer keeps a TypeScript and a WGSL copy of the sight formula

***sound** · confidence **medium** · Contracts and seams · from Slice 04*

**The choice.** The fog is drawn on the GPU, and debug overlays on the CPU, and neither can import Rust. Today `sightMultiplier` in `packages/battle-renderer/src/sightOverlay.ts` mirrors `sight::multiplier` (used by the lobe overlay and the fog oracle, a CPU reference for fog tests), and `fogShape` in `frame/fogTerm.ts` mirrors it in shader code. Tests check both against the formula's anchor values. The alternative, publishing the reach per direction, would cost bandwidth per eye.

**The gap.** The renderer needs the formula.

**The reach.** Changing the rule means changing three places; tests catch a mismatch.

**Verdict.** sound — named mirrors with tests.

### A weapon's shot counter counts rounds, so a squad volley adds one per soldier

***sound** · confidence **medium** · Contracts and seams · from Slice 05*

**The choice.** Each published weapon pose carries `shots`, rounds launched since the battle began. A cannon's rises by 1 a shot; a squad's rifle mount rises by the number of soldiers who fired. The renderer reads a rise between two publications as a shot, and learns which soldier fired from each tracer's `shooter_member`. The alternative, counting firing events, would lose how many rounds a volley had.

**The gap.** "Cumulative shot counter" didn't say rounds or events.

**The reach.** Muzzle flashes and recoil.

**Verdict.** sound.

### A gun's published elevation is the pitch of its last round

***sound** · confidence **medium** · Contracts and seams · from Slice 05*

**The choice.** `WeaponPose.elevation` is the launch angle of the mount's last launched round (from its launch velocity), held between shots and 0 before the first. For a squad volley it is the last soldier's round. So a gun's barrel moves when it fires, not while it lays on a target; the renderer eases toward it. Bearing turns every tick. The alternative, publishing the aim elevation continuously, needs an aim solve every tick.

**The gap.** The contract named elevation without saying aim or launch.

**The reach.** Barrels snap to firing elevation at the shot.

**Verdict.** sound.

### Enemy weapon poses are shown only while the enemy is identified this tick

***sound** · confidence **medium** · Contracts and seams · from Slice 05*

**The choice.** An enemy tank is identified: the publication carries its true mount states. When it is lost (even during the short grace before a contact is dropped), no pose is published or remembered. When it is re-identified, its shot counter shows the true total, so shots it fired unseen count in one step — which reveals nothing the fire contacts didn't already give.

**The gap.** "Mounts of identified enemies".

**The reach.** What an enemy's turret shows; fog-of-war disclosure.

**Verdict.** sound.

### Own blasts are always published; enemy blasts only where the side sees

***sound** · confidence **medium** · Contracts and seams · from Slice 05*

**The choice.** A player's shell bursts in unseen ground: the burst is shown. An enemy shell bursts there: it is not. Own tracers and impacts already publish whole, so own blasts match them. An enemy blast is published only when its burst point's fog cell is seen. The blast point is the impact point.

**The gap.** "Clipped to seen ground" didn't separate own from enemy.

**The reach.** What the player learns from explosions.

**Verdict.** sound.

### Large integers in the observation travel as two 16-bit halves

***sound** · confidence **medium** · Contracts and seams · from Slice 05*

**The choice.** The observation is packed as 32-bit floats, which hold integers exactly only up to 2²⁴. Soldier ids, shot counters, shooters, guided missile ids and ground cells travel as `<name>Lo`/`<name>Hi` pairs of 16-bit halves (limbs). The tick stays a single float; it passes 2²⁴ after 6.5 days at 30 Hz.

**The gap.** "Integers go into 16-bit limbs" didn't say which.

**The reach.** Any new integer field that can grow large needs limbs.

**Verdict.** sound.

### A full ground snapshot always goes in one record, never split

***sound** · confidence **medium** · Contracts and seams · from Slice 08*

**The choice.** On a side switch two minutes into the village, blue knows about 4.4k cells: a 70 KB snapshot in one publication. After a big barrage it can be about 200 KB. Snapshots are sized by what the side has learned, not by the map. Ordinary patches stay small (p95 about 7 KB a tick at 100 a side). The rejected alternative was chunked snapshots, which need partial cursors.

**The gap.** Not specified.

**The reach.** A late-battle side switch sends one large record; a hitch there would be the first sign to chunk.

**Verdict.** sound — revisit only if a side switch hitches.

### The client keeps a dense ground view: four bytes per map cell, plus a changed-cell list

***sound** · confidence **medium** · Contracts and seams · from Slice 08*

**The choice.** The client's `GroundView` (`web/src/battle/sim/ground.ts`) is one RGBA8 texture's worth of bytes per map cell (10 MiB for the village) and the exact list of cells changed since the renderer last asked. Past a sixteenth of the map unasked, the list collapses to "everything changed". A delta that doesn't start at the view's revision, or a new epoch that isn't a snapshot, throws: the transport is ordered, so that is a bug, not something to recover from. The alternative was a sparse client store.

**The gap.** The consumer's shape was left to the scars slice.

**The reach.** The renderer uploads dirty cells to a texture directly. Memory grows with map area, not with what is learned.

**Verdict.** sound.

### Order markers are sent as a full record every publication, not as deltas

***sound** · confidence **medium** · Contracts and seams · from Slice 35*

**The choice.** A squad's soldiers keep moving toward their spots. Each publication resends every own unit's `memberOrders` whole: 4 numbers a soldier, about 1 KB for 100 soldiers a side. The alternative was a delta cursor like the ground patch uses, sending only the rows that changed. That would make the reader keep state for bytes that don't matter.

**The gap.** The slice said "deltas when they change".

**The reach.** A much larger army, or a network client, might want deltas later. The record's layout doesn't stop anyone adding a cursor then.

**Verdict.** sound at this size. Revisit if own-unit rows ever grow by an order of magnitude.

### A resolved cover spot lives on the soldier, not on the order

***sound** · confidence **medium** · Contracts and seams · from Slice 33*

**The choice.** When a squad is ordered somewhere, it claims cover places there on the same tick as the right-click (commands apply before movement). The chosen place is stored as the soldier's own spot (`Soldier.spot`, with `Soldier.cover` the tier it gives), and later cover moves as his post (`Soldier.post`). It is not copied onto the order. The publication reads both from the soldier. How places are claimed (the squad's area, most soldiers able to engage, then strongest cover, then least walking) is the squad-area rule (see the anchor and claim entries).

**The gap.** The seam said "stored on the order".

**The reach.** One owner for a soldier's destination; the order markers and cover icons read it there.

**Verdict.** sound — a second copy on the order would be a second owner.

### The wheel shifts pitch by the curve's change, keeping any tilt the player dragged

***sound** · confidence **medium** · Camera and controls · from Slice 09*

**The choice.** The player middle-drags to tilt the camera 0.1 rad flatter than the curve, then scrolls. Today the wheel adds the curve's change in pitch between the old and new distance to the current pitch, so the 0.1 rad offset survives the zoom. A camera that was on the curve stays on it. The alternative, snapping pitch onto the curve on every wheel notch, would throw away the player's tilt.

**The gap.** "Pitch linked to zoom in the wheel gesture only" didn't say what happens to a dragged tilt.

**The reach.** Camera feel.

**Verdict.** sound — respects the player's input.

### The field of view is one fixed angle at every zoom

***sound** · confidence **medium** · Camera and controls · from Slice 09*

**The choice.** The camera's vertical field of view is `presentation.camera.fov_y` (0.8 rad) at every distance. The alternative, a field of view that narrows as you zoom out (Defilade looks almost orthographic from above; Broken Arrow uses a wide lens near the ground), would need a second curve.

**The gap.** The contract had no per-zoom field of view.

**The reach.** Perspective strength at the ends of the zoom. Adding a per-zoom curve later is a new fixture key.

**Verdict.** sound — simplest, and there is a clear place to add a curve if framing feedback asks.

### Ctrl+right-click always means attack-move to the ground

***sound** · confidence **medium** · Camera and controls · from Slice 09*

**The choice.** The player Ctrl+right-clicks on an enemy tank. Today the selection attack-moves to the ground point under the cursor (moves there, engaging whatever it meets); the enemy and any building under the cursor are ignored. The alternative would treat Ctrl+click on an enemy as a plain attack, or on a building as a garrison.

**The gap.** Whether the modifier or the thing clicked wins.

**The reach.** The modifier's meaning is fixed, so it stays predictable.

**Verdict.** sound — a modifier with one meaning is easier to learn.

### Double-click selects every own unit of the type; again, the type's role

***sound** · confidence **medium** · Camera and controls · from 27f presentation leftovers*

**The choice.** Two left clicks on one unit within the right-click gesture's window (`DOUBLE_CLICK_MS` 350 ms, `DOUBLE_CLICK_PX` 6 px) select every own unit of its type. A second double-click on that type, or Ctrl + double-click, widens to every own unit sharing the type's first role, the one its symbol shows. Shift adds to the selection. The widening is remembered only briefly: it lapses when the next double-click starts after the double-click window, and it ends when the selection changes any other way (`useUnitControl` reports each change through `SelectClicks.selectionChanged`). So a click elsewhere, a box selection or a group key starts over. A third quick click begins a new double-click. All own units count, not just those on screen, because the maps are small. The owner is `web/src/battle/input/selectSimilar.ts`. Today each role has one type, so the widening selects the same units. A unit test proves the widening with a synthetic catalog.

**The gap.** The user asked for select-similar. The on-screen scope, the timing and the widening key were not specified.

**The reach.** On large maps "all own units" may need to become "on screen", as in most RTS games.

**Verdict.** sound.

### The camera tour is keyed to fractions of the run, so the short run flies it faster

***sound** · confidence **medium** · Menu and benchmark · from Slice 10*

**The choice.** The benchmark flies the camera through six phases (strategic, pan, zoom, ground, combined, return; 10/20/20/20/20/10 % of the run). Keyframe times are fractions of the run, so the 60 s short run flies the same five-minute tour five times faster. A slow frame never shortens the path, because the camera is sampled by elapsed time. So short and full runs aren't interchangeable; frame-cost rows compare short runs to short runs. The alternative, a separate short tour, would need its own anchors and its own version.

**The gap.** The short run must report every phase, but phases were defined for five minutes.

**The reach.** How frame-cost numbers compare over time.

**Verdict.** sound.

### The benchmark redraws every frame and counts the whole frame callback as CPU time

***sound** · confidence **medium** · Menu and benchmark · from Slice 10; Slice 00*

**The choice.** Normally the viewport draws only when something changes, so an idle measurement would just report the display's refresh rate. The benchmark forces a redraw every frame and takes no camera input, so its numbers are an upper bound on per-frame work. "CPU time" is the viewport's whole frame callback: interpolation, draw encoding, readout placement. The alternative, timing only the draw encode, would hide interpolation and DOM costs.

**The gap.** What "CPU time" measures, and how to measure an on-demand renderer.

**The reach.** Benchmark CPU numbers include presentation work, not just rendering.

**Verdict.** sound.

### Post-processing settings are fixed for the life of the post chain

***sound** · confidence **medium** · Renderer frame · from Slice 13*

**The choice.** `PostSettings {exposure, grade, bloom}` is baked in when the post chain is built: bloom's numbers compile into its shader and the grade is written once. Changing the light means rebuilding the frame, which the lab's `rebuild()` does. The alternative, live-editable uniforms, would allow a tuning slider without a rebuild.

**The gap.** "PostSettings owns exposure, AgX and bloom" left live editing open.

**The reach.** A future in-game light change (a day/night cycle) would need live uniforms.

**Verdict.** sound for a fixed-light battle.

### A depth-only pass runs before any colour, at 4× multisampling

***sound** · confidence **medium** · Renderer frame · from Slice 12*

**The choice.** The fog needs the scene's depth before colour is shaded, to know which eyes matter to each 16-pixel tile. Today a depth prepass writes the frame's 4× multisampled depth first; the colour pass reuses it and shades only the surface the prepass left (compare `greater-equal`, no depth writes). Elsewhere the repo's one compare is `greater` (reverse-Z). The depth texture is sampleable, so the fog's tile cull reads sample 0. The shared vertex stage marks its position `@invariant` so both passes compute bit-identical depth (TypeGPU's types omit the attribute, so it is cast in). Rejected: a separate single-sample depth (disagrees at edges) and last frame's depth (a frame of lag at the fog edge).

**The gap.** Spike 02 needed depth before colour and left the arrangement open.

**The reach.** One extra geometry pass; translucent geometry is not in it.

**Verdict.** sound.

### Overlays draw into their own 4× multisampled target and composite over the finished frame

***sound** · confidence **medium** · Renderer frame · from Slice 12*

**The choice.** Overlay marks (order lines, contacts, the x-ray of hidden units) are drawn into their own multisampled target (about 32 MiB at 1080p), resolved and laid over the post-processed world with premultiplied alpha, so their colours are exactly as authored and not graded or fogged. The alternative, dropping multisampling for overlays, would save memory but jag thin lines.

**The gap.** Spike 01 left dropping it open.

**The reach.** Overlay line edges stay smooth, and overlays can depth-test against the world.

**Verdict.** sound; revisit under a memory budget.

### Resizing rebuilds targets in the background and swaps them whole

***sound** · confidence **medium** · Renderer frame · from Slice 12*

**The choice.** The player resizes the window. The frame builds new size-dependent targets and the post chain asynchronously (its pipelines compile asynchronously), skips drawing meanwhile, then swaps them in and asks for a redraw. The lab's `frame()` waits for a pending rebuild. A resize can show one undrawn frame.

**The gap.** Post's pipelines are async.

**The reach.** Resize is glitch-free apart from that one frame.

**Verdict.** sound.

### An eye that moved keeps showing its old sight map until its turn to rebuild comes

***sound** · confidence **medium** · Renderer frame · from Slice 14*

**The choice.** A tank drives forward. Its fog eye's sight map (which ground it can see, around where it stood) is expensive to rebuild, and only 48 are rebuilt per frame. Today a moved eye keeps its old map, looked up from the position it was built at, until its rebuild turn; brand-new eyes, a new world or new sight-blocker knowledge rebuild at once. The eye's facing, sight shape and range are always the current publication's, since those are applied per pixel. Eyes are keyed `unit:slot`, so an eye that did not move keeps its map across publications. The alternative, rebuilding every moved eye every frame, would spike GPU cost with many units moving.

**The gap.** The spike said rebuilds were "capped per frame" but not what a waiting eye shows.

**The reach.** With 100 units a side, every eye is current within two frames of a publication; the village's handful always is. A far larger army would see fog lag a few frames behind moving units.

**Verdict.** sound — a bounded lag of a frame or two is invisible at battle speed.

### The GPU pose kernel reads a dense clip table: every frame of every clip, missing channels filled with the rest pose

***sound** · confidence **medium** · Renderer frame · from Slice 20*

**The choice.** A soldier plays a walk clip crossfading into a kneel. The GPU kernel needs every joint's transform at any frame. Today each body's clips are stored densely: every frame of every clip, every joint, absent channels filled with the bind pose (about 63 joints × 310 frames × 32 B ≈ 0.6 MB per body; `models/clipTable.ts`). One GPU thread per soldier walks the skeleton in private memory; one crossfade. The source project's snapshot banks and upper-body layer were not ported.

**The gap.** The source's machinery served a crowd runtime we do not have.

**The reach.** Memory grows with clip length × joints per body type. An upper-body layer (aim while walking) would be a second control added later. One thread per soldier with a large private array is not proven at the largest battle sizes.

**Verdict.** sound — simple and predictable; revisit only if a profile says so.

### Grass reaches the frame through the world layers, from the catalog; some fog checks turn it off

***sound** · confidence **medium** · Renderer frame · from Slice 18; Slice 19b*

**The choice.** A new route builds its battle world. `buildWorldLayers` (`battle-renderer/src/worldMesh.ts`) fills `WorldLayers.grass` with the installed catalog's grass appearances, beside the trees' `scenery`, and only when the route passes appearances from the one loader (`villageAppearances()`). So grass and trees follow one path from the catalog to the frame. Every route with appearances draws grass: the battle routes, ballistics, geometry's surface view. Some pass `null`: the foundation patch, the workbench's ground, and the fog lab (`routes/fog.tsx`), because blades in unseen ground stand up over the seen field behind and move the sight-edge line its checks trace by blade heights. The fog-look lab has a grass toggle: its gate frames draw grass, as the village does, and its pixel-identity and rim checks use bare ground, since dense grass flips a stray pixel now and then. The alternative was a second grass-only loader in the battle view.

**The gap.** The spec did not say how grass kinds reach the frame, nor which labs draw grass.

**The reach.** Any new lab that builds its world with appearances gets grass automatically, and pixel-exact checks in it must allow for the stray-pixel tie.

**Verdict.** sound — one path from catalog to frame; opt-outs are for measurement only.

### The effect pass sits inside the lit world, before the fog look

***sound** · confidence **medium** · Renderer frame · from Slice 25*

**The choice.** Effects draw after the world pass resolves and before the fog mask pass (which gives unseen ground its look), single-sampled into the lit HDR image. They test against the world's depth by reading it and fade into what they meet ("soft particles"). So bloom, colour grading and tone mapping treat them like any light. They also lower the fog mask's coverage by their own strength, so a burst over unseen ground is as bright as over seen ground; the feed already decided what may be shown there (own rounds anywhere, enemy rounds only over seen ground). Three shapes share one pipeline: camera-facing streaks (tracers, flash tongues, sparks; never thinner than `min_px`, dimmed instead), glows (flashes), and flipbook sprites (premultiplied, two frames blended).

**The gap.** Where effects sit in the pass graph.

**The reach.** Any future translucent world effect goes here. Smoke over unseen ground skips the fog's hatching.

**Verdict.** sound — effects get the same light response as the world.

### The player's own units hidden behind things are drawn through as an x-ray silhouette

***sound** · confidence **medium** · Renderer frame · from Slice 27b*

**The choice.** A squad walks into a wood, and the canopy hides it. The parts of the observing side's own soldiers and vehicles that the world hides are drawn through it as a flat silhouette: pale blue for own units, and the scheme's selection colour when selected (`presentation.overlay.xray` `own`, `selected_alpha`; `ModelInstance.xray` carries the colour). This covers anything in front: trees, houses, a ridge. Enemies are never x-rayed, because that would say more than the fog does. The depth prepass draws the x-ray into the overlay target with an inverted depth test ("behind") and a small depth margin (`XRAY_DEPTH_BIAS`), so a prone man or a track's lower run doesn't fleck where it dips under the ground. Impostor cards, used for units beyond about 230 m, are not x-rayed. The alternatives were a canopy cutaway, which changes how the forest reads from above and needs per-tree fading, and an outline pass.

**The gap.** The slice named three options and chose none.

**The reach.** Any new "unit behind something" cue should reuse this path. Pale blue could read as a ghost (critique note).

**Verdict.** sound. It is the common RTS convention, and it is the cheapest of the three.

### Ground marks are paint on ground surfaces and grass, from a screen-size paint target, never on bodies

***sound** · confidence **medium** · Renderer frame · from Slice 27e (follow-ups); Slice 27 (muzzle flash) — coverage of the ground-versus-bodies rule*

**The choice.** The user wanted marks "literally on the ground and glowing a bit", under smoke, painting the grass blades. A painted route crosses a crater, a road verge, a slope and a sandbag wall: the paint drapes over the crater, tank tracks and trampled grass, the road edge, the slope and past the map edge, but never onto a body (building, ruin, sandbags, fence, wreck, tree, soldier, hull). Under the shipped scheme the painted marks are the selection's circles and selected soldiers' markers, a blocked route, travel chevrons, supply rings, suppression and impact rings, the objective zone and the map border. Each frame they are drawn from the camera into `targets.paint` (screen size, rgba8, about 8 MiB at 1080p), depth-tested against the ground-only half of the depth prepass (terrain, props, backdrop, trees), and pulled 1 m toward the eye along each vertex's view ray so the pixel is unchanged but the depth clears the ground. The ground layers (terrain, grass, backdrop) read the paint at their own pixel and take it as albedo and emissive (see the paint-is-a-light entry), lit and shadowed as their own surface, taking `paint.fog_keep` 0.35 of the fog. A grass blade takes whichever covers more, the paint under it or the paint at its pixel, so strokes are never speckled by blades. One place decides who reads paint: fog's `FogLayer.painted`. The `paintedGround` layer reads it, and so does `paintedFaces`, which holds the water and the props that block no mover (the bridge deck and rubble) and paints only their upward faces (see the deck-and-water entry). Buildings, trees, wrecks, the fallen and units never read it, so smoke and effects draw over a stroke and it can never land on a hull. Chosen over a world-space texture (too coarse at a low camera, or tens of MB) and over per-fragment stroke lists (too costly).

**The gap.** The user gave the look; the technique and what counts as ground versus body were delegated.

**The reach.** Any new ground mark goes in a `painted` mesh; any new ground surface (rocks, clutter, a road decal) must bind `paintedGround` or marks vanish on it, and a new body is never painted. Painted marks are fogged and shadowed, so they read differently from overlay marks.

**Verdict.** sound — first-principles "ground versus body" with no per-kind exceptions, exact at every camera for one texture; medium on the tuning numbers.

### The farthest tree tier is a small mesh crown, not an impostor card

***sound** · confidence **medium** · Renderer frame · from Slice 19; Slice 23*

**The choice.** A tree 1 km away is a few pixels. Its tier 3 is an 80-triangle crown (20 for a hedge shrub), drawn from a static buffer in `frame/sceneryLayer.ts`. The card path exists (`impostorCards.ts`, used for far soldiers) and the workbench bakes tree impostor atlases, but trees do not draw them; switching is a scenery-layer change with its own visual gate. Leaf cards with an alpha atlas were also rejected: alpha-tested cards need a discarding depth prepass and shadow caster, and shimmer under 4× MSAA; foliage detail is per-pixel leaf clumps in the tree's own space, fading with pixel size.

**The gap.** The plan named impostors without saying when trees get them; the card path landed scoped to infantry.

**The reach.** Far forests cost triangles, not texture lookups. If trees ever dominate frame cost, tier 3 should switch to the one impostor system soldiers use.

**Verdict.** sound — a deferral with a clear next step.

### A battle installs every appearance it could draw: its units, its map's props, and every prop type not marked `map_only`

***sound** · confidence **medium** · Renderer frame · from Slice 24; Slice 36*

**The choice.** The village loads, and later a scenario event drops a sandbag line, a tank becomes a wreck, and a house becomes a ruin. The models layer installs every unit kind's appearance; the appearance each map prop takes; and every appearance of every prop type whose catalog appearance is not `map_only` (only the building and the bridge deck are), since a battle can leave or place any of those anywhere: wrecks, ruins, rubble, crates, sandbags (`PropAppearances.drawnFor` in `packages/battle-renderer/src/models/propAppearance.ts`). Forest trees are left to the scenery layer and grass to the grass pass. In the village that is about 132 MiB of buffers, mostly the houses' LOD0; the extra non-map bundles are small (largest about 24k triangles). The alternatives were installing only the map's props (a dropped crate had nothing to draw it) or the whole catalog.

**The gap.** Which appearances to load was not specified.

**The reach.** Memory scales with the map's building kinds plus a small fixed cost; no prop can appear invisible. A smaller vertex format or shared indices would cut house cost.

**Verdict.** sound.

### A prop is drawn by the appearance its catalog type declares, fitted to its box; the renderer knows no prop names

***sound** · confidence **medium** · Renderer frame · from Slice 24; Sim lane (review fixes, props catalog)*

**The choice.** A wreck, a 20 m field wall and a shelled house are placed. Each prop type's catalog `appearance` says how it is drawn: `drawn_by` (a scenery kind, `building`, or `forest` for trunks the forest's trees draw), `modular` (drawn as repeating segments), `map_only` (only ever authored on the map) and `remains_state` (remains draw as the thing they came from, in that state). The world layout carries this as `propAppearance`, and `PropAppearances` (`battle-renderer/src/models/propAppearance.ts`) reads it. Among the appearances of that kind, the one whose `footprint_half_m` is nearest the box (least summed |log scale|) draws it, scaled per axis. A modular prop (wall, fence, sandbag line) repeats its module along the box's long side, each stretched to fill, instead of smearing one module over 20 m. A ruin (`remains_state: "ruin"`) draws its building's own appearance in its ruin state, on the building's plan at its authored height; otherwise the generic ruin is fitted. `structureModels(props, known, …)` drops every map prop a known prop replaces and adds every known prop in one list, so a swap is atomic and an unseen collapse stays unseen. `drawn_by: "building"` pairs with the asset catalog's building appearances. The alternative was hard-coded tables and name checks in the renderer (a kind-to-model table, `"trunk"`/`"building"` string tests, a ruin special case).

**The gap.** The spec didn't say how placed boxes pick and fit art; props-as-data implied the renderer should stop knowing names, and the binding shape was open.

**The reach.** A new prop type draws without renderer code when an existing scenery kind fits; art must declare its footprint, and fitting stretches art that doesn't match a placement's proportions.

**Verdict.** sound — medium only because the data-binding change reached into renderer internals outside the sim lane that made it.

### Each soldier is posed from his own feed data, with fixed priorities for firing, posture and facing

***sound** · confidence **medium** · Renderer frame · from Slice 20; Slice 23; Post-close review (presentation)*

**The choice.** A rifle squad of eight advances and two men fire. The pose driver (`battle-renderer/src/models/poseDriver.ts`) reads its own small `FeedFrame`: per unit its id, kind, side, position, yaw, soldiers by id and position (with optional posture, shot count and lean), mounts, deployment and suppression, plus the fallen. The battle fills it from the observation (`apps/battle-lab/src/poseFeed.ts`), the workbench from synthetic frames, and the renderer package never imports the web observation. Each soldier is posed from his own data, blended only with himself by member id. **Gait:** from his own velocity: walk from `presentation.pose.gait.walk_mps` (0.25), run from `run_mps` (2.2), 0.25 s crossfades. **Firing:** he fired when his `FeedSoldier.shots` rises: the feed's count of his launches, so a shot is timed once, not on every frame that shows the same publication. A feed that leaves `shots` out names no one, and then a rise of the squad's hand-weapon shot counter counts as the whole squad firing. He holds a firing pose for the rules' `cover.lean_hold_s`. **Posture:** the simulation's own posture for him wins when published; else out on a lean point he kneels; else he is prone at or above the rules' `suppression.collapse_level`; else kneeling while firing; else standing. No hysteresis: the simulation's recovery delay keeps it from flickering. An identified enemy never goes prone from suppression, because his side can't know it (the feed sends 0). **Facing:** his own velocity above `gait.facing_mps`; else the hand weapon's bearing once it has fired, else the unit's heading, with a per-man gaze stray once the squad has settled; turning at `turn_rad_s` 6. Nothing reads a formation slot. The alternative was one pose for the whole squad from the squad's heading.

**The gap.** Pose thresholds are presentation and no slice owned them; weapon shots are counted per mount, not per soldier, and the spec named no priorities.

**The reach.** This is the firewall for future per-soldier (Company of Heroes-style) movement: soldiers already animate from their own motion. Poses that mean a game state (prone, firing) are tied to rule values, so they stay in step with the simulation. The unit's heading remains only as the idle facing of a soldier who is still and has not fired.

**Verdict.** sound — per soldier wherever the feed allows.

### Paint lands on a bridge deck and on water, on faces that look up

***sound** · confidence **medium** · Renderer frame · from Post-close review (presentation)*

**The choice.** A tank parks on the bridge and the player selects it; another unit's blocked route crosses the river. Order and ground marks are paint: drawn into the lit world on surfaces the fog layer `FogLayer.painted` covers (see the painted-marks entries). Before, only the terrain was painted, so the tank's ring vanished on the deck and the dashes faded mid-river. Now a second fog layer, `paintedFaces`, is painted and fogged face by face. The water draws through it and takes the paint as its surface. A static prop joins it when its body stops no mover class; the pose marks it `ground: true`, taken from the world layout's `blockingPropKinds`, so the catalog needed no new field. Today that is the bridge deck and rubble. The model shader reads the paint only on faces that look up (a smoothstep of the normal's z from 0.5 to 0.8), so a deck's sides stay clean, and no other layer reads it. Impostor cards (the flat pictures that stand in for far models) don't read the paint, so a deck far enough away to be a card shows no marks. The movement scene checks both surfaces: on the deck, the share of the parked tank's ring shown went from 0 to 96%; mid-river, a blocked route's dashes went from 38% to 89% as bright as on the bank. The alternative was a catalog flag for "paintable", a second field that says what the body's blocking already says.

**The gap.** The review found marks missing on the deck and the water. Which surfaces take paint, and how a prop qualifies, were open.

**The reach.** Any future walkable prop that blocks nobody (a ramp, a pontoon) takes paint with no code. A far deck shows no marks. An unprimed critique of the close-ups found no mark drawn over a body. It also raised points outside the fix, all left for a later look pass: the water reads as asphalt-like with no bridge shadow, hard-edged shade wedges, faint lines that are likely other units' routes under Space, yellow-on-grass contrast, and, at low confidence, dust near the deck ring that may not veil it.

**Verdict.** sound. Medium, because deriving "paintable" from "blocks nobody" is an inference the user didn't state.

### Shadow softness is a width in metres, and the bias grows with it

***sound** · confidence **medium** · Light, terrain, grass and trees · from Spike 01; Slice 13*

**The choice.** A soldier's shadow should be equally soft whether the near or a far cascade draws it. Today softness is a world width, `cascades.softness_m` (0.2 m). Each cascade turns that into a filter radius in texels (its own texel size), clamped to 1–4 texels. The normal bias (how far a surface is pushed off itself before testing the shadow map, to stop surfaces shadowing themselves) is at least `normal_bias_min_m` (0.03 m), or `normal_bias_texels` (1.5) texels, and grows with the filter radius. The ported code used a fixed 0.6 m bias, which would lift soldiers' shadows off their feet at the default camera, and a softness in texels, which blurred far cascades more metres than near ones.

**The gap.** The source was tuned for one camera reach.

**The reach.** Contact shadows under small bodies.

**Verdict.** sound.

### The sky's brightness is one knob that scales both the view and the lighting

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 13*

**The choice.** `sky.radiance` scales the sky both as seen and as it lights the world. The rejected alternative, a brightness for the visible sky only, would make the haze at the horizon brighter than the sky beside it.

**The gap.** Delegated tuning method.

**The reach.** Sky tuning is one number.

**Verdict.** sound.

### The ground mesh's vertex colour is a tint over the biome material, its alpha the tint's weight

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 16; Slice 20*

**The choice.** The battle draws the ground with the biome's field patchwork, but the geometry lab's traversal view wants red/grey passability colours and the workbench wants a plain scale grid. Today each ground vertex's colour is a tint and its alpha says how much: 0 draws the biome (the battle and every lab), 1 draws the tint alone (the traversal view, render-only fixtures, the workbench's measured ground). The alternative, a separate material per view, would duplicate the ground pipeline.

**The gap.** The traversal overlay and the render-only patch predated the biome.

**The reach.** Debug views stay on the production ground pipeline; a future view can blend a tint partially.

**Verdict.** sound — one ground path serves every view.

### Field plots are a seeded binary split of the land walked per pixel in the shader, with no texture

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 16*

**The choice.** The ground looks like a WARNO-style patchwork of fields. Today `generatePlots(site, biome)` cuts the map (plus `field_rules.extent_m` around it) into convex plots: first along roads (only where the road runs along most of the cut, so a road line never extends across open country), then across by size or into strips (`strip_chance`), large tracts turning their heading. The shader walks the same split tree per pixel (about 4,000 plots on the village, ~13 cuts deep) and uses the nearest cut as the distance to the plot edge, where a verge grows; edges wander by a small warp. Plots around buildings are the settlement's meadow (`settlement_m`, `settlement_kind`). The rejected alternative was a baked plot-id or distance texture, which blurs or stair-steps with zoom and costs memory.

**The gap.** Plot generation was delegated.

**The reach.** The per-pixel tree walk is a GPU cost on every ground pixel; plots are purely visual and never affect the simulation.

**Verdict.** sound — resolution-independent edges at a measured cost.

### Ground detail finer than a pixel fades to its average colour

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 16*

**The choice.** Zoomed out, a field's furrows and mottle are smaller than a pixel and would shimmer. Today the fine mottle and drill rows fade by the pixel's footprint (`fwidth` of the world position), and verge, road and rect edges widen their blend to a pixel, so the patchwork neither shimmers nor shifts brightness with zoom. Noise hashes integer lattice corners rather than a `sin` hash, which loses precision kilometres out.

**The gap.** Anti-aliasing of procedural ground was left to the implementer.

**The reach.** Any new procedural ground detail should follow the same footprint fade.

**Verdict.** sound — standard analytic anti-aliasing.

### A grass clump's colours are stored relative to its own mean; the ground supplies the mean

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 18*

**The choice.** A wheat clump grows on a golden field next to a green meadow. When the field places the clump, it replaces the clump's average colour with the ground's colour (albedo) at that spot and keeps only the clump's variation around its average. So grass and the painted ground beyond it always agree, and a clump averages to exactly its ground. The workbench still shows the authored colours. The rejected alternative was storing a nominal "ground colour" in each grass bundle, which would be a second owner of the biome's palette.

**The gap.** The spec did not say who owns grass colour: the grass art or the ground.

**The reach.** Grass colour is tuned through the biome's ground palette, not the grass art. A grass kind cannot be deliberately off-colour from its ground (for example, flowers of a different hue on average).

**Verdict.** sound — one owner for field colour; the cost is less freedom per kind.

### The biome owns grass growth and tuning: `summer.json.grass`

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 18*

**The choice.** A designer wants shorter grass on the verges. The biome file (`fixtures/biomes/summer.json`, key `grass`) holds it:
- `growth`: per plot kind (or `verge`), which grass appearance grows and its density and height. Meadow grows meadow, pasture grows pasture, young crop grows crop, wheat grows wheat, hay grows stubble, ploughed land grows sparse meadow weeds (density 0.04, height 0.6), and the verge grows meadow at height 1.1.
- tuning: `pixels_per_clump` 28, `max_clumps_m2` 40, `fade_m_per_px` [0.08, 0.14], `near_tier_px` 40, `min_blade_px` 0.8, `clear_m` {road 0.2, prop 0.3, area 0.5}.
- `wind`: heading 35°, lean 0.08, gust 0.18 over 45 m at 5 m/s, flutter 0.04 at 1.3 Hz.
- `capacity`: [20000, 120000] clumps (near and far tier buffers).

`validateBiome` checks every field, and refuses a growth key that names no plot kind.

**The gap.** Delegated: density, sway and detail distances were to live in the biome, with no schema.

**The reach.** A second biome (winter, desert) is a new file with its own grass block. The capacities are fixed buffer sizes; a denser map could hit them.

**Verdict.** sound — look numbers sit where the look is tuned.

### Grass technique constants stay in code, not the biome

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 18*

**The choice.** Some grass numbers are how the technique works, not how the field looks, and they stay as constants: the 4 m tile, the 0.4 m jitter, 7 m patches of taller and lower grass, grass thinning and lowering over 1.2 m past a road's or a wood's margin, the ground-normal weight 0.55, blade roughness 0.9, the tier meshes [LOD0, LOD2], and the far "plain" colour blend (`grassPass.ts`, `grassField.ts`). The alternative was putting every number in the biome file.

**The gap.** The spec asked for "every provisional number in the fixture", which read literally would include technique constants.

**The reach.** Tuning any of these needs a code change. If the look ever needs one (say, patch size), it should move to the biome.

**Verdict.** sound — move one to the biome when the look needs it.

### Grass takes fog per fragment and receives sun shadow but casts none

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 18*

**The choice.** A clump stands right on the line between seen and unseen ground. Each blade fragment asks fog about the ground under it at the root's height, and writes into the fog mask (the per-pixel record the fog pass reads; see the fog-mask entry) as ground. So the sight edge and its rim cut through a clump as sharply as through bare ground. Asking once per clump would have cut the edge clump by clump. Grass receives the sun's shadow cascades but casts no shadow of its own (cost; the source game's cast none either).

**The gap.** The spec did not say how grass meets fog or shadow. Fog's tile cull lifts pixels by the tallest canopy; a map with no forest gets no lift for grass (under a metre).

**The reach.** Unit shadows lie flat over grass. On a forestless map, fog edges on tall grass could flicker; revisit if seen.

**Verdict.** sound — fog is a ground feature and grass is treated as ground.

### Each grass blade gets a tiny depth nudge so crossing blades don't flicker

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 18*

**The choice.** Two blades cross at almost the same depth. On Apple's GPU, which one wins changed from frame to frame even with identical input, so a still frame shimmered and the overlay-isolation check (it compares two captures pixel by pixel) failed. Each blade now moves its depth by up to 1/2000 of the depth, chosen by a hash of the clump and blade (at most 5 cm at 100 m). A still frame is then almost bit-stable: about one pixel flips in some runs. The overlay-isolation check allows 16 such stray pixels (`web/scenes/_overlays.mjs`, `STRAY_MAX`). The alternative was accepting the flicker, or rendering grass without depth ties at all.

**The gap.** Found by experiment; nothing in the spec covered GPU depth ties.

**The reach.** Pixel-exact checks over grass must tolerate a few stray pixels, or turn grass off (which the fog lab and the fog-look pixel checks do).

**Verdict.** sound — a deterministic tie-break is the general fix for an order-dependent depth tie, not a point fix.

### Far grass clumps blend into the ground's colour and lighting

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 18*

**The choice.** From the default camera, grass 150 m away appeared as dark speckle. Between 0.6× and 1.1× the fade's first footprint, a clump's tint and shading normal now blend to the ground's, so near grass keeps blade texture and past about 100 m grass reads as the painted ground. The alternative was keeping full blade shading to the fade distance.

**The gap.** The spec did not say how far grass hands off to the ground.

**The reach.** There is a deliberate detail drop between about 65 and 110 m. Far field texture is the terrain material's job.

**Verdict.** sound — matches the source game's "far grass matches the ground".

### The shadow floor lives on the light and applies to every world material

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 19b*

**The choice.** A sun shadow must never be darker than fog, or it reads as fog. `presentation.light.shadow_floor` (0.4) is the share of sunlight a shadowed surface keeps (`sun = mix(floor, 1, shadow)`), carried to the GPU in the environment uniform. Every world material shades through the environment frame's one `shade` function, so terrain, grass, trees, props and units all get it. It keeps shade in the sun's warm hue with the surface's relief. The alternative was raising the sky fill light, which would have lifted lit ground as much as shade and tinted it blue.

**The gap.** The spec said "raise the shadow floor" without saying where.

**The reach.** Shadows are softer everywhere. Any new material must shade through `shade` to agree.

**Verdict.** sound.

### No ambient occlusion is added to the world

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 19b*

**The choice.** Every material passes AO (ambient occlusion, darkening where light is blocked by nearby geometry) as 1. The only occlusion a forest floor would get is its canopy's, which darkens it, the opposite of what was needed. The alternative was screen-space or baked AO.

**The gap.** The spec named AO as a tool.

**The reach.** Nothing darkens where trunks meet the floor; that contact shading stays open.

**Verdict.** sound.

### The forest floor's drawn edge is a ragged verge lying mostly outside the simulation's forest rectangle

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 19b*

**The choice.** A forest in the simulation is a rectangle, which drew as a ruled slab. The drawn floor now ends in a verge (`forestVergeInside`) half its width outside the rectangle, plus wander and patches, running from about 0.5 m inside to 4.5 m outside. The rectangle stays the rule for sight, cover and movement; the verge is only paint. Grass reads the same function and stops at whichever edge is farther out, so it never grows inside a forest and its line is as ragged as the floor's. The biome's `forest_floor` block holds the numbers. The alternative was feathering inside the rectangle.

**The gap.** "A feathered verge, while the rect stays authoritative".

**The reach.** The drawn wood looks up to 4.5 m bigger than the rule wood. A unit standing on the verge is in the open.

**Verdict.** sound — paint may extend, rules don't move.

### Sun flecks lift the canopy shadow on the forest floor only

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 19b*

**The choice.** The shadow map treats a tree crown as solid, so the forest floor was one flat dark slab. The terrain fragment adds sun flecks (`groundDapple`): `sun = max(shadow, flecks)`, on the forest floor only. The rejected alternative was holes in the tree shadow casters, which would change every cascade's cost and the trees' own shading.

**The gap.** Not specified.

**The reach.** Things standing on the forest floor (soldiers, props) don't get flecks.

**Verdict.** sound.

### Scars are reconstructed with a cubic spline and cut with hard, one-pixel edges

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 17b; Slice 17*

**The choice.** The texture stays one byte-per-channel texel per learned 1 m cell; only its reading is smarter. `scarCubic` rebuilds a smooth field with a cubic B-spline (four bilinear taps), so a thresholded crater edge is a circle, not a diamond. Features are drawn as hard iso-lines anti-aliased over exactly one pixel at any zoom (`crossing`): the bowl's lip (0.3 of a full crater's depth, wandering by noise over 0.45 m), the outer edge of the thrown-soil ring (about a metre round every bowl), and each rut's edge. A hard crater lip was chosen because a soft dark disc read as a tree's shadow, and no sun shadow here has a hard ragged edge. Draw order: trampled, tracks, scorch, fresh soil thrown over the scorch, then the bowl darkened by depth (soil under scorch read as a stain; a bowl lighter than its scorch read as a mound). Rejected: a 2× supersampled texture (twice the uploads, a second grid to keep in step).

**The gap.** Reconstruction and edge treatment were delegated.

**The reach.** Overlapping craters sum into one depth field, so a dense barrage reads as merged pits, not separate bowls.

**Verdict.** sound — crisp at any zoom without more data.

### Scorch is flecked ash at the burst's heart, never a dark wash

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 17b; Slice 17*

**The choice.** Scorch draws only where its weight passes 0.45 (the burst's heart), as flecks: noise at 4.5 per metre crossing an edge that tightens toward the centre, never on the bowl or its soil ring, ash grey to char black. Tried and dropped on the frames: a noisy wash (read as smoke and cloud shadow), radial spokes (swirled or read as stripes), a dark collar (read as a drop shadow).

**The gap.** The spec asked for "tighter, sooty-radial scorch"; the radial part did not ship.

**The reach.** A future burst-centre publication could bring back radial scorch.

**Verdict.** sound — readability over the asked-for radial pattern, which failed on frames.

### Two scar weaknesses are deferred: dense crater fields and grazing views

***sound** · confidence **medium** · Light, terrain, grass and trees · from Orchestrator, after slice 17b; Slice 17b*

**The choice.** A dense barrage's overlapping craters merge into one depth field whose low points read as cast shadows; separating them needs the ground layer to publish burst centres (a simulation contract change). At very low camera angles a bowl is a few pixels tall and parallax can't open it. Both are deferred: real play rarely shows a hand-placed crater lattice, and the whole-frame critique in real battles would reopen them if it flagged craters.

**The gap.** The gate ("could any dark region be mistaken for shadow or fog?") was met for single craters, soot and trampling, not these two cases.

**The reach.** Publishing burst centres would unlock crisp overlapping craters and radial scorch.

**Verdict.** sound to defer — reopen only if a battle critique flags craters.

### Shade is filled by a warm tint on the sky light, and stays warm so fog and shadow differ in hue

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 13; Slice 19b*

**The choice.** Under a blue sky with no bounced light, shade on green grass turned teal. The sky light is multiplied by `sky.fill`, an RGB tint (`[1.0, 0.75, 0.47]`), standing in for light bounced off sunlit ground, which the environment map lacks. A cooler fill would make shade look more natural, but it pulled the forest floor's hue toward the cool fog and failed the check that shadow and fog stay apart in hue, so the fill stays warm and shaded dirt stays slightly reddish: telling fog from shadow wins. The rejected alternatives were a warm shadow tint in the colour grade (which would also recolour every dark surface and the fog) and a cooler, more realistic fill.

**The gap.** The slice owned the teal-shadow finding, not the method, and nothing said which wins when natural shade and fog legibility conflict.

**The reach.** Every shadow's colour; it can look reddish on tan dirt. Any future fill or grading change must re-run the fog-look hue check.

**Verdict.** sound.

### The land past the map edge is the same ground material, lit, hazed and fogged like the map, fading to a distant colour

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 13; Slice 16*

**The choice.** At the strategic zoom the camera sees past the playable map. Beyond it lies a flat band of ground out to `presentation.light.backdrop.reach_m` (40 km) (`frame/backdrop.ts`), drawn with the same biome ground material: the field patchwork runs on past the edge for `field_rules.extent_m`, then fades over 600 m to the biome's `distant` colour. It is lit by the same sun and sky and hazed by the same aerial term as the map, and it takes the fog like the map's ground (drawn sight runs on past the edge; see the fog-past-the-edge entry). It is never shadowed, never casts and is never pickable. It sits at the map's lowest ground, so there is no step at the edge, and below the horizon the sky reads the colour just above the horizon, removing a khaki band and a hard edge at eye level. The alternatives, a skybox, a void or a flat-coloured plate, make the map read as a tabletop.

**The gap.** Spike 01 left "a backdrop or horizon treatment" open, and the plan did not say what lies past the map.

**The reach.** Every low camera frame. Hedgerows and copses stand on this backdrop; the map edge is visible only by the border mark, not by a change of ground.

**Verdict.** sound — one material, no second look to keep in sync.

### Grass thins, takes scar colour and lies flat on scars, in its build pass

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 17*

**The choice.** A tank drives across a meadow and a shell lands beside its track. The grass build pass reads the same scar sample as the ground. A clump is left out with weight max(crater bowl, scorch, tracks × `tracks_thin`) × `thin`, takes the scarred ground's colour, and stores how far tracks or trampling lay it over (× `flatten`) in its colour's alpha byte; the vertex stage leans it along its own yaw and sinks it, with wind scaled down. The clump record stays 32 bytes. The field regrows after any scar upload; a paused battle with unchanged ground never regrows. The alternative was a separate trample buffer or pass.

**The gap.** The grass plan left "a trample input to the same build pass" open.

**The reach.** Tank tracks and squad paths show in grass without a new buffer.

**Verdict.** sound.

### Terrain mottle is dry strips along plot rows, hue only, never darker

***sound** · confidence **medium** · Light, terrain, grass and trees · from Slice 16b*

**The choice.** A player looks across farm fields in sun. `mottle(xy, footprint, across, plot)` returns `(value, dry)`: `value` is only fine 4 m brightness noise; `dry` is broad variation as ochre strips, stretched 16× along the plot's rows (40 m long, 2.5 m wide, `mottle_scale_m` [4, 40]), drawn afresh per plot so they end at its edge. `groundTint` shifts colour toward ochre at unchanged luminance. It's one-sided: nothing gets greener, bluer or darker (a two-sided shift read as a cool, dark patch — a shadow). A strip's edge is measured in metres (noise over its slope), so it's firm, not a soft fringe. Verge, road and forest floor read only the brightness term; grass takes the strips through `groundColour`. The alternative, broad soft dark blotches, kept being read by critiques as cloud shadow.

**The gap.** The mottle's shape was delegated.

**The reach.** Under full grass cover the strip edge blurs (each clump takes its root colour); softening grass colour is grass's domain.

**Verdict.** sound — no broad darker patch can pass for a shadow.

### Unseen ground is told apart from sun shadow by hue, ruled lines and a rim, not by darkness

***sound** · confidence **medium** · How fog looks · from Slice 15*

**The choice.** A tree's sun shadow and a sight shadow both darken a field. Today unseen is cooled toward a night blue (sun shadows are warm, because the sky's fill light is warm) and ruled with fine lines no shadow has. The lines brighten by a share of the fogged colour plus an absolute `floor`, so they stay equally visible inside a sun shadow: a shadow inside fog reads as the same fog, darker, not a second fog layer. The boundary gets a rim on its seen side. The rejected option was dim-and-cool alone, which the spike's critique read as shadow.

**The gap.** The plan asked that fog never be mistaken for shadow without saying which cue does it.

**The reach.** Any lighting change that makes shadows cool (a blue night sky fill) would weaken the hue cue; the lines and rim remain.

**Verdict.** sound — three independent cues, none of them brightness.

### A tree is seen or unseen whole, judged at its crown's heart with no facing test

***sound** · confidence **medium** · How fog looks · from Slice 19*

**The choice.** A wood sits at the edge of blue's sight. Probing each leaf's own face split crowns into a seen half and an unseen half, painting vertical stripes across the wood. Today every fragment of a tree probes fog at its crown's centre with no facing test and no roof rule; the simulation's rule for sight into foliage (fading with foliage crossed) decides it, and an unseen tree takes the fog style whole. Trees past the map are fogged the same way, so fog runs on past the playable area. Trees are opaque and in the depth prepass.

**The gap.** Trees are porous volumes, not faces; the fog rule for them was open.

**The reach.** Fog edges through a wood follow tree outlines, not a smooth line.

**Verdict.** sound — matches how the simulation treats foliage.

### The fog look is one screen-space pass fed by a fog mask every world material writes; its edge and rim are drawn only across the ground

***sound** · confidence **medium** · How fog looks · from Slice 14; Slice 15; Slice 15b*

**The choice.** A wall's sight shadow falls across a field. Every world fragment writes two outputs (`WORLD_OUT = {color, fog}`, `frame/targets.ts`): its lit colour into the 4× MSAA HDR target, and `fogCoverage(seen, alpha)` = (unseen, seen, ground, alpha) into an MSAA mask. Units, the sky, and anything that is never fogged write zero. Then `frame/fogMaskPass.ts` runs: distance rows → distance columns (a bounded distance transform finding, for each ground pixel, how far it is to the nearest seen and unseen ground) → compose (in HDR, before post-processing, fading the unseen side in over the style's `edge_softness` pixels) → rim (a line `rim.width_px` wide on the seen side, in display colour after post and before overlays, so it is exactly the style's colour and never blooms). `FogVisibility` says where is seen; the mask pass says how unseen looks, and no material has its own fog-look branch. The edge and rim are drawn only where seen ground meets unseen ground. A face (wall, roof, tree) is seen or unseen whole and keeps a hard edge, and a surface in front of another (a seen roof against unseen ground) makes no edge; drawing a rim wherever seen met unseen would ring every roof, every trunk against a seen field and every gap in a wood in white. The alternative was each material applying the unseen look itself with a per-pixel binary edge, which cannot know its neighbours, so it can neither soften nor rim, and a sight shadow read as a second cast shadow.

**The gap.** The contract asked for an `edge_softness` key and a clearer "seen/unseen boundary", which a per-material fog term cannot deliver, and named the pass but not how materials feed it.

**The reach.** One more full-screen pass every frame (two distance passes, a composite, a rim), about +63 MiB of 1080p targets. Every new world material must write the fog mask output correctly (ground, face, or never-fogged) or it gets no fog. A sight shadow reads as a ground feature; walls never get a rim, even at their own seen/unseen split.

**Verdict.** sound — the rim is the cue a sun shadow's edge never has, and the pass is the single owner of how unseen looks on screen.

### Drawn fog follows forest ground a side has seen cleared, by tanks or by fire

***sound** · confidence **medium** · How fog looks · from Slice 34b; Orchestrator, after slice 34b; Slice 34c*

**The choice.** A tank carves a lane through a wood, or a barrage fells a patch of trees, and blue sees it happen. The GPU fog marches a foliage grid (canopy and optical depth per 8 m cell, depth packed in 0.005 steps, full block at most 1.275). That grid is the load-time grid minus the trees standing on ground the side has seen cleared (`WorldGeometry::export_foliage_cleared`, via `WorldView.foliage_cleared`): a cell is open wherever its centre is cleared, which matches the simulation's `foliage_at` exactly (a test pins that). `GroundView.clearedCount` keys it; the session re-exports when the count changes, and the fog layer re-uploads only the foliage. So a lane or clearing opens in the drawn fog when the side learns it, agreeing with the simulation's fog field. The alternative, the grid from load time only, drew foliage over a clearing the side can see through.

**The gap.** The seam didn't say whether drawn sight follows destruction.

**The reach.** Drawn fog and simulated sight agree after forest destruction. Every newly cleared cell rebuilds all eye maps, so a long carve in view rebuilds often (a known cost).

**Verdict.** sound — drawn fog agrees with simulation sight.

### A pixel counts as seen or unseen only when wholly so

***sound** · confidence **medium** · How fog looks · from Slice 15b*

**The choice.** An unseen wall stands on seen ground. A pixel is unseen when more than half its samples are unseen ground and none is seen, and seen likewise; a pixel with both gets neither look. World positions are also interpolated at the centroid (`WORLD_VARYING`), kept though it does not remove the fringe alone. The alternative, a "more than half the samples" rule, drew dashes of rim along every such wall's silhouette, because fog calls some ground samples just under the wall unseen.

**The gap.** Not specified; found in frames.

**The reach.** Mixed pixels at silhouettes get neither look, a thin unstyled seam.

**Verdict.** sound.

### The soldier is the Quaternius UBC body with clothing cut as shells from it

***sound** · confidence **medium** · Models and the asset pipeline · from Spike 03*

**The choice.** Every infantry model starts from the Quaternius UBC `SuperHero_Male` body and its 65-joint rig. Uniform, boots, gloves, plate carrier and helmet are shells cut from that body, smoothed toward round tubes and pushed out a little, so they bend with the body using its own skin weights (`packages/scene-assets/blender/infantry_kit.py`). Hard kit is placed on the shell and bound 100 % to one bone. The alternative, separately modelled and weighted clothing, costs far more modelling.

**The gap.** The slice named the rig but not the body or how kit is modelled.

**The reach.** Every infantry kind shares one skeleton and inherits its weights; a different body means a new rig.

**Verdict.** sound.

### One model file holds all four detail levels, told apart by name

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 11*

**The choice.** A model is one GLB. Its mesh objects carry a `_LOD0`…`_LOD3` suffix (finest first) for the four detail tiers; an unsuffixed mesh appears in every tier. The alternative, four files per model (what `~/dev/game` did), multiplies sources and hashes.

**The gap.** "4 tiers finest first" didn't say files or names.

**The reach.** Every exported model; one source hash per appearance.

**Verdict.** sound.

### Moving parts are the model's empties, with required names

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 11*

**The choice.** An articulated part (turret, gun, wheel, deploy leg) is a Blender empty; mesh objects fold into their nearest empty ancestor. Names are checked exactly: `turret` → `gun` → `muzzle`, `hmg` → `hmg_gun` → `hmg_muzzle`, `wheel_*`, each `deploy_leg_<id>` with `_jack` then `_pad`, and the mast chain `deploy_mast` → `_2` → `_3` → `_head`. The alternative, articulating mesh objects directly, mixes geometry and pivots.

**The gap.** What makes a node articulated.

**The reach.** Every vehicle model's hierarchy.

**Verdict.** sound.

### Infantry are measured in a standing-aim pose, with `eye` and `muzzle` sockets

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 11*

**The choice.** A soldier model's height, eye and muzzle are measured in one reference pose, the skeleton's `aim_reference` (a required `stand_aim` clip), and compared with the physics numbers (`infantry_eye_m`, `infantry_muzzle_m`). The sockets are empties named exactly `eye` and `muzzle` under a joint.

**The gap.** Spike 03 asked for a standing-aim pose without naming it.

**The reach.** Every infantry model.

**Verdict.** sound.

### Soldiers use two clip sets on one rig: rifle and launcher

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 21*

**The choice.** A rifleman and an AT soldier both walk, but their hands hold different weapons. The weapon is fixed rigidly to the right hand in every clip, so a weapon held differently needs its own clips. Two skeleton entries share one rig: `quaternius-ubc-rifle` (rifle and recon DMR) and `quaternius-ubc-launcher` (the AT soldier, with a shoulder carry). Both have the same 53 joints and the same clip names. A squad's soldiers must all use one skeleton (`AppearanceCatalog` refuses otherwise), so a squad shares one clip set. The alternative was one clip set with weapon-specific hand adjustments at runtime.

**The gap.** The spec said both "recon and AT variants need their own holds" and "clips shared once per skeleton".

**The reach.** A new weapon grip means a new clip set (about 258 KiB). Pose code picks clips by name, so it does not care which set.

**Verdict.** sound — simple, and the cost is small.

### One skinned mesh per soldier appearance, with four authored tiers

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 21*

**The choice.** A soldier's helmet, pouches and rifle are all separate parts in the art. At export, rigid kit is skinned 100% to its bone and joined with the body, so each soldier appearance is one skinned mesh. It has four tiers `soldier_LOD0..3` (about 27k / 6.9k / 2.35k / 680 triangles), made by the ported LOD technique (a budget per mesh island, with small islands dropped at LOD2–3). The alternative was separate rigid meshes attached to bones at runtime.

**The gap.** Delegated: the polygon count per tier.

**The reach.** One draw per soldier per tier, one pose palette. Swapping kit means re-exporting the appearance, not changing it at runtime.

**Verdict.** sound — the simplest shape for the pose kernel.

### The soldier fit checks allow a lower eye and a higher launcher muzzle

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 21*

**The choice.** The validator checks that a soldier's eye and muzzle sit where the simulation puts them. The standing-aim pose leans into the weapon, so the drawn eye is at about 1.50 m on a 1.70 m soldier. The eye tolerance is `eye_m` 0.12 (catalog `tolerances`), and the AT soldier alone gets `muzzle_m` 0.2, since the launcher's bore on the shoulder sits at about 1.56 m against the simulation's 1.4. The alternative was changing the simulation's eye and muzzle heights, or the art.

**The gap.** The fit authority is one flat eye and muzzle height per soldier.

**The reach.** Validation only; nothing in play changes.

**Verdict.** sound — the art is right and the rule is a simplification.

### Each village building is a courtyard farm that fills its box

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 22*

**The choice.** A placed building is a 30 × 24 × 8 m box, and the simulation blocks sight across all of it. One small house in the middle would leave most of that box empty while still blocking sight. So each of the three houses (`house_a` 30 × 24, `house_b` 34 × 28, `house_c` 26 × 22, all 8 m) is a courtyard farm whose outer walls stand on the box's faces: a dwelling, a barn, a stable wing, and a yard wall with a gate. The houses share one plan scaled to each box. The alternative was a single house per box.

**The gap.** "Houses sized to the prop half-extents" with 30 m boxes.

**The reach.** Fog edges and blocked sight lines fall on real walls.

**Verdict.** sound — the drawn world matches the rule world.

### Texture channel packing follows glTF, with the side-tint mask in ORM alpha

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 21b*

**The choice.** Albedo is sRGB, its alpha the wear threshold; normal is tangent space (glTF convention); ORM is linear occlusion, roughness, metalness, with alpha a side-tint mask multiplied into the material's `tint`. Occlusion must share the metallic-roughness image (glTF allows a separate one; here that's a `structure.texture` error).

**The gap.** The seam named channels, not packing.

**The reach.** Imported glTF assets with separate occlusion must be repacked.

**Verdict.** sound.

### Textures are uncompressed RGBA8 with full mips at 256 px, under a 32 MiB budget

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 21b*

**The choice.** Textures are RGBA8 with box-filtered mips (albedo averaged in linear light, normals renormalised), no GPU compression. Every recipe and tier is 256 px; detail comes from tile size in metres (1.6 mm a texel on a soldier's camo, 16 mm on a tank). The texture arrays size to the largest installed layer. The budget is 32 MiB for a battle's model textures (the village uses about 18 MiB); it is recorded in `frame-cost.md`, not enforced in code. A deterministic BC7 encoder would save about 4× but add a dependency.

**The gap.** Format and resolution were delegated.

**The reach.** Revisit compression if textures approach the budget.

**Verdict.** sound.

### UVs are box projection in metres

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 21b*

**The choice.** Each material is UV-mapped by box projection in metres (`box_uv`) after modelling. Seams fall where the projection switches axes; tiling recipes hide them, and every LOD tier samples at the same scale. The alternative, hand-unwrapped islands, doesn't suit scripted models.

**The gap.** UV method.

**The reach.** Textures can't carry unique painted details (numbers are geometry or decals).

**Verdict.** sound.

### Vertex colour is macro variation over the texture, and its alpha drives wear

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 21b*

**The choice.** On a textured material, vertex colour stores variation relative to the texture's mean at a third (`colour_scale` 3); the shader multiplies albedo by `colour × colour_scale`, so dust, ash and rust can lighten as well as darken. Vertex alpha is wear: where it passes the albedo's alpha threshold the material's `wear` colour shows, with a crisp 0.04 edge (`WEAR_EDGE`). To fit a tangent, the 48-byte vertex keeps its size by narrowing joint weights from 16-bit to 8-bit. One material function (`modelSurface`) serves lit drawing and impostor bakes. Trees and grass are untextured.

**The gap.** How textures combine with the existing vertex paint.

**The reach.** Scripted weathering stays in the model build.

**Verdict.** sound.

### The three variants differ in headgear, kit colour, pack, skin and hair, all from existing textures

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 22b*

**The choice.** `infantry_kit.py <kind> <a|b|c>` builds each from one `LOOKS` table. a is the original soldier. b: helmet with scrim and goggles, shemagh, coyote vest, slim hydration pack, darker skin, black hair. c: bare banded helmet, no pack, no knee pads, ranger-green vest, lighter skin, sandy beard. Recon keeps its rucksack in all three (c wears a boonie hat). Every variant reuses existing texture recipes, so the texture budget does not move.

**The gap.** The number and content of variants were delegated.

**The reach.** At battle range only the pack silhouette really reads; headgear is lost.

**Verdict.** sound — medium because the variety is weak at game range; the user judged models good enough for now.

### Squads at rest vary gaze, idle tempo and stance per soldier, seeded from his id

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 22b*

**The choice.** The pose driver's `restManner(soldier)` gives each soldier, from his id: a gaze stray within ±1.0 rad, an idle tempo 0.8–1.25×, and every fourth man standing watch in the aim stance instead of idle. While the squad's shot counter rises, every man faces the aim; the stray returns after 4 s of quiet, over 6 s. Walking keeps per-id stride phase. Phase offsets alone left squads at rest in one identical stance, read as clones.

**The gap.** "Pose-phase variants" with a shared clip set.

**The reach.** Presentation only: picking stays on the simulation's boxes.

**Verdict.** sound.

### The bridge has piers and abutments below its deck, reaching 2 m under it

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 22b*

**The choice.** The bridge's collision box is a 0.8 m deck. The art hangs a substructure under it down to 2 m below (`BED_M`): two wall piers at a third of the half-stream (`BANK_M` 12 m, sized to the geometry lab's 24 m channel) with cutwaters, and abutments with wing walls at the banks. The appearance's ground tolerance widens to 2.05 m to allow it, per the README's "widen a tolerance per appearance" rule.

**The gap.** The validator required art on the ground plane.

**The reach.** A bridge over a much wider channel than 24 m would stand its abutments in the water; a per-map bridge span is future work.

**Verdict.** sound.

### The pose driver draws a lean as an eased slide out and back, kneeling to fire

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 27d*

**The choice.** The sim says "soldier 3 is out on his lean at point P". The pose driver eases him from his tucked-in spot to P and back (`presentation.pose.lean`: `out_s` 0.25, `back_s` 0.4, sine in-out). His walk cycle reads his published (tucked-in) position, so a lean never plays as a walk. While out he kneels to fire, pinned or not; between bursts a pinned man lies behind his tree. Leans are read from the observation by soldier id and are not interpolated.

**The gap.** The slice did not say how a lean is drawn.

**The reach.** It reads as a step out, not a bend round the bark (matches the sim's geometry). A pinned man tucked in reads as "went prone".

**Verdict.** sound — medium, a presentation taste call.

### Wrecks are separate static models, made from the live vehicle models by one destruction script

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 22; Slice 22b; Orchestrator, after slice 22b; Slice 36*

**The choice.** A tank is destroyed. The simulation places a wreck prop and the renderer draws a wreck appearance fitted to that prop's box. Wrecks are their own static models, not a burnt material on the live vehicle. `blender/wreckage.py` owns destruction modelling: it takes the live tank, truck and jeep parts and warps, dents, sags, tears, hollows and cuts them, so every hole opens onto an interior. Burnt paint (soot, charcoal, ash on top faces, rust) goes on; fire vents char their heart and blister the paint around them; the tank's turret is displaced, its gun droops and its track is a broken chain snaking off astern; the truck sits on its rims with its shelter burnt through; the jeep burns its tyres onto the rims and hangs its HMG off the pedestal. Wrecks are the vehicle scripts run with `--wreck`, and only wrecks take these helpers, so live models stay byte-identical. The alternatives were the spec's "burnt material variant" on the live model, or a hand-made wreck per vehicle.

**The gap.** The spec said "a burnt material variant" and "deformed and torn geometry" with no technique.

**The reach.** Each vehicle needs a wreck model, and any new vehicle gets one from the same helpers. A wreck is scenery, so it takes fog and fitting like any prop.

**Verdict.** sound — reads as destroyed, not as a recoloured live vehicle; the user closed the close-range critique ("current wrecks look fine").

### Deploy motion is authored on the model's parts as data, and the validator checks the pads reach the ground

***sound** · confidence **medium** · Models and the asset pipeline · from Spike 03; Slice 20*

**The choice.** A supply truck deploys: side beams swing out, jacks drop, a mast rises and telescopes. The simulation gives one deployment progress (0 to 1). Each moving part carries custom properties on its model node: `deploy_start`, `deploy_end` (its window), `deploy_move_{x,y,z}` (metres, parent frame) and `deploy_turn_{x,y,z}` (degrees). On the truck: beams 0–0.3, jacks 0.2–0.5, mast raise 0.35–0.65, telescoping 0.6–1.0. Progress moves each part linearly through its window, read by `articulation.ts`. Validation finding `nodes.deploy_motion` fails a supply model with no deploy window, or whose deployed pads do not reach within `ground_m` of the ground. The alternative was phases coded per vehicle in the renderer, tying code to one model.

**The gap.** One progress value drives legs and mast together, and the spike's deploy motion was a Blender function nothing carried into a bundle.

**The reach.** Any new deploying vehicle is art-only work that authors its own phases.

**Verdict.** sound — motion is authored where the art is.

### A hull model must fill the simulation's box, ignoring its guns, with a separate looser tolerance for the roof

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 11; Slice 22*

**The choice.** The tank's simulation box is 2.4 m tall, but its antennas reach 3.45 m. The validator's `fit.hull_extents` compares the model's rest geometry, minus the parts its mounts carry, against the type's hull half extents, in both directions (too small or too big). It has two tolerances: `hull_extent_m` 0.1 m for the sides and `hull_top_m` for the top (0.1 m by default, 1.1 m for the tank and the jeep, for antennas and cupolas). Ground contact is `basis.ground`. One symmetric tolerance would have let the sides stray by a metre too.

**The gap.** "Vehicle bounds against the hull extents" didn't say which parts count, and the spike said "per-appearance tolerance" for antennas without a shape.

**The reach.** Validation only: every vehicle's hit box matches what's drawn, sides tightly.

**Verdict.** sound.

### Every model source, trees and hedgerows included, is project-owned art built by scripts in `packages/scene-assets/blender/`

***sound** · confidence **medium** · Models and the asset pipeline · from Slice 19; Slice 22*

**The choice.** Someone wants to rebuild the tank, or change the trees. `packages/scene-assets/blender/` holds the builders: `parts.py` (primitives, detail tiers, the vertex-colour paint bake, export), `textures.py` (baked texture sets), `masonry.py` (village paints and builders), `wreckage.py`, and one script per model family (`tank.py`, `supply_truck.py`, `jeep.py`, `house.py`, `props.py`, `trees.py`, plus the infantry scripts). `build_sources.sh` rebuilds all of `assets/source/` through the asset CLI's `blender` command. Wrecks are the vehicle scripts run with `--wreck`; houses and their ruins are one script with `--ruin`. `trees.py` writes `assets/source/trees/{tree_broadleaf, tree_spreading, tree_tall, hedge_shrub}.glb` deterministically (a rerun reproduces every hash); each has four detail tiers sampled from one lobed shape on finer or coarser spheres, so the silhouette holds across tiers. They are `scenery` appearances marked `project-owned`, and must fit under the lowest forest canopy of the fixture (finding `fit.canopy`, read through `scene-assets/src/authority.ts`). The alternatives were hand-authored .blend files, and free CC0 packs for trees.

**The gap.** The spec named three scripts, not where they live, how props and wrecks are made, or where trees come from.

**The reach.** Every model is reproducible from code with no licence exposure; `parts.py` is the one owner of the paint bake. Changing trees is a script edit, though hand-authored or photo-real trees would replace this.

**Verdict.** sound.

### Smoke and dust are lit by the world's own sun and sky

***sound** · confidence **medium** · Effects and sound · from Slice 26*

**The choice.** Each smoke or dust sprite is an albedo shaded like a soft ball: the sun on its sunward side (wrapped, as light scatters through smoke), plus sun scattered toward the eye when seen against it (Henyey-Greenstein, g 0.4), so backlit dust glows instead of going dark, plus the sky's diffuse light from the environment's prefiltered map, with the sheet's own relief on top. The effect pass binds the environment uniform and map directly (`EffectLight`). Fire keeps its own colour. Smoke neither casts nor receives sun shadow.

**The gap.** The lighting model was delegated.

**The reach.** A column turns with the sun and takes the sky's colour; changing environment lighting changes smoke automatically.

**Verdict.** sound — physically grounded and consistent with world lighting. Open: no smoke shadows.

### Effect budget: 8,192 instances, smoke limited to 4,096 and thinned evenly

***sound** · confidence **medium** · Effects and sound · from Slice 25; Slice 26*

**The choice.** The effect pass holds 8,192 instances (`capacity`); past it an effect is dropped and counted. Smoke sources together may use at most `smoke_budget` 4,096. Past that, every source thins alike: each puff and flame is kept by its own seeded draw, so every known wreck still smokes (fainter) and combat effects are never starved. A burning wreck holds about 51 instances, a smouldering one about 20; 2,000 wrecks burning at once held 3,955 with none dropped.

**The gap.** Budgets were delegated.

**The reach.** Large battles degrade smoke density before fire effects.

**Verdict.** sound — graceful, fair degradation.

### No speed-of-sound delay: a far gun is heard when its flash is drawn

***sound** · confidence **medium** · Effects and sound · from Slice 40*

**The choice.** A tank fires 1 km away. Real sound would arrive about 3 s after the flash. Here it plays at the flash; distance is heard as level, the far sample and an air low-pass instead.

**The gap.** The spec did not say.

**The reach.** Onset lines up with the visual event; a fixture knob could add delay later.

**Verdict.** sound — medium: a delay is more realistic and some players expect it.

### The listener stands a quarter of the way from the camera's target toward its eye

***sound** · confidence **medium** · Effects and sound · from Slice 40*

**The choice.** `presentation.audio.listener_eye_share` 0.25 places the ear between the look-at point and the camera, facing where the camera looks, level. At the eye, a strategic zoom would hear nothing; at the target, a low camera's own position would not matter. Level falls off as `ref_m` 15 over distance (`rolloff` 1), silent past 3,000 m, not started under 0.004. Browser panners pan by direction only, so what the code ranks and what is heard agree. The air low-pass runs from 18 kHz at the listener to 1.2 kHz at 1,200 m.

**The gap.** Listener placement in an RTS camera is unspecified.

**The reach.** All mix balance depends on it.

**Verdict.** sound — tuning, medium.

### 48 voices at once, 16 of them loops; louder new sounds steal the quietest

***sound** · confidence **medium** · Effects and sound · from Slice 40*

**The choice.** `presentation.audio.budget` allows 48 voices, 16 of them loops (engines, running gear, turrets, reverse, fires, motors); ambience is outside the budget. Priority is loudness at the listener: a louder new transient takes the quietest's place with a 20 ms fade, a quieter one is dropped; loops are re-chosen every frame, loudest first.

**The gap.** No voice budget in the spec.

**The reach.** Bounds audio cost in big firefights.

**Verdict.** sound — 48 is a guess the live check showed ample.

### Shots are inferred once, from shot-counter rises paired with the next publication's new rounds, and flashes and gunfire share that detector

***sound** · confidence **medium** · Effects and sound · from Slice 25; Slice 40; Slice 27 (muzzle flash)*

**The choice.** A rifleman fires one shot. The simulation raises his squad's shot counter on tick N, but the round first appears in flight on tick N+1, and the publication has no round id linking a shot to its round. `LaunchTracker` (`packages/battle-renderer/src/effects/launches.ts`) is the one place that decides "a gun fired". A mount's shot counter rising means it fired. A hull's mount fires from its own muzzle (its catalog `mounts` row) along its bearing and elevation. A squad's rise of n waits for the next publication and is assigned to up to n soldiers who start a new round of that mount's kind there; a round is "new" when its first point is not where one of last tick's still-flying stretches ended. So a garrisoned squad flashes at its firing slot, and lone shots flash too. A unit first seen, or seen again after leaving view, shows no shot until its counter rises once more. The effects frame and the sound frame each run one tracker over the same publications, so a muzzle flash and its report always agree. The flash is then drawn on the model's drawn muzzle (see the drawn-muzzle entry). The alternatives were looking for the round in the same publication as the rise, which missed every lone shot (sustained fire hid it by pairing each rise with the previous tick's rounds), or sound re-deriving launches its own way, which risks a flash without a bang.

**The gap.** The feed has no round id; the renderer has to infer the pairing, and the spec did not say how. Sound needed launches that effects already found inline.

**The reach.** A flash and its sound are at most one tick (50 ms) late. Any future effect keyed on squad launches inherits this pairing. A clipped enemy round re-entering seen ground on the tick its shooter fires again could flash at the wrong place; not seen in play.

**Verdict.** sound — the best inference without adding a round id to the publication, matching when the simulation actually flies a round; medium because it is inference.

### The battle view draws fire only as effects; labs keep the diagnostic flight overlay

***sound** · confidence **medium** · Effects and sound · from Slice 25*

**The choice.** In the battle view, tracers and hit marks are drawn only by the effect pass; the flat tracer lines and remembered strike marks of `buildBattleOverlay` are not drawn in play. Labs (weapons, readouts, ambush, garrison, ground, consequences, ballistics) keep them as diagnostics beside the effects. `/lab/ballistics` also feeds effects from its own flight events, shaped as a publication would carry them. The alternative was drawing both in play.

**The gap.** Whether the diagnostic flight overlay stays in play once effects exist.

**The reach.** One owner of drawn fire in play; labs remain free to show debug lines.

**Verdict.** sound — a clean cutover with diagnostics kept where they help.

### Contacts are drawn as a world-hatched area with a red glow; a last sighting is a pale ghost

***sound** · confidence **medium** · In-world UI and HUD · from Slice 15*

**The choice.** Blue hears a firing report or loses sight of an enemy. The observation gives an approximate area (centre, radius, freshness, source), never the exact unit. Today `buildContactGlyphs` (`contactGlyph.ts`) draws a `ContactGlyph`: every contact has a red glow and a diagonal hatch; a last sighting is a pale ghost (hatch plus outline, glow hugging the rim); a firing report keeps an even red fill hatched in red. Glyphs fade with `contactFreshness` and vanish at expiry. The hatch is anchored to the world (no direction of its own, so it never hints at a heading). Numbers are in `presentation.contacts` (hatch every 16 m, 1.2 m wide, outline 1.6 m, etc.). `battleOverlay.contactLayer` is the only caller.

**The gap.** The contract gave "a hatch plus the red glow" but not how firing and last-seen differ.

**The reach.** The contact glyph is the only visual for approximate enemy information; any new contact source needs a look here.

**Verdict.** sound — source is visible, precision is not.

### A moving vehicle shows two painted chevrons behind it that pulse in the direction of travel

***sound** · confidence **medium** · In-world UI and HUD · from Slice 27e; Slice 27e (follow-ups)*

**The choice.** A tank reverses. Two small chevrons appear behind its marker circle (starting 1 m outside it), pointing against its facing; on a forward move they point along it. A pulse runs through them in the travel direction (`orders.march`: 1 cycle a second, dimming by 0.6). The pulse phase is stored in the vertex (the painted-marching mesh, `WorldMeshes.paintedMarching`) and driven by the frame's presentation clock, never wall time, so a paused or captured frame is still. They show only under the moving vehicle, when it is selected or Space is held, never at the destination. Squads get none: they never reverse, and their route already shows direction. Chevrons are always ground paint, whatever the scheme.

**The gap.** The user asked for travel chevrons that march; placement, clock and which units get them were delegated.

**The reach.** The presentation clock rule keeps every animated mark deterministic in tests.

**Verdict.** sound.

### Each own unit's readout floats beside it on a leader line, with no box, and nudges ease

***sound** · confidence **medium** · In-world UI and HUD · from Slice 27e; Slice 27e (follow-ups)*

**The choice.** Each own unit has a callout (its weapons' rings and rounds) in the DOM. It floats with its near bottom corner 30 px to the side and 34 px above the unit's anchor (a point 2 m over it). A leader line runs from a 2 px ring on the unit to that corner and along the callout's foot. At the right screen edge it hangs left. Leaders are dotted and neutral; a selected unit's is solid and cyan (the HUD accent), so they never read as another order line. Placement never goes under a HUD bar or over another callout, and keeps every unit's anchor clear. A selected unit's callout is bright with name and weapon captions; others are dimmer, rings and rounds only. Rings: reload dashed amber outside, aim cyan inside; why a weapon can't fire is an amber glyph in the middle. No box: a dark text stroke and glow carry legibility. When a callout must move to clear another, it eases toward its new place with a 50 ms time constant on the presentation clock (`easeNudge`, about 95% in 150 ms); layout uses the targets, so the next one clears where this one is going. A paused or rewound clock snaps to the settled layout, so paused callouts still clear each other as the camera moves; a new callout and a left/right flip snap. Destinations carry no text (user): the marker and route say whose it is.

**The gap.** The user gave the reference (a unit connected to its readout by a line); placement, hierarchy and motion were delegated.

**The reach.** Anything marked `data-occludes-readouts` pushes callouts away. `ReadoutLayerHandle.place` takes the frame's presentation clock.

**Verdict.** sound — medium, a taste call on placement and weights.

### The battle HUD is a slim top bar plus a strategy-game command bar along the bottom

***sound** · confidence **medium** · In-world UI and HUD · from Slice 27e; Slice 27e (follow-ups)*

**The choice.** The user said "game UI, not b2b saas" and asked for a command bar at the bottom. The battle's HUD (`[data-testid=battle-panel]`) is two bars, both marked `data-occludes-readouts`: `header.hud-top` (title, status, run controls, sound) and `footer.hud-bottom` (the unit card, a 6 × 2 command grid, the radio and command log). Dark glass, thin accent edges, monospace small caps; custom controls, no browser widgets. Command tiles show a glyph, a short name and a key chip; each button's accessible name is exactly its old text, so tests and screen readers are unchanged. The unit card for one unit shows name, segmented strength and pinning gauges, and each weapon's state; for two or more it is compact rows (`.ro-group`: "N units selected", then one row per unit). The log reads terse lines like `✓ #1 move tank #0 to (84, 148) · tick 27`.

**The gap.** Layout, contents and styling were delegated.

**The reach.** Accessible names are the stable test seam. A later critique still read the chrome as "a dev dashboard with a sci-fi skin" (run controls, sound slider, log coordinates, empty space); those are open for a chrome pass.

**Verdict.** sound — medium, taste.

### Ground paint behaves like a light: mostly self-lit, saturated against the tone mapper

***sound** · confidence **medium** · In-world UI and HUD · from Slice 27 (muzzle flash) — the paint is a light*

**The choice.** The user asked that painted ground marks look like light shining up. The paint's settings are under `overlay.paint` and `overlay.glow`:
- `paint.albedo` 0.15 and `glow.ground` 0.4: the hue is mostly emissive, so shade and cast shadows barely darken it.
- `paint.saturation` 1.3 pushes the colour away from grey, because the AgX tone mapper and the colour grade pull bright hues toward white.
- The paint target holds colour over a range of 2 (`PAINT_RANGE` in `frame/fogTerm.ts`), so a mark can glow past full value.

**The gap.** "Look like a light" had to become numbers, and the tone mapper fights saturated brights.

**The reach.** Shadows barely read on paint. Scene checks for shadows on paint ask only for "strictly darker".

**Verdict.** sound. It is the user's look.

### Colour roles are a fixture scheme; the shipped `yellow-orders` draws order marks as true-yellow overlay and the selection as amber ground paint

***sound** · confidence **medium** · In-world UI and HUD · from Slice 35; Slice 27e; Slice 27e (follow-ups); Slice 27 (muzzle flash) — painted chosen over overlay; selection colour option C; option sheet; colour roles; user picked yellow-orders*

**The choice.** The player selects a rifle squad and a tank, orders the squad across a field to a hedge, and holds Space. Three colour roles are drawn. **order**: the route, the destination marker (a vehicle's circle, or a squad's area ring with each soldier's spot inside), arrowheads, queued waypoints (at `queued_alpha` 0.5), Space's current markers under every own unit (at `current_alpha` 0.55) and the cover pips. These are true yellow `[1.0, 0.9, 0.3]` in the overlay layer: composited after tone mapping so the colour is exact, drawn over dust and smoke, still hidden by a hull, wall or ridge in front (depth-tested). **selected**: the selected unit's own circle and arrowhead, amber `[1.0, 0.6, 0.12]` ground paint in the world, lit, shadowed, fogged and under smoke, glowing past its colour by `selected_glow` 1.2. **soldier**: the marker under each soldier of a selected squad, also amber paint, told from the squad circle by shape and size. The fixture says this: `presentation.overlay.orders.scheme: "yellow-orders"` picks one of `orders.schemes`, each giving the three roles a colour and a layer (`"world"` paint or `"overlay"`); `resolveOrderScheme` in `packages/battle-renderer/src/orderOverlay.ts` turns it into the `OrderStyle` one builder draws, with no code branch per scheme. The other scheme, `white-orders` (white painted orders, yellow overlay selection), stays in the fixture as a switch. Colour carries meaning in few places: one order colour for every order kind; a blocked route is red and always paint; the cover tiers are pips (see the cover-pip entry). A moving vehicle's travel chevrons are paint. There is one route per unit and never a line from a single soldier (see the order-is-the-unit's entry). Cyan is only the HUD accent (`presentation.hud.accent`) and, as pale blue, own units' x-ray. The user picked this scheme from a side-by-side option sheet.

**The gap.** The spec gave no colour or layer for orders versus the selection. The user chose the scheme; making it a data switch with a layer per role, and the three-role split, were the agent's.

**The reach.** Changing the look is one fixture line. Anything new that draws an order mark must take a colour role, not a hard-coded colour. Order marks are never hidden by smoke, a trade the user accepted. Scene checks read the shipped scheme's layers, so switching the scheme needs them retuned. The light-cover pip shares the order yellow, so anything telling them apart must use position.

**Verdict.** sound — the user's pick, expressed as data with one owner. Since superseded: the UI simplification pass made every ground mark paint, and the scheme switch is gone (decisions).

### Order and HUD lines are a fixed width on screen: 5 px strokes, thinner marks for a soldier's own markers

***sound** · confidence **medium** · In-world UI and HUD · from Slice 27e; Slice 27 (muzzle flash) — strokes 50% thicker; strokes at 5 px*

**The choice.** The player zooms from street level to the strategic view; order lines keep the same on-screen weight. Lines are sized in screen pixels at the camera's target: routes and rings `overlay.orders.line_px` 5, marker outlines, arrowheads and chevrons `mark_px` 3.75, on both layers, and the supply, suppression and impact rings and the objective zone use the same weight. A soldier's own marker and his destination spot keep a finer `soldier_mark_px` 1.5 and `soldier_line_px` 2, so small per-soldier marks don't clot into blobs. On the ground no line is narrower than `min_line_m` 0.05 m. Marks are rebuilt only when the zoom crosses a ×1.25 step (the map border's mechanism, shared), and `metresPerPxAt` in `camera3d.ts` is the one owner of that projection. Unit tests read the weights from the fixture and pin them relative to each other. The alternative was widths in metres, which vanish when zoomed out and swamp the view up close.

**The gap.** The user gave the feel (thin, then 50% thicker), not the mechanism or whether per-soldier marks count.

**The reach.** Every mark builder takes `metresPerPx`; labs that don't follow their camera use the opening camera's scale.

**Verdict.** sound — medium on the weights, which are the user's taste.

### A unit's marker is a circle with a filled arrowhead on its rim, used everywhere, and routes are clipped so they never cross an arrowhead

***sound** · confidence **medium** · In-world UI and HUD · from Slice 27e; Slice 27e (follow-ups); Slice 27 (muzzle flash)*

**The choice.** A rifle squad faces east and the player orders it further east. Every unit mark is "the unit plus its facing": a circle with a small filled arrowhead on its rim (the user picked it from a sheet of five shapes), drawn by one function, `circleMarker`. Under a moving unit, and under a vehicle that is selected or when Space is held, the arrowhead points where the unit faces now (its published yaw); for a squad the circle surrounds its soldiers. At the destination it is a vehicle's marker, or a squad's area ring (anchor and radius at the destination), with the arrowhead at the final facing and each soldier's spot marker (a 0.45 m circle) inside. A vehicle's circle is its hull's half-length (from the catalog's `half_extents_m`) plus `orders.vehicle_marker_margin_m` (0.7 m), so a new vehicle needs no new number and the ring peeks out from under any hull. The arrowhead is capped at 1.5 m (`MARKER_HEAD_MAX_M`). A squad's circles are drawn at `area_draw_scale` 0.8 of their true radius (the user's visual scale; the real area is unchanged). `routeBetween` (`orderOverlay.ts`) clips every route, queued legs too, at the circles it joins: when a route leaves or arrives within 60° of a circle's facing it is clipped at the arrowhead's tip, otherwise at the rim, so a route never runs over an arrowhead. With Space, a holding squad's area is the same ring round its anchor, faint; a route waiting for the way to clear adds a broken ring round the unit. There is no other area ring.

**The gap.** The user chose the shape and the "no spokes, area ring, circle under the unit" look; sizes, clipping and when each circle shows were left to the implementer, and the arrowhead overlap was a critique finding.

**The reach.** Any new unit type gets markers from catalog data; any new marker with a protrusion needs the same clipping. At 0.8 scale, a soldier can stand visibly outside his squad's drawn circle.

**Verdict.** sound — medium on the sizes, which are taste.

### The scenario variant and seed live behind a chip in the status line, in a small dialog

***sound** · confidence **medium** · In-world UI and HUD · from Slice 27e (follow-ups)*

**The choice.** The player wants another seed of the ambush. The status line's "Ordinary ambush · seed N" is a bracketed chip button (`aria-label=Scenario`) that opens a glass dialog: the variants as a `radiogroup "Variant"` list, the `Seed` field with ◂ ▸ steps. The dialog is opaque and marked `data-occludes-readouts`. A new seed or variant remounts the battle, closing the dialog. The status text keeps its wording, so status checks still pass. The alternative was a variant dropdown and seed field in the top bar, which read as tooling.

**The gap.** The critique flagged the controls; the replacement was delegated.

**The reach.** Scenes open the dialog before each change.

**Verdict.** sound — medium, taste.

### A mixed selection's attack reaches only its armed units; an unarmed unit keeps its orders

***sound** · confidence **medium** · In-world UI and HUD · from Post-close review (presentation)*

**The choice.** The player selects a tank and a supply truck and right-clicks an enemy squad. The tank attacks. The truck, which has no weapons, is left out and carries on with what it was doing. `commandReach.ts`, the one owner of which selected units a command reaches (see the mixed-selection entry), counts a unit as able to attack when its type has mounts (weapons). Every attack path goes through that test: right-clicking an enemy, Ctrl+right-click, and an attack-move or attack-ground click. X (attack-move) and G (attack-ground) arm only when an armed unit is selected, and the command bar lights them the same way. With only the truck selected, nothing is sent. The alternative was to give the unarmed unit a plain move to the clicked point in place of the attack-move. It was rejected because the player asked for an attack, not a move. The readouts scene checks it with a tank and a truck.

**The gap.** The review found attacks sent to units that can't fire. What an unarmed unit should do instead was open.

**The reach.** Only the player's input changes; scripts and the AI are untouched. A future unarmed type, such as a scout car, sits out attack orders, so a mixed group sent by attack-move leaves it behind.

**Verdict.** sound. Medium, because "move along anyway" is a fair RTS reading too. To reverse it, have the attack-move path send unarmed units a plain move.

### Infantry's all-round sight is catalog data, not a rule in code

***sound** · confidence **medium** · The catalog · from Slice 04*

**The choice.** A squad sees evenly in every direction (settled by the user). Today that is data: the abstract `squad` entry in `fixtures/units/generic/infantry.json` sets `sight_shape {1, 1, 1}`, and every squad type inherits it. Nothing in code forces it; a catalog author could give a squad a directional shape, and it would then look along the squad's heading. The alternative, pinning infantry to `{1, 1, 1}` in code, would be a rule that asks "is this infantry?", which the catalog's "behaviour comes from components" rule forbids.

**The gap.** Whether infantry's shape lives in data.

**The reach.** Types stay uniform: one sight component for everyone.

**Verdict.** sound — consistent with the catalog. If the user wants isotropy guaranteed, add a load check that squad bodies have an even shape.

### Scenery — props, trees, hedgerows, grass — are workbench appearances of a `scenery` unit, extended through one table

***sound** · confidence **medium** · The catalog · from Slice 20; Slice 19; Post-close review (presentation)*

**The choice.** An artist adds a sandbag model. Today a scenery appearance's catalog entry is `{unit: "scenery", scenery: "<kind>", states: {...}}`: a static bundle, one GLB per state, four detail tiers, drawn instanced and impostor-baked. The kinds are one table, `SCENERY_KINDS` (`scene-assets/src/scenery.ts`): each row lists the states the art must carry and what it stands for, props, a forest tree, or nothing. A prop row says only `{ kind: "prop" }`; which prop types it draws comes from the prop catalog, where each type's `appearance.drawn_by` names its kind (`propsDrawnBy(props, kind)`). The workbench's box overlay takes the first of those types the map places, else the first. A test holds both directions: every prop row draws some type, and every type's `drawn_by` names a prop row, `building` or `forest`. Trees, hedgerows and grass carry one `summer` state per biome season. Buildings keep their own unit with `intact` and `ruin` states. An unknown kind fails `structure.scenery_kind`.

**The gap.** The user decided scenery should be workbench-viewable without giving a schema.

**The reach.** New scenery is a table row plus art, bound from the prop catalog; a winter season adds a state per kind. A row with no prop type drawn by it fails the test, which is why the old `trunk` row went (the trunk prop is drawn by `forest`).

**Verdict.** sound. One extension point, and the prop catalog is the one owner of which art draws which body.

### A prop's art is authored to one declared simulation box and checked against it

***sound** · confidence **medium** · The catalog · from Slice 22; Post-close review (presentation)*

**The choice.** A house is placed on the map as a 30 × 24 m box. Each static prop appearance declares the box it is modelled to (`footprint_half_m` in `assets/catalog.json`). A validator finding, `fit.footprint`, measures the art against it; a building's ruin is checked at the ruin height, which the fit authority reads from the catalog prop type's `destroyed.into.height_m` (`scene-assets/src/authority.ts`). The building appearances share one authored ruin state, so `fixtureAuthority` throws, naming them, when the prop types they draw leave remains of differing heights. Keying the height per appearance would need a link from appearance to prop type that the asset catalog doesn't have. A catalog test holds every building to a village placement, every wreck to a hull box, and every ruin to a building's plan at ruin height. The workbench draws the declared box. The box travels into the runtime catalog and the installed appearance (`footprint`), where the renderer fits each placed prop from it. The alternative was sizing art per placement with no declared box.

**The gap.** The user's 2026-09-26 decision said "validated against its simulation footprint" with no schema.

**The reach.** The simulation box is the authority; art must fit it. The renderer's fitting (next entries) depends on it.

**Verdict.** sound.

### The lab presents every scenario with the shipped catalog

***sound** · confidence **medium** · The catalog · from Slice 27f*

**The choice.** Presentation reads types from the generated shipped catalog (`UNITS`). Every lab scenario runs on it. A scenario with its own catalog would need its own view passed to the session. Lab diagnostics and the command log keep `kind #id` text. The player's unit card and callouts show only the type's name.

**The gap.** The slice did not say whether scenarios could carry private catalogs to the renderer.

**The reach.** Modded or per-mission unit types need the session to take a catalog view.

**Verdict.** sound for now.

### A test forbids hand-written vector and matrix helpers outside `math`

***sound** · confidence **medium** · Tooling and tests · from Slice 28*

**The choice.** `web/tests/mathOwner.test.ts` fails on declared helpers named like vector or matrix math (cross, dot, normalize, lerp, lookAt, clamp and so on, with an optional 2–4 suffix), `Vec`/`Mat`/`Quat` type aliases, or files named like a vector module, outside `math`. It allows the two reverse-Z builders and tests its own detector.

**The gap.** How to hold the "use `math`" rule.

**The reach.** Name collisions must be renamed (an order overlay's X mark became `crossMark`).

**Verdict.** sound.

### The workbench draws the simulation's body beside each appearance

***sound** · confidence **medium** · Tooling and tests · from Slice 20*

**The choice.** An artist checks whether a wall model matches the wall the simulation collides with. Today the workbench (`workbench/benchWorld.ts`, `footprint`) draws what the simulation knows: a soldier's cylinder, a vehicle's hit box; for a prop, the catalog's authored footprint box else the map's first prop of that kind, with its blocking classes and whether it hides sight, read from the simulation through WASM (`blockingPropKinds`, `occludingPropKinds`); for a tree, the forest trunk and canopy ring. Nothing is mirrored in TypeScript.

**The gap.** Props are sized per placement, so no single box is "the" prop.

**The reach.** Art/simulation mismatches are visible before battle.

**Verdict.** sound.

### Soldier exports are byte-stable except the coarsest tier

***sound** · confidence **medium** · Tooling and tests · from Slice 21*

**The choice.** Re-running the infantry export should give the same file. Positions snap to 0.1 mm, custom normals are cleared, and islands and faces go in canonical order (`mesh_lods.py`), so clips and LOD0–2 are byte-identical run to run. LOD3's heavy collapse still varies in which vertices it keeps. The bake from a committed source is deterministic regardless. The alternative was chasing Blender's threaded decimation until LOD3 was stable too.

**The gap.** The spec asked for deterministic sources, not how far to go.

**The reach.** Re-exporting a soldier can change its LOD3 hash with no real change; a reviewer should expect that.

**Verdict.** sound — minor, and the runtime bake is deterministic.

### The soldier-to-rifle parity check stays a scratch comparator, not a test

***sound** · confidence **medium** · Tooling and tests · from Slice 21*

**The choice.** The ported clip script had to match the frozen `rifleman.glb` exactly before any changes (it did: every joint and frame). The comparator was run once and not committed. It needs the frozen reference and the sources, both in LFS, which a worktree with pointers only can't run, and the frozen reference was a one-time gate, not a standing contract. The alternative was a committed test that pulls LFS.

**The gap.** The parity row said what to compare, not the tool.

**The reach.** Nothing checks clip parity going forward; later clip edits are intentional and listed in `assets/spikes/README.md`.

**Verdict.** sound.

### No automated test guards the performance regression

***sound** · confidence **medium** · Tooling and tests · from Slice 29*

**The choice.** The 40% slowdown came from a compiler inlining decision, visible only as instructions retired. There is no portable, unprivileged per-thread instruction counter a `cargo test` could read, and a wall-time or call-count test would either flake under load or not see compiler output at all. So there is no test; the guard is comparing the report's instruction column across builds.

**The gap.** The brief offered a guard as optional.

**The reach.** A future regression of the same kind is caught only if someone runs the report.

**Verdict.** sound — revisit if a CI host with stable hardware counters appears.

### Scenario checks are geometric, computed from the true battle state

***sound** · confidence **medium** · Tooling and tests · from Slice 30*

**The choice.** A check reads the simulation's own state, not anything published to a player: a soldier's 0.3 m disc against every prop that blocks infantry; a hull box against props that block vehicles; closest soldier pairs within and between squads; soldier height against the ground; "in cover", which calls the simulation's own cover rule (`sim::cover::at` through `cover_tier`) against the named threat; how far short an attack-move first halts; a prop's displacement. The alternative was adding new published fields only for tests.

**The gap.** Which measurements define "moved right" was delegated.

**The reach.** Checks can reach internal state freely, so they test rules, not the publication.

**Verdict.** sound — the cover check now uses the real tier, so it cannot drift from the game.

## Sound, confidence high (189)

### A kinetic round is one with no blast; anything with a blast bursts on every hit

***sound** · confidence **high** · Simulation rules · from Slice 06*

**The choice.** A round with `blast_radius_m` 0 (rifle, HMG, AP) is kinetic and can glance off armour. A round with a blast (HE, grenade, ATGM) always detonates on a hull (`damage::meet_hull`). The alternative, a per-row "kinetic" flag, is a second field that could disagree.

**The gap.** The contract said "kinetic" and "HE" without naming grenades or missiles.

**The reach.** Only blast-free rows can ricochet.

**Verdict.** sound.

### The armour face a round meets is judged at the moment of the hit

***sound** · confidence **high** · Simulation rules · from Slice 06*

**The choice.** A tank turns while a round flies. The face that decides penetration and ricochet is the face the round met, judged just off the struck surface, at the hull's pose at the moment of the hit (`face_toward`). Damage uses the same judgement. The alternative, the pose at the end of the tick, could credit a side hit to the front.

**The gap.** The contract fixed the pose, not the face rule.

**The reach.** Every hull hit.

**Verdict.** sound.

### "Did it burst" is decided once and carried on the impact

***sound** · confidence **high** · Simulation rules · from Slice 06*

**The choice.** The impact resolver decides detonate, bounce or stop, and `Impact.detonated` carries it. Blast damage and the published blasts read that flag rather than re-reading the blast radius. The alternative has two places deciding.

**The gap.** Who reads `Detonate`.

**The reach.** One owner of "did it burst".

**Verdict.** sound.

### After a bounce, a round ignores only the hull it just glanced off, for that tick

***sound** · confidence **high** · Simulation rules · from Slice 06*

**The choice.** A round glances off tank B. For the rest of that tick it can't hit B again, but it can hit tank A, even if it glanced off A earlier. Near misses keep the closest pass per unit across all legs.

**The gap.** How long the exclusion lasts.

**The reach.** Rounds can ping between two hulls.

**Verdict.** sound.

### A crater is light cover for a soldier standing in it, from every side, once it is half deep

***sound** · confidence **high** · Simulation rules · from Slice 07*

**The choice.** An HE shell digs a crater; a rifleman later lies in it while taking fire. Today cover (`sim/src/cover.rs`) treats a crater under a soldier's feet as a cover body of tier `cover.crater` ("light") whenever its fill is at least `cover.crater_min_fill` (0.5), whatever the direction of fire; the strongest body protecting him widens the incoming round's spread by its tier. Cover protects soldiers only, so a tank parked in a crater gains nothing. The spread uses the true ground layer; seeking cover uses the craters the side has seen. The alternative, a crater strength on the old forest-cover scale, went away with that scale.

**The gap.** The slice named one crater-cover key without saying how it combined with other cover.

**The reach.** Every shelled area becomes light infantry cover; balance of attacks through artillery-churned ground depends on it.

**Verdict.** sound — craters are just another tiered body in one cover rule.

### Track and footprint wear is one mark per cell entered, never decays, and changes nothing

***sound** · confidence **high** · Simulation rules · from Slice 07*

**The choice.** A tank drives across a field. Today each vehicle has two track points at ±`ground.track_gauge` (0.75) of its half width, and each living, ungarrisoned soldier is one point; entering a new cell adds `tracks_per_pass` (24) or `trampled_per_pass` (6). Every channel saturates at 255; nothing decays. Wear is cosmetic: a test pins combat identical with the channels off.

**The gap.** Accumulation was delegated.

**The reach.** Ground wear is drawn and digested but never feeds rules.

**Verdict.** sound.

### The ground layer is stored in 16 × 16-cell tiles allocated on first mark, digested per tile

***sound** · confidence **high** · Simulation rules · from Slice 07*

**The choice.** A long battle shells part of a large map. Today ground cells live in 16 × 16 tiles created only when first marked, so storage grows with ground touched and never past the map's area (about 4 bytes per cell; 10 MiB for the whole village if all marked). Cells are row-major inside a tile, tiles row-major. The battle digest folds a per-tile hash refreshed once per tick (`GroundLayer::seal`), so digesting stays cheap however much ground is marked.

**The gap.** Cell order and the "bounded by map area" rule were delegated.

**The reach.** Replay digests include the ground; any new channel must go in the tile hash.

**Verdict.** sound.

### The hull-distance speed fix is written into the code, not left to the compiler

***sound** · confidence **high** · Simulation rules · from Slice 29*

**The choice.** When a shell bursts, the simulation measures how far many points (soldiers, fragments) are from a tank's hull box. Each measurement needs the hull's rotation (a sine and cosine of its heading). An unrelated change had made the compiler stop reusing that sine and cosine, so the same two numbers were recomputed for every point and combat got about 40% more expensive. Today the code computes them once per hull: `Unit::hull_frame()` returns a `HullFrame` (the hull box with its rotation already worked out), and callers that measure many points (`weapons.rs`) use it. `Unit::hull_distance` stays as `hull_frame().distance(p)` for one-off callers such as blast damage. The alternative was an `#[inline]` hint, which would restore the fast code today but leave it to the optimiser again.

**The gap.** The performance brief said "fix it" and named no method.

**The reach.** Any future hot geometry that repeats a per-body transform should hoist it the same way. Battle outcomes did not move (digests identical).

**Verdict.** sound — a source-level hoist survives future compiler and inlining changes; a hint does not.

### One rotation formula for the whole simulation

***sound** · confidence **high** · Simulation rules · from Slice 29*

**The choice.** Rotating a 2D point by a heading happens all over the simulation. `math::Rotation` is the only place the formula lives; `V2::rotated(yaw)` just builds a `Rotation` and applies it. The alternative, two hand-written copies, could drift apart (for example if someone reordered the arithmetic in one), and because floating-point order changes the last bits, battles would stop replaying identically (`Battle::digest`, the battle's fingerprint, would change).

**The gap.** Not specified; it came out of the hull-distance fix.

**The reach.** New rotation code must go through `Rotation`.

**Verdict.** sound — one owner for a formula that feeds the determinism check.

### A guided missile that loses its guide coasts for a time set in its own fixture section

***sound** · confidence **high** · Simulation rules · from Slice 38*

**The choice.** An anti-tank missile is steered by its launcher while the launcher stands still, lives and sees the target. When that support ends (the user's rule), the missile flies straight on for a short "coast", then goes to ground. The coast time is `guided.release_coast_s` (0.5 s), a new top-level section in `fixtures/village.json`, parsed as `contract::ballistics::GuidedRules`. It is not a field on the `atgm` weapon row, because it describes what any guided round does once released, not one launcher. `flight::validate_guided` refuses zero, negative or non-finite values at battle setup: at zero the aim point would sit right under the missile and it would circle.

**The gap.** The spec named `guided.release_coast_s` but not where it lives or what values are legal.

**The reach.** A second guided weapon inherits the same coast unless someone moves the number per weapon.

**Verdict.** sound — a rule about all guided rounds belongs in one rules section.

### The flight module computes the coast point; the battle only decides when to release

***sound** · confidence **high** · Simulation rules · from Slice 38*

**The choice.** Each tick `Battle::guide` checks whether each launcher still supports its missile. The check reads the sighting sensed on the previous tick, before this tick's flight, so a missile is released at the start of the tick after the sighting lapsed, from its last supported position. On release it calls `Projectiles::release(id, coast_s, world)`. The flight module fixes the missile's aim point at `position + velocity × coast_s`, dropped to the ground height beneath (`world.height_at`). The missile turns toward that point (about 1°, well inside its 60°/s turn limit) and glides in a straight, shallow line into the ground about 90 m past release. Over a rise the line can hit the slope first, which still counts as going to ground. If the point lies beyond the map there is no ground to drop to: it keeps the missile's height and flies level until its lifetime ends or it leaves the map. The rejected shape was level flight for 0.5 s then a hard dive, which lands farther than "the coast" and needs a second guidance phase.

**The gap.** The dive's shape was delegated, as were the timing and the off-map case.

**The reach.** `GuidedMissile.point` in the observation shows the coast point after release; the guidance point was already in the digest, so no seam changed. Smoke screens, if built, release missiles through the same own-sight check.

**Verdict.** sound — one straight glide is the simplest reading of "coasts, then goes to ground".

### Cover is a scatter rule in `sim::cover`; seeking cover is movement's

***sound** · confidence **high** · Simulation rules · from Slice 33*

**The choice.** One module owns what cover *is*: `cover::at(world, ground, hulls, rules, soldier, shooter) -> Option<Tier>` returns the strongest *tier* (light, medium or heavy, the contract's `CoverTier`) of any cover body within `cover.reach_m` (1.5 m) of the soldier and between him and the shooter, or of a crater under his feet at least `crater_min_fill` (0.5) full. `cover::spread(tier)` is the multiplier on the incoming round's scatter. The same rule, applied to a side's gathered knowledge (`cover::Known`), offers places (`cover::spots`), assigns them (`cover::claim`) and finds firing places (`cover::step_out`). `movement::take_cover` owns *when* a squad seeks cover and walking there.

**The gap.** The seam named `spots(world, knowledge, threat)`; the knowledge became a gathered `Known` so one squad's resolve reads the prop index once.

**The reach.** Weapons, movement and the test runner's `InCover` check all read one rule, so what a soldier seeks is exactly what protects him.

**Verdict.** sound.

### A holding squad that re-resolves against a seen enemy turns to face it

***sound** · confidence **high** · Simulation rules · from Slice 33*

**The choice.** When a holding squad re-resolves against an enemy it has seen, engaged or last seen, its facing (`Unit.yaw`, which is what an idle squad publishes) turns to the enemy. Infantry sight is all-round, so this changes no sighting; it is presentation and order facing.

**The gap.** Q9 asked squads to face threats without saying when.

**The reach.** Idle squad markers point at the fight.

**Verdict.** sound.

### Transient bodies expire on a tick and every side forgets them

***sound** · confidence **high** · Simulation rules · from Slice 34*

**The choice.** A body with `lifetime_s` (a smoke screen) has an expiry tick kept by `Battle` (in the digest). At expiry the body goes and every side forgets it, with a revision bump; the world revision bumps when it appears and goes, a side's when it learns it.

**The gap.** Q28 asked for transient bodies without lifecycle rules.

**The reach.** Smoke and other short-lived obstacles share one path.

**Verdict.** sound.

### Trees topple rather than slide: a body row property

***sound** · confidence **high** · Simulation rules · from Slice 34b*

**The choice.** A body whose row sets `topples` (the trunk) is knocked down, not shoved, when a vehicle that could shove it meets it, wherever it stands (no slide, no chain check), in `movement::push::shove`; `Battle::knock_down` removes it. The shove slowdown still applies for that tick (a tank keeps 1/3 against a medium trunk). Toppled kinds are not drawn apart as movable props: they fall.

**The gap.** Q16 said heavy pushers knock trees down, not how.

**The reach.** Any future toppling body is one column.

**Verdict.** sound.

### Never fire through a building or wall at what's behind it

***sound** · confidence **high** · Simulation rules · from Slice 27c*

**The choice.** A tank sees an enemy past the corner of a house, but the arc meets the house. The building has hp, but it occludes, so the tank holds. Firing into a house at an enemy beyond it would be degenerate play. Test: `a_tank_never_fires_through_a_house_at_ground_beyond_it`.

**The gap.** It was the user's refinement of the fire-through rule.

**The reach.** Houses are shelled only when they are the ordered target (next entry).

**Verdict.** sound (user decision).

### A soldier "leans" by stepping sideways past the edge of tall cover, and one line test decides whether his round gets through

***sound** · confidence **high** · Simulation rules · from Slice 27d*

**The choice.** A rifleman stands behind a tree trunk. The enemy is on the far side. To fire, he needs a point where his round clears the trunk. `sim::lean::points` finds those points for any footprint (a trunk, a house corner, a parked tank): take the two edges of the footprint that the enemy's line of view just grazes; step past each edge far enough that his body clears it (his radius plus `cover.lean_clear_m`, 0.1 m); keep the step level with where he stands, sideways across the enemy's view. Points further than `cover.lean_max_m` (1.5 m) are thrown away, because a longer move is a walk, the cover search's job. The nearest point wins; if both are equally near, the right-hand one. So a man behind a trunk's middle has a lean point about 0.5–0.8 m to one side, and a man in the middle of a long hull has none and must be moved by the cover search. The result is a step out, not a bend round the bark: his body fully clears the cover, so a round from the lean point can never touch it. One function, `lean::reaches(world, hulls, from, to, past)`, is the only test of "does his round get there": no terrain, no round-stopping body except `past` (the body he leans round), no live vehicle hull in the way (grazing a hull within 0.1 m counts as hitting it; a line that ends inside a hull is aimed at that hull). The cover search (`take_cover::Fight`) and the fire code (`weapons::fire_from`) both call it, so they can never disagree about whether a shot is possible.

**The gap.** The slice said soldiers lean out round tall cover, with no geometry and no rule for how the cover search and the fire code agree.

**The reach.** Every future cover or fire rule reads this one test; a new body type gets lean points from its footprint with no code. The "step, not bend" choice means a lean can never chip the cover it leans round.

**Verdict.** sound — general (footprint-driven, one owner for the line test), and the numbers are fixture data.

### Cover counts as "tall" when its top is higher than the soldier's muzzle, read from the body, never its kind

***sound** · confidence **high** · Simulation rules · from Slice 27d*

**The choice.** A soldier behind chest-high sandbags fires over them. Behind a trunk or a house corner, he can't, so he claims a lean point instead. The rule is height, not type: a cover body whose top (`cover::Body.top`) stands above his muzzle is tall; anything lower he fires over. He claims a lean behind tall cover even when some enemies are already in his straight line, because others may not be, and his rounds pass the body he leans round anyway. The alternative, a list of "tall kinds" (trees, buildings), would break for every new prop.

**The gap.** The slice said "tall cover" and gave no rule.

**The reach.** A new prop's lean behaviour follows from its body height in the catalog. A test scenario's sandbags had to drop from 1.5 m to 1.2 m to stay "fire over" cover.

**Verdict.** sound — first-principles (the body's own height), matching the project's rule against named special cases.

### Each rifleman shoots the first enemy his round can actually reach, from where he stands or from his lean

***sound** · confidence **high** · Simulation rules · from Slice 27d*

**The choice.** A squad's rifles are assigned targets in turn. Each soldier, starting from his own turn, fires at the first enemy soldier his side sees whose line his round reaches (straight, or from his lean point). Only if there is none does he fall back to his assigned soldier, which may mean shooting into a breakable body in the way. Without this, men behind a row of trunks kept chipping a stranger's tree while a clear line to another enemy existed (a test pins this: red with the rule off).

**The gap.** The slice named the goal (men fight from cover) but not who each man aims at.

**The reach.** Target choice now depends on per-soldier lines, which future weapon types inherit.

**Verdict.** sound — targets what a soldier would really shoot at.

### A squad can open fire if any one of its soldiers can, and his own lean's hull never blocks him

***sound** · confidence **high** · Simulation rules · from Slice 27d*

**The choice.** A squad stands at a house corner. Its middle is behind the house, so a check from the squad's middle says "can't fire". `weapons::engage` now tries each soldier's own fire point when the middle is blocked or a friendly hull is in line. A soldier leaning round a friendly tank's hull is never refused because of that same hull (`friendly_in_line`'s `leaning_round`). Without this, a squad at a corner never fired.

**The gap.** The engage check predates per-soldier leans.

**The reach.** Any future squad-level fire gate must stay "any soldier can", or corner fights stop.

**Verdict.** sound — required for leans to matter at all.

### "Able to fire" is judged against the enemies the side has seen, where their bodies are

***sound** · confidence **high** · Simulation rules · from Slice 27d*

**The choice.** When the cover search asks "can he engage from here?", it tests lines to the enemy soldiers his side currently sees, at their bodies' real positions (a leaning enemy at his lean point), within the squad weapon's range (`weapons::squad_range`, the rifles). An enemy tucked behind his own trunk blocks the line to him: that's his cover working. It never uses enemies the side hasn't seen.

**The gap.** The slice did not say which enemies, or which positions, the check uses.

**The reach.** Keeps cover choice honest to fog of war.

**Verdict.** sound.

### A soldier's rounds pass only the cover he leans round; with no lean he holds fire rather than shoot his own cover

***sound** · confidence **high** · Simulation rules · from Slice 27c; Slice 27d*

**The choice.** A squad behind a row of trunks returns fire at a squad in the open. Rifles break wood, so without a rule each man's rounds would chip the trunk he hides behind (88 hits in 30 s in the test). The game rule, named in the README's list of hard-coded rules: a soldier's own rounds pass untouched through the tall cover he leans round to fire. A man who has claimed a lean round a trunk carries that trunk on every round he fires, from the lean point or straight from where he stands (`FirePoint.past`, from `lean.past()`, carried on the round as `Shooter.cover` and digested). The trajectory solver (`solve_launch_past`) and flight (`WorldGeometry::raycast_past`, `passes`) fly through that one body with no damage; enemy rounds still hit and wear it. A man with no lean gets no pass at all. If every line from where he stands is blocked by the body he hides behind (`weapons::hides_behind`, the same test that gives him his cover tier) or by a hull, he holds his round until his squad re-plans its cover, instead of firing into his own wall. So a man tucked behind a long wall cannot shoot through it. The alternative was simulating muzzle clearance, or passing any body a standing soldier takes cover behind.

**The gap.** Giving rifles structural damage surfaced the problem, and the own-cover rule had to be reconciled with leans, which the slice did not say how to combine.

**The reach.** Cover is one-way for its user, but only round the one body he leans on. Every future cover or fire rule reads the lean's `past` body; tests `a_soldier_fires_back_past_the_trunk_he_takes_cover_behind` and `enemy_fire_still_strikes_and_wears_the_trunks` pin it.

**Verdict.** sound (user decision) — the film picture (a man behind a wall doesn't shoot the wall), one owner for the pass, and simpler than simulating muzzle clearance.

### Soldiers step out of an oncoming vehicle's path; vehicles never wait for soldiers

***sound** · confidence **high** · Simulation rules · from Slice 32*

**The choice.** A tank drives toward a squad resting on its route. The tank's path over the next `yield_horizon_s` (2 s) at its speed, plus half its hull length, is a *threat* strip. Any soldier (either side, moving or resting, not garrisoned) within the hull's half width plus his radius plus `yield_margin_m` (1 m) of that strip steps straight out of it, on the side he stands, at full pace, before doing anything else. Hulls are solid to his walking and planning. The vehicle never waits for soldiers (`vehicle_conflict` only considers other vehicles). If a hull still ends a step over a soldier, he is pushed out through its nearest side (and out of any prop that leaves him in), unhurt. The alternatives were vehicles slowing or waiting for infantry, or a fan of per-soldier side-step candidates with a minimum headway.

**The gap.** Q23 said vehicles and soldiers must not interpenetrate, not who yields.

**The reach.** Tanks drive through friendly infantry areas on time. Nobody is ever run over; that would be a new rule.

**Verdict.** sound — matches how a film viewer expects infantry to react to a tank.

### Cover only widens the spread of rounds aimed at a soldier; only a garrison's building lowers fragment hits

***sound** · confidence **high** · Simulation rules · from Slice 33*

**The choice.** A grenade bursts beside soldiers standing in a forest. Cover never softens a hit and never lowers the chance a fragment strikes (Q5: cover is scatter-only): it only widens the spread of rounds aimed at a soldier. A forest gives cover only behind its individual trunks, and a crater only as a light-tier body. The one exception is a garrison's building, which still lowers the fragment chance (`damage::fragment_exposure`). The alternative was area protection: a forest rectangle, or a crater, that reduces fragment hits and widens spread everywhere inside it.

**The gap.** Q5 made cover scatter-only without listing which area protections go.

**The reach.** No forest-rectangle special case; a big balance shift for anything fighting in woods, which the balance spec owns.

**Verdict.** sound — one rule instead of a forest special case, in line with the first-principles preference.

### One store holds every prop's damage and destruction (`sim::structures::Structures`)

***sound** · confidence **high** · Simulation rules · from Slice 34c*

**The choice.** A shell hits a fence panel that has never been hit before. `sim::structures::Structures` keeps four things: the damage each worn prop has taken, each remains body mapped to the body it replaced, every destroyed id, and the bodies removed with nothing in their place. A prop never hit counts as whole, so nothing has to register new props. `hp(world, id)` is the row's `hp` minus the damage taken. All four are folded into the digest. A collapsed building is simply gone from the world; the garrison system holds no building damage of its own. The alternative was damage kept per system (buildings in garrison, props elsewhere).

**The gap.** The seam didn't say who owns damage.

**The reach.** Every damage source (direct hits, blast, rounds passing through) writes to this one place. Replays and digests cover it.

**Verdict.** sound. It gives one owner.

### When soldiers claim cover spots, only a soldier who stays put keeps his neighbours away

***sound** · confidence **high** · Simulation rules · from Slice 37*

**The choice.** Sandbags are shot to a rubble strip, and three soldiers stand a step off it, 2 m apart. `cover::claim` hands out spots best offer first. Each taken place must be `spacing` from other taken places and `keep_clear` (2 m) from other soldiers, but only a soldier who is settled keeps his neighbourhood clear. Settled means he is allowed to stay and is offered nothing better than where he stands. So all three step onto the rubble together. The alternative, every soldier not yet placed keeping 2 m clear round him (even one about to leave for a better spot), made the three block each other's spots and all stay out. Test: `soldiers_beside_free_cover_step_in_together`.

**The gap.** The claim rule didn't say who reserves space.

**The reach.** Squads fill freed cover together instead of hanging back.

**Verdict.** sound. It is the general property (reserve only what you will keep), not a patch.

### A gun fires into what it can see past and can break; otherwise it holds

***sound** · confidence **high** · Simulation rules · from Slice 27c*

**The choice.** A tank is ordered to shell a house, and a sandbag wall stands on its line. `weapons::fires_into` decides whether the gun fires. It fires along its arc into the first body in the way when that body has hit points, doesn't block sight (`occludes: false`), and can be broken by the rounds the mount has left of the loaded kind (see the next entry); a body the ordered ground point lies in is the target, not an obstacle. Anything else holds fire: terrain, bodies with no hp (tooth, ruin, bridge deck), occluders (building, wall), or bodies too tough for what's left. When the gun fires into a blocker, the burst point for the friendly-in-line check is the blocker, not the target. This applies to ordered and automatic fire alike (user decision): a tank firing at will shoots into sandbags in front of a squad, and an AT team inside a wood fires its missiles into a trunk on its line rather than holding. Nothing changed on the wire: a gun firing into a blocker reports `Aiming`/`Firing`, and a hold reports `BlockedTrajectory`. The alternative, any static body on the arc making the gun hold forever, is a freeze state.

**The gap.** The brief said "the weapon does structural damage" and no more.

**The reach.** Together with structural damage on MGs and rifles, the battlefield wears down under fire, which is the user's goal.

**Verdict.** sound. It is a user-driven physical rule.

### Fences and crates stop no rounds; a round through a destroyable body that doesn't stop rounds still wears it

***sound** · confidence **high** · Simulation rules · from Slice 27c*

**The choice.** A rifle squad fires at enemies behind a wooden fence. `fence` and `crate` have `stops_rounds: false` (`fixtures/props/generic/obstacles.json`, user decision). They keep light cover (which widens spread against a soldier behind one), still block movement and pushes, and stay destroyable. Rounds, lines of fire and blast pass them, so a fence does not mask a fighting position behind it. A round passing through such a body with hp still damages it: flight reports `FlightEvent::Pass { projectile, prop, point, time }`, found by `WorldGeometry::passes` (once per body per round), and damage applies `structural_damage × armor` while the round flies on. So an HE shell knocks a fence panel down and continues to the house, and rifles with tiny structural damage chip it. The alternative was fences and crates as round-stopping walls.

**The gap.** Only the user's "a wooden panel doesn't stop a bullet".

**The reach.** Soft cover is concealment-plus-spread, not a wall. Any future "soft" body uses the same flag.

**Verdict.** sound.

### Facade aim targets only bodies that can be destroyed; garrisonable is a body property

***sound** · confidence **high** · Simulation rules · from Sim lane (review fixes, props catalog)*

**The choice.** A squad fires at an enemy inside a building. If the aim point is inside a body that fire can destroy (it has `hp`), the shot aims at the body's near face (facade aim). A ruin has no `hp`, so a point inside a ruin is aimed at directly. Widening the rule to "any body that stops rounds" was tried and moved digests. Whether soldiers can garrison a body is its own body column, `garrison: true` on the building row; the garrison code, the village scenario and the scripts read it. The alternative was keying both on the building kind.

**The gap.** The review asked for facade aim and garrisoning to key on physical properties rather than the building kind.

**The reach.** Any destructible prop draws facade aim. Any prop marked `garrison` can be occupied.

**Verdict.** sound. It keys on physical properties, not names.

### Structural damage per weapon: HE and ATGM break things, the HMG and rifles grind slowly

***sound** · confidence **high** · Simulation rules · from Slice 34c; Slice 27c*

**The choice.** The village weapons (`fixtures/village.json` `weapons`) set `structural_damage`: tank HE 100, ATGM 100 (a warhead wrecks structures like a shell), tank AP 10, grenade 10, HMG 4 per round, rifle 0.25 per round. Trunk hp is 100, so one HE shell or one missile fells a tree. Sustained HMG fire fells one in about 10 s, and a squad's rifles in about 19–22 s at 30 m. Each is pinned by a test within ±30%. The user set these: a rifle value of 0.5 read as "mowing down trees". The alternative was rifles and HMGs with no structural damage.

**The gap.** Rifles and HMGs had no structural damage.

**The reach.** With unlimited ammunition, MGs and rifles now fire into and wear any breakable non-occluder on their line: trunks, sandbags, wrecks. Cover wearing away is the user's stated goal.

**Verdict.** sound (user decisions).

### A missile's guide is the soldier who fired it; his fall releases it

***sound** · confidence **high** · Simulation rules · from Post-close review (sim)*

**The choice.** An AT team's gunner launches a missile at a tank and is shot while it flies. The missile is released: it coasts `guided.release_coast_s` (0.5 s) and goes to ground, as when its launcher moves or loses sight (see the coast entries). The missile's support record (`Support`, what guidance checks each tick) names its guide, `Support.operator` (his `Soldier.id`; `None` for a hull's missile, whose crew guides while the hull lives), and it enters the mount digest. Guidance runs before damage in a tick, so the release comes on the tick after his fall, as the launcher's own death already did. Before, the missile flew on guided while anyone in his team lived. Rejected: releasing when the launcher passes to the next soldier, which is the same moment, stated less directly.

**The gap.** The user's guidance rule named the launcher. For a squad weapon the launcher is a man, and which man was open.

**The reach.** A named digest change. Any future man-portable guided weapon inherits it.

**Verdict.** sound. It is the user's guidance rule, applied to the man holding the launcher.

### The village referee counts a unit as a fighting force when its type carries a weapon

***sound** · confidence **high** · Simulation rules · from Post-close review (sim)*

**The choice.** The village's referee decides the scripted battle's result from the attacker's combat units: the village is captured when one of them holds the objective zone, with no defender alive in it, for the hold time, and the attack is defeated when none is left. A combat unit is one whose type carries a mount (`Catalog::mounts` is non-empty). So a supply truck parked in the zone captures nothing, and an attacker left with only the truck has lost. Rejected: "has no supply capability", which would count a future unarmed scout car as a fighting force. No shipped digest moves, since the supply truck is the only type without a mount.

**The gap.** The referee had a hard-coded idea of what counts. The finding asked for it from data, and which data was open.

**The reach.** A new unarmed type counts as non-combat with no change.

**Verdict.** sound. Carrying a weapon is what fighting means.

### Sight eases from front to side to rear as side·sin² + end·cos²

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from Slice 04*

**The choice.** A tank sees its full range ahead (`front`), half abeam (`side`), 30 % astern (`rear`). In between, the reach multiplier at angle θ off forward is `side·sin²θ + front·cos²θ` in the front half and `side·sin²θ + rear·cos²θ` in the back half (`sight::multiplier`). The curve is smooth (flat at 0°, 90° and 180°, so no seam behind the vehicle) and never rises away from the front when `front ≥ side ≥ rear`, which the target cull relies on. The alternative, linear in angle, has corners at 90°.

**The gap.** Delegated: how the side band is interpolated.

**The reach.** Spotting, the fog sweep, the published sight, and the renderer's fog, which mirror this formula.

**Verdict.** sound.

### Each tick's facing is snapshotted before fire, and is part of the battle digest

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from Slice 04*

**The choice.** A tank's turret turns this tick while it fires. Spotting, the fog sweep and the publication all read `Unit::sight_forward`, a field taken after movement and before sensing each tick (and at setup), so they agree on one direction. It is included in `Battle::digest` (the hash replays compare). The alternative, reading the live turret bearing, would let the three disagree mid-tick.

**The gap.** Where "the bearing snapshot taken before fire" lives.

**The reach.** Digests include it.

**Verdict.** sound.

### The target cull uses the widest reach across the whole target

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from Slice 04*

**The choice.** Before testing a target's sample points one by one, spotting rules out targets that are obviously too far. With directional sight, the reach depends on bearing. Today the cull uses the widest reach over the arc the target covers (`Sight::reach_within`), so it never throws away a target that the per-point check would see.

**The gap.** Which bearing the cull uses.

**The reach.** Spotting matches `sees_point` exactly.

**Verdict.** sound.

### The fog sweep's ray spacing is set by the front reach; each ray stops at its own reach

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from Slice 04*

**The choice.** The simulation's fog sweep casts rays out from each eye. Today the number of rays is chosen for the front reach (the longest), and each ray stops at the reach for its direction. So resolution at the tip is unchanged, and rays behind the vehicle do fewer steps. The alternative, fewer rays for a shorter mean reach, would coarsen the front.

**The gap.** Ray spacing under directional sight wasn't specified.

**The reach.** Sweep cost and accuracy.

**Verdict.** sound.

### A side learns ground marks when its fog sweep sees them, and never forgets them

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from Slice 08*

**The choice.** A shell leaves a crater behind a hill. Red doesn't see it until a scout crests the hill. At each fog sweep (each side, every 6 ticks), every ground cell whose centre lies in a seen 8 m fog cell is copied whole from the authoritative ground layer into that side's `KnownGround` (`sim::ground`, owned by `SideKnowledge`). A learned cell is never forgotten; ground out of sight keeps the marks it had when last seen, even if the battle changes it. The alternative was learning continuously, or forgetting marks over time.

**The gap.** The decision named the fog rule, not when learning happens or what a stale cell holds.

**The reach.** This decides what each side's client is ever sent. Knowledge lags the real ground by at most one sweep, like the fog display.

**Verdict.** sound — hidden information matches fog exactly.

### Learned ground is delivered as "every cell stamped after your cursor", with no journal

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from Slice 08*

**The choice.** The client last saw revision 40 of blue's ground knowledge and needs what changed since. Each change is stamped with the side's knowledge revision. Each 8 m tile keeps a `u32` stamp per cell and the newest stamp in the tile, so answering any cursor skips unchanged tiles whole (`KnownGround::changes_since`). Learning also skips a fog cell whose tiles the ground layer has not edited since the side last learned there (a per-fog-cell edit counter); learning without it gives the same result, so it is a speed-up, not state. Memory is about 8 B per map cell per side if everything were learned, plus 8 B per fog cell (reported as `Load.known_ground_bytes`). The alternative was an append-only journal of changes.

**The gap.** Delegated: storage for "only changed cells".

**The reach.** Any consumer at any cursor is answered from state alone; no history to trim.

**Verdict.** sound.

### The fire-through rule reads the true world, like the trajectory solver

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from Slice 27c*

**The choice.** A body the firing side hasn't learned about already blocks its shot, because `solve_launch` reads the world, not the side's plan. `fires_into` reads that same body's row and hp from the world. A side never learns a prop by holding fire at it or firing into it. The alternative, judging a blocker from the side's knowledge, would disagree with the solver that found it.

**The gap.** Unstated.

**The reach.** A gun can know a hidden body's hp implicitly through its decision to fire. That leak is invisible to the player.

**Verdict.** sound. It is consistent with the solver.

### Observers identify on alternate ticks; knowledge still updates every tick

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from decisions.md 27 perf (named decisions), decision 3*

**The choice.** Line-of-sight identification (`sensing::evaluate`) runs for a given observer only when `(tick + id)` is even (`sensing::SENSE_EVERY` = 2, `sensing::due`), so half the observers run each tick. The others keep their last sightings, minus a fallen observer, a fallen target, or soldiers who have since fallen. The side's knowledge still folds the combined list every tick. Track positions (read from the target's true pose that tick), `last_seen`, the acquisition grace and lost-sight contacts stay per tick. Only gaining or losing sight lags, by at most one tick (50 ms). The kept sightings are battle state and enter `Battle::digest`. This saved about 7% of an endurance run.

**The gap.** A performance cut that changes outcomes, named and approved by the user.

**The reach.** Any rule that needs same-tick sight must read the kept sightings knowing they may be one tick old.

**Verdict.** sound. The user approved it.

### A vehicle's sight turns with the mount named in `sensors.on`, or with its hull when none is named

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from Slice 04; Slice 27f*

**The choice.** A tank has two turreted mounts, the cannon and a roof HMG, each with its own bearing. Its optics look where the cannon points, because the tank type's `sensors.on` names `"cannon"`; the HMG never steers sight. A type with no `on` (the truck, the jeep, infantry) looks along its hull or heading (`sight::forward`). The alternative, "the first turret mount", depends on mount order and would also have turned the jeep's sight with its pedestal HMG (a turret mount too), moving every digest.

**The gap.** A rule by type ("tanks look along the turret") had to become data, and the obvious generic rule was wrong for the jeep.

**The reach.** Any vehicle can tie its sight to any mount, or to none, by naming it in data.

**Verdict.** sound — a first-principles field, not a rule by type.

### A side learns a destruction or a knocked tree only by seeing the spot, or by touching the body itself

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from Slice 34b; Slice 34c*

**The choice.** Blue shells a sandbag wall that red can't see; a blue tank knocks down a tree in a wood red isn't watching. A side learns a body is gone only by contact or by sight. The pusher's own side forgets a tree it knocked at once, because it touched it. Fire has no "toucher", so no side learns a destruction by contact. Every other side that planned with the body keeps it standing in its plans and in what it draws (`SideGeometry::standing`) until its fog sees the footprint; then it learns the remains like any new body. A map body seen destroyed with nothing left in its place publishes as a `KnownProp` with `destroyed: true`, which only removes its map prop; a body destroyed unseen publishes from `standing`; a felled or knocked tree shows as its cleared ground once seen. A ruin that closes an authored building's footprint is the exception every side plans with (see the known-remains entry). The digest carries each side's standing set, and the world's cleared-ground mask. The alternative was revealing destruction to everyone at once.

**The gap.** The planning knowledge rule (L1) did not cover knocked trees or destruction by fire, and the seam's `replaces` had no form for "nothing in its place".

**The reach.** Fog of war holds for forest lanes and shelled cover. Pathing on stale knowledge can route through a gone wall, or round one that is already gone.

**Verdict.** sound — it follows the knowledge rule.

### Only buildings, ruins and walls block sight; everything else is seen through

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from Slice 34*

**The choice.** A blue scout looks across a field at a red squad behind a wrecked truck and some crates. Sensing's line of sight is `world::sight_clear`, which meets only bodies whose catalog row sets `occludes`: building, ruin and wall. Trunks, crates, fences, sandbags and wrecks don't cut a sighting ray, so the scout sees the squad; a forest hides through its foliage instead. Rounds, blasts, fire lines and cover's line checks use `raycast`/`segment_clear`, which meet only `stops_rounds` bodies. The fog's 8 m occlusion grid samples cell centres: a wall thinner than a cell that covers no cell centre hides nothing there. There is no low field-wall type; low walls in scenarios are sandbags. The alternative, every solid body blocking sight, would make every crate a sight shadow.

**The gap.** Q25 said "only big static bodies occlude", not which.

**The reach.** Two separate ray families (sight vs rounds) that future bodies must fill in both columns for.

**Verdict.** sound.

### A forest is a rectangle with a density; foliage blocks sight exponentially

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from Slice 34b*

**The choice.** A map forest is `{rect, density, canopy_height_m, trunk_radius_m, trunk_height_m, trunk_clearance_m}`. The fixture's `forests.densities.<name>` holds `trunk_spacing_m, trunk_jitter, concealment_infantry, concealment_vehicle, attenuation_per_m, canopy_radius_m` (`contract::scenario::ForestDensity`). A trunk's body row has `conceals` (0.7). A scout looking into a wood sees to `range · exp(−depth)`, where depth is the foliage he looks through, and optical depth `sensors.foliage_full_block` (1.0) blocks a ground ray outright. Nothing else about a forest changes sight: there are no edge ramps or forest range multipliers. `WorldView.foliage()` exports the grid for the renderer's fog. The alternative was a forest as an area with its own sight rules.

**The gap.** The seam named density fields, not their units.

**The reach.** Forests became bodies; sight, fog and concealment read one grid.

**Verdict.** sound.

### A side's sight blockers are the map's blocking props, minus those it saw fall, plus any it learned

***sound** · confidence **high** · Sight, sensing and the simulation's fog · from Slice 14*

**The choice.** A house blocks sight. Red shells it into a ruin. Blue's drawn fog should keep treating the house as whole until blue actually sees it fall. The list of sight blockers the fog traces against (`knownOccluders`, `frame/fogInputs.ts`) is: the static map's props whose type blocks sight, minus any the side has learned was replaced (a known prop's `replaces`), plus every known prop that blocks sight (a wreck or ruin the side has seen). Which types block sight comes from the simulation, not a TypeScript copy: `world_layout()` exports `occludingPropKinds`, built from each catalog prop type's `body.occludes` (`crates/sim/src/world/export.rs`). The list is rebuilt only when the side's known props change. The alternative, reading the true world, would leak information (a ruin blue never saw would open blue's fog).

**The gap.** The plan said only that known props update the blocker list "the way `known_props` does", and TypeScript had no owner of the "which types block sight" rule.

**The reach.** A wreck or ruin the side has not learned never blocks its fog (pinned by a test in `web/tests/fog.test.ts`). New prop types get their sight-blocking from their catalog row automatically.

**Verdict.** sound — drawn fog follows knowledge, and the rule has one owner in the simulation.

### Soldiers are individual bodies whose squad position is their centroid

***sound** · confidence **high** · Movement, cover and pushing · from Slice 31*

**The choice.** There is no squad formation. Each `Soldier` has his own `position` (x, y, z), `velocity`, `spot` (his place where the current move ends) and `leg` (the route waypoint he walks toward), plus later per-soldier movement state. All of it enters `Battle::digest`. `Unit.position` for a squad is the living soldiers' centroid, recomputed by `Unit::settle` whenever soldiers move, sit, leave, fall or join. A garrisoned squad's centroid is its seated slots' middle, not the building centre. The alternative kept a squad centre and computed soldier positions as fixed offsets from it.

**The gap.** The slice named the fields, not where per-order state lives.

**The reach.** Everything that reads a squad's position (sight, targeting, publication) now reads a derived centroid.

**Verdict.** sound — per-soldier truth, one derivation for the squad.

### A squad ends each move in a fresh seeded random arrangement around its goal

***sound** · confidence **high** · Movement, cover and pushing · from Slice 31*

**The choice.** You send a rifle squad to a crossroads. `sim::arrangement` (the one owner of where soldiers stand) draws one spot per living soldier, uniformly over a disc of diameter `infantry_movement.spread_m` (12) × √(squad size / `spread_squad_size` (8)), so every squad gets the same ground per soldier (12 m for a rifle squad, about 8.5 m for recon). Each spot is at least `spacing_m` (2) from earlier ones (24 tries, else the farthest). The draw is then recentred on the goal so the squad's middle lands where it was sent. Any spot that isn't standing room for a 0.3 m disc, or not reachable in a straight line from the goal, moves to the nearest one that is. If 2 m spacing won't fit, it retries at 1 m; failing that, everyone heads for the goal. The draws come from `arrangement::rng(seed, unit, tick)`, a stateless mix: the same spots in a replay, new ones for every order, no new random state to digest. For orders, solids are the side's known props (hidden props never shape an order); for placements, the true world. Then take-cover may swap each spot for the best cover nearby. Starting squads spawn the same way (tick 0).

**The gap.** The spec deleted the formation without defining where soldiers stand.

**The reach.** Squads look like loose groups, never blocks; replays stay exact.

**Verdict.** sound.

### Soldiers collide as 0.3 m discs with props and hulls, sliding along faces

***sound** · confidence **high** · Movement, cover and pushing · from Slice 31*

**The choice.** Each soldier is a disc of `physics.soldier_radius_m` (0.3 m). Each step he is pushed out of every prop that blocks infantry (walls, crates, wrecks) and every live vehicle hull of either side, sliding along the face he meets (`Obb2::push_out`), never into one. A soldier may always step out of a solid he already stands in; the same rule holds for `arrangement::reachable`, so a garrisoned soldier whose slot lies inside a building's clearance can escape a collapse. Soldiers also keep apart from each other. The alternative, collision only at the squad's centre, let soldiers walk through walls and hulls.

**The gap.** The body rules were scheduled for later slices, but without them soldiers stood inside hulls.

**The reach.** All infantry movement, cover and pushing build on this.

**Verdict.** sound.

### A soldier on the move steers for a point a few metres up his own lane beside the squad's route

***sound** · confidence **high** · Movement, cover and pushing · from Slice 32*

**The choice.** A squad ordered across a field gets one planned route, the *corridor* (a polyline of waypoints). No soldier walks the corridor line itself. Each soldier keeps his own *lane*: the corridor shifted sideways by his *lane offset* (`Soldier.lateral`). Every tick he finds the corridor leg he is on (`Soldier.leg`), projects himself onto it, and heads for the point `infantry_movement.steer_ahead_m` (3 m) further along, shifted by his offset. The offset he wants is where his final spot sits across the corridor's last leg (so the lane turns when the corridor turns), plus a seeded sine *wander* (`wander_m` 0.7 m, period `wander_period_s` 9 s stretched 0.75–1.25× per soldier, phase seeded from battle seed, squad and soldier). His actual offset creeps toward that at `lane_shift_mps` (1.5 m/s), so he never jumps sideways. The alternative, everyone following one line or fixed formation slots, gives a single-file conga line. Code: `movement::soldier::soldier_steer`.

**The gap.** The spec delegated the steering formulation.

**The reach.** Every infantry movement feature (cover seeking, obstacle avoidance, future per-soldier behaviours) builds on "one shared corridor, individual lanes". It keeps squads as loose groups of individuals, matching the per-soldier presentation the user wants.

**Verdict.** sound — the standard "carrot on a path" follower, made per-soldier and deterministic.

### A soldier cut off from his lane heads for the furthest visible corridor point, and plans his own way back only if none is visible

***sound** · confidence **high** · Movement, cover and pushing · from Slice 32*

**The choice.** A soldier whose lane point is behind a wall first heads for the furthest corridor point ahead he can walk to in a straight line (3 m, 2 m, 1 m ahead, then his own projection). Only if none is clear does he run his own small route search (`final_leg`, below) to the first corridor point ahead that is *standing room* (a spot his disc fits), within half his search window, else to his own spot. He runs that search at most once a second. Standing room matters because the corridor can run through a parked vehicle: vehicles are not in the route grid.

**The gap.** The spec gave no rule for stragglers who lose their lane.

**The reach.** Bounds per-soldier search cost; stragglers left behind a wall rejoin without the whole squad replanning.

**Verdict.** sound — cheap straight-line checks first, a real search only when needed, rate-limited.

### The last stretch to a soldier's spot is his own fine route, reused until his side learns something that blocks it

***sound** · confidence **high** · Movement, cover and pushing · from Slice 32*

**The choice.** Near the end of a move, when the corridor has `final_leg_m` (12 m) or less left, or his spot is within 6 m, and his spot is inside his search reach (`window_m` 40 m halved, less 2 m), a soldier stops following his lane and plans a personal route straight to his spot. That route is kept until his side's knowledge changes *and* a known body now crosses what is left of it, or his spot changes. The alternative, replanning whenever knowledge changes at all, wastes searches and makes soldiers twitch.

**The gap.** The spec said "replan only when the route no longer fits" (Q13) without saying when the personal final stretch starts.

**The reach.** Arrival precision around cover and doorways rests on this.

**Verdict.** sound — replan-on-invalidation, the general rule.

### A soldier's own route is A* on a half-metre grid over the exact boxes, string-pulled

***sound** · confidence **high** · Movement, cover and pushing · from Slice 32*

**The choice.** `movement::final_leg` lays a 0.5 m grid over a square window (40 m) centred between the soldier and his target. A cell is open when his disc at its centre meets no known box and the ground is walkable; both end cells count as open, and a solid he already stands in is ignored. A* finds a path, then it is string-pulled (shortcuts taken wherever a straight line is clear) against the exact boxes. Consequence of the grid: a gap is sure to pass from 1.1 m wide, may pass from 0.6 m depending on alignment (a test pins 1.2 m threads, 0.5 m does not).

**The gap.** The spec did not name the fine-path algorithm.

**The reach.** Sets the narrowest gap infantry reliably use; any future "squeeze through" behaviour starts here.

**Verdict.** sound — standard, bounded, deterministic.

### Soldiers keep personal space softly, and never finish a step inside another soldier

***sound** · confidence **high** · Movement, cover and pushing · from Slice 32*

**The choice.** Two squads crossing paths: anyone of either side within `personal_space_m` (0.8 m) pushes a soldier's velocity away, weighted by how deep inside, plus half as much to his right when the other man stands ahead, so head-on pairs pass instead of stopping. As a hard rule, a step never ends within two radii of another standing soldier unless it takes him further out; if pushing him out of other discs would put him in a body, he stays put. Soldiers are updated in place in unit and member order (deterministic), with neighbours from 2 m buckets. Idle soldiers do not make way for walking ones; they only get out of vehicles' way.

**The gap.** Q10 asked for personal space, not its form.

**The reach.** Crowds at doorways and gaps behave as bodies without a crowd solver.

**Verdict.** sound — simple, deterministic, and the pass-on-the-right bias fixes head-on deadlocks in general.

### Each soldier's pace swings and each starts a fraction of a second apart, seeded per order

***sound** · confidence **high** · Movement, cover and pushing · from Slice 32*

**The choice.** On an order, each soldier's speed swings between `1 − pace_variation` (0.88) and full speed over two thirds of his wander period, from a seeded phase (`Soldier.pace`). Start delays are uniform up to `stagger_s` (0.8 s), less the smallest, so the first man leaves on the order's own tick. That matters because weapons that must notice a unit moving (a guided missile's operator, a stationary weapon losing aim) see it at once. A fixed per-soldier pace was the alternative; it strung a squad out 12 m over 100 m.

**The gap.** The spec asked for variety without a mechanism.

**The reach.** Squads look alive without anyone falling steadily behind; the average squad is about 6% below the rule speed.

**Verdict.** sound.

### A soldier arrives on his spot, or takes where he stands if someone blocks it; the squad arrives when all have

***sound** · confidence **high** · Movement, cover and pushing · from Slice 32*

**The choice.** Within 5 cm of his spot a soldier snaps onto it. Within 1 m and unable to move (someone stands there), he takes where he stands as his spot. The squad has arrived when every living soldier is on his spot. The *stall watch* (the check that forces a replan when a squad makes no progress) sums every living soldier's distance to his spot, including men not yet set off, so staggered starters don't read as a stall.

**The gap.** Arrival and stall for per-soldier movement were unspecified.

**The reach.** Order completion, cover re-resolve on arrival and UI "arrived" states all key off this.

**Verdict.** sound.

### Vehicles of both sides collide box against box; a unit's published blocker names only a friendly unit

***sound** · confidence **high** · Movement, cover and pushing · from Slice 34*

**The choice.** Two tanks of opposing sides meet head-on on a road. Every vehicle meets every other vehicle's box, whatever the side; the higher-id vehicle detours after a stall. A unit's published `blocker` field names only an own unit: naming an enemy by its internal id would reveal what contact information must not.

**The gap.** Q14 removed the friendly-only rule; the publication leak was unstated.

**The reach.** Fog of war stays intact in the publication.

**Verdict.** sound.

### Tracks pivot on the spot; wheels turn only while rolling

***sound** · confidence **high** · Movement, cover and pushing · from Slice 39*

**The choice.** `movement/drive.rs`: tracks turn at their own rate, stand and pivot beyond 60° of heading error, and slow by the cosine within it (unchanged tank driving). Wheels turn at most `min(1/radius, turn rate / speed)` per metre, slow to half speed at full lock, and never turn when stopped: a blocked truck keeps its yaw, a blocked tank still pivots.

**The gap.** Q29 said tracked and wheeled differ, not the follower.

**The reach.** No kinodynamic planner: the planner's route is followed with vehicle kinematics layered on.

**Verdict.** sound.

### Taking the facing on arrival, and the facing also decides which side of cover the squad uses

***sound** · confidence **high** · Movement, cover and pushing · from Slice 35*

**The choice.** A squad arrives at a wall with a facing east. It turns to that facing at once (`Unit.yaw`). A seen or engaged enemy still overrides the facing when the holding squad re-checks its situation (Q9). A tracked vehicle pivots toward the facing at its turn rate while at rest (`Unit.turn_to`, in the digest; `drive::pivot`). If the turn would push its hull into a solid body or another vehicle, it gives the turn up. The drag facing also sets the order's cover threat: the threat is taken to be far along the facing, not along the way the squad travelled. So the squad lines up on the side of the wall away from where it will face. The alternative was to take the threat from the travel direction, which would put the squad on the wrong side of the wall.

**The gap.** The spec gave the facing but didn't link it to cover or to vehicle turning.

**The reach.** "Face this way" doubles as "expect the enemy this way". Any later threat-direction input has to fit alongside it.

**Verdict.** sound. It is how a player uses a facing in Total War-style games.

### A squad holds an area round a fixed anchor point, set by orders and placement, never by where its soldiers happen to stand

***sound** · confidence **high** · Movement, cover and pushing · from Slice 27d*

**The choice.** A squad arrives at a destination. It gets an anchor, `cover::Anchor { at, halt }` on `Unit.anchor`, and its soldiers look for cover anywhere inside a circle round it. The radius (`cover::area_radius`) is half the spread of the squad at full strength (the dead never shrink it) plus `cover.search_m`. The anchor is set at placement (scenario spawn, leaving a garrison), by each new order's destination (`take_cover::at_order`, including pursuits), and by an attack-move's halt (`take_cover::halt`). A halt that lapses and resumes inside the area it set keeps the old anchor, so a one-tick flicker never walks the squad along. It is never set from the soldiers' own positions, which would let the area drift as men move to cover. It is digested (part of `Battle::digest`, the replay check). The movement code's own planning start is named `set_off`.

**The gap.** The slice said soldiers seek cover "in the area" but did not say what fixes the area.

**The reach.** Everything about where a holding squad fights reads this anchor; it is also published and drawn as the squad's area ring.

**Verdict.** sound — stable, order-driven, deterministic.

### Buildings are heavy cover, but a squad ordered into a building walks straight to the door

***sound** · confidence **high** · Movement, cover and pushing · from Slice 27d*

**The choice.** Building prop types carry `cover_tier: heavy` in the prop catalog (`fixtures/props/`), so squads fight from round a house's faces. A squad ordered to garrison a building, though, would claim cover spots round the house on the way and dawdle there. `take_cover::at_order` offers no cover spots for a garrison order: the squad gathers at the door. A garrison inside keeps its own building shelter rule, unchanged.

**The gap.** Making buildings cover created the dawdling case; the slice didn't cover it.

**The reach.** Any new order kind that ends inside something should likewise skip cover on the way.

**Verdict.** sound.

### A replan toward the same goal keeps each soldier's spot; only a new goal redraws them

***sound** · confidence **high** · Movement, cover and pushing · from Slice 31; Slice 32*

**The choice.** A squad walking to a crossroads replans mid-walk because it stalled, or because its side learned of a new prop. If the goal is the same, each soldier keeps his spot, pace and start time (`keep_spots`); only a kept spot the side now knows lies inside a solid, or a soldier without one (a replacement), takes the nearest free spot. Only a new goal redraws the arrangement, paces and start delays. The alternative, redrawing on every replan, visibly reshuffles the squad mid-move.

**The gap.** The spec did not say when an arrangement is redrawn or whether it survives a replan.

**The reach.** Knowledge changes and replans are invisible to the player; squads don't reshuffle.

**Verdict.** sound.

### Every solid body blocks infantry, tree trunks included; soldiers walk between trunks on the fine grid

***sound** · confidence **high** · Movement, cover and pushing · from Slice 34; Slice 34b; Orchestrator, after slice 34b; Slice 34c*

**The choice.** A squad crosses a medium wood. A body whose catalog row sets `blocks.infantry` stops soldiers' discs, and every solid row sets it, the trunk included (`fixtures/props/generic/nature.json`). Soldiers thread between trunks on the 0.5 m sub-cell route grid and their own fine routes, so gaps smaller than a 2 m cell keep woods open to squads. A trunk gives cover from behind it like any body; it is not a ground body you stand inside. Only heavy vehicles knock trunks over by pushing. Rubble and the bridge deck block nobody. The alternative, trunks soldiers walk through with the forest as area cover, is what the user's rule (Q27: "every solid body blocks infantry") rejected.

**The gap.** Q27 and a later slice's contract contradicted each other; the forest had been modelled as an area.

**The reach.** Forest movement is slower and file-shaped through geometry, not a speed multiplier alone (forest speed also applies), and cover in woods is per trunk.

**Verdict.** sound — the user's rule, first-principles bodies.

### A holding squad re-resolves its cover only for changes within its reach, at most once a second

***sound** · confidence **high** · Movement, cover and pushing · from Slice 33; decisions.md 27 perf (named decisions), decision 1*

**The choice.** A squad holds a hedge. To re-resolve is to re-pick who stands where and who can fire from where. It happens when the squad arrives or an attack-move halts; when its side's planning knowledge changes within reach; when the threat's bearing swings past `cover.swing_deg` (45°); or when a vehicle a soldier hides behind moves more than `cover.vehicle_moved_m` (2 m). "Within reach": every planning change a side makes (a body learned, moved, seen gone, forgotten) is logged as a circle (`SideGeometry`'s change log, `movement::changed_near`), and a holding squad reacts only when a logged change, or a newly learned crater, lies within its area's radius plus `cover.search_slack_m` (2 m) of its anchor. The log keeps the latest 1,024 changes; a squad whose last resolve is older than the log re-resolves anyway. Each squad checks once a second on its own tick of the second, at most once per `cover.reresolve_s` (1 s). A soldier who already holds the best place he could claim stays. A moving squad never re-resolves; its destination spots stand until arrival. So when a tank is destroyed across the map and becomes a wreck, the squad at the hedge does nothing. The alternatives: re-resolving on any change anywhere cost about 16% of heavy fights and made squads shuffle for no visible reason; watching the known-ground revision let cosmetic marks (trampled grass) change combat.

**The gap.** "World change within the cover radius" and "a new crater" had no mechanism; the reach limit is a performance cut that changes outcomes, named and approved by the user.

**The reach.** Cover decisions are local and cost stays bounded on big maps. A far body that opens or closes the squad's firing line is noticed only at the next swing or arrival; future rules that let far bodies matter to a holding squad must add their own trigger.

**Verdict.** sound — the user approved the cut; squads stay still when well placed.

### A soldier who can't fight from his place steps to the nearest reachable place in the squad's area he can fight from, or sits out

***sound** · confidence **high** · Movement, cover and pushing · from Slice 33; Slice 27d*

**The choice.** After cover places are claimed against a seen enemy, one soldier's place gives him no line to any enemy, straight or by a lean. He searches rings every 0.5 m outward: within `cover.step_out_m` (4 m) he takes the best cover, then the nearest place; beyond that, the nearest ring with any place, out to the full width of the squad's area and never outside it. A place must be standing room, `spacing_m` from the others, reachable on foot from where he is (`arrangement::reachable`, which stops a step offering the far side of a wall), and must let him engage straight or from a free lean point there. With none he sits out. The alternatives were no step-out (men hide uselessly) or only the short 4 m reach, which stranded men once squad areas grew.

**The gap.** The planning decision (D3) named the step-out, not its search or reach.

**The reach.** Guarantees a holding squad ends mostly able to fight; works with the squad-area claim rules.

**Verdict.** sound.

### A soldier's steer point stays on his own leg of the route, so he can't twitch at a corner

***sound** · confidence **high** · Movement, cover and pushing · from Slice 37; Slice 37b (sim)*

**The choice.** A squad rounds a fence end. One man walks a "lane", a line offset about 7 m to one side of the squad's route (the corridor). His steer point is 3 m ahead on the leg he is on, never past its end, shifted across that leg (`Corridor::lane_point`). He walks each leg's lane to its end, then turns, and counts a leg as passed when his projection is within 0.3 m of its end. The lane check (`lane_offset`, looking `lane_lookahead_m` ahead) follows the lane as he walks it (`Corridor::lane`), so it doesn't cut corners. The unbuilt alternative, steering at the corridor point 3 m ahead shifted across whichever leg that point lay on, turns the shift with the next leg once past a waypoint and puts his target behind him, so he steps back, and the cycle repeats (490 reversals in 40 s in the test). Two other alternatives were rejected: a blend across the corner still moves backward on sharp turns, and a rule that "a lane may turn only in plain sight of the waypoint" funnelled a squad through one gap of a tooth line instead of a gap each. The known cost is a small bulge on the inside of a turn; at a fence end, the two widest-lane men can also walk into a pocket and back for about 3 s.

**The gap.** The lane rule didn't say what a lane does at a corner.

**The reach.** Every squad route uses this. The no-twitch check (below) guards it.

**Verdict.** sound. With one clamp, the target only ever moves forward.

### Remains that stopped a mover are known to every side

***sound** · confidence **high** · Movement, cover and pushing · from Sim lane (review fixes, props catalog)*

**The choice.** A house is shelled into a ruin. Both sides' route planners treated the house as a known obstacle, so both treat the ruin that replaces it as known too (`Prop::known_to_all`): remains that close an authored body's footprint are planned with by everyone, with no need to see them first. Rubble blocks nobody, and wrecks are never authored, so neither is affected. Today this covers exactly the ruin. The alternative was a ruin special case in code, or hiding the ruin from sides that did not see the collapse.

**The gap.** Which remains every side knows needed a general rule instead of a ruin special case.

**The reach.** Any future authored body whose remains still block inherits this.

**Verdict.** sound.

### Soldiers placed outside an order use the same arrangement rules

***sound** · confidence **high** · Movement, cover and pushing · from Slice 31; Post-close review (sim)*

**The choice.** A squad leaving a building stands in an arrangement round the exit nearest its heading (`garrison::exit_spots`, each spot checked with the squad's 0.5 m path clearance). Survivors of a collapsing building scramble out to wherever one soldier's body fits (`physics.soldier_radius_m`, 0.3 m) and stay there (no regrouping). An orderly exit keeps the squad's path clearance on purpose: the squad marches on from there, so its spots must be where its planned routes can start. A seated soldier's position is his slot. A replacement joins at the free spot nearest the squad's middle, `spacing_m` from squadmates and reachable (`arrangement::nearest_free`). Flight collision sweeps each soldier's own movement for the tick. There are no formation helpers: every placement goes through `sim::arrangement`. The alternative was keeping formation slots for non-order placements.

**The gap.** How non-order placements work without a formation.

**The reach.** One owner of standing spots for every case.

**Verdict.** sound.

### The sight shape is a contract type; the simulation owns every rule about it

***sound** · confidence **high** · Contracts and seams · from Slice 04*

**The choice.** `SightShape {front, side, rear}` and the published `UnitSight` live in `contract` (the crate of shared data types for fixtures and observations). Every function of them — the multiplier, a unit's reach, where it looks, the snapshot, validation — is in `sim::sight`. The alternative, the struct and its functions together in `sim`, would leave fixture parsing depending on the simulation.

**The gap.** The seam named `SightShape` under `sim::sight`, but data types live in `contract`.

**The reach.** Where future sight rules go.

**Verdict.** sound.

### Buildings and ruins are their own frame input, built from what the side knows

***sound** · confidence **high** · Contracts and seams · from Slice 12*

**The choice.** A player sees a house that fell out of sight as it last stood. Today "structures" is a separate input to the frame, `BattleFrame.setStructures(instances)`, which a route fills from its side's knowledge (standing buildings, remembered ruins and wrecks) and which is drawn as models. The alternative, bundling structures into the overlay or the static world, would mix knowledge with the map.

**The gap.** The seam named "a structures layer" without its shape.

**The reach.** Every route that shows buildings.

**Verdict.** sound.

### The publication's layout is per battle and names its round kinds

***sound** · confidence **high** · Contracts and seams · from Slice 05*

**The choice.** A round kind is an index into the rules' weapon rows in name order, and the layout (the description of the packed observation's fields) lists those names as `roundKinds`. So the layout depends on the battle's fixture: `Battle.observation_layout()` replaces a free export, and the worker's `ready` message and every decoder test read it from their battle. The alternative, a fixed kind enum in code, couldn't name weapon rows that live in data.

**The gap.** A static layout couldn't name fixture-defined kinds.

**The reach.** Every observation reader reads kinds by name from the layout.

**Verdict.** sound.

### A tracer names the soldier who fired it, never a vehicle crewman

***sound** · confidence **high** · Contracts and seams · from Slice 05*

**The choice.** `VisibleSegment.shooter_member` is the soldier id when the round's firing body is a soldier; a single-operator infantry weapon (grenade launcher, ATGM) names its operator; a vehicle's gun gives none. The alternative, naming a crew slot, has no soldier to point to.

**The gap.** How to name a vehicle's shooter.

**The reach.** Flashes land on the right soldier.

**Verdict.** sound.

### A tracer is a polyline with its ricochet points

***sound** · confidence **high** · Contracts and seams · from Slice 06; Slice 05*

**The choice.** `VisibleSegment {path, ricochets: [{point, normal}], own, kind, shooter_member, hit, impact_normal}` replaces a from/to line: `path` bends where the round ricocheted, and `hit` (`none|ground|hull|prop|soldier`) says what ended it, with a normal only when there was a hit. Packed, a row is `pointCount, ricochetCount, own, kind, shooterLo/Hi, hit, nx, ny, nz` plus path and ricochet sections; a straight round costs 16 floats. Enemy polylines are clipped leg by leg with the 8-sample fog rule: only pieces over seen ground are sent, a seen ricochet stays a corner, and the hit rides the last piece only when its end is seen.

**The gap.** "A polyline per round per tick" without a layout.

**The reach.** Decoder, tracers, sparks, sound.

**Verdict.** sound.

### Camera functions write into a caller's output, and hot paths allocate nothing per frame

***sound** · confidence **high** · Contracts and seams · from Slice 28*

**The choice.** Camera functions take the output first: `eyePosition(out, p)`, `viewMatrix(out, p)`, `projMatrix`, `viewProjMatrix`, `invViewProj`, `screenRay(out, p, x, y)`, `unprojectToPlaneZ(out, …): boolean`, `cameraUniformData(out, camera)`, `receiverRange(out, camera, box)`. `projectPoint(out, viewProj, world)` takes a prebuilt matrix, and `screenRayFrom(out, inverse, eye, x, y)` serves callers casting many rays. `rayInstanceDistance` returns `Infinity` on a miss. `frameCamera` returns scratch its next call rewrites. The per-frame camera publication, receiver range, cascade scratch and instance packing allocate nothing; instance packing keeps a staging array per kind. Still allocating on purpose: `liveCamera`'s object spread, the `CascadeFit` records, the controller's new camera object per moving frame (object identity tells the viewport to redraw), and overlay mesh builders.

**The gap.** "No allocation per frame" without saying how far to restructure.

**The reach.** Callers must not keep returned scratch.

**Verdict.** sound.

### The renderer's fog is handed the side's eyes and the world's sight blockers, not a precomputed visibility grid

***sound** · confidence **high** · Contracts and seams · from Slice 14*

**The choice.** A blue recon squad stands at a street corner. Every time the simulation publishes blue's view, the renderer needs to know which pixels on screen blue can see. Today the frame gets `BattleFrame.setFog(FogInput | null)`, where `FogInput = { world: FogWorld, sight: FogSight }` (`frame/fogInputs.ts`). `FogWorld` is the static map that sight is cut by: the simulation's terrain height grid, its forests, and the three sensor rules the fog shares with the simulation's own sight sweep (`fog_target_height_m`, `forest_attenuation_m`, `forest_full_block_m`). `FogSight` changes with each publication: one `FogEye` (a position, facing, range and sight-shape) per eye the side's units publish (`OwnUnit.sight`), plus the sight blockers the side knows (`knownOccluders`). The GPU then traces sight per pixel from those eyes. The unbuilt alternative was to keep drawing the simulation's coarse 8 m seen/unseen grid (the observation's visibility field) as the fog; that grid still travels in the observation, but only the fog lab's agreement check reads it.

**The gap.** The plan named the fog's inputs but not their shape.

**The reach.** Every route passes `session.fog` from `useBattleSession`; the fog depends on the published eyes matching the simulation's own eyes (which the garrison per-facade eye rule relies on). Any future sight rule must be mirrored in these inputs or the drawn fog and the simulation disagree.

**Verdict.** sound — the fog is traced from the same eyes and rules the simulation uses, so it is sharp and agrees with it.

### Scenarios can burst a weapon's round on the ground with no shot, to lay crater fields for labs and tests

***sound** · confidence **high** · Contracts and seams · from Slice 07*

**The choice.** A test needs a crater field without minutes of shelling. Today a scenario event `burst {point, weapon}` (`EventAction::Burst`, TS `LabEvent`) marks the ground exactly as that weapon's real burst would, but flies nothing, hurts nobody and is not published. Setup rejects an unknown weapon row. Replays pin it through the scenario digest.

**The gap.** Labs and tests need craters on demand.

**The reach.** Part of the scenario schema; a future authored battle could use it for pre-cratered ground.

**Verdict.** sound.

### The digest covers each side's learned ground and revision, but not the delivery stamps

***sound** · confidence **high** · Contracts and seams · from Slice 08*

**The choice.** `Battle::digest` (the hash that proves two runs are identical) folds each side's learned cells (as tile hashes, like the ground layer) and its knowledge revision. It leaves out the per-cell stamps and the per-fog-cell edit counters: stamps only shape patches, whose replay parity a test pins record for record, and the counters change no result. The alternative was digesting everything.

**The gap.** "Learned cells are digested" without saying what else.

**The reach.** Changing how patches are shaped does not move digests.

**Verdict.** sound.

### Ground patches trail the publication record, four floats per cell

***sound** · confidence **high** · Contracts and seams · from Slice 08*

**The choice.** Each publication (the per-tick record the simulation sends the client) carries a ground patch after the fog bitset. Each cell is four floats (16 B): the row-major cell index as two 16-bit halves (`cellLo`, `cellHi`), `craterScorch` = crater + scorch × 256, and `tracksTrampledCleared` = tracks + trampled × 256 + cleared × 65536 (`sim::publication`). The header gains `groundEpoch`, `groundSide`, `groundBase`, `groundRevision`, `groundFull` and `groundCellCount`; the grid (`cellM`, `cols`, `rows`) is in the layout. The rejected alternative was tile-grouped packing (12 B a cell), which needs a second decoder shape.

**The gap.** Delegated: the patch's packing.

**The reach.** Adding a ground mark means packing it into one of these floats (below 2^24) or widening the record.

**Verdict.** sound — one simple decoder.

### The delivery cursor lives in the publisher; epochs restart the stream with a full snapshot

***sound** · confidence **high** · Contracts and seams · from Slice 08*

**The choice.** The player switches from blue to red. `sim::publication::Publisher` (held by the worker's battle handle) keeps the cursor. Its first record, a side change, and `resync_ground()` open a new epoch (1, 2, …) with a full snapshot. The worker's `side` request always resyncs, even to the same side, and the client first invalidates its view, so any in-flight patches from the old epoch are dropped as stale. The alternative was the client asking for what it lacks.

**The gap.** The contract named the fields, not who counts epochs.

**The reach.** Side switches and reloads are always a clean restart; no partial state crosses an epoch.

**Verdict.** sound.

### The publication lists bodies a side believes moved, and the renderer draws movable props from knowledge

***sound** · confidence **high** · Contracts and seams · from Slice 34*

**The choice.** A side's `known_props` also lists each authored body it places elsewhere, with `replaces` naming the original id. The renderer draws every movable kind (the layout's `movablePropKinds`, rows not immovable) apart from the static world mesh, from the side's knowledge (`apartKinds`), so a shoved crate moves in the side's picture and an unseen shove leaves it standing. Layout kind names are the fixture's ids (`bridge_deck`, `tank_wreck`). Grass does not yet clear under a shoved body or a wreck.

**The gap.** How moved bodies reach the drawing was unspecified.

**The reach.** Every future movable/destroyable prop is drawn from knowledge by the same path.

**Verdict.** sound.

### Sound hears only what the publication and drawn poses carry, heard at the camera

***sound** · confidence **high** · Contracts and seams · from Slice 40*

**The choice.** Per decoded observation, sound takes the same `EffectPublication` the effects take plus `o.audible` (hearing cues). Per animation frame it takes the pose driver's `PoseFrame` (vehicle positions, track travel, turret yaw; soldier positions) and the presentation clock, heard at the camera the viewport draws with (`session.hear(camera)`). Nothing reads simulation state or an unseen unit.

**The gap.** The seam for audio input was unspecified.

**The reach.** Audio cannot leak fog of war; sound is purely a client presentation layer.

**Verdict.** sound.

### A move order carries a direction: forward or reverse

***sound** · confidence **high** · Contracts and seams · from Slice 39*

**The choice.** `Order::Move.direction` and `MoveOrder::direction` (`MoveDirection`, default `forward`, so old scripts and replays read as before). Attack-moves are always forward; an upgrade of a move keeps its direction. A reverse move steers the tail along the route at the reverse fraction; on a straight route the facing never changes. Infantry ignore it.

**The gap.** Q31 asked for reverse without the command shape.

**The reach.** A command-schema field every client and replay carries.

**Verdict.** sound.

### Own and seen enemy vehicles publish whether they are reversing

***sound** · confidence **high** · Contracts and seams · from Slice 39; Orchestrator, after slice 39*

**The choice.** Own units publish `direction` (null without a move) and `reversing` (driving backwards this tick: a reverse move or a turn's reversing leg); the numeric layout carries `direction` (an index into a `directions` list, −1 when absent) and `reversing` (0/1). Identified enemy vehicles also publish `reversing` (`IdentifiedUnit.reversing`), since backing up is as visible as their position; so their reverse whine plays. Unseen enemies stay hearing cues only.

**The gap.** Which side of the fog the flag belongs on.

**The reach.** Observation schema fields; the audio reads them.

**Verdict.** sound — leaks nothing the position doesn't.

### Cleared ground is published in the existing mark float and drawn as full track wear

***sound** · confidence **high** · Contracts and seams · from Slice 34b*

**The choice.** A ground cell's second mark float is `tracksTrampledCleared` = `tracks + trampled·256 + cleared·65536`, exact in f32. The client keeps `GroundView.cleared` and folds a cleared cell into full track wear in the scar texture, with no shader change. The ground layer's cell grows to five bytes.

**The gap.** How the new channel reaches the client.

**The reach.** Packed publication field; one more packed channel won't fit f32 exactly.

**Verdict.** sound.

### A move order can carry a facing, and a side sees its own units' planned end state, never the enemy's

***sound** · confidence **high** · Contracts and seams · from Slice 35*

**The choice.** The player right-drags to order a squad to a hedge and point it north. The command `Order::Move` carries an optional `facing` (a world bearing in radians). It defaults to none, so older scripts and replays read the same. The sim keeps it on the unit's move order and folds it into the battle digest, the hash that proves two runs are the same battle. The observation is what one side is allowed to know each tick. For each of its own units it now carries `final_facing`, the way the unit will face when it arrives, and `member_orders`, one row per soldier in `members` order. Each row holds his spot (`x, y`), `coverNow` (the cover tier he has where he stands) and `coverThere` (the tier his spot gives). In the published record these are the own field `finalFacing` and a section `memberOrders` sized by `memberCount`. The layout gains a list `coverTiers`, and a tier is an index into it, with −1 for none. The enemy view (`IdentifiedUnit`) gets none of this. A metamorphic test, `nothing_of_the_enemys_plan_reaches_the_other_side`, gives red different orders and checks that blue's records stay bit-identical. The alternative was to publish plans for identified enemies too, which would give away orders the fog should hide.

**The gap.** The spec asked for final markers and per-soldier cover markers, but gave no wire shape and no rule about who sees them.

**The reach.** Every order marker the renderer draws reads these fields. Any later "predict the enemy's move" feature has to be built from what the side sees, not from these fields.

**Verdict.** sound. It is the smallest seam that draws the markers, and the enemy's plan stays out by construction.

### A village spawn row may set its unit's engagement

***sound** · confidence **high** · Contracts and seams · from Slice 37b (sim)*

**The choice.** `fixtures/village.json` `spawn` rows are `[type, x, y]` or `[type, x, y, engagement]`, where engagement is `fire_at_will` or `return_fire_only` (`sim::village::SpawnRow`). Without the column, the side's default holds: red AT teams hold fire, and everyone else fires at will. With it, the row wins, on red too, since a row that says so means it. An unknown value, a short row or a fifth element fails the load. A custom deserializer keeps the error exact. No row sets it today.

**The gap.** The handoff didn't say whether an explicit column overrides red's AT default.

**The reach.** Balance can give a single unit a fire policy without a script.

**Verdict.** sound.

### The observation publishes each soldier's lean and a squad's own area; seen enemies show leans but never their area

***sound** · confidence **high** · Contracts and seams · from Slice 27d*

**The choice.** The renderer needs to draw a soldier leaning and the player's squad area. The observation (what the sim tells one side) gains, for own units, `OwnUnit.member_leans: Vec<Option<MemberLean>>` (per soldier: `LeanSide { Left, Right }` and the lean point) and `OwnUnit.area: Option<SquadArea { anchor, radius }>` (none for a vehicle). Enemy units the side has identified get `IdentifiedUnit.member_leans` too, since you can see a man lean out, but no area: that is the enemy's plan, and publishing it would leak it. The flat publication (the typed-array layout the web reads) carries own `areaX`, `areaY`, `areaM` (NaN for a vehicle) and a `memberLeans` section (side −1 and NaN point while tucked in), with a layout key `leanSides`. The sim state behind it (`Unit.anchor`, `Soldier.lean`, `lean_since`, `leaning_until`, `tucked_until`) is digested. The village scene's plan-leak check covers the area.

**The gap.** The slice did not say what the observation carries, or what an enemy may see.

**The reach.** Any presentation of leans or areas reads these fields; the no-area-for-enemies rule is a fog-of-war boundary.

**Verdict.** sound — shows what is visible, hides what is intent.

### The world geometry is built from the whole rules

***sound** · confidence **high** · Contracts and seams · from Sim lane (review fixes, props catalog)*

**The choice.** Grid cell sizes come from the rules, so `WorldGeometry::new(map, &Rules)` takes the whole rules object. The wasm `world_layout`, `WorldView` and `FlightLab` take a scenario's rules JSON (`VILLAGE_RULES` in the lab) instead of separate props, forests, physics and ricochet sections. The prop catalog travels through the same seam unchanged.

**The gap.** The review found the world rebuilt parts of rules piecemeal.

**The reach.** The wasm world APIs take one rules blob. Any new world-shaping rule arrives for free.

**Verdict.** sound.

### Replays refuse to load after the rules or map shape changes; no migration

***sound** · confidence **high** · Contracts and seams · from Slice 27f; Sim lane (review fixes, props catalog)*

**The choice.** A replay's header hashes the serialized rules (`config_digest`) and the map (the scenario digest). The rules now carry the whole catalog, and bridges now carry `deck`. A replay recorded before either change does not load. No compatibility shim was written. None are checked in.

**The gap.** The spec did not address replay compatibility across schema changes.

**The reach.** Every catalog or map schema change invalidates saved replays. Long-lived replays would need a migration or version policy.

**Verdict.** sound for a pre-release game.

### Every vehicle mount turns about its own pivot and fires from its own muzzle, declared on its catalog row, and models are checked against it

***sound** · confidence **high** · Contracts and seams · from Spike 03; Slice 11; Slice 36; Slice 27 (per-mount muzzles)*

**The choice.** A tank's turret faces north and its roof machine gun (HMG) swings to fire east. The HMG's rounds start at the HMG's own barrel on the cupola, not at the cannon's tip. Each row in a unit type's `mounts` list (`fixtures/units/generic/*.json`) carries `on` (the earlier turret mount that carries this one; absent means the hull carries it), `pivot_m` (where the mount turns, in its carrier's frame: forward, left, up from the hull origin) and `muzzle_m` (the muzzle, measured from the pivot along the mount's own bearing; absent means a hand weapon, see the soldier-muzzle entry). World muzzle = position + pivot turned by the carrier's bearing (or the hull's yaw) + muzzle turned by the mount's own bearing (`weapons::muzzle`). The numbers are read from the models: the cannon pivots at [0, 0, 1.45] with its muzzle at [5.9, 0, 0.55]; the tank HMG is `on: "cannon"`, pivot [-0.25, -0.58, 2.35], muzzle [1.43, 0, 0.32]; the jeep's pedestal HMG has pivot [0, 0, 1.68] and muzzle [1.43, 0, 0.32]. Where the art is physically forced, the row follows the art: a muzzle behind its pivot at rest would be a gun pointing backwards, so the jeep's row puts the muzzle ahead of the pedestal's bearing, where a real pedestal gun can put it. Range, arc obstructions, friendly-in-line, firing and building-facade aim all read the mount's own muzzle, and `muzzle()` has no branch on unit kind. The same rule lives once more in TypeScript, `packages/scene-assets/src/mountMuzzle.ts` (`mountMuzzles`, `muzzleOffset`), read by two consumers: `LaunchTracker` (a hull shot's fallback flash point and the gunfire sound's position) and the asset validator. The validator fits the model to the numbers, not the reverse: `fit.vehicle_muzzle` checks each mount's drawn muzzle at rest against its row, and `fit.muzzle_arc` turns the mount's yaw node through 12 bearings (12 × 12 with its carrier's, for a carried mount) and checks the muzzle stays on the simulation's arc. A fixture listing a different number of mounts than the model draws is also a finding. Which rig draws each mount is declared on the appearance (see that entry). The alternative, one muzzle offset per unit kind, fired the roof HMG from the cannon's tip or from mid-air to the side, and let model and simulation keep separate offsets unnoticed.

**The gap.** The user gave the rule ("each mount fires from its own muzzle"). The data shape (carrier chain, pivot plus muzzle), where it lives, and how models and simulation are kept in agreement were the agent's.

**The reach.** Any new vehicle gets correct muzzles from data alone, and its model must put its muzzle nodes where the catalog says or the asset check fails. A mount can ride another mount one level deep. The change moved every digest (a named change).

**Verdict.** sound — the user's rule, expressed first-principles in data, with one definition checked in both directions.

### A type's dense index is its id's rank in sorted order, and the observation carries that rank

***sound** · confidence **high** · Contracts and seams · from Slice 27f; Sim lane (review fixes, props catalog); Post-close review (sim)*

**The choice.** The packed publication tags each unit and prop with a small integer kind. That integer is the type id's position in the alphabetically sorted list of ids. Units: at, jeep, recon, rifle, supply, tank. Props: bridge_deck, building, crate, fence, jeep_wreck, rubble, ruin, sandbags, supply_wreck, tank_wreck, tooth, trunk, wall. The publication's `unitKinds` table and the layout's `propKinds` table carry the lists, and readers decode only through them. The observation itself holds the rank, not the id: `OwnUnit`, `IdentifiedUnit` and `Corpse` `.kind` are a `TypeIndex`, and `KnownProp.kind` is a `PropKind`, serialized as the number. So `publication::pack` writes the rank directly and needs no kind tables, and the wasm `pack_observation` test seam lost its two table arguments; the packed record is unchanged. The village scripts and the opponent read types by index (`Catalog::get`). Rejected: a binary search over the sorted ids, still a string search per unit per publication. The alternatives for the rank itself were declaration order (which depends on file and `extends` order) or a hand-kept enum.

**The gap.** Type ids are data-driven strings; the dense index the packed wire needs had to come from some rule, and the review found the pack converting ids to ranks per unit per publication.

**The reach.** Adding a type shifts the index of every type sorted after it. That is safe because every reader goes through the table. Anything reading the observation in Rust looks a type up by index.

**Verdict.** sound. It is deterministic as the catalog grows, and nothing converts per publication.

### The digest hashes where ground was cleared, not how much

***sound** · confidence **high** · Contracts and seams · from Sim lane (review fixes, props catalog); Post-close review (sim)*

**The choice.** Tanks flatten forest into cleared lanes. `Battle::digest` (the hash that proves two runs are the same battle) hashes the cleared-ground mask: its count of non-empty words first (`digest_cleared`), then every non-empty word with its index, so a hash can't be read as a prefix of another's. A test pins the count for a world with nothing cleared. So two equal-sized lanes in different places digest differently. The alternative, hashing a count of cleared cells, cannot tell where the lanes are. Adopting this moved every digest after any clearing (a named digest change) while the battle itself was unchanged.

**The gap.** The digest's coverage of this state was a review finding. How to fix it was open.

**The reach.** Replay parity now catches divergence in where lanes are.

**Verdict.** sound. A digest must distinguish different states.

### Large GPU data reaches lab viewports through a stable feed object, not as React props

***sound** · confidence **high** · Contracts and seams · from Slice 27a; Slice 27b*

**The choice.** A long battle runs in a development build. React 19's dev profiler logs each commit's changed props and expands typed arrays element by element, so passing the overlay mesh (a `Float32Array` rebuilt every publication), the fog grids or the world layers as props copies megabytes per commit and runs the page out of memory. Large, often-changing data goes through a `Feed<T>` (`apps/battle-lab/src/feed.ts`, `useFeed`): React sees an unchanged prop, and the viewport subscribes to updates. `LabViewport`'s `world`, `overlay` and `fog` props are `FeedSource`s, and the session returns `fogFeed`. A scene guard (`web/scene.mjs` `guardMeasures`) makes any profiler detail over 5,000 entries a console error, which fails the scene. Production builds are unaffected. The alternative was passing everything as props.

**The gap.** The props-for-everything pattern was never decided. It grew by default.

**The reach.** Any new big buffer passed into a viewport must be a feed, or the guard fails the scene.

**Verdict.** sound. It fixes the cause, and a check pins it.

### A soldier's slot is fixed when he joins, published, and hashed in the digest

***sound** · confidence **high** · Contracts and seams · from Slice 27f; Post-close review (sim)*

**The choice.** `Soldier.slot` says which soldier kind a man is and what he carries. The starting soldiers take slots 0..n. A replacement takes the first slot no living soldier holds. `Battle::digest` (the hash that proves two runs are the same battle) hashes the slot right after the soldier's id, so two battles that differ only in a man's slot digest differently; a test pins it with a cloned unit whose one slot is changed. Hashing it moved every digest with no change in any outcome (a named digest change). The publication carries it: `memberIds` rows and corpses gain `slot`, so the renderer draws each man as his own kind. That was an additive layout change. The alternative, leaving the slot out because the members list already implies it, was how it was first built.

**The gap.** The spec did not say whether derived per-soldier state belongs in `Battle::digest`.

**The reach.** A future rule that lets a slot change after joining, such as picking up a dropped weapon, is already covered by the digest.

**Verdict.** sound. A digest must cover the state that decides who carries what.

### Cell sizes are refused where a world is built, not only at battle setup

***sound** · confidence **high** · Contracts and seams · from Post-close review (sim)*

**The choice.** A fixture sets `fog_cell_m` to 0. `Battle::new` runs every rule validation before building the world geometry, and `sensors::validate` refuses a non-positive fog cell. `WorldGeometry::new` also refuses non-positive fog and ground cells itself, because the wasm `world_layout` and `WorldView` build a world without `Battle::new`. The alternative, validating only in `Battle::new`, would let the browser build a world that divides by zero.

**The gap.** The finding named a zero cell; where the check lives was open.

**The reach.** Every path that builds a world gets the check.

**Verdict.** sound.

### Zoom is the camera's orbit distance, and pitch follows a curve in log distance

***sound** · confidence **high** · Camera and controls · from Slice 09*

**The choice.** The battle camera orbits a target point on the ground. "Zoom" is the distance in metres from the eye to that target (`presentation.camera.zoom_min` 25 to `zoom_max` 2000). How steeply the camera looks down (its pitch) is read from `pitch_curve`, a list of `[distance, pitch]` points, interpolated in the logarithm of distance, so each wheel notch changes the view by the same proportion whether near or far. At load, `CameraController` refuses a curve that doesn't cover the zoom range, isn't monotonic (pitch never falling as you zoom out), or leaves the pitch limits. The alternative, a unitless 0–1 zoom with linear interpolation, would bunch all the change at one end.

**The gap.** The contract named `zoom_min`, `zoom_max` and `pitch_curve` without units or interpolation.

**The reach.** Every wheel gesture, and every future camera retune, which is a fixture edit.

**Verdict.** sound — metres are the natural unit and log spacing is how strategy cameras feel.

### Keys bind by their position on the keyboard, not by the letter printed on them

***sound** · confidence **high** · Camera and controls · from Slice 09*

**The choice.** A French player on an AZERTY keyboard presses the key where W sits on a US keyboard (it is labelled Z). Today that pans the camera forward, because every binding (commands and camera) reads `KeyboardEvent.code`, the physical key, in one table (`web/src/battle/input/commandBindings.ts`). The alternative, binding by `KeyboardEvent.key`, would scatter WASD across the keyboard on non-US layouts. The hint chips show the US label ("T").

**The gap.** The slice didn't say position or letter.

**The reach.** Every key binding and any future rebinding UI.

**Verdict.** sound — what strategy games do.

### The camera's target always sits on the ground

***sound** · confidence **high** · Camera and controls · from Slice 09*

**The choice.** The player pans the camera across a 20 m ridge while zoomed right in (25 m, nearly level). Today every camera step snaps the target's height to the ground under it (`groundAt`, the session's surface height), so the eye rides up and over the ridge. Without it, the eye would go underground. Labs with no ground keep the target's height.

**The gap.** The controller's inputs had no terrain height.

**The reach.** Any camera motion on a battle route.

**Verdict.** sound.

### Every lab uses the village's camera rig and the village's light

***sound** · confidence **high** · Camera and controls · from Slice 09; Slice 13*

**The choice.** Labs (the `/lab/*` debug routes) have no presentation block of their own. Today `LabViewport` takes the village fixture's `presentation.camera` (`villageCamera.ts`) and `presentation.light` (`villageLight.ts`) unless a lab passes its own. So lab wheels clamp to 25–2000 m and labs are lit like the battle. Cameras that a scene or lab places directly are not clamped. The alternative, a camera and light per lab, would drift apart from the real battle.

**The gap.** Labs had no presentation numbers.

**The reach.** Every lab frame looks and moves like the battle.

**Verdict.** sound — one camera config, one light.

### T packs a selection only when all of it is already deploying or deployed

***sound** · confidence **high** · Camera and controls · from Slice 09*

**The choice.** The player selects two supply trucks, one deployed and one packed, plus a tank, and presses T. Units that can't deploy (the tank) are left out. Of the rest, if every one is deployed or on its way to deployed, T packs them all; otherwise it deploys them all, so this press deploys both trucks (`toggleDeployment` in `useUnitControl.ts`). The alternative, flipping each unit on its own, would leave a mixed selection still mixed.

**The gap.** "T deploys or packs".

**The reach.** Any future toggle command on a mixed selection.

**Verdict.** sound — one key brings a mixed selection into one state.

### The reverse-click zone size is a presentation fixture section the simulation ignores

***sound** · confidence **high** · Camera and controls · from Slice 39*

**The choice.** `controls` in `fixtures/village.json` (`reverse_zone_length_m` 30, `reverse_zone_margin_m` 2) is read only by the web (`web/src/battle/input/reverseZone.ts`). With one vehicle selected, a right-click in the strip behind its hull, from the rear face back 30 m and the hull's half width plus 2 m each side, is a reverse move.

**The gap.** Where control tuning lives.

**The reach.** Control tuning stays out of the simulation's config digest.

**Verdict.** sound.

### R arms reverse move, X arms attack-move

***sound** · confidence **high** · Camera and controls · from Slice 39*

**The choice.** A hard cutover: `R` arms reverse move, `X` arms attack-move (Ctrl+right-click still attack-moves). The command bar shows "Reverse (R, or right-click behind one vehicle)"; the acknowledgement log says "reverse move". Bindings live in `web/src/battle/input/commandBindings.ts`.

**The gap.** Key choice.

**The reach.** Changes muscle memory for attack-move.

**Verdict.** sound.

### A right-click orders on release; a drag of 1 m or more sets the facing

***sound** · confidence **high** · Camera and controls · from Slice 35*

**The choice.** The player presses the right button on a field and drags toward a treeline. The press point is the goal. The order goes on release. If the release is more than the click slop away, the facing points from the goal toward the release point on the ground (`useUnitControl::dragFacing`). A drag under 1 m on the ground sets no facing. A drag never counts as the second click of a double-click. The alternative was to order on press, which leaves no room for a drag.

**The gap.** The spec said "right-drag sets facing" without the gesture's timing.

**The reach.** Every right-click order now waits for the button to come up.

**Verdict.** sound. It is the standard RTS gesture.

### The benchmark's blue side is scripted inside the simulation itself

***sound** · confidence **high** · Menu and benchmark · from Slice 10*

**The choice.** The `/benchmark` run needs blue to play the village's supported-attack script in the browser. Today `Battle.scripted(scenario, seed, plan)` (the WebAssembly battle handle) holds a `sim::village::ScriptedBlue`, the same one the native village trials use, and each step sends the script's orders through the ordinary command path first. So a scripted battle records and replays exactly like a played one. A live blue command would be refused as out of sequence, so the benchmark takes no input. The alternative, a TypeScript copy of the script in the page, would be a second owner of "a script plays blue" that could drift.

**The gap.** The script existed only in Rust trials.

**The reach.** Any future scripted or AI side in the browser can use the same hook.

**Verdict.** sound — one owner.

### The benchmark starts at tick 6300 of village seed 20260925, stepped there for real

***sound** · confidence **high** · Menu and benchmark · from Slice 10*

**The choice.** The benchmark should start "in heavy contact". Today it runs the ordinary village on seed 20260925 and steps the real simulation to tick 6300 (210 s) with `advance`, decoding every tick, before timing begins (`web/src/battle/benchmark/scenario.ts`, `useSimSession`'s `warmTo`). By then the two sides exchange 30–55 rounds a tick. Because every tick is decoded, the presentation carries the memory a player would have at that point (the fallen, known ruins, cues). It costs 13–17 s of preparation. The alternative, stepping silently or loading a snapshot, would start with an empty presentation memory.

**The gap.** "Warm-started to heavy contact" with no tick or method.

**The reach.** Any rules change moves what tick 6300 looks like; the fingerprint (below) does not pin the fixture's rules.

**Verdict.** sound.

### Changing the benchmark's tour or start requires a new version, enforced by a test

***sound** · confidence **high** · Menu and benchmark · from Slice 10*

**The choice.** Someone moves a tour keyframe. `benchmarkScenario.test.ts` hashes the scenario (seed, start tick) and the tour and pins that hash per `id@version` (today `village-contact-6300-v1`), so the test fails until the version is bumped. Fixture rule tuning is deliberately outside the hash. The alternative, a convention with no test, would let old and new rows be compared silently.

**The gap.** "Changing anchors bumps the version" needed enforcement.

**The reach.** Rows across versions are known not to compare; rules changes still move numbers within a version.

**Verdict.** sound.

### GPU time has one owner: the frame's timestamp timer, read every two seconds

***sound** · confidence **high** · Renderer frame · from Slice 10; Spike 01*

**The choice.** The frame measures its own GPU time (`frame/gpuTiming.ts`): two marker passes bracket the frame, each a 1×1 cleared render pass carrying `timestampWrites`, because empty compute passes write no timestamps on Apple's Metal. It keeps a rolling window of about 240 frames. The benchmark reads `BattleFrame.stats().gpu` every 2 s and files the reading under the current phase; the first reading waits a full window so warm-up frames stay out. There are no per-pass splits, because passes overlap on a tile-based GPU and only the total is meaningful. The alternative, the benchmark timing each frame itself, would be a second timer.

**The gap.** "GPU passes where available", and the frame already owned a timer.

**The reach.** The chart's GPU line has a point every 2 s, and a window that straddles two phases is filed under the later one.

**Verdict.** sound — one owner of GPU timing.

### One registry owns every GPU resource's lifetime and counts its bytes

***sound** · confidence **high** · Renderer frame · from Slice 12*

**The choice.** `frame/registry.ts` owns every GPU buffer and texture, with nested scopes (targets that depend on the window size) and slots (replaceable buffers). Its byte counts come from `trackGpuAllocations`, which sizes textures including every sample, layer and mip, and throws on an unknown format rather than reading zero. The alternative, each pass managing and counting its own, would leak and under-count.

**The gap.** Whether the registry counts itself.

**The reach.** Stats, the benchmark and disposal checks read one tracker.

**Verdict.** sound.

### Camera matrices are stored at float32, exactly as the GPU holds them

***sound** · confidence **high** · Renderer frame · from Slice 28*

**The choice.** The `math` package computes in double precision. Camera and cascade matrices are `math` matrices created with float32 storage (`createGpuMat4` in `camera3d.ts`), so each rounds once as it's stored. With double-precision matrices, cascade fits moved by about 0.1 mm, shadow edges moved, and scene clicks projected a few millimetres off, so the simulation got a different command. With float32 storage, CPU picking, `projectToCss` and the cascade fit use exactly the matrices the shaders read. Vectors stay double precision.

**The gap.** "Replace with `math`" and "nothing visible changes" conflicted.

**The reach.** Every camera-path matrix is float32.

**Verdict.** sound.

### The reverse-Z projection builders stay our own

***sound** · confidence **high** · Renderer frame · from Slice 28*

**The choice.** `math` builds only forward-Z projections and has no infinite-far form. `perspectiveReverseZ` and `orthographicReverseZ` stay in `camera3d.ts`, writing into a `math` matrix. `viewMatrix` keeps the old top-down fallback (up becomes +Y within about 2.6° of vertical). The alternative, composing `math`'s projection with a depth flip, matches only up to rounding.

**The gap.** `math` lacks them.

**The reach.** The two allowed exceptions in the math grep test.

**Verdict.** sound.

### The renderer's own math file holds only what `math` lacks

***sound** · confidence **high** · Renderer frame · from Slice 28*

**The choice.** `renderer-core/src/math.ts` keeps `smoothstep` (cubic; `math` has only a quintic `fade`) and `rayBox3Interval` (entry and exit distances on a `math` `Box3`; `math` only answers yes/no). Everything else uses `math`.

**The gap.** What stays local.

**The reach.** Where a missing helper goes.

**Verdict.** sound.

### The ground and the static props are separate layers, and the world is handed to the frame as named layers

***sound** · confidence **high** · Renderer frame · from Slice 16; Slice 19*

**The choice.** The fog treats ground and faces differently (ground probes need no facing test; walls do). Today the frame's `setWorld` takes `WorldLayers {terrain, props, structures, water, scenery, grass}`: `terrain` is a `TerrainSurface {mesh, site, plots, biome}` (the simulation's exported triangles as-is, flat-shaded, no smoothing or resampling), drawn with its own pipeline on the fog's ground path; `props` (walls, the map skirt) take the faces path; `structures` are building models; `scenery` holds placed trees (`WorldScenery {placement, appearances, lodPx}`), `grass` the grass kinds. The rejected alternative was a per-vertex "is ground" flag in the vertex colour's alpha, which would have fought the ground tint.

**The gap.** The seam allowed either a flag or separate layers.

**The reach.** Every static-world route draws through these layers; a new world surface picks its layer (and so its fog path) explicitly.

**Verdict.** sound — the fog's ground/face distinction follows a structural split, not a flag.

### Every model — soldiers, vehicles, buildings — is drawn through one palette-skinned vertex path in one models layer

***sound** · confidence **high** · Renderer frame · from Slice 20*

**The choice.** A soldier, a tank with a turning turret and a static house all need drawing. Today they share one path: each model is a mesh skinned by a palette of matrices. Soldiers' palettes come from the GPU pose kernel; articulated vehicles are rigidly skinned, one matrix per node (about 30) posed on the CPU; static buildings use the identity slot. The models layer (`models/modelLayer.ts`) draws in the frame's own shadow, depth-prepass and colour passes, lit and shadowed like the world. The alternative was separate renderers per kind.

**The gap.** The seam said to port the crowd, pose kernel and palette code but not how vehicles and buildings draw.

**The reach.** There is no second model renderer; trees are the one exception, in their own scenery layer.

**Verdict.** sound — one path for lighting, shadow and fog behaviour.

### Deaths play once at the corpse, then become static corpses

***sound** · confidence **high** · Renderer frame · from Slice 23*

**The choice.** A soldier seen alive is killed. He plays his death clip where the corpse lies, facing his own last heading, then becomes a static corpse. One first seen already dead is static at once, at the published yaw (which is the squad's heading). The alternative was snapping every corpse to the published yaw.

**The gap.** The published yaw is the squad's heading, and the display rule forbids a shared facing where a soldier's own is known.

**The reach.** Corpses of soldiers you never saw alive all face the squad's way.

**Verdict.** sound.

### Corpses are posed once on the CPU and drawn as one static, chunked population

***sound** · confidence **high** · Renderer frame · from Slice 23*

**The choice.** A long battle leaves 14,000 bodies. A corpse is its body mesh posed once, at the bundle's `corpse_pose`, at install (`posedMesh`, CPU skinning of positions and normals per tier), then drawn with an identity pose. Corpses are one static population (`BattleFrame.setCorpses`, called only when the list changes), grouped in 64 m chunks: a chunk off screen is skipped whole, and a chunk too far for any corpse in it to be above `impostor_px` draws as one range of cards. The alternative was skinning each corpse every frame.

**The gap.** "A static corpse instance, never skinned" gave no form.

**The reach.** About +0.1–0.2 ms GPU at 14,000 corpses. Each change of the list re-chunks the whole population on the CPU; appending is the next step if that shows.

**Verdict.** sound.

### No proxies are drawn in the battle; picking uses the simulation's bodies

***sound** · confidence **high** · Renderer frame · from Slice 23; Slice 24*

**The choice.** The player clicks a soldier. Picking reads the simulation's bodies (`picking.ts`, `bodyBox`: a vehicle's hull box, a soldier's cylinder as a box), so the tank's gun sticking past the hull is not clickable. Every proxy mesh for units and structures (tank, truck, outrigger, mast, infantry, structures) is deleted in a hard cutover; `proxies.ts` keeps only the lab marker, the crate box and `infantry` as a lab pick box. `BattleFrame.setStructures` takes the fitted prop models; `WorldLayers.structures` carries the map's props. `LabViewport`'s one `frame` callback returns `{instances, picks, models, corpses}`. The alternative was keeping proxies as a fallback.

**The gap.** The seam named new models, not the proxies' fate or what picking reads.

**The reach.** What you can click is exactly what the simulation has, never what the art overhangs.

**Verdict.** sound.

### The renderer takes the side's ground marks as a texture it patches in tiles

***sound** · confidence **high** · Renderer frame · from Slice 17*

**The choice.** The simulation keeps a ground layer of 1 m cells (craters, scorch, tracks, trampling), and each side learns the cells it sees. `BattleFrame.setGround(ground: GroundMarks | null): boolean` is called every animation frame. `GroundMarks` (`frame/scarTexture.ts`) is `{cellM, cols, rows, marks, takeChanges()}`, which the client's `GroundView` already satisfies. A new view (new client, rebuilt frame, scars toggled off and on) or an `{all: true}` change (a new epoch, a side switch) uploads the whole texture. An exact change list uploads only the 16 × 16-cell tiles holding a changed cell, merged along tile rows (`scarUploadRects`), straight from the view's bytes with no staging copy. Nothing is written when nothing changed. The spec had named a `GroundSurface` consuming patches; there is none, so the view's change list is the patch stream. `FrameStats.scars` reports bytes written.

**The gap.** The named seam didn't exist.

**The reach.** Part of battle-renderer's public API (`scene.ts`) and the viewport's per-frame input (`ViewportFrame.ground`).

**Verdict.** sound — minimal uploads through an existing shape.

### Scars live in the terrain bind group, one texel per cell, shared by ground and grass

***sound** · confidence **high** · Renderer frame · from Slice 17*

**The choice.** The scar texture (rgba8, one texel per 1 m cell) and its parameters join the terrain bind group (`terrainLayout`), visible to fragment and compute. The terrain shader and the grass build pass already bind that group, so one texture and one `groundScars` function serve both. The backdrop beyond the grid reads nothing.

**The gap.** Where the texture binds.

**The reach.** Anything else reading scars (e.g. decals) should use the same function.

**Verdict.** sound.

### Overlay marks carry a soft halo from one blur pass; ground paint glows by its own emissive

***sound** · confidence **high** · Renderer frame · from Slice 27e; Slice 27e (follow-ups)*

**The choice.** The user asked for a "holo-tactical" glow, then for less of it. Everything still in the overlay (the yellow order marks, contacts, the x-ray, garrison and guidance marks) gets one halo: the resolved overlay is blurred at half resolution (Gaussian, rows then columns, `glow.radius_px` 8) and laid under the overlay as a premultiplied halo (`O + (1 − O.a) · k · blur`, alpha kept ≤ 1) at strength `glow.overlay` 0.73 (a third of the first look, the user's call). Chosen over per-builder glow shaders because one pass covers every overlay without touching the builders; an additive glow was rejected because it clamps over white and breaks the overlay isolation check's maths. The world's bloom never sees an overlay. Painted ground marks don't take this halo: they glow by their paint emissive (`glow.ground` 0.4, next entry). The DOM callouts glow by CSS (`glow.callouts` 1).

**The gap.** The glow technique and strengths were delegated.

**The reach.** One halo strength for all overlay marks; separating contacts from orders would need a second overlay target. Costs two half-resolution textures.

**Verdict.** sound.

### Overlay marks and paint lie at the same height; the overlay's depth is biased, not its position

***sound** · confidence **high** · Renderer frame · from Slice 27 (muzzle flash) — paint and overlay marks lie at one height; Post-close review (presentation)*

**The choice.** A selected squad's amber paint circle must meet its yellow overlay route exactly at the rim. Every order and ground mark lies on the ground: there is no mark height to set. `lift_m` is gone from the paint style, the orders style, `map_border` and the consequence rings. Each raster alone decides how its marks clear the ground. The paint pulls toward the eye against ground-only depth (`PULL_M`). The overlay scales its clip depth by 1 + `OVERLAY_DEPTH_BIAS` (0.003, in `frame/overlayPass.ts`): its pixel doesn't move, its depth clears the ground by about 7.5 cm at the 25 m ground camera and 20 cm at the default camera, and a hull, wall or ridge in front still hides it. A smaller bias lost to draped chords sagging below convex ground at glancing views; a larger one would reach a prone soldier's height at the default camera. It replaced a 0.5 m pull along the view ray, under which order ink drew over soldiers' bodies. The orders tour checks that at pitch 0.6, with Space held, no order ink lands on a soldier's body above 0.12 m (40 px before, 0 after), and the rim-join and route-over-grass checks hold. One step remains, `STACK_M` (4 cm). It isn't a mark height: it keeps an arrowhead or chevron from tying in depth with the ring it meets, since the overlay writes depth. The contacts, sight, deployment, garrison and ground-cell overlays keep their own lifts, because they are callouts or raised glyphs, not ground marks. The alternative, lifting the overlay 0.3 m, made the route end about 5 px off the circle at the default pitch.

**The gap.** Mixing two layers raised the question of how they line up. The spec had no answer.

**The reach.** No mark builder takes a height. A new ground mark gets its clearance from the raster it draws in.

**Verdict.** sound. A depth bias clears the ground without moving the mark or reaching a body.

### One presentation clock drives poses, wind, eased motion and every effect's bounded life

***sound** · confidence **high** · Renderer frame · from Slice 18; Slice 23; Slice 24; Slice 25; Slice 26*

**The choice.** The player pauses the battle. Everything drawn freezes: soldiers' breathing, turret easing, the grass's sway, smoke puffs. The clock is `TickInterpolator.time(now)` (`web/src/battle/present/interpolate.ts`): the simulation tick being shown, blended between the previous and the latest publication, standing still while no tick arrives. `ObservationFeed` (`apps/battle-lab/src/poseFeed.ts`) turns each decoded publication into a `FeedFrame`; one pose driver poses soldiers and vehicles on that clock, and `ViewportFrame.clock` carries the same number to the frame's `setClock` for the wind. Every effect has a `{start, end}` (`EffectLifetime`) on this clock; `maxEffectLifetime` bounds them all (about 11 s in the village) and `validateEffects` refuses an unbounded table. A smoke source's puffs and flames each have their own birth time (`start + k / rate`) and seed, so a hidden tab or a slow worker (publications arriving far apart) draws exactly the smoke a steady stream would, and after a gap only what would still be alive is made. A tracer on a tiny stretch lives at most 1 s past its tick. A publication with the same tick as the last is ignored; an earlier tick means a new battle or reset and forgets every effect, source and hull. A still camera with a still clock redraws the very same frame, so captures are deterministic. Routes with no battle draw still grass. The alternative was wall-clock time, so a paused battle would still breathe, sway and smoke, and effects would depend on frame and worker timing.

**The gap.** The spec named a `PoseDriver(ObservationView, PresentationClock)` but not what clock, what pausing does, or how effects survive irregular publication arrival and resets.

**The reach.** Every lab that plays a battle goes through this one feed and clock (`useBattleSession`). Any future ambient motion (smoke drift, flags) must read this clock or it will move during pause. Replays and screenshots are stable.

**Verdict.** sound — a paused battle is a still frame, and the picture is deterministic in clock time.

### Order overlays test depth against the world before grass, so grass can't punch holes in them

***sound** · confidence **high** · Renderer frame · from Slice 27b*

**The choice.** A route ribbon is drawn over a meadow. Grass blades write depth in the colour pass, so overlays testing against that depth would be speckled wherever a blade stands taller than the mark. The depth prepass (the world without grass) is copied into `FrameTargets.overlayDepth` before grass draws, and the overlay pass tests against that copy. Ground cues lie over the grass but are still hidden by terrain, props, trees and units. The rejected alternative was lifting marks above the blades (1.1 m), which would float routes off slopes.

**The gap.** The spec didn't say how cues relate to grass.

**The reach.** It costs a 4× MSAA depth target (+31.6 MiB at 1080p). Every overlay (contacts, x-ray, order marks) inherits "over grass, under bodies".

**Verdict.** sound.

### Cascaded shadows cover where the map is, not from the camera's near plane

***sound** · confidence **high** · Light, terrain, grass and trees · from Spike 01; Slice 13*

**The choice.** The sun shadow uses four cascades (nested shadow maps, finest nearest the camera), each 2048², out to 2,600 m. Today the split distances are spread over the "receiver range" — the depth span from the camera where the map actually lies — not from the camera's near plane. Each cascade's depth is measured as a fraction of that range, `(depth − splitNear) / (cappedFar − splitNear)`. From a camera 1 km up, a near-plane split would put the finest cascades over empty air. Dividing by the far distance alone squeezed the cascades until three of them shaded one pixel (darker bands). Each cascade's light-space depth fits its own slice plus a 300 m margin each side, not a fixed 2,500 m. The numbers are `presentation.light.cascades` (`count` must be 4, the layout size; `split_lambda` 0.5, `max_far_m` 2600). Code: `light/cascadePolicy.ts`.

**The gap.** "3–4 cascades retuned for 0.3–1.6 km" without saying how.

**The reach.** Shadow sharpness at every camera height.

**Verdict.** sound — measured: no acne, no detached shadows.

### Road, forest and water edges on the ground are drawn from the simulation's own shapes, not from triangle tags

***sound** · confidence **high** · Light, terrain, grass and trees · from Slice 16*

**The choice.** A road runs diagonally across the ground mesh. Tagging whole triangles as road gave sawtooth edges. Today the simulation exports each road segment (`WorldGeometry::export_roads()`, `WorldView.roads()`: `ax, ay, bx, by, halfWidth`, stride 5) and the ground shader paints road wherever a pixel is within half width of a segment — the same rule `ground_surface_at` uses — with the blend centred on that edge. Forest and water rects are exported and used the same way (water wins over road, as in the simulation). Triangle tags remain only for the traversal view.

**The gap.** "Roads stay where the simulation has them", with only centroid-tagged triangles on hand.

**The reach.** One new WASM method, no simulation state, no digest change. What the player sees as road is exactly where the simulation's road rules apply.

**Verdict.** sound — drawn surface kind and simulated surface kind share one rule.

### Grass is regrown on the GPU from world tiles every time the view moves, with no CPU copy of the field

***sound** · confidence **high** · Light, terrain, grass and trees · from Slice 18 (technique; taken from ~/dev/game; rejected from ~/dev/game)*

**The choice.** The player pans the camera over a meadow. A compute pass (a GPU program that runs outside drawing) runs one workgroup for each 4 m square of world ground ("tile") inside a window sized to how far grass can be seen. It drops tiles outside the view, then walks that tile's candidate clumps in a fixed order. Candidate j stands at the j-th point of an R2 sequence (an evenly spread series of points) inside the tile, shifted by a fixed 0.4 m jitter. It exists only where the ground's grass density is above (j + ½) per tile. So a clump's position depends only on its tile and its number, and it never moves when the camera moves. The ground under each clump decides what grows there: the plot's grass kind or the verge's, and nothing on roads, forests, water or prop footprints. Each clump sits on the simulation's ground triangle and takes the ground's colour. Clumps are sorted into a near tier (LOD0, the finest mesh, 4 segments a blade) or a far tier (LOD2, 2 segments) by their height in pixels, then drawn with one indirect draw per tier (a draw whose count the GPU wrote itself). The field is rebuilt whenever the view changes or the side's learned ground scars change (`frame/grassPass.ts`, `terrain/grassField.ts`).

From `~/dev/game` it takes the technique only: GPU routing into tiers, a shading normal mostly borrowed from the ground so grass lights like the ground and never glitters, about one clump per `pixels_per_clump` pixels of ground, a travelling gust band keyed to a clock, far grass matched to the ground colour, and no depth prepass. It rejects that game's CPU-placed records and uploads (its measured pain), its per-vertex curved blade, its one-by-one popping, and its quality ladder (22–31 ms on Metal). The unbuilt alternative was porting that CPU-resident three-tier field as-is.

**The gap.** The spec said "port grass.ts and grassField.ts (near, mid and far tiers)" and gave no method.

**The reach.** Two tiers, not three: density falling with distance, not a third tier, carries the far field. Anything that changes where grass grows (scars, trampling, new ground kinds) must be a function the compute pass can read on the GPU. Grass costs about 1 ms of GPU at the default view.

**Verdict.** sound — the cost bar was met with margin, and GPU regrowth removes the upload problem instead of tuning it.

### Grass reads the terrain's own ground functions and the simulation's height grid

***sound** · confidence **high** · Light, terrain, grass and trees · from Slice 18*

**The choice.** The grass must stop exactly where the painted road starts. Rather than a second mask, the terrain material's ground lookup is split into shared GPU functions: `groundSite` (which plot, its edge distance, and signed distances to roads and forests), `groundWater`, `groundVerge` and `groundColour`. The terrain draws through them, and the grass compute pass calls the same ones. Plot records carry their kind; prop footprints (`x, y, yaw, hx, hy`) are part of the terrain site. Grass is seated on the simulation's height grid (`TerrainSurface.grid`) through the one triangle rule shared with fog (`frame/triangleRule.ts`). A surface with no grid grows no grass (the traversal view, the foundation patch, the workbench's ground). The alternative was baked grass masks or a separate "grass on/off" flag.

**The gap.** The spec did not say where grass masks come from.

**The reach.** Anything that changes the ground's paint (roads, verges, scars) changes where grass grows, for free. The terrain material's structure is now load-bearing for grass.

**Verdict.** sound — one owner for "what is on the ground here".

### Crater relief is shading only; collision, sight and navigation never see it

***sound** · confidence **high** · Light, terrain, grass and trees · from Slice 17; Slice 17b*

**The choice.** A crater's depth tilts the terrain's shading normal (its slope from taps either side, and a rim where the neighbourhood is deeper; `relief_m` 2.6 at a full crater, tilt capped at 40° because steeper walls turned to the blue sky and read as a grey sheen). For low camera angles the shader adds parallax (`groundScarsSeen`): it reads the scars again at points stepped down the view ray by the bowl's depth, so the near wall hides the floor and the far wall faces the eye. The bowl floor keeps 0.7 of its soil's brightness (darker read as a black cast shadow); soil and ash are fully rough. The sun-shadow lookup and the fog's lighting term keep the true geometric normal, so shadows and fog are unchanged, and the simulation's triangles never move (the slice contract; "craters never enter navigation"). The grass build reads the plain sample, without parallax.

**The gap.** How to show a hole without moving geometry.

**The reach.** Very low views still show a 0.9 m bowl as only a few pixels; opening it needs relief geometry.

**Verdict.** sound — consistent with the rule that craters are cosmetic to sight and pathing.

### Drawn forest trees are exactly the simulation's trunks: one tree per trunk, placed by one seeded rule

***sound** · confidence **high** · Light, terrain, grass and trees · from Slice 19; Slice 22; Slice 34b*

**The choice.** A player looks at a wood. Trunks stand on the forest density's grid, each moved up to `jitter · spacing` per axis by a stream seeded from the forest's index and rect, and dropped outside the rect, near a road or another body, or off the map. The world, the WebAssembly view and the renderer read the same trunks, and the renderer draws exactly one tree per trunk, with no filler trees; every crown lies inside its forest's rect and under its canopy height (`scenery/placement.ts`, pinned in `web/tests/scenery.test.ts`). Species, tint and size come from the biome's `trees` block. The trunk prop type's appearance is `drawn_by: "forest"`, so the trunk body draws nothing itself; the forest's tree is its picture. A tree on ground the side has seen cleared (knocked down or felled) is not drawn. The alternatives were decorative filler trees, or a separate trunk model placed on each trunk body, doubling the tree.

**The gap.** "Tree lines like WARNO's, placed from the authored forests" came without a placement rule, and the user's prop list included trunks while the forest already drew trees.

**The reach.** What the player sees as a tree is exactly something the simulation collides with, conceals by and fires into; forest density is set by the simulation's forest data, and trunk and tree line up by construction, not by prop fitting.

**Verdict.** sound — first-principles bodies, one owner per visible object.

### One light block in the fixture, one light owner in the renderer

***sound** · confidence **high** · Light, terrain, grass and trees · from Slice 12; Slice 13*

**The choice.** The battle has one light. `presentation.light` in `fixtures/village.json` holds the sun (azimuth, elevation, intensity), `shadow_floor`, `sky {turbidity, radiance, fill}`, `haze`, `backdrop`, `exposure`, `grade`, `bloom` and `cascades`, more than the six fields the seam named, because the sky, fill and haze are light too and every look number lives in the fixture. `light/sceneLight.ts` validates it by field name and owns `sunDirection`, the one sun vector everything derives from. `EnvironmentFrame` (`frame/environmentFrame.ts`) builds the sky, the environment map, the sun and the cascades once and hands every world material the same shading. There are no named light presets (golden, dusk, noon, overcast) in code. The world is lit with the full ported environment (physically based shading, sky, cascaded sun shadow, aerial haze, bloom, AgX tone mapping). The alternative was a set of named presets switched in code.

**The gap.** The seam listed fewer fields and left presets open.

**The reach.** A new lighting mood (night, overcast) is a new light block, not a preset switch.

**Verdict.** sound.

### Fog probes ground at target height and faces just outside themselves

***sound** · confidence **high** · How fog looks · from Spike 02*

**The choice.** A pixel on open ground asks "can an eye see a target standing here?", so it tests the point 1 m up (the simulation's target height). A pixel on a wall, roof or unit tests the point `face_probe_m` (0.1 m) out along its surface normal, and only eyes in front of that face count (`fogProbePoint`, `fogSeenSurface` in `frame/fogTerm.ts`). The alternatives measured (testing the surface point itself, or testing faces at target height) lit up roof bands up to 6 m wide. A separate roof rule (`roof_reach_m`) handles roofs above every eye.

**The gap.** The slice didn't say what a fragment on a wall or roof tests.

**The reach.** All fog on structures, trees and slopes.

**Verdict.** sound.

### Scars show wherever the side has learned them, fogged or not, as last seen

***sound** · confidence **high** · How fog looks · from Slice 17*

**The choice.** The scar texture is a copy of the side's learned ground view, so it never holds a cell the side hasn't seen. A crater seen an hour ago under what is now fog is drawn as remembered; fog only changes how it looks. The alternative, hiding scars under fog, would erase known ground.

**The gap.** Fog's effect on scars.

**The reach.** Scars never leak unseen enemy activity.

**Verdict.** sound — consistent with how every other known thing shows under fog.

### Fog is computed past the map's edge as if it were inside; a red border marks the playable area

***sound** · confidence **high** · How fog looks · from Slice 27b*

**The choice.** From the strategic height the camera sees land beyond the map. The backdrop and the off-map scenery now run `FogTerm` like the map does. A sight ray leaving the map keeps its running horizon, and beyond the edge is open ground at the map's lowest height, with no occluders or foliage. So a side's sight shape runs on past the edge instead of stopping there. The sim's knowledge still stops at the map edge; the drawn sight past it is presentation only. A red line along the inside of the map edge marks the playable area (`presentation.map_border`: colour `[0.86, 0.16, 0.12, 1]`, `width_px` 4, `min_width_m` 0.5). It is drawn as ground paint, and its width is set in screen pixels at the camera's target, rebuilt at each ×1.25 zoom step (`borderWidthM`). The first proposal was "everything off-map is unseen". The user replaced it because it cut sight shapes at the edge.

**The gap.** The spec didn't cover what lies past the map.

**The reach.** A map whose edge is higher than its lowest ground would draw sight past it slightly short (the village's edges are at ground level).

**Verdict.** sound. The user decided the fog. The border style is agent discretion within that.

### Every drawn layer takes fog as ground, as faces, or never; units are never fogged

***sound** · confidence **high** · How fog looks · from Slice 12; Slice 15; Slice 20; Slice 23; Slice 24*

**The choice.** The fog asks different questions of ground and of upright faces (see the probe entry), so each draw binds a fog class (`fogVisibility.ts`). The terrain, grass and backdrop are "painted ground" (ground that also takes order paint). Static props, buildings, ruins, wrecks and trees are faces: each face is seen or unseen by who can see it, so standing buildings and remembered ruins darken inside the fog. Units are never fogged. A red tank that blue has identified stands where blue cannot currently see (a scout spotted it and moved on): it is drawn lit and in full colour, because whether a unit is drawn at all is decided by identification, and fog only styles the world. Inside the models layer, `modelFog(pose)` (`models/modelFog.ts`) decides by pose kind: posed units (skinned soldiers, articulated vehicles) take the never-fogged group; static props take faces; corpses take ground, seen or unseen whole as the ground under them (as faces, their sides turned from every eye drew blue speckles in plain view). The alternatives were fogging only the ground, which leaves buildings behind a ridge at full colour; fogging unit faces like walls, which draws an enemy's back faces as black specks and fogs units the side has identified; or a fog branch per model type.

**The gap.** The fog function needs a ground flag and its owner wasn't named; the plan did not say whether fog applies to units, and the fog slice named only units and faces.

**The reach.** Any new drawn layer or model kind picks a fog class (by pose kind for models), with no new branch. Anything drawn per unit (models, impostor cards) must keep the never-fogged path; a future "fade stale enemies" look would be a separate cue.

**Verdict.** sound — units are shown by identification, one rule with no double-counting against fog.

### Fog looks are named styles in the fixture with a soft edge, a rim and an optional veil; `dusk` is the default

***sound** · confidence **high** · How fog looks · from Slice 15; Slice 15b*

**The choice.** A player looks at ground their side cannot see. It is drawn through a `FogStyle` from `presentation.fog {style, styles: {name: FogStyle}}`, swappable live through `BattleFrame.setFogStyle`. A `FogStyle` is `{dim, cool, tint, saturation, veil, lines {strength, floor, spacing_px, width_px, angle_deg}, edge_softness, rim {width_px, color, alpha}}`: darken, desaturate, pull toward a night tint (rescaled to unit brightness so it only tints), optionally add a glow at a set HDR brightness over all unseen (`veil`), rule fine lines, fade the unseen side in over `edge_softness` pixels from the seen side (so seen pixels never change outside the rim), and draw a rim over seen ground pixels within `rim.width_px` of an unseen one, in display sRGB after post so its colour is exact and it never blooms. Softness and rim are capped at 8 px (`FOG_EDGE_REACH_PX`); the distance passes search `ceil(max(rim, softness)) + 1` pixels. One WGSL function, `fogLook` (`frame/fogStyle.ts`), applies a style from the mask pass. Six styles ship: `veil`, `dusk`, `night`, `grey-veil`, `blue-highlight`, `scanlines`. `dusk` is selected (dim 0.5, cool 0.6, saturation 0.45, no veil, lines 0.5 with floor 0.015 at 5 px, soft edge 1 px, rim 2 px), chosen by the user after previewing all six. `/lab/fog-look` has sliders for every number and writes the block to paste. The alternatives were one hard-coded look, or an unbounded blur for the edge.

**The gap.** The contract named `{dim, cool, saturation, edge_softness}`, asked for a clearer edge without a form, and asked for live tuning and later A/B of other looks.

**The reach.** Every world material's unseen pixels; the user swaps a look by editing one `style` string. A wider rim or softer edge needs the 8 px cap raised, which costs pass time.

**Verdict.** sound — data-driven, bounded, and already exercised by the user's own pick.

### The weapon hold is baked into the clips; the weapon rides the right hand

***sound** · confidence **high** · Models and the asset pipeline · from Spike 03; Slice 11*

**The choice.** Library idle, walk and run clips don't hold a rifle. Today the clips are baked with a weapon-hold arm layer: inverse kinematics to wrist targets defined in the weapon's own frame, with gripping fingers, over the library's legs (`clips_infantry.py`). The weapon is parented to `hand_r`, and at bake any mesh parented to a bone is skinned rigidly to it; a mesh that is neither skinned nor under a joint is refused (`structure.unskinned_mesh`). So there is no runtime attachment system. The alternative, authoring each rifle clip whole or attaching weapons at runtime, costs more authoring or more runtime code.

**The gap.** "Rest authored in Blender" didn't say whether locomotion is authored whole or layered.

**The reach.** Swapping a soldier's weapon means a new bake, not a runtime swap.

**Verdict.** sound.

### Baked models are content-addressed bundles; one catalog is authored, one generated

***sound** · confidence **high** · Models and the asset pipeline · from Slice 11*

**The choice.** `assets/catalog.json` is authored: sources, basis, clip flags, tolerances, tints. The bake writes `assets/runtime/<hash>/bundle.bin` (in Git LFS) and a generated `assets/runtime/catalog.json` mapping names to hashes. `assets/runtime/` is Vite's public directory, so bundles are served at the site root. The loader reads only the runtime catalog; `asset check` fails when it is stale.

**The gap.** Which catalog is authored and which generated.

**The reach.** Caching (hash-named files never change) and the asset workflow.

**Verdict.** sound.

### The bundle format is BGAB: a canonical JSON header plus aligned typed arrays

***sound** · confidence **high** · Models and the asset pipeline · from Slice 11*

**The choice.** A bundle is a JSON header (sorted keys, numbers rounded to float32, so the same input gives the same bytes) plus 4-byte-aligned binary arrays: float32 positions and UVs, 16-bit normals, 8-bit colours, and for skinned meshes 8-bit joint indices with 16-bit weights summing to 65535. Clips are sampled at `sample_hz` (30) as 16-bit quaternions and float32 translations, each joint channel absent, constant or animated; clip scale tracks are ignored. Textures are carried as baked channels.

**The gap.** Delegated internal encoding.

**The reach.** The loader and every consumer.

**Verdict.** sound.

### Models convert from glTF's Y-up to Z-up, then turn by a per-source yaw

***sound** · confidence **high** · Models and the asset pipeline · from Slice 11*

**The choice.** The game is Z-up with +X forward. Each source is converted from glTF's Y-up, then turned by its catalog `basis_yaw_deg` (90 for the Quaternius rig, 0 for Blender-scripted vehicles). The turn is folded into each top joint's bind pose and clip tracks. Articulated parts' frames are converted the same way, so an unrotated Blender part yaws about its local +Z.

**The gap.** Spike 03 described a +90° root rotation for the rig only.

**The reach.** All posing; a turret turns by rotating its local +Z. The `basis.*` checks catch a wrong basis.

**Verdict.** sound.

### Unweighted leaf joints are dropped from the skeleton

***sound** · confidence **high** · Models and the asset pipeline · from Slice 11*

**The choice.** The rig has `_leaf` joints at finger and toe tips. The bake drops a `_leaf`/`_leaf_l|r` joint that has no weights and nothing under it, unless the skeleton keeps it, and lays every body out on its skeleton's joints by name. A body whose joints don't match fails `structure.skeleton`.

**The gap.** Which joints are kept.

**The reach.** Smaller skeletons; gloves must not weight leaf joints.

**Verdict.** sound.

### Every art source must be in the reuse manifest under an allowed licence

***sound** · confidence **high** · Models and the asset pipeline · from Slice 11*

**The choice.** Each source's content hash must be an entry of the reuse manifest with a licence in `ALLOWED_LICENCES` (CC0-1.0, MIT, project-owned) and the user's acceptance recorded (`accepted_by`). An LFS pointer is hashed by its object id, so neither check needs the file downloaded.

**The gap.** Which list, and how pointers hash.

**The reach.** No art without provenance can ship.

**Verdict.** sound.

### A vehicle's culling bounds cover every pose the driver can reach

***sound** · confidence **high** · Models and the asset pipeline · from Slice 20*

**The choice.** A tank's gun swung astern sticks out behind the hull. If culling used the rest pose's box, the gun could vanish at the screen edge. Today articulated `bounds` (`posedBounds`) sweep deploy in quarters and, packed and deployed, 32 turret bearings × 3 gun pitches × 16 HMG bearings × 3 HMG pitches, placing each node's 8 box corners, widening wheels to their disc and padding for the gaps between bearings. The spike tank grew from 9.5 × 3.7 m to 12.1 × 12.1 m. Fit checks still measure the rest pose; views frame the far pose (`farPoseBounds`).

**The gap.** An open item from the model bake: which pose the bounds describe.

**The reach.** Conservative bounds mean a few more models drawn near the screen edge.

**Verdict.** sound — correct by construction and cheap.

### Grass clumps are generated from a spec, not modelled by hand

***sound** · confidence **high** · Models and the asset pipeline · from Slice 18*

**The choice.** A grass kind needs a mesh, and no art existed. `grassClumpGlb(name, spec)` (`scene-assets/src/grass.ts`) writes the clump's GLB from the catalog entry's `grass` spec, deterministically: blade count, radius, height, width, lean, a colour ramp, a chance of dry stems and a chance of seed heads. `bun run --cwd web asset -- grass` writes the sources and records each hash as a project-owned entry in the reuse manifest; the bake then treats them like any GLB. `web/tests/grass.test.ts` fails when a committed source differs from what its spec generates. The alternative was hand-modelled or third-party clumps.

**The gap.** The new user decision needed art, and none existed.

**The reach.** Changing a grass kind's look means editing numbers in the catalog and regenerating, not modelling. The committed GLB and its spec can never drift apart silently.

**Verdict.** sound — reproducible and cheap to tune.

### Explosion and dust flipbooks are Unity Labs CC0 sheets

***sound** · confidence **high** · Models and the asset pipeline · from Slice 25*

**The choice.** Fireballs and smoke use two CC0 sprite sheets from Unity Labs: `Explosion00` (5×5 frames, fire) and `Cloud01` (8×8, dust/smoke), converted from TGA to PNG in `assets/third-party/effects/` (Git LFS), each listed in the reuse manifest. They load as one 1024² layer each of an sRGB 2D texture array, premultiplied and mipmapped at load (about 10.7 MiB).

**The gap.** Research named the packs, not the sequences.

**The reach.** Flames reuse the explosion sheet's hot frames (a critique noted they look static).

**Verdict.** sound — licensed, small, one atlas.

### Model textures ride in bundle format 3, stored once and shared by content hash

***sound** · confidence **high** · Models and the asset pipeline · from Slice 21b*

**The choice.** A model bundle (the baked file a unit or prop is drawn from) can carry `textures: Texture[]`, each `{id, format, width, height, levels}` with every mip level, where `id` is the sha256 of format, size and pixels. A material points at `textures {albedo?, normal?, orm?}` by index and may carry `wear?` and `colour_scale?`; a mesh may carry `tangents?`. A texture is stored once per bundle however many materials and detail tiers use it, and the renderer keys GPU layers by `id`, so two bundles sharing a texture share one layer. `FORMAT_VERSION` is 3; older bundles don't decode (all were rebaked).

**The gap.** The texture seam's shape.

**The reach.** Every asset pipeline and loader reads this format.

**Verdict.** sound.

### Texture recipes are procedural numpy noise, not Blender bakes

***sound** · confidence **high** · Models and the asset pipeline · from Slice 21b*

**The choice.** `packages/scene-assets/blender/textures.py` builds about two dozen recipes (camo, paint, rubber, steel, canvas, track links, burnt metal, multicam, webbing, skin, woods, stone, concrete, asphalt, soil, …) from periodic value noise, fBm, domain warp, Worley and blur, so they tile seamlessly and produce identical bytes every run (fixed seeds, PNGs without timestamps). A Blender render bake would have to prove determinism. All sources stay project-owned.

**The gap.** Recipe method was delegated.

**The reach.** New materials are new recipes in that file.

**Verdict.** sound.

### An infantry kind has three look variants; soldier id picks one, so neighbours never match

***sound** · confidence **high** · Models and the asset pipeline · from Slice 22b*

**The choice.** A soldier kind in the unit catalog lists several appearances (`fixtures/units/generic/soldiers.json`: rifleman `["rifle", "rifle_b", "rifle_c"]`, and likewise recon and AT). `AppearanceCatalog.resolve(kind, side, id, slot)` gives soldier `id` the member `id mod n` in list order. Soldier ids are issued in order per squad, so consecutive soldiers never share a look and every squad of three or more shows all three. A vehicle type names exactly one appearance. All appearances a squad can wear must share one skeleton (refused otherwise), so they share one animation clip set. Corpses pass the fallen soldier's id, so a man keeps his look when he falls. A random hash would leave a four-man team all one variant about one time in 27.

**The gap.** The seam named the call, not the pick rule or how clips stay shared.

**The reach.** New variants are a catalog list entry; any kit on a new skeleton needs its own soldier kind.

**Verdict.** sound.

### Unit silhouettes are rendered on the CPU at asset time from the baked models

***sound** · confidence **high** · Models and the asset pipeline · from 27f presentation leftovers; Slice 27f*

**The choice.** Each unit's card icon (`units/<type>.svg`) is a side view traced from its baked model by a small CPU rasteriser in the asset CLI (`packages/scene-assets/src/silhouette.ts`). Hulls are drawn at rest. A squad is drawn as its first three soldiers in their far pose, 0.8 m apart. The icon is traced at half coverage and simplified. It is deterministic, takes about 0.2 s for six types, and runs in vitest. `asset icons` also deletes icons nothing generates, and `asset check` fails on a missing, stale or orphan icon. The alternative was rendering with the game's GPU renderer. A coverage-only side view does not need it.

**The gap.** The brief suggested "our own renderer or the bake path".

**The reach.** Icons regenerate from the models, so a model change updates the icon. They are shapes only, never shaded.

**Verdict.** sound.

### Vehicle articulation is one small named input type beside the asset code; only the pose driver derives it from the battle

***sound** · confidence **high** · Models and the asset pipeline · from Slice 20; Slice 24*

**The choice.** A tank drives, turns its turret and fires. An `Articulation` (`packages/scene-assets/src/articulation.ts`) is `{turret_yaw, gun_pitch, recoil, hmg_yaw, hmg_pitch, travel_l, travel_r, deploy}`: turret yaw relative to the hull, HMG yaw relative to the bearing of the mount its catalog row names in `on` (the turret, on the tank), or to the hull when `on` is empty, recoil in metres the gun runs back along its bore, one travel distance per side (wheels roll by travel over radius, tracks scroll by travel over link pitch; a turn in place counter-rotates the sides), and deployment progress. Two owners split the work: the pose driver turns the feed (hull pose, each mount's `WeaponPose`, travel, deployment) into an `Articulation`, and scene-assets' `articulate` maps it onto the model's named nodes, which the bake's posed bounds, the validator and the renderer share. Articulated models pose into preallocated node matrices. The alternative was the spec's `VehicleRig {turret, gun, hmg, wheels, tracks, deploy}` class, duplicating both owners.

**The gap.** The spike named the inputs, not their owner or reference frames; the seam named a type the code already split.

**The reach.** A new vehicle part (a second turret) is a named node plus an articulation field; the battle never poses nodes directly.

**Verdict.** sound — one owner per concern.

### Which rig draws a mount is declared on the model's appearance, not on the mount row

***sound** · confidence **high** · Models and the asset pipeline · from 27f presentation leftovers*

**The choice.** The tank's appearance entry in `assets/catalog.json` declares `mounts: { cannon: "gun", HMG: "hmg" }`, and the jeep's declares `{ HMG: "hmg" }`. The rig names are the keys of `MOUNT_NODES`. The bake copies this into the runtime catalog, and `mountRoles(type, draws)` in `packages/scene-assets/src/units.ts` is the one owner that answers "which rig draws this mount". Which rig draws a mount is a fact about a model: a variant with its own model may rig the same mount differently, and the simulation never reads it. A validator finding, `fit.mount_draw`, catches a mount with a muzzle left undeclared, a declared name the type lacks, an unknown rig, or two mounts on one rig. The alternatives were the brief's "a mount names its model node" on the simulation's mount row, or guessing from the mount's name (`/hmg/i`).

**The gap.** The brief put it on the mount. The simulation's mount rows are strict (`deny_unknown_fields`), and the rig is presentation-only.

**The reach.** Each model declares its rigs once. A new model with a new mount must declare it or fail validation.

**Verdict.** sound.

### Combat effects have one owner fed by what the side's publication says

***sound** · confidence **high** · Effects and sound · from Slice 25; Slice 26*

**The choice.** Each tick the simulation publishes, per side, what that side may know: stretches of round flight, bursts, which units fired (shot counters per weapon mount), and known props. `EffectFrame` (`battle-renderer/src/effects/effectFrame.ts`) is the only place that turns this into drawn effects. It takes one `EffectPublication {tick, segments, blasts, shooters, smokes}` per decoded publication and, each animation frame, writes instances for the effect pass at the presentation clock (the smoothed time the view shows). A segment is `{path, ricochets, kind, shooter, hit, normal}`. A shooter is `{key, position, half, yaw, members, mounts}`, where `half` is a hull's half extents (null for infantry) and each mount has `{bearing, elevation, shots, kind, muzzle}`. A smoke source is `{key, kind, center, yaw, half}`. The app adapter `apps/battle-lab/src/effectFeed.ts` builds it: shooters are own units and identified enemies; smoke sources are known props whose kind some unit type's hull names as its `wreck` in the catalog. `useBattleSession` feeds every decoded publication (not just React's latest state) and builds in its per-frame callback, so every view on `BattleView` (village, replay, benchmark, endurance) draws effects.

**The gap.** The slice named the owner and inputs, not the call pattern or the smoke input's shape.

**The reach.** Every future effect (a smoke screen, new round kinds) is a new input row to this one class, not a second particle system. The sound feed reads the same launch derivation (`launches.ts`).

**Verdict.** sound — one owner, fed only by published causes.

### Effect timing: a publication for tick T covers the time from T−1 to T

***sound** · confidence **high** · Effects and sound · from Slice 25*

**The choice.** The view runs a presentation clock between simulation ticks. A publication for tick T is drawn over the clock span (T−1)/hz to T/hz (hz is the tick rate, 30). A tracer's head runs along its stretch over that span, so a round still flying continues seamlessly in the next tick's stretch. The streak is its style's length, clamped to the stretch. A muzzle flash starts at the span's start; an impact and a blast at its end; a ricochet's sparks at the corner's share of the stretch. Paused, the clock holds and so do effects.

**The gap.** How tick-stepped data maps onto smooth frame time.

**The reach.** Every effect is placed in time this way; sound shares the launch timing.

**Verdict.** sound — continuous streaks, pause-safe.

### Smoke shows over fogged ground as over seen ground; dust only from seen hulls

***sound** · confidence **high** · Effects and sound · from Slice 26*

**The choice.** A known wreck is drawn wherever the side knows it, fogged or not, and its smoke with it (the user's rule: if a wreck is shown, its fire and smoke are shown). Smoke over unseen ground is lit smoke, slightly tinted where it is thin. Dust is raised only by hulls the side currently sees (own units and identified enemies), from their published positions: one puff off each track every 1.5 m covered, thicker with speed up to 8 m/s.

**The gap.** Which smoke and dust fog may show.

**The reach.** Dust never leaks an unseen enemy's movement.

**Verdict.** sound — matches the user's rule and leaks nothing.

### Smoke is presentation only; a future gameplay smoke body would reuse the look

***sound** · confidence **high** · Effects and sound · from Slice 26*

**The choice.** The simulation has no smoke. Wreck smoke hides nothing: the test `smoke_is_presentation_only` shows the village battle gives the same digest and knowledge with the smoke look thickened or deleted. A future smoke-screen body (the user's Q28: bodies that obscure but block nobody) would be published as knowledge and fed as another `EffectSmokeSource` kind with its own `presentation.effects.smoke.<kind>` row. Only the feed and that row name wrecks.

**The gap.** Whether wreck smoke should block sight.

**The reach.** Players may see "smoke" that does not block sight; real smoke screens need a simulation body first.

**Verdict.** sound — keeps looks and rules apart.

### The sound bank is built on the first click, one sound per task

***sound** · confidence **high** · Effects and sound · from Slice 40*

**The choice.** Browsers allow audio only after a user gesture. On the first click or key the bank is synthesised one sound per task (about 250 ms of main-thread work in all; the 12 s ambience the longest at about 85 ms). The battle is silent for that fraction of a second.

**The gap.** When to pay synthesis cost was unspecified.

**The reach.** No load-time cost; no long frame.

**Verdict.** sound.

### An unseen enemy's noise plays as a vague, positionless cue, at most every 1.5 s

***sound** · confidence **high** · Effects and sound · from Slice 40*

**The choice.** Positional sounds come only from what the side sees: own units, identified enemies, visible rounds, blasts, known fires, drawn poses. A hearing cue (the side heard something it cannot see) plays one short vague sound (distant gunfire, an engine, footsteps, voices) panned by its sector relative to the camera, at a gain by near/far band, low-passed when far, with no position. The same cue (category, moving, sector, band) repeats at most every `presentation.audio.cues.repeat_s` (1.5 s). Captions still list every cue.

**The gap.** The spec asked for fog-of-war-safe audio without the rule.

**The reach.** Hearing gives direction, never a location.

**Verdict.** sound.

### Pause is detected as a standing clock; stale sounds are dropped, not played late

***sound** · confidence **high** · Effects and sound · from Slice 40*

**The choice.** When the presentation clock stops for `hold_s` (0.25 s wall), transients fade out in 30 ms and none start; loops play on. When it moves again, anything more than `late_s` (0.25 s) behind is dropped (hidden tab, fast-forward). An earlier publication or a clock running backwards is a new battle and clears everything, as the effects do.

**The gap.** Pause, fast-forward and replay semantics for audio.

**The reach.** No bursts of backlog sound after a tab switch.

**Verdict.** sound.

### Engine, track and turret sounds follow the drawn model's motion

***sound** · confidence **high** · Effects and sound · from Slice 40*

**The choice.** A vehicle's speed is the mean of both sides' rolled distances this frame, eased over about 0.25 s (so a pivot, one track backwards, still loads the engine); load is speed over the kind's `full_speed_mps`. Engine rate and gain go idle-to-load; tracks or wheels fade in with the square root of load; the turret whine follows traverse rate. The reverse whine plays by load while the vehicle is reversing (the publication's `reversing` flag).

**The gap.** How to drive vehicle loops was unspecified.

**The reach.** Sound matches what is drawn, not the simulation's internals.

**Verdict.** sound.

### Per-kind sound tables fall back to a default row

***sound** · confidence **high** · Effects and sound · from Slice 40*

**The choice.** `shots`, `impacts`, `impact_scale`, `blasts`, `vehicles`, `fires` and `cues.sounds` each have a `default` row, so a new unit or round kind sounds like something before its row exists. `motors` has none: only a kind with a motor (the guided missile) hums in flight.

**The gap.** Missing-row behaviour.

**The reach.** New catalog units are never silent by accident — but also never flagged as missing sound.

**Verdict.** sound.

### Every fire the side knows plays a fire loop that fades with the flames

***sound** · confidence **high** · Effects and sound · from Slice 40*

**The choice.** Each known smoke source plays the fire loop at its footprint's middle, full while it burns, easing to `smoulder_gain` over the last tenth of the burn, silent once the effects say it is out: the same clock as the flames. Fires are on the ambience bus.

**The gap.** Fire audio unspecified.

**The reach.** None beyond audio.

**Verdict.** sound.

### Sound plays in battle views and the contacts lab only

***sound** · confidence **high** · Effects and sound · from Slice 40*

**The choice.** `useBattleSession({ sound: true })` in the battle view (village, replay, benchmark, endurance) and the contacts lab; other labs are silent. The benchmark never gets a gesture, so its frame cost carries no audio.

**The gap.** Which surfaces get audio.

**The reach.** Benchmark numbers exclude audio cost.

**Verdict.** sound.

### A muzzle flash is placed on the drawn gun every frame, not at the simulation's launch point

***sound** · confidence **high** · Effects and sound · from Slice 27 (muzzle flash)*

**The choice.** A tank drives down the road and fires. The simulation says where the round left: the hull's position plus the mount's muzzle offset, as of the end of the tick. The drawn tank is somewhere slightly different. It is interpolated between ticks, its barrel has just recoiled, its gun may be pitched up. If the flash stood at the simulation's point, it would float up to 0.4 m ahead of a moving barrel, or sit a few pixels past the recoiled tip. Today each flash remembers who fired it: the shooter, the mount, and for a squad the soldier. Every frame it asks the renderer where that gun's muzzle is drawn. For a vehicle that is the mount's muzzle node under its turret, pitch and recoil. For a soldier it is the `muzzle` socket on his rifle in his current animation clip. The flash's glow, tongue and fireball start there and point where the round went. If nothing of the shooter is drawn, the flash falls back to the published launch point. The owners are `DrawnMuzzles` (`packages/battle-renderer/src/models/drawnMuzzles.ts`), which poses only the live flashes' muzzles, once a frame each, and the `MuzzleSource` that `EffectFrame.build` takes. `drawnMuzzleSource` in `apps/battle-lab/src/effectFeed.ts` maps effect keys to sides. The unbuilt alternative was to make the simulation's muzzle model match the art exactly. It never can, because recoil and pitch are presentation only.

**The gap.** The spec said flashes appear at the muzzle. It did not say which muzzle, the simulation's or the drawn one, when the two differ.

**The reach.** Effects must be built after the models are posed each frame. Any new flash-like effect (smoke puffs, shell ejection) should ask the same `MuzzleSource`. The tracer still starts at the simulation's muzzle, so a tracer and its flash can sit apart by the recoil distance, about 0.4 m on the cannon.

**Verdict.** sound. Presentation follows what is drawn. The simulation's muzzle is for rules.

### Mute and volume have one owner, stored in the browser, shown in menu and battle panel

***sound** · confidence **high** · In-world UI and HUD · from Slice 40*

**The choice.** `soundSettings` (`packages/battle-audio/src/settings.ts`, kept in `localStorage` as a convenience) owns mute and volume. `SoundControls` shows them in the main menu, the battle panel and the contacts and sound labs. Sound is on by default (the browser still holds it until the first gesture); muting suspends the audio context, and unmuting starts afresh so nothing stale plays.

**The gap.** Audio settings UI and persistence.

**The reach.** Per-browser setting, not per account.

**Verdict.** sound.

### A mixed selection's command bar lights a command when any unit can do it, and sends it only to those

***sound** · confidence **high** · In-world UI and HUD · from 27f presentation leftovers*

**The choice.** Select a tank and a supply truck, then press Deploy. Deploy is lit because the truck can deploy. Only the truck deploys, and the button shows "1/2" in the warn colour (`data-reach="1/2"`). Garrison goes to squads only, and leave-building only to units inside one. A right-click on a building with no squad selected now moves there instead of sending a garrison order the simulation would refuse. `web/src/battle/input/commandReach.ts` (`reach(command, selection, units)`) is the one owner for the bar, the hotkey and the right-click. This fixed two live bugs: garrison was refused for the whole selection when it held a vehicle, and Deploy was sent to every selected unit.

**The gap.** The user asked for a union of capabilities. What the partial case shows and where the order goes were not specified.

**The reach.** Every future command must declare which units it reaches in `commandReach.ts`.

**Verdict.** sound.

### An order is drawn as the unit's, never as lines from individual soldiers

***sound** · confidence **high** · In-world UI and HUD · from Slice 27 (muzzle flash) — an order is the unit's*

**The choice.** The user said lines should never run out of individual soldiers. A squad's order is one route from its circle's rim to its area ring. Queued legs are one line per unit. A holding squad's soldiers walking to their posts show the posts as markers only, with no line to each man. A vitest counts the route polylines under Space and pins that they equal the units with an order.

**The gap.** The user stated the principle. This removed the last per-soldier line.

**The reach.** Per-soldier movement stays visible through the soldiers themselves, never through order lines.

**Verdict.** sound. It is the user's rule.

### Every overlay and ground-mark colour lives in the fixture, except lab tracer colours

***sound** · confidence **high** · In-world UI and HUD · from Slice 27e; Slice 27e (follow-ups)*

**The choice.** The player sees supply rings, suppression and impact rings, the objective zone, order marks, the x-ray and glows. `presentation.overlay` in the fixture holds every one of their colours: `supply { ready, idle, serving, waiting }`, `consequences { suppression, impact }`, `zone`, `orders`, `xray`, `glow` and `paint`; `presentation.hud` holds the HUD theme (font, accent, enemy, text colours, glass). Each is validated at load (`validateSupplyStyle`, `validateConsequenceStyle`, `validateZoneColor`, `validateOrderStyle`, `validateHudTheme`, `validateCalloutGlow`), with `mesh.ts` `isRgba` as the one colour check, and builders take their colours as arguments. The lab reads them through `apps/battle-lab/src/villageOverlay.ts`. The lab tracer colours (a diagnostic) stay in `battleOverlay.ts`. The alternative was colour constants kept in each mark's own module.

**The gap.** "Every colour in the fixture" was said for what was being restyled, not the supply, suppression and zone marks.

**The reach.** A retheme is a fixture edit; builders take their colours as arguments.

**Verdict.** sound.

### A selected unit has one colour wherever it shows, its x-ray included

***sound** · confidence **high** · In-world UI and HUD · from Slice 27e; Post-close review (presentation)*

**The choice.** A selected squad walks behind a house. Its bodies are never tinted while visible; the selection shows as the amber circle painted on the ground under it, the colour scheme's `selected` role (see the colour-roles entry). The parts the house hides are drawn through it as the x-ray, a flat silhouette of the player's own hidden units (see the x-ray entry). A selected unit's x-ray takes the same `selected` colour: `villageOverlay.ts` reads it from `resolveOrderScheme` (which turns the fixture's chosen scheme into the style order marks are drawn with), and the fixture keeps only its opacity, `presentation.overlay.xray.selected_alpha` (0.62). The HUD follows the same role: `hudProperties` sets the CSS variable `--hud-selected` from it, and a selected unit's leader line (the line from the unit to its readout callout) is stroked in it. A selected vehicle's travel chevrons are always paint in `selected`, glowed by `selected_glow`. The alternative, a separate x-ray colour key in the fixture, was how it was first built. That key stayed the order yellow when the scheme moved the selection to amber, so a player asking "is that my selected unit behind the house?" saw the colour of routes and destinations.

**The gap.** The x-ray and HUD colours were chosen while the selection was yellow, and nothing tied them to the scheme's `selected` role.

**The reach.** Changing the selection colour is one fixture edit, and every place the selection shows follows it. Anything new that marks the selection must take the `selected` role, not a colour of its own.

**Verdict.** sound. The selection has one colour, with one owner.

### The biome is a validated JSON file, `fixtures/biomes/summer.json`

***sound** · confidence **high** · The catalog · from Slice 16*

**The choice.** Changing field colours or tree species should be a data edit. Today `summer.json` holds `{seed, palettes, plots, field_rules, verge, road, shore, forest_floor, trees, grass, scars}`: named sRGB palettes, plot kinds (name, palette, weight, furrow, mottle, roughness), and the rules for fields, trees and grass. `water_bed`, `water` and `distant` palettes are required. `validateBiome` rejects a bad field by name. Every lab route reads it through `apps/battle-lab/src/villageBiome.ts`.

**The gap.** The seam named only the top-level keys.

**The reach.** A winter biome is a second file with the same schema.

**Verdict.** sound — mirrors how light and fog presentation are validated.

### Every grass kind is a catalog scenery appearance with a `blades` flag

***sound** · confidence **high** · The catalog · from Slice 18*

**The choice.** A plot of wheat needs wheat-looking grass. Five grass kinds exist: `grass_meadow`, `grass_pasture`, `grass_crop`, `grass_wheat` and `grass_stubble`. Each is an ordinary entry in the asset catalog (`assets/catalog.json`): `{unit: "scenery", scenery: "grass", states: {summer: <glb>}, grass: <generator spec>}`. It is baked into four detail tiers like any other model, installed by the one appearance loader, and shown by the workbench (`/workbench?bundle=grass_meadow`) and the `asset sheet` tool. The scenery row for grass sets `blades: true` (`scene-assets/src/scenery.ts`). The validator then requires every tier to be the same blades in one canonical layout (finding `structure.grass`), so one index list per tier draws every kind. The unbuilt alternative was a separate grass-only asset path or per-kind card textures.

**The gap.** The user decided (2026-09-26) that each plot kind gets "blade or card sets", with no schema.

**The reach.** A new grass kind is a catalog entry plus a biome growth row. Grass art must keep the one-layout-per-tier rule, or the field cannot draw it.

**Verdict.** sound — grass joins the one appearance pipeline instead of forking it.

### The unit catalog names each unit's appearance; a squad's soldiers must share one skeleton

***sound** · confidence **high** · The catalog · from Slice 21*

**The choice.** The battle needs to know what to draw for "rifle squad, soldier 7, red side". `AppearanceCatalog.resolve(kind, side, id, slot)` (`scene-assets/src/appearanceCatalog.ts`) answers from the unit catalog: a vehicle type draws its one `appearance`; a soldier draws from his slot's soldier kind's appearance set (head, kit, colours), the member picked by his id, so neighbours never look identical, and the same one alive and dead. It returns the appearance name and the side's tint, or null when nothing is installed (nothing is drawn). On construction it refuses a soldier kind, or a squad type, whose appearances use more than one skeleton. The alternative was a free map from unit kind to model with no checks.

**The gap.** The spec named the class only.

**The reach.** Adding a soldier variant is a catalog edit. A mixed-weapon squad needs its weapons to share a clip set.

**Verdict.** sound — the catalog is the one owner of "what draws this unit".

### Wrecks are prop types per vehicle, each keeping its vehicle's cover tier

***sound** · confidence **high** · The catalog · from Slice 34; Slice 33*

**The choice.** `jeep_wreck`, `supply_wreck` and `tank_wreck` extend an abstract `wreck` row (`fixtures/props/generic/wrecks.json`). A vehicle's hull row names the wreck it leaves; validation requires the wreck's tier to equal the vehicle's weight-class tier (Q24). The renderer draws all three with the one `wreck` scenery appearance family, picking the model whose footprint best fits the box.

**The gap.** The spec said wrecks get rows, not how a wreck knows its vehicle.

**The reach.** A new vehicle needs a wreck row; wrecks can be destroyed into lighter wrecks.

**Verdict.** sound.

### Unit and prop types resolve in `contract::catalog`, one resolver for every consumer

***sound** · confidence **high** · The catalog · from Slice 27f*

**The choice.** A scenario's `rules.catalog` is a list of JSON documents in which types can `extends` other types. The resolver that flattens them lives in the `contract` crate, which gained a `serde_json` dependency for the merge. Rules resolve the catalog as they deserialize, so the simulation only ever sees a flat catalog. Native tools gather `fixtures/units/**` and `fixtures/props/**` through `sim::fixtures`. The browser's scenarios carry the generated view's `documents` (`VILLAGE_RULES` in `apps/battle-lab/src/scenarios.ts`), which resolve inside wasm when the rules deserialize, like any other rules. The wasm export `resolve_catalog` is used only by tests that resolve a catalog of their own. The slice had named a `sim::catalog` "exposed through the contract". Putting it in `contract` makes resolution part of parsing rules.

**The gap.** The slice said where the resolver would be exposed, not where it would live.

**The reach.** The simulation, wasm and every native tool share one resolver. `contract` now depends on `serde_json`. Catalog files, and a test's documents given to `resolve_catalog`, are read through `contract::catalog::parse_document`, which refuses a key written twice (see the duplicate-key entry).

**Verdict.** sound.

### TypeScript reads a generated, pre-resolved catalog, `fixtures/catalog.json`

***sound** · confidence **high** · The catalog · from Slice 27f; Sim lane (review fixes, props catalog); Post-close review (sim)*

**The choice.** Many TypeScript readers need a unit type's numbers synchronously when a module loads: the reverse zone, picking, readouts, the Node asset CLI. Wasm starts asynchronously in the browser, so it cannot answer at import time. The simulation's own resolver (`Catalog::view`) therefore writes `fixtures/catalog.json`, holding every unit and prop type already flattened, and `village.json`'s weapon rows with their `extends` resolved (`weapons`, written by `sim::fixtures::catalog_view`). `scene-assets` exports those rows as `WEAPONS`, and the weapons, ballistics and supply labs read them, never the raw rows. A Rust test (`crates/sim/tests/catalog.rs`) fails when the file is stale, and `BLESS_CATALOG=1` rewrites it. Test-only catalogs still resolve through wasm `resolve_catalog`. The preferred route, wasm in Node too, was not taken.

**The gap.** The user preferred wasm everywhere. Synchronous module-scope readers made that awkward.

**The reach.** `fixtures/` holds one generated file that must be regenerated whenever a catalog file changes. TypeScript never resolves `extends` itself.

**Verdict.** sound. There is still one resolver, and staleness is caught by a test.

### A resolved catalog resolves to itself; a patch sets values, never adds to them

***sound** · confidence **high** · The catalog · from Slice 27f; Post-close review (sim)*

**The choice.** A wheeled scout car `extends` the tracked tank's frame and writes `"mobility": { "wheeled": … }`. The merge (`catalog::merge_entry`) makes it wheeled: in the `units` section, a one-key variant component (`body`, `mobility`) that names a different variant replaces the parent's instead of merging into it. A type's `parts` gather along the chain, the parent's first, without duplicates. Everything else merges as before: objects key by key, named lists by name, anything else replaced. The same merge applies to `extends`, to parts' patches and to `sim::fixtures::patch_catalog`. It is limited to units because a prop's `body` is a struct, where a one-key override must merge. `capabilities` stays a deep merge, so a child adding `supply` keeps its parent's `deploy`. The merge is idempotent: resolving an already-resolved catalog changes nothing, so a scenario can carry the generated view's documents directly. As a consequence, a part (a reusable modification) that gives "+20 armour" must state the final armour number, not an increment.

**The gap.** The spec described `extends` and parts but not merge semantics, and the first merge blended a wheeled child into a tracked parent.

**The reach.** Future upgrade or veterancy parts cannot be additive modifiers without a new merge operator. A variant still can't drop a key or a named mount it inherits (see `fixtures/README.md`).

**Verdict.** sound.

### Roles map one to one onto the six shipped types, and scripts select by role

***sound** · confidence **high** · The catalog · from Slice 27f*

**The choice.** Each unit type carries `roles`: rifle `infantry`, recon `recon`, at `at`, tank `mbt`, supply `logistics`, jeep `light_vehicle`. The scripted attacker and the defender pick units by role. A "tank" is `mbt`. The fighters exclude `logistics` and `recon`. "Hurt" means half health for an `mbt` hull and half strength for a squad. Every script therefore picks the same units as before.

**The gap.** The walk named roles but not the mapping, nor that scripts should key on roles instead of type ids.

**The reach.** A new tank type is picked up by every script automatically when it carries `mbt`.

**Verdict.** sound.

### Deploy and supply are per-type capabilities; the rest of the service rules stay global

***sound** · confidence **high** · The catalog · from Slice 27f*

**The choice.** A supply truck's type carries `capabilities: { deploy: { seconds: 15 }, supply: { stock: 600 } }`. The service radius, rates and prices stay global rules. Supply requires deploy.

**The gap.** The spec did not say which service numbers belong to the type and which to the rules.

**The reach.** A second supply vehicle can have its own deploy time and stock, but not its own radius or rates.

**Verdict.** sound.

### Which prop a forest's trees and a bridge's deck are is data

***sound** · confidence **high** · The catalog · from Sim lane (review fixes, props catalog); Post-close review (sim)*

**The choice.** The rules' `forests.tree` names the tree prop type (`"trunk"`, one type for now, per density later). The lane a tank knocks through a wood reads that type's body too: `clear_lanes` asks whether it `topples` and what its `weight_class` is, instead of assuming the trunk. No shipped digest moved. Each map bridge names its `deck` (`"bridge_deck"`).

**The gap.** Trees and decks were hard-wired kinds.

**The reach.** Different tree species or deck types are data. Maps with bridges must name their deck.

**Verdict.** sound.

### Prop and vehicle bodies are catalog rows: a prop type is `{body, destroyed, appearance}`, and every reader reads its own column

***sound** · confidence **high** · The catalog · from Slice 34; Slice 34c; Sim lane (review fixes, props catalog)*

**The choice.** A tank shells a sandbag wall in front of a squad. What happens is read from data. Props are the catalog's `props` section, resolved with the units by the same loader and the same `extends` (files `fixtures/props/generic/{structures,obstacles,nature,wrecks}.json`; the three wrecks extend an abstract `wreck` frame). A prop type is `{body, destroyed, appearance}`. `body` (`contract::catalog::PropBody`) holds `blocks {infantry, vehicle}`, `stops_rounds`, `occludes`, `weight_class`, optional `cover_tier`, `lifetime_s`, `conceals`, `hp`, `armor` (default 1, range 0–1, scales direct hits), and the flags `topples` and `garrison`. `destroyed` is one of three: `"removed"` (gone), `"cleared"` (gone, with the ground under it cleared too; allowed only for a body that `topples`, such as the trunk), or `{into: {prop, height_m}}` (replaced by another prop type at that height). It sits beside `body` because it names another type by id, so the body row stays a small `Copy` value on every placed prop. `appearance` says how it is drawn (see the prop-drawing entry). So the sandbags (`hp` 150, into `rubble` at 0.3 m) become a rubble body on the same footprint when their hp is gone; `rubble` blocks nothing, stops no rounds, can't be moved, gives light cover to whoever stands in it, and draws with the ruin appearance fitted to its box. A building owns its own numbers (`hp` 400, into `ruin` at 2 m). Each vehicle type in `fixtures/units/` has `body.hull` with `weight_class`, `push_class` and the `wreck` prop it leaves, and `sound {profile, loudness_m}`. Each placed prop carries its row (`world::Prop::body`), so collision, sight, cover and weapons each read their own column. Resolution validates as named `CatalogError::Prop` errors: `hp` and `destroyed` together, `armor` in range, `cleared` only on a toppling body, every `into` chain ending rather than looping, and a unit's `wreck` naming a prop type. A column is added only when something reads it, so there is no `name` or `description` yet. The alternative was per-kind fields and tables scattered over the rules and readers (`buildings.hp`, per-kind blocks and occludes lists, cover tables, per-class hearing ranges).

**The gap.** The user asked for props as catalog data with string ids and `extends`, and the spec named `{hp, destroyed}` and an armour factor; the row shape, the destroyed states and the remains' heights were the agent's.

**The reach.** A new prop, destroyable obstacle, wreck type or destruction chain is data only; this is the base the whole catalog work built on. Wrecks chain down through lighter wreck types the same way (see the prop hit-points entry for the numbers).

**Verdict.** sound — first-principles data, no named special cases.

### A key written twice in a catalog file is an error

***sound** · confidence **high** · The catalog · from Post-close review (sim)*

**The choice.** An author defines `"tank"` twice in one units file. Plain JSON parsing keeps the last one silently. `contract::catalog::parse_document` refuses the first key written twice in any object instead, returning `CatalogError::DuplicateKey { key, line, column }`; a syntax error is `CatalogError::Syntax`. It is a serde seed, so the document is read once. `sim::fixtures::catalog_documents` (the catalog files) and the wasm `resolve_catalog` read through it. `village.json` is still read with plain `serde_json`: the finding named the catalog.

**The gap.** The finding named duplicate catalog keys. Where to catch them and whether the rules file counts were open.

**The reach.** Catalog files can't hide a shadowed entry. A duplicate key in `village.json` still wins silently.

**Verdict.** sound.

### A catalog's structure and weapon references are refused as the rules load

***sound** · confidence **high** · The catalog · from Post-close review (sim)*

**The choice.** A new vehicle's HMG row names a weapon row `village.json` lacks, or puts its muzzle on a mount that doesn't carry it. The rules fail to load, naming the entry, instead of the battle panicking at setup. `catalog::check` holds each hull mount to the hull or an earlier turret mount, with a `muzzle_m`, and neither `squad` nor `special`; a wreck to its vehicle's cover tier (`WeightClass::cover_tier`, now in `contract`); and every speed, turn, reverse fraction, turning radius, sight, loudness and deploy range. `check_soldier` holds each soldier kind to positive hit points and hand weapons: no `turret`, `on`, non-zero `pivot_m` or `muzzle_m`, and not both `squad` and `special`. A soldier mount with a muzzle would fire from mount geometry about the squad's middle. Weapon rows live in a different section from the catalog, so they are checked where the whole rules load: `Rules` deserializes through a private mirror of its fields (`#[serde(try_from = "UncheckedRules")]`), and `Catalog::check_weapons` names the unit type (`CatalogError::Rule`) or soldier kind (`CatalogError::Invalid`) whose mount names a missing row. What remains of `units::validate_types` is the drive rules, `validate_drive`, and `Arsenal::new`'s two panics are `expect`s the load checks guarantee. Rejected: checking in `Battle::new` (the finding's complaint) and a hand-written `Deserialize` (more code than the mirror).

**The gap.** The finding said structural checks ran too late. Which checks and where they go were open.

**The reach.** A broken catalog can't reach a battle; every consumer that deserializes rules gets the checks. The list of what is checked is kept in `fixtures/README.md`.

**Verdict.** sound. Structure is refused at load, where the author learns of it.

### Any lab page with `?inspect` shows a pass inspector

***sound** · confidence **high** · Tooling and tests · from Slice 12*

**The choice.** Adding `?inspect` to a lab URL shows a panel: the view (final, world only, overlays on black, overlays on white), GPU frame time, texture and buffer bytes, and the shadow receiver range. Scenes use its views to check overlays in isolation. Without the parameter it is hidden, so screenshots don't change.

**The gap.** "A pass inspector in the lab" had no shape.

**The reach.** Lab only.

**Verdict.** sound.

### Validator failure cases are GLBs generated in code

***sound** · confidence **high** · Tooling and tests · from Slice 11*

**The choice.** Each validator finding code has one golden failing GLB, generated in code (`web/tests/sceneAssets/synthetic.ts`) rather than checked in. A table test fails when a new code has no golden case.

**The gap.** "One golden-failure GLB per finding code".

**The reach.** Tests need no LFS.

**Verdict.** sound.

### The GPU fog is checked against the simulation's own sight formula and a CPU copy of the lookup

***sound** · confidence **high** · Tooling and tests · from Slice 14*

**The choice.** The fog shader has its own copy of the sight-shape formula (how far an eye sees to the front, side and rear). To be sure it matches the simulation, Rust exports test vectors from `sim::sight::multiplier` (`sight_multiplier_vectors()` over WASM); a vitest pins the TypeScript mirror against them and a browser scene pins the WGSL `fogShape` against them. A CPU mirror of the whole GPU lookup (`frame/fogOracle.ts`) runs on seeded synthetic sight maps beside the GPU, skipping the ~2% of cases that sit within float noise of a threshold. The GPU answers are reached through lab-only readback probes (`BattleFrame.fogProbes`, `window.__lab.fog()`); the production frame never reads back. Deliberately broken shaders turned both checks red.

**The gap.** The plan said "pinned against Rust oracle vectors" without saying which layer is the oracle.

**The reach.** The lookup has one owner (WGSL); the CPU mirror never draws. Any change to the sight formula in Rust shows up as a failing fog test.

**Verdict.** sound — a two-sided oracle catches drift in either direction.

### The workbench shows art that fails validation, through the same loader the battle uses

***sound** · confidence **high** · Tooling and tests · from Slice 20*

**The choice.** An artist drops a GLB with a missing socket into `/workbench`. The bake would refuse it, but the artist needs to see it. Today a validation returns `preview` (the bundle as built, errors or not); the workbench encodes it as runtime files (`previewRuntime`), serves them from memory and installs them through the one `AppearanceLibrary.load`, hash-checked, never baked. Single-file validation lives in `scene-assets` (`loose.ts`, `validateLoose`), shared by `asset validate` and the drop zone; a loose file's basis yaw is the catalog's, else 0, and the workbench offers a unit and yaw picker (a wrong yaw shows side-on in the front view). Guessing yaw from joint names was rejected.

**The gap.** The workbench must show art the bake refuses.

**The reach.** What the workbench shows is exactly what the battle would draw.

**Verdict.** sound — one loader, no preview-only renderer.

### `asset sheet` renders a contact sheet headlessly through the production frame

***sound** · confidence **high** · Tooling and tests · from Slice 20*

**The choice.** A reviewer wants to judge a new model without opening a browser. Today `asset sheet <appearance|glb>` drives the workbench in headless Chromium and writes `throwaway/sheets/<name>/`: a contact sheet of eight views (with a 1.8 m figure, the hit box and sockets), one strip per clip or vehicle motion, `stats.json` and the impostor atlases; `--accept` copies them to `assets/review/<name>/`. Tiles are rendered by a second `BattleFrame` at 512 px and composed on a canvas, so the sheet is the production renderer at a fixed size, not a page screenshot.

**The gap.** The plan asked for review sheets without a mechanism.

**The reach.** Model review is scriptable, matching the "workbenches are scripted" preference.

**Verdict.** sound.

### Third-party asset packs are recorded by hash but never committed

***sound** · confidence **high** · Tooling and tests · from Slice 21*

**The choice.** The infantry scripts read Quaternius's CC0 packs. The packs are not in the repo. Their reuse-manifest entries sit under `packs/…` with a `source_url`; the manifest test checks their licence and that they are absent from the repo. `blender/packs.py` downloads them into a local cache (`~/.cache/battlegame/packs`, or `$BATTLEGAME_PACKS`) and checks each file's hash when a script reads it. Each export writes its own project-owned entry naming the packs it came from. The alternative was committing the packs (in LFS).

**The gap.** How a hash-only entry passes a manifest test that hashes files in the repo.

**The reach.** Rebuilding infantry sources needs network access once. The repo stays free of redistributed third-party files.

**Verdict.** sound.

### Simulation speed is measured in instructions retired, and the endurance report prints them

***sound** · confidence **high** · Tooling and tests · from Slice 29*

**The choice.** This machine is shared with other agents, so wall-clock timings swing with load. The count of CPU instructions a process executed ("instructions retired") does not. `endurance_report` (a 10-minute battle run for performance) prints a "step instructions G" column per five minutes of battle and a whole-run total, counting only `Battle::step` (not the packing of what is published to the client), plus the final digest. `village_report` prints the run's total too. It reads the count through macOS's unprivileged `proc_pid_rusage` (`crates/sim/examples/common/instructions.rs`) and prints a dash on other systems. The alternative was comparing wall time on a quiet machine, which rarely exists here. AGENTS.md now makes this the standard measurement.

**The gap.** The brief asked for instruction counts but no tool printed them.

**The reach.** Every future performance change is judged by this column. It only works on macOS.

**Verdict.** sound — a load-independent number is the right yardstick on a busy machine.

### Movement scenarios are Rust data shared by the tests and the picture tool

***sound** · confidence **high** · Tooling and tests · from Slice 30*

**The choice.** Movement is checked with small staged battles ("scenarios"): a map, units, events (craters, props added mid-battle), scripted orders, a duration, a seed and a list of checks. They live as a table in `crates/sim/tests/movement_scenarios.rs`, written in the contract's own JSON shapes (`MapDefinition`, `UnitSetup`, `ScenarioEvent`, `ScriptedOrder`). A per-scenario `rules` value is merged over the village rules (JSON merge patch); its `catalog` key patches unit catalog entries by section and id. Orders go in as fixture scripts at tick 1, so they take the real command path. The picture tool `examples/movement_shots.rs` includes the same file (`#[path]`) and calls the same `run`, so the pictures and the assertions can never show different battles. Each check can carry `pending: Some("<slice>: <rule>")`: it is still run and reported ("pending (fails)" or "pending (passes: un-pend it?)"), never asserted.

**The gap.** The data format was delegated.

**The reach.** New movement or cover rules get proven by adding a row here. Today no check is pending.

**Verdict.** sound — one table, two consumers, no drift.

### Movement shots draw a fixed top-down camera per scenario

***sound** · confidence **high** · Tooling and tests · from Slice 30*

**The choice.** `movement_shots` renders each scenario north-up under a caption band, at most 1280×800, framing the units, goals, props and bursts plus 10 m, kept inside the map (a whole-map view left crates a few pixels wide). Soldiers are drawn at true 0.3 m radius, never under 2.5 px; props at true size, shaded by their real cover tier; a 10 m scale bar. A frame every 6 ticks (5 per second) and GIFs at 10 fps, so twice real time; a contact sheet with 560 px cells. PNGs come from the `png` crate as a dev-dependency (the WebAssembly build never pulls it); GIFs from ffmpeg with `-bitexact`, so two runs give identical bytes. Without ffmpeg the PNGs still land and a warning says so.

**The gap.** The drawing was delegated.

**The reach.** Agents review movement from these files (the user looks last).

**Verdict.** sound — deterministic, legible artifacts.

### Every movement scenario also checks that no soldier twitches

***sound** · confidence **high** · Tooling and tests · from Slice 27a*

**The choice.** The scenario runner (`crates/sim/tests/movement_scenarios.rs`) adds `CheckKind::NoTwitch` to every scenario. While a soldier has somewhere to go, it counts his reversals and his longest still stretch. Having somewhere to go means his spot or post is more than 0.5 m away, he has set off, and his squad is not waiting, route-blocked, packing or halted by an attack-move. A reversal is a step more than 135° off his last one. A still stretch is time spent inside a 1 m circle. The bound is at most 20 reversals and at most 8 s still. With the corner fix reverted, the fence scenario fails it (464 reversals).

**The gap.** Twitching was found by eye in GIFs.

**The reach.** Any movement change that reintroduces oscillation fails every affected scenario.

**Verdict.** sound.

### Tests bring a wall down with fire instead of a debug removal command

***sound** · confidence **high** · Tooling and tests · from Sim lane (review fixes, props catalog)*

**The choice.** Three tests need a wall gone. They bring it down with fire: three ATGM bursts at the wall (100 structural damage each, 4 m blast, so nearby crates survive). The wall leaves rubble, and each test's claim holds. There is no command that deletes a body (`RemoveProp` does not exist). The alternative was a debug removal command in the command schema.

**The gap.** The review flagged a command that could delete bodies outside the rules.

**The reach.** No command can remove a body arbitrarily. Tests must use in-rule destruction.

**Verdict.** sound.

## Trivial discretion (113)

Naming and cosmetic calls with no reach beyond their file, one line each.

- Default camera framing lives at `presentation.camera.default {target, distance, yaw}`, pitch from the curve.
- Wheel and orbit speeds are fixture keys `zoom_speed` and `orbit_speed`.
- Screen-edge pan is kept and runs at `pan_speed`, combining with held keys.
- Benchmark tour anchors and phase split (blue line, village, midpoint; 30–1500 m; one extra full turn) are delegated keyframes.
- The benchmark scene writes its frame-cost row to evidence, never into `frame-cost.md`.
- `/benchmark` is a registered fixture route; unknown paths fall back to the main menu.
- Lab and test red vehicles east of blue face west (`yaw: π`).
- The lab sight-lobe overlay: cyan filled lobe plus heading arrow, white ring for all-round sight.
- Copied files are formatted by oxfmt; "verbatim" means apart from imports and formatting.
- `cascades.count` is in the fixture but must equal 4, the shader layout's size.
- Finite HDR output and the sRGB boundary are pinned by CPU-side tests.
- `hit` replaces the old `impact: bool` on tracers.
- Own poses are counted by `mountCount`; identified poses carry `poseCount`.
- `pack_observation` is a WebAssembly test seam for decoder round-trips.
- The weapons lab's feed inspector (`FeedInspector`) is a text panel.
- Only CC0/MIT/project-owned art; the UAL2 animation pack is out.
- Basis checks use reference parts (muzzle ahead, eye above half height, front wheels ahead).
- Packages reach `math` through Vite and TypeScript aliases in `web/` and the asset CLI's resolve hook; a subpath gets an alias when first used.
- The oblique-AP ballistics preset uses a lab-only spent AP row (penetration 120).
- The flight lab's tank bodies are armoured (shape code 2).
- `ricochet_trace` example CLI: `cargo run -p sim --release --example ricochet_trace [row] [incidence_deg] [rounds] [penetration]`.
- One parallel-ray tolerance, 1e-10.
- Presentation interpolation uses `math`'s `lerp`, `vec3.lerp` and `deltaAngle`.
- Fog sight-map radius is the eye's farthest lobe reach, `range × max(front, side, rear)`, not an isotropic range.
- The fog tile cull also bounds each pixel by the tallest canopy above its ground, since canopy is not in the depth prepass.
- The fog mask debug view is the pass-inspector view `fog-mask`: white seen, black unseen, post skipped; the mask keeps each surface's alpha.
- The fog lab places blue's eight units in the street on the village's ordinary variant; red's garrisons are the garrison check.
- Fog GPU cost is measured paired (fog on/off/all-rebuild interleaved) under `FOG_COST=1`, since Metal overlaps passes.
- Contact glyph hatch is draped every 12 m, glow on 48-segment rings, outline on 128 (for CPU rebuild cost).
- `/lab/fog-look` is the shared street scenario under a 16:00 sun (0.42 rad) with sliders and a copyable `presentation.fog` block.
- "Seen pixels identical" checks run with bloom off, since bloom from fogged HDR spills a few levels into seen pixels.
- Plots within `field_rules.settlement_m` of a building are the settlement's meadow kind.
- The backdrop and cascade receiver box sit a metre below the lowest ground, not 6 m under the skirt.
- The workbench route is `/workbench`; its views are q-front, front, left, rear, top, and battle-near/mid/far at the village rig's zooms.
- Asset hot reload is a Vite plugin (`assetWatch`) that rebakes on `assets/source/` or catalog changes and sends `assets:rebaked`.
- The workbench scene builds its GLBs from scene-assets test builders and intercepts fetches, so nothing is written to `assets/`.
- Tree and hedge reuse entries are technique-only (`treeCrown.ts`, `sceneryDetail.ts`, `terrainScenery.ts`), nothing copied.
- Fog eyes are keyed `unit:slot` so unmoved eyes keep their maps.
- Grass pass binds 8 storage buffers in the compute stage, the default limit; its draw keeps the fragment stage's fog buffers at 4.
- The workbench draws grass as static scenery, with no wind sway.
- Grass is not masked by units: tall grass can hide a tank's lower hull.
- Soldier skeleton has 53 joints: glove fingertip and foot leaf joints are folded or dropped at export.
- Soldier clip edits from the frozen spike (low-ready hold, idle breathing, run lean, bigger kneel recoil, rebuilt prone, weapon eased onto the chest at death) are critique-driven art, listed in `assets/spikes/README.md`.
- Soldier body shape and kit placement (slimmed UBC body, pinned hems, ray-cast belt and pouches) are art calls.
- Each soldier starts looping clips at his own phase (`loopStart`, golden ratio over his id) so squads don't move in lockstep.
- Truck art departs from the frozen spike: stowed leg beams inboard, axles moved, deploy motion linear per window (the articulation's rule).
- Tank turret front is a broad flat face with angled cheeks; pivots unchanged.
- `veil` stays a selectable fog preset; the default is the user's `dusk` (dimmed and cooled, with rim).
- Fog-look gate frames: `default-wall` and `ground-wall` reproduce the single-wall wedge; street wedge frames are `default-wedge` and `ground-street`.
- Fog-look's hue check: CIELAB a*b* distance of 12 between the darkest 1% of seen and unseen ground (`HUE_MARGIN`).
- Frame view `ground-mask` (`setMaskView`) shows ground coverage for checks.
- The biome's `forest_floor` block (palette, patch, mottle, roots, verge, dapple, roughness) is validated by name; its palette is no longer required by name.
- `/lab/ground?village` is the paused village ground inspector, with a side switch and stream readout.
- The endurance report packs blue's publication every tick outside the timed step and reports ground patch bytes.
- Suppression halos and strike rings are ground paint (`LIFT_M` 0) so prone soldiers and corpses lie over them.
- Lab probes `suppressModels` and `timePoseKernel` (`MODEL_COST=1` on endurance) measure model cost.
- `endurance_report` comparison battle: seed 1, craters off (`ground.crater_depth_per_m` 0), 9,000 ticks; comparison builds were `git archive` extracts in `throwaway/` (deleted).
- The guidance tests stand a wall (`add_prop` event) in for smoke to blind a launcher.
- `t1-attack-move-halt` patches infantry ground sight to 40 m and rifleman hp to 10⁶ (tests halting, not the fight).
- Extra scenarios (`t0-tank-around-wreck`, `t2-one-man-gap`); the door scenario's door is 5 m.
- Scene env vars `VILLAGE_TOURS`, `SMOKE_GIF`, `EFFECT_COST`.
- Lab probes `suppressScars`, the grass probe's `laid`, `FrameStats.scars`.
- The ground scene's village inspector page is 1920×1080.
- `ScarSample` carries `at` so parallax and noise agree.
- `/lab/ground`'s authored crater grids are jittered up to 0.4 of spacing (seeded) so they don't read as an egg-crate.
- A zero tangent on a sliver triangle gets a perpendicular fallback in `mergeParts`, not a validator error.
- Validator findings `texture.size`, `texture.mips`, `texture.tangents`, `structure.texture`.
- Workbench surface sheet and texture channel checkboxes (`setTextureChannels`); albedo-off draws the texture's mean.
- Tolerances: `tank_wreck` footprint 3.0 m, `field_wall` 0.15 m (`assets/catalog.json`).
- Art detail added during texture critiques (fieldstone walls, stencilled ammo boxes, bridge girders, tank tools, MOLLE kit, redesigned wrecks).
- Per-soldier digest fields (`lateral`, `pace`, `start`, `path`, `path_revision`, `planned_at`, `route_from`) and cover `Watch` state are in the battle digest.
- Movement numbers live in `infantry_movement`; mechanism constants (3 slide passes, 0.3 m route-point pass, 5 cm on-spot, 1 m settle, 5 cm sight graze, once-a-second rejoin, push weights, lane shift ladder) stay in code.
- Pushing digest: prop poses, last-moved tick, each side's revision and last-seen poses, expiries.
- The boonie hat's crown stands up to helmet height rather than widening the soldier-height tolerance.
- No per-soldier stature scale: variants and stances cleared the clone read without it.
- `supply_truck_wreck` footprint tolerance widened to 2.0 m for thrown debris.
- The bridge deck lost its centre line and approach tiles; road dust films its paving.
- Lab water surfaces lowered from −0.5 to −1.5 m so bridge piers show.
- Jeep, fence, sandbags and dragon's-tooth models are delegated art (triangle counts in the catalog); two new texture recipes, `hessian` and `soil`.
- The authority lab drops a sandbag line, a fence and five teeth at tick 1 and adds a jeep to its column.
- The jeep is validated like the tank minus turret and tracks; its whip antenna sets `hull_top_m` 1.1.
- Space shows every own unit's orders via `useHeldKey("Space")` (`ShowOrdersBinding`); held-key tracking swallows the release so a focused button isn't clicked.
- Scenario check `EndsFacing` added to the movement runner.
- The layout gains `destroyablePropKinds`; the renderer draws those apart (`apartKinds`) and skips destroyed structures.
- No field wall or crates placed in the village: the slice's list named neither.
- Village scenarios run on a window of the village's own map (`village(window)`).
- Fixture edits made as in-place text edits so diffs show only moved rows.
- Whole-battle frames are the `battle` tour (`_battleLook.mjs`, `BATTLE_TICK` 8100): strategic, line, default, ground, wreck, craters, low view, each also HUD-free.
- The command-driven tours (`woods`, `cleanup`) run in `village`; the pose-only tours (`battle`, `edge`) run in a new `village-watch` fixture on `/battle/village/watch`, where a blue script plays under a free camera (`BattleView`'s `scripted.pilot` is optional).
- The `cleanup` tour steps every fresh battle to tick 60 before measuring, so reset cycles compare the same battle.
- A worktree needs only `assets/runtime/**` and `assets/third-party/**` from LFS to run the game.
- Lean side tie-break: the shorter step; on an exact tie, right.
- Lean numbers are fixture rules: `cover.lean_max_m` 1.5, `lean_clear_m` 0.1, `lean_apart_m` 0.8, `lean_hold_s` 1.5.
- Pose lean slide `presentation.pose.lean` `out_s` 0.25, `back_s` 0.4, sine in-out.
- Test sandbags in `t1-cover-shot-away` lowered 1.5 → 1.2 m so they stay below a muzzle.
- The `village-lean` lab scene (`/battle/village/lean`) shows a leaning squad at the ground camera.
- `ModelInstance.xray` is an rgba or null; the model record packs its rgb in `data.w`, alpha in `scale.w`.
- Queued waypoint rings are 2.8 m (`QUEUED_R`); chevrons start 1 m outside the marker (`CHEVRONS_GAP_M`).
- Lab hooks `setOverlayGlowStrength` and `suppressPaint` exist for paired shots (`setPaintEmissive` was removed after close: no caller).
- `GLOW_SHEET=1`, `MARKS_SHEET` capture comparison sheets in the orders tour.
- The rifleman, tank-cannon and HMG flash look (fireball billows forward of the tip; flipbook rotation random) is unchanged from the effects slice.
- A dropped model validated as a type (`validate --type`, the workbench drop) is rigged like the type's own model unless it is itself a catalog source.
- Catalog files: `fixtures/units/roles.json`, `units/generic/{soldiers,infantry,tanks,vehicles}.json`; faction id `generic`; an id is defined once across files.
- Weapon rows gain `name`, `description`, `icon`; ring captions and AP/HE labels read row names; the arsenal keys by `Weapon.id`.
- `sim::fixtures::patch_catalog` lets tests tune one catalog entry; "all soldiers tough" patches `rifleman`.
- `movement_shots` is a `sim` example with `png` as a dev-dependency; the `shots` feature and the wasm exports `build_id`, `WorldView::obstacle_revision` are gone.
- Role symbols: filled friendly frame, heavier strokes, shown in the accent colour beside the silhouette; group rows carry both at 18 px.
- `WATCH_TOURS=cover` and the garrison scene's `COVER_LIGHT=1` are opt-in tours that re-shoot the cover sheet.
- `MARKS_SHEET=<tag>` shoots labelled stroke comparison frames.
- The pose driver's vehicles and the feed's mount arrays are keyed by `sideKey(id, side, "blue")`; soldiers stay keyed by soldier id, which is unique across sides.
- `resolveOrderScheme` validates every scheme in `schemes`, not only the chosen one, and names the broken ones.
- The TypeScript `CarriedMount` type went: without `carriers` it was `MountRow`; only Rust reads `carriers`.
- `weapons::participants` is private; its only callers are in `weapons.rs`.

## Post-close user changes (presentation)

The user's requests of 2026-09-28, after close.

### A main-menu entry is one link: the whole card clicks

***sound** · confidence **high** · Lab app · from the user, 2026-09-28*

**The choice.** Each entry in `MainMenu.tsx` is one `<a class="menu-card">` holding its title and note, named by the title (`aria-labelledby`) and described by the note (`aria-describedby`). Hover lifts the card and lights an amber left edge; keyboard focus draws an amber outline. Before, only the title was the link, and clicking the description did nothing. The benchmark scene clicks the description and checks it navigates.

**Verdict.** sound.

### Sound starts at once when the page already had its gesture

***sound** · confidence **high** · Battle audio · from the user, 2026-09-28*

**The choice.** The benchmark's late sound was a bug, not loading. `BattleAudio` only created its `AudioContext` on a `pointerdown` or `keydown` *after* it was built. The benchmark builds it when you click Short run, so that click came too early, and nothing more comes while input is off. Sound came in only when you next clicked or pressed a key. The synthesised bank takes about 0.25 s; that was never the delay. Now, when the page has sticky user activation (`navigator.userActivation.hasBeenActive`), `BattleAudio` starts at once. The benchmark scene checks the context runs, and positional sound plays, from the run's first second with no further gesture. The strategic opening at 1,300 m also made the first seconds faint (about −27 dB); the distance floor below fixes that too.

**Verdict.** sound.

### Distance never takes a heard sound below 30%, and far sounds sound far

***sound** · confidence **medium** · Battle audio · from the user, 2026-09-28 · provisional*

**The choice.** `presentation.audio.distance` gains `floor` 0.3. A positional sound's gain is `floor + (1 − floor) × ref/(ref + rolloff·(d − ref))` (`distanceGain`): full within `ref_m`, easing down to 30% and never below it within `max_m`. So at the farthest zoom (listener about 500 m off) your units still sound at about a third. The old start threshold is renamed `cull`. `presentation.audio.air` colours distance, from nothing at the listener to full at `far_m` 500:
- a low-pass from `near_hz` 18 kHz to `far_hz` 1.6 kHz (was 1.2 kHz at 1,200 m);
- a reverb send up to `wet` 0.35, into one convolver per bus: a synthesised 1.8 s outdoor tail (`reverb_s`), darkening as it decays;
- the onset ramped in over up to `attack_s` 12 ms.

Hearing is unchanged: the same voices start from the same observation, and cues are still direction-only. Only level and colour moved; far-band cues take the full far colour. The sound scene renders one shot up close and from the farthest camera. The far shot measures 73% RMS (34% peak) of the close one. Its energy above 4 kHz drops from 8.9% to 0, and it keeps 14% of its energy after 0.5 s against 0 up close. The alternative, a flat 30% gain past some distance, loses the sense of zoom.

**The reach.** Every positional sound. With the floor, distant sounds now compete for the 32 transient voices, loudest first. A seen enemy's fire can take a voice from quieter blue sounds; an unseen one still cannot.

**Verdict.** sound; the numbers are provisional.

### An occluding structure takes fog whole

***sound** · confidence **medium** · Renderer fog · from the user, 2026-09-28*

**The choice.** Every known occluder box takes fog as one piece: buildings, ruins and walls, the props whose body `occludes`. A compute pass (`FogVisibility`, `wholeFn`) runs one workgroup per occluder. It samples the occluder's four walls and its roof every `fog_geometry.whole_step_m` (1.5 m) with the ordinary face and roof rules, against every eye that can reach it, and flags it seen if any sample is. `fogTerm` then answers "seen" for any fragment inside a seen occluder's box, whatever the layer:
- walls and roofs;
- the courtyard's ground and grass;
- a prop standing in the courtyard.

The box is grown by twice the face probe, and reaches 3 m above the top for a ridge and 2 m below the base. An occluder with no seen sample falls back to the per-fragment test, so a seen pixel is still never fogged. In practice that fallback finds it unseen all over. The flags recompute only when the maps or eyes change.

The boxes, a lookup grid and the flags travel in an `r32uint` texture (`wholes`), not a storage buffer. The terrain's fragment already binds WebGPU's default eight storage buffers.

The fog-look scene checks two frames. With every blue eye on, the building casting the wedge has 0 fogged pixels of 557 sampled over its footprint, courtyard included; before, its courtyard and inner walls were in fog. With one placed eye behind building A, the building it can't see is fogged at 952 of 952. The benchmark showed no measurable GPU cost (frame mean 4.0 ms against 5.7 ms before, both within this machine's run-to-run noise).

**The gap.** The alternatives were to take the sim's sight of the building, which could disagree with the drawn fog and fog a seen pixel, or to flag from the fragments that happen to be on screen, which would flicker with the camera.

**The reach.** Wrecks, rubble and sandbags don't occlude, so they stay face by face. A sight line thinner than the sampling step can still leave one pixel seen on an otherwise fogged building (the fallback).

**Verdict.** sound.
