# Choices

- **Deferred mechanics stay disabled.** Source art is tracked separately from `assets/catalog.json`; a model file never makes a unit selectable. This preserves the existing card/catalog contract.
- **Every disabled card receives source coverage.** The first art pass uses one explicit GLB per card, including deferred aircraft, rotorcraft, support, drones, infantry kits, and ground cards. This gives UI and future mechanics work a stable asset identity.
- **Deferred-family silhouettes are provisional.** The disabled-family generator provides faction-colored, role-readable source placeholders. Family-specific meshes replace them as movement, weapons, mounts, and animation contracts are implemented.
- **Family branches are authored in one deterministic exporter.** Aircraft, rotorcraft, support, drones, ground vehicles, and infantry kits use ID-driven silhouette branches so regeneration is repeatable and does not create a second disabled-card asset catalog.
- **Challenger 3 keeps a family-authored model.** It is stored beside the Challenger family source and remains unbound while its mechanics are deferred.

## Part B: reference-built models (2026-10-06)

- **Scope is every unit, playable first.** The user asked for every unit. The roster's 40 runtime vehicles and 17 infantry kits come first because players see them; the 86 disabled cards follow in slice 18 and stay unbound.
- **Materials before geometry.** Slice 13 fixes tyres, glass and paint on the existing models before any are rebuilt, so the reported defect (tyres not black) is gone early and every later rebuild inherits the material helpers.
- **Tyre cause, as found.** The rubber recipe is near black; the exporters' dust film (`dirt=.3`, `DUST` 0.15, tyre wholly below `rise`) brightens it two to three times, to about the dark-olive body's tone, and an oversized painted rim covers most of the face. Fixed by a `tyre()` helper and guarded by a validator role check, not by per-family colour tweaks.
- **References are committed, licensable only.** Public domain, CC0, CC BY, CC BY-SA, with file page, author and licence recorded; downscaled to 1600 px; under `assets/references/`, never read by runtime or bake. Rejected: keeping them in scratch (the previous pass did, and they are gone) or in the spec folder (they outlive the spec).
- **Frames come from the fixture catalog.** The exporters read the archived manifests by a path that no longer exists. The resolved catalog is already the authority on fit, so exporters read it rather than restoring the old path.
- **Budget is measured, then fixed.** The previous pass forbade inventing unit budgets; slice 14 sets one from the pilot's measured triangle counts and frame cost, then validation holds later families to it.
- **Blender before-sheets are evidence, not a gate.** `specs/unit-models/assets/before/` keeps the 2026-10-06 renders as the comparison baseline; the review gate is `asset sheet` in the production renderer.
- **Not drafted in parallel at first.** Superseded the same day: the user ran `/write-spec`, and four biased drafts were synthesized (see Synthesis below).
- **Generic pre-roster units are not remodelled** (user, 2026-10-06). `tank`, `jeep`, `supply` are not on the roster (the US light vehicle is the M1151 HMMWV); they become labelled test units in Part A.

## Part A: roster-only content (2026-10-06)

- **Generic units are kept as explicit test units** (user, 2026-10-06: tests use fake units unless there is an explicit reason to use the roster; the roster changes all the time). This was already practice on main (`847c813c`); Part A renames them `test_*`, moves them to `fixtures/units/test/` and fences them out of the game's catalog. (Named "stand-in" in earlier drafts; renamed, see decision 10 below.)
- **Test-unit art is kept, labelled and not remodelled.** Scenes, labs and the benchmark draw test units, so they need art; it becomes `test_*` under `assets/source/test/`. Its wrecks become the test units' own wrecks. (Superseded: renaming the generic wrecks by class; every vehicle now gets its own wreck.)
- **Tests, scenes, labs serving them, the benchmark and the endurance lab use test units**, so a roster change moves none of their verdicts or baselines. Superseded: an earlier draft of this plan moved labs and scenes onto roster units. Model cost is measured in a dedicated roster lab scene instead (slices 09, 14, 19).
- **A battle without factions is refused** (user, 2026-10-06: "obviously invalid"). `/battle` without a faction errors by name; the recipe-planned player battle and its address parameters are deleted. Recipes survive only as inputs to developer tools (the map workbench) and tests, naming test units. Rejected: defaulting to US against Eastern (a silent default hides a broken link).
- **Convert or delete, never leave** (user, 2026-10-06). Every place a generic unit or the village reaches the game becomes roster units or is deleted.
- **Encounter opponent and referee move to `sim::encounter`.** They are production behaviour (every encounter battle and the menu reel), so they are kept and renamed, not deleted.
- **No compatibility.** No id aliases, no village replay reader, no storage migration (the user's default).
- **The generator's `village` settlement class stays.** The user suggested `internal_test`, believing it was test-only. It isn't: `fixtures/map-presets.json` places it in ordinary Play maps, as the centre of small and medium maps of some types and as a secondary settlement on most, between `hamlet` and `small_town`, with its own district mix and street plan. Renaming it `internal_test` would mislabel game content. The user confirmed (2026-10-06): "if the word village is used in this context it's perfectly ok".
- **Developer menu stays in production builds** (user, 2026-10-06: the audience is technical). Its tools are labelled as tools, not game content.
- **One spec, not two** (user, 2026-10-06). Roster-only content and model work were briefly a separate `specs/roster-only-content/`; merged here as Part A.
- **Two catalogs** (user chose option A, 2026-10-06): `fixtures/catalog.json` is the game's, roster only; a test catalog adds test units for labs, scenes, the benchmark and the endurance lab. Refined by decision 5 below: the test catalog is resolved at run time, not committed. Rejected: one catalog with a `test` flag (fakes still ship, one missed check leaks them); labs on roster units (roster changes would move scene verdicts).
- **Reuse what the generic units earned** (user, 2026-10-06: "the generic tank is basically an Abrams with nice death animations"). Their tuned sounds seed the vehicle presentation classes (slice 01), the generic tank's model seeds the Abrams pilot (slice 14), the jeep's crew module puts crew on roster vehicles (slices 09, 15, 16), and their wrecks become the test units' own (slice 05).
- **Reference photos are resized**, long edge at most 1600 px (user: "resize them into something reasonable").
- **Paint is the vehicle's real nation's, US in desert tan** (user chose option C, 2026-10-06). Rejected: one scheme per faction (reads as three armies at a glance, but no longer looks like the real vehicles); US green.
- **All 86 disabled cards are remodelled in this spec, with icons** (user chose B, 2026-10-06). Rejected: deferring each family to its mechanics spec (A), or ground-only (C). Their icons come from their models like every other unit's, and the picker shows them on unavailable cards.
- **Scarce references: use what exists, generate the rest** (user chose A, 2026-10-06, adding image generation). Licensable photos and line drawings first; missing views generated with the latest gpt-image through the duet CLI, conditioned on real references, labelled as generated with their prompt and inputs, and never trusted over a real photo. Rejected: viewing non-licensable photos uncommitted (B); blocking a family on missing views (C).
- **Coordinator plus a wide worker pool** (user chose A and said to parallelise even more, 2026-10-06). Six workers to start, more while the machine keeps up; shared helpers stay coordinator-owned. Rejected: one serial session (B); Codex workers (C).
- **Look: stylised strategy-game readability** (user chose D of four generated directions, 2026-10-06). Real layouts from references, bold bevels, simplified shapes, painted edge highlights, moderate weathering. Rejected: clean factory (A), field-worn realism (B), war-film clutter (C). Exaggeration never changes physical envelopes.
- **Players zoom in to admire units; target machine is the Mac mini** (user, 2026-10-06). Tier 0 must hold up close; the triangle budget is set against that machine.
- **No per-unit variation yet** (user: "can look same for now, but in future we can build in variations"). No variation machinery in this spec.

## Unknown-unknowns sweep (2026-10-06)

Two read-only sweeps (art pipeline; content and catalog) found the cards below. The user agreed with every recommendation except 2 and 7, which they decided otherwise.

1. **Shared textures and a download limit** (slice 10). Every page downloads ~1.3 GB (215 raw bundles, measured); unit bundles embed copies of 18 MB of distinct textures 765 MB over; nothing limits it; the device never requests its texture-layer limit (256 default, 234 used). Rejected: only adding a byte limit.
2. **Every vehicle has its own unique wreck** (user, overriding the recommendation): "you should be able to tell which unit died by looking at the wreck", and it is the unit's size. The wreck prop records its unit type; footprint matching goes (slice 11). Rejected: wreck by mobility class then size (recommended); renaming the three generic wrecks by size.
3. **The HMMWV gets its real frame** (slice 14). Its model is the JLTV's file and its frame the JLTV's; a real-size HMMWV fails fit. Digests move; HMMWV-only balance sample. Rejected: switching the card back to the JLTV.
4. **Dressing has its own allowance** (slice 12). `dressing_*` nodes are excluded from ±0.1 m hull fit and held to a per-class allowance so they can't read as cover. Rejected: keeping all detail inside the hull box.
5. **The battle session owns its unit catalog; the test catalog resolves at run time** (slice 04). 76 files read the module-level catalog (11 import it directly), so a lab's test units would silently not draw, and the editor, publication and asset pipeline know one file. Rejected: committing two catalog files.
6. **Presentation from physics and base weapons** (slice 01). A new catalog field would change the engine id; roster weapons (derived rows) already miss every weapon-keyed table. Rejected: a `presentation.vehicle` field.
7. **The menu reel is a film** (user, overriding the recommendation, and noting they had said so before): it may use any units, including menu-only units with any stats, and anything may change, but every approved shot must stay **exactly** the same. Units wear roster looks (blue US, red Eastern; red AT looks like the Kornet team); an event-log test and per-shot keyframe comparison against a recording of the approved reel are the gate (slice 08). Rejected: real roster stats with re-staging.
8. **Roster base soldiers and intermediates are abstract**; "no change" is proven by diffing resolved cards, not by a parity file that carries its own catalog; the digest move is accepted (slice 02).
9. **`recipe_id`/`encounter_seed` leave the request contract**, unknown parameters are refused; engine id moves (slice 07). Rejected: keeping them as fixed values.
10. **"Test unit", not "stand-in"**, which already names the prop placeholder box (slice 05).
11. **market-town-test holds `assault`**; market-town keeps only the menu encounter (slice 06).
12. **Every unit switches detail with zoom** (user, 2026-10-06): the renderer already picks tiers by projected height; the art contract now requires each tier to draw strictly fewer triangles by a recorded ratio and refuses a mesh copied to every tier (slice 12).

Sharp edges, decided on evidence and shown to the user: vehicles have no impostor (tier 3 is the far view); crew from soldier tier 1, per-faction source, weapon hidden by role, vanish at the wreck; track links in texture (UVs scroll); workers export and validate, the coordinator bakes and commits runtime output; five exporters point at the moved manifests and 13 receipts are already stale; disabled cards share 52 files and the authored ground models are unreferenced; the disk filled on 2026-10-06.

## Synthesis of four drafts (2026-10-06, `/write-spec`)

Four drafters got the same neutral brief (the interview's decisions and measured facts, not this ladder), each with a lens: A fewest slices (Claude), B risk first (Codex), C seam quality (Claude), D parallel throughput (Codex). The canonical ladder takes:

- **Session-owned catalog before everything that depends on it** (B, C, D agreed): slice 04 now precedes test units, the menu and the village removal. The earlier ladder had the menu ahead of the session catalog, which would have needed a temporary path for menu units.
- **One resolver, three document sets** (C): game, test, menu; only the game's committed. Pages load only the appearances their session binds (C), so game pages stop fetching test art.
- **Weapon base key published by the one resolver** (`sim::fixtures::admit`), not derived in TypeScript; **`vehicleClass` owned by `units.ts`** and reused by budgets (C).
- **Pilot covers every budget class** (A, B, C): Abrams, Stryker, HMMWV, rifle squad; references for those families come first, testing reference scarcity early (B). Rejected: pilot including an aircraft and a rotorcraft (A); their budgets come from the first disabled lane family, with the tracked-heavy budget as a ceiling.
- **References inside each lane** (A, D), with only the schema and check up front (C's B7). Rejected: a separate references slice for every family before modelling.
- **Lanes in parallel after the pilot, closeout once at the end** (B, D). The earlier ladder closed out before the disabled cards, which would have run the full suite twice.
- **Coordinator batch integration** (D): workers export and validate only; the coordinator merges lanes in batches and bakes.
- **Wreck physics stays on `Hull.wreck`; the published prop gains `wreck_of`** (C); map-placed wrecks name their unit (A). **Wreck art is a linked scenery appearance** with pieces. Rejected: wreck as a state of the vehicle appearance (C), which changes unit bundle roles while scenery wrecks with pieces already exist and validate.
- **Crew module fixed in the pipeline slice, frozen before the pilot** (D), since the pilot's HMMWV and Abrams use crew.
- **Wrecks and art rules split** (fog audit): the wreck contract (sim, publication, renderer) and the validator's art rules are judged differently.
- **Menu units take the roster hull** (C found the Abrams and HMMWV don't fit the test hulls at ±0.1 m). Rejected: scaling roster art to the test hull (distorts the look).
- **Dressing replaces the existing `hull_top_m: 1.1` overrides** (A, C): one owner for dressing.
- **Corrections to the brief adopted:** material string extras already survive (`material.ts:51`); only node extras are numeric, so dressing is marked by name (A, C); track UVs scroll on track-material vertices only (A); a map download limit exists but covers only on-request content (B, D); missing factions aren't refused today (B); eight exporters read the archived path (B).
- **Not adopted:** a vehicle impostor (A, C). Vehicles draw tier 3 far away; the pilot measures whether that costs too much, and an impostor is the named alternative only if it does. Merging village removal with test units into one slice (A): kept separate (05 then 06) so each has one verdict, with 06 after 05 so the renames land once.
- **Weapon presentation shipped another way** (main `6c258b3d`, 2026-10-06): `kindTable.inheritRows` walks `extends` client-side for weapons without a row. Slice 01 keeps only the vehicle class and the unit-keyed overrides; the synthesis's published `base` field is dropped so weapon presentation keeps one owner.
- **No number is a requirement** (user, 2026-10-06): "numbers like the 50 MB download limit are arbitrary... I don't want you to spend too much time over-optimizing for some random number that you came up with yourself." Budgets, limits, ratios and allowances are loose tripwires set from measurement with room to spare; when one is exceeded and nothing visibly suffers, raise it. An earlier session had set the map download and kit bundle limits (`schema.ts`) to 50 MiB; both were raised to 256 MiB on 2026-10-06 and marked as tripwires.

## Slice 03 (implementation)

- **One file per moved type**: `crates/sim/src/encounter/defender.rs` and `referee.rs`, private modules re-exported as `sim::encounter::{Defender, Referee}`. Rejected: one shared `opponent.rs`, which would put the referee under the opponent's name. The moved bodies are byte-identical to the village originals.
- **Spec records left as written**: the unit-models README's problem statement and `visualizations/unknowns-map.html` still say "village Defender/Referee"; they describe the state the plan started from. The battle-foundation README's ownership line was updated, since it states current ownership.
## Slice 02 (implementation)

- **Base kinds live in `fixtures/units/roster/shared.json`** (`roster_rifleman`, `roster_grenadier`, `roster_scout`, `roster_at_rifleman`); the other three extend `roster_rifleman`, as the generic kinds extend `rifleman`, so `rifles` stays the first mount and the grenadier's launcher second.
- **Base kinds carry no appearance**: every concrete roster soldier names its own, so the generic `rifle*`/`recon*` sets would only be dead inheritance.
- **"Generic appearances" dropped from the intermediates** means the gunners' `operator_appearance` `active`/`carried` (`at*`) and the `rifle*` sets on the abstract `roster_marksman_rifle_body` and `roster_heavy_sniper_body`; the gunners keep `active_pose`, which every team member inherits.
- **The test reads documents by root**: `sim::fixtures::documents` became public so the test admits `units/roster`, `units/ground`, `units/roles.json` and `props` through `fixtures::admit`; slice 04 replaces it with the named game set.
- **Card diff identical** (2026-10-06): resolved `fixtures/catalog.json` before and after, restricted to the 142 cards, their 56 units, the 39 soldier kinds their squad slots name, and the weapons: identical. The only change is the 13 intermediates leaving `soldiers` (and the view's `documents` copy); the file diff is 1000 deletions.
- **No parity fixture moved.** The parity-reading suites (sim publication, skirmish, objectives, endurance, world_geometry, buildings; mapgen encounter, skirmish, compiler, layout_cli; contract maps, templates, catalog) pass unchanged: none pins a digest of a full-catalog battle. `config_digest` of a battle whose rules hold the full catalog does move (13 fewer soldier kinds); no committed file records one.
## Slice 01 (implementation)

- **Class names compose, not enumerate:** `vehicleClass` returns `<tracked|wheeled>_<weight_class>`, plus `_logistics` for a hull with the `logistics` role (`tracked_heavy`, `wheeled_medium_logistics`). A combination no hull fields yet takes the `default` row until it gets one.
- **HMG collision → `hmg-combat`** (the jeep's); the tank's `hmg-bass` override goes. **Rifle collision → `rifle-combat`**; recon's `recon-rifle` goes. Both dropped recipes stay in the library for audition. Every other unit override already equalled its weapon's default, so folding deleted `sounds.json` `units` and left `defaults` unchanged.
- **`sounds.json` `defaults` inherit too** (`inheritWeaponChoices` in `battle-lab/src/soundFeed.ts`, the battle's `gameSoundCatalog`): without it every derived weapon fell to `defaults.default`, so a roster tank cannon fired the rifle recording. Same `inheritRows` owner as the presentation tables.
- **Unit and mount identity leave the effect publication** (`EffectShooter.kind`, `EffectMount.name`, `Launch.unitKind`/`mountName`): per-unit sound choices were their only reader.
- **The sound catalog refuses an unknown section** (a stale `units` is an error, not silently ignored).
- **The sound workbench lists vehicle classes read-only, for audition.** Their loops and levels live in `game.json` `presentation.audio.vehicles`, which the workbench does not publish; a loop is replaced for every class through its effect slot. The firing editor lists every weapon row, derived ones included, showing which ancestor a row inherits from.
- **`wheeled_medium` takes `supply`'s row as the slice says, so it has no turret loop** although most of its hulls (Stryker, BTR, Boxer) carry turrets. Left for a sound pass; a turret loop driven by whether the hull has a turret mount would be the physical rule.
- **Found, not fixed: burst cadence mismatch on inherited choices.** `marksman_rifle` and `heavy_sniper` (single shots) inherit `rifle-combat` (3 rounds, 0.1 s), and `autocannon` (0.2 s) inherits `hmg-combat` (0.1 s), so each round plays a whole burst. Not new: before this slice all three fell to `defaults.default`, which is also `rifle-combat`. The shipped-cadence test checks only rows a weapon owns. Fixing it means choosing recordings, which is a call to make by ear.
- **Wheeled medium class gets a turret loop** (coordinator, slice 01 integration): the class copied the supply truck's row, so Strykers, BTRs and Boxers turned turrets silently. It now plays the shared `turret` loop at gain 0.08, between the light (0.05) and tracked (0.12) classes; a by-ear pass may retune it.
## Slice 09 (implementation)

- **Frame helper is a sibling of `parts.py`** (`blender/catalog_frames.py`), plain Python with no Blender, so it is unit-tested on fake catalogs (`catalog_frames_test.py`). A variant is the one unit type whose `appearance` names it; its frame is that type's `body.hull` extents and eye and its `mounts`, each mount's role from the appearance's `mounts`; body dimensions are `2 × half_extents_m`; a family is every appearance whose source is in `assets/source/roster/<family>/`, ordered by id. A per-variant exporter takes an appearance id (written to its catalog source) or an output `.glb` named after one, refuses another family's, and a family of one needs no argument (this also fixes the trucks' default path, which resolved against the CLI's `web/` working directory).
- **Re-exports are byte-identical.** Before switching, Bradley re-exported through the old script (archive path) matched its committed blob; after switching, every variant of Abrams (armor.py), Bradley (light_armor), BMP (eastern_armor), Boxer (europe_carriers), Fennek (remaining_ground), Leclerc (remaining_tanks), T-90 (eastern_tanks) and HEMTT (logistics), 13 GLBs, hashes to its committed LFS oid. So art hashes are unchanged and no GLB is committed. Every bound family's archived manifest frame equals its catalog frame exactly (extents, eye, mount order, pivots, muzzles, roles, names), except the HMMWV, whose manifest frame was never exported (slice 14). The stale receipts of Bradley and Leclerc did not change output: their scripts changed only in ways that move no bytes.
- **Appearances no unit draws have no frame.** The disabled families' exporters (T-14, T-15, Type 15, BRM, Jaguar, JLTV, Challenger 3) now refuse by name; their models come from slice 18. The research `dispatch_gate` check in `remaining_tanks.py` went with the manifest.
- **Receipts** (`source-receipt.json`, written by armor.py and eastern_tanks.py) record `frames: fixtures/catalog.json` instead of a manifest hash, and the eastern tanks' reference limits are inlined. Nothing reads receipts; regenerated ones were not committed since no GLB changed.
- **Infantry keeps reading the archived infantry manifest** (`specs/done/...` directly, no dead path): it reads equipment lengths and appearance sources, not a physical frame. Slice 17 replaces it.
- **Parts library** (`vehicle_parts.py`, nothing imports it yet): static parts make no empty (every empty is an articulated bundle node); wheel parts make a `wheel_*` empty with `radius_m`; `track_run` reproduces armor.py's belt exactly and drops its rubber pads, which hung under the track node and so scrolled; a `black` material role covers bores and openings; tiers per piece are fixed inside each part; sprocket teeth scale with radius.
- **Crew module.** Faction soldier: `CREW_SOLDIER` maps each faction to an appearance (`rifle_squad_active_a` for all three, since the roster rifle squads share one look until slice 17). Tiers: vehicle tier t carries the soldier's tier t + 1, and tier 3 carries no crew. Weapon: every connected piece skinned wholly to the `hand_r` joint is cut; the soldiers carry no separate weapon node (the weapon is joined into the one skinned mesh), so the joint it rides is the node used; on both `rifle.glb` and `rifle_squad/active_a.glb` this cuts exactly the faces the old material rule cut, at every tier. Materials: crew materials are renamed `crew_<name>` (shared between crewmen), and only those are restored after export. The jeep (test art) passes `assets/source/infantry/rifle.glb` explicitly and was not re-exported; a scratch re-export validates clean under its catalog settings and drops from 64512 / 17314 / 5616 / 1798 to 26114 / 8554 / 2538 / 364 triangles, the crew having been most of the jeep.
- **References.** `references.json` is `{entries, gaps}`; licences are SPDX ids (`CC0-1.0`, `CC-BY-*`, `CC-BY-SA-*`) or `public-domain`, and `ours` for generated views only; an unpulled LFS image is hash-checked by its pointer oid and its size skipped with a warning; a family folder without `references.json` fails `check`. The sheet's family is the source's roster folder and its variant the appearance id; `side` pairs with the vehicle's left view, a new `q-rear` studio view serves `three_quarter_rear`, and `detail` has no model view. Verified with a stub library for `us_m1_abrams_sep_v3_trophy`, then deleted.

### Triangle baseline (2026-10-06)

Triangles per tier 0 / 1 / 2 / 3, from `asset validate` on each committed source (the sheet's `stats.json` has no tiers for a catalog appearance). All 43 vehicle sources validate.

| Appearance | Triangles |
|---|---|
| us_m1_abrams_sep_v2 | 25942 / 10186 / 3284 / 1296 |
| us_m1_abrams_sep_v2_trophy | 26774 / 10522 / 3428 / 1392 |
| us_m1_abrams_sep_v3_trophy | 27346 / 10710 / 3484 / 1392 |
| europe_leopard_2_2a6 | 26942 / 10298 / 3408 / 1362 |
| europe_leopard_2_2a7v | 28062 / 10642 / 3464 / 1362 |
| europe_leopard_2_2a8 | 28894 / 10978 / 3608 / 1458 |
| europe_challenger_challenger_2_tes | 18382 / 6360 / 2338 / 1292 |
| europe_leclerc_xlr | 16834 / 5368 / 2282 / 1328 |
| europe_kf51_panther_prototype_main_battle_tank | 19122 / 6080 / 2500 / 1472 |
| eastern_t_72_t_72b3_2016 | 25816 / 10566 / 3520 / 1468 |
| eastern_t_80_t_80bvm | 25688 / 10718 / 3408 / 1372 |
| eastern_t_90_t_90m_proryv | 27096 / 10910 / 3278 / 1228 |
| eastern_type_99_ztz_99a | 17586 / 5614 / 2388 / 1324 |
| us_m2_bradley_ifv_m2a4 | 13828 / 5134 / 2208 / 1244 |
| us_m3_bradley_cfv_m3a3 | 13884 / 5154 / 2220 / 1256 |
| europe_cv90_cv9040c | 11688 / 4260 / 1830 / 1040 |
| europe_cv90_mk_iv | 12072 / 4386 / 1870 / 1076 |
| europe_puma_level_c | 13158 / 4726 / 1574 / 992 |
| europe_ajax_tracked_reconnaissance_vehicle | 13600 / 4820 / 1858 / 1052 |
| eastern_bmp_ifv_family_bmp_2m_berezhok | 9804 / 3888 / 1918 / 1224 |
| eastern_bmp_ifv_family_bmp_3 | 9756 / 4056 / 1778 / 1072 |
| us_stryker_m1126_icv | 8278 / 4136 / 1162 / 784 |
| us_m1127_stryker_rv_reconnaissance_vehicle | 8354 / 4172 / 1194 / 816 |
| us_stryker_m1134_atgm | 8906 / 4396 / 1258 / 880 |
| us_stryker_m1296_dragoon | 8640 / 4270 / 1218 / 840 |
| us_lav_lav_25a2 | 8364 / 4210 / 1198 / 828 |
| us_lav_lav_at | 8558 / 4308 / 1238 / 880 |
| europe_boxer_apc | 10530 / 4502 / 1218 / 716 |
| europe_boxer_rct30 | 12172 / 5180 / 1438 / 796 |
| europe_vbci_infantry_fighting_vehicle | 10166 / 4396 / 1054 / 708 |
| eastern_btr_btr_82a | 9002 / 4472 / 1358 / 904 |
| eastern_zbl_08_wheeled_ifv | 9878 / 4836 / 1354 / 864 |
| us_acv_acv_p | 11128 / 4694 / 1270 / 724 |
| us_m1151_hmmwv_hmg | 7502 / 3162 / 868 / 700 |
| eastern_tigr_tigr_m | 7064 / 3096 / 878 / 660 |
| europe_fennek_reconnaissance_vehicle | 5308 / 2432 / 660 / 524 |
| europe_vbl_machine_gun_scout | 4170 / 2116 / 588 / 492 |
| us_m977_hemtt_general_resupply | 21304 / 8682 / 2404 / 848 |
| europe_man_hx_general_resupply | 19954 / 8422 / 2290 / 706 |
| eastern_ural_4320_general_resupply | 15978 / 7440 / 2048 / 690 |
| tank (test) | 56670 / 17858 / 3398 / 976 |
| jeep (test) | 64512 / 17314 / 5616 / 1798 |
| supply_truck (test) | 25340 / 8222 / 1526 / 264 |

**Frame-cost baseline: pending for the coordinator.** The GPU browser runs here (a sheet takes about 14 s), but no lab scene holds roster vehicles, and the slice's column-of-roster-vehicles scene is new lab code; it was not built in this slice.
## Slice 10 (implementation)

- **One transport for every bundle.** Skeleton clips, unit, scenery and kit bundles all travel the same way (texture pixels out, gzip), not only unit and scenery bundles: one reader path instead of raw and gzip branches. The catalog keys stay the original content hashes; every bundle, texture and library hash in `assets/runtime/catalog.json` is unchanged (checked against the previous catalog: 227 appearances, skeletons, library).
- **Texture table layout.** A texture's file is the preimage of its existing content address (the header line `textureId` already hashed, then every level), gzipped at `<encoded hash>/texture.bin`; its gzip record lives in the catalog's one `gzip` table under the texture id, so the raw-hash check of `unpackGzip` is the texture-id check. The texture table proper is `textures`: bundle hash → its texture ids in the bundle's order, so the download can be counted from the catalog alone.
- **A bundle's travelling content** is its encoding with each texture's `levels` left out, under its own magic (`BGAT`), so it can never be read as a bundle. The art identity is verified on every read by joining the textures back and hashing the whole encoding (`gzip.ts` `readBundle`): the same browser hashing cost as before (it hashed every raw bundle), traded for keeping one identity rule. Rejected: recording a second hash for the stripped content.
- **The library counts toward the catalog load, not a map's download.** The loader fetches it with the catalog, so the map download is now what a map fetches on request beyond the load (kits, family looks, and their textures the load did not already fetch).
- **Catalog-load limit: 512 MiB** (`CATALOG_LOAD_MAX_BYTES`). Measured 2026-10-06: 129.6 MiB on the wire after this slice, against 1188.5 MiB before (eager raw bundles plus the library). About four times the measured load, room for Part B's detailed models and wrecks; a tripwire, not a target.
- **Texture-layer floor: 2048** (`TEXTURE_ARRAY_LAYERS_FLOOR`), what the Mac mini's adapter reports for `maxTextureArrayLayers` (queried 2026-10-06 in the scene runner's Chromium: apple, metal-3, 2048). Counted at bake over every baked appearance (an upper bound on any page): albedo 131, surface 234 today. Over the floor is a bake error, `budget.texture_layers`.
- **Stale test numbers fixed in passing.** Three loader tests still set kit and map sizes at the old 50 MiB limits (raised to 256 MiB in `78603b18`); they now use the constants.
## Slice 12 (implementation)

- **One owner for unit art rules: `packages/scene-assets/src/unitArt.ts` `UNIT_ART`**, a table keyed by `vehicleClass` or `soldier`, with a `default` row every class and missing field falls back to (as `scenery.ts` is per kind). Slice 14 adds class rows. Wrecks take the default ratio until slice 11 says whose wreck each is.
- **Recorded numbers, all loose tripwires** (not measured; raise when real art exceeds them and nothing visibly suffers): `tier_ratio` 0.9 (each tier draws at most 0.9 of the triangles of the one before). Today's least-reducing catalog unit is the VBL at 0.84. Dressing: `bulky_m` 0.3 (any dressing at most 0.3 m past any hull face), `thin_m` 0.15 (above that, only parts at most 0.15 m across), `thin_top_m` 4 (up to 4 m over the hull's top). Thin parts get no extra reach sideways. The class budgets (`tier_triangles`, `bundle_bytes`, `textures`) are empty and unenforced until slice 14.
- **Every new rule is an error from the start; no warning tier, no expected-failure list in code.** `asset validate` over all 130 catalog unit and wreck appearances (84 soldiers, 43 vehicles, 3 wrecks; 2026-10-07) finds no tier finding: every mesh names its tier, and the least-reducing step is 0.84. Severity therefore never has to let a bake through, and lanes can't drift. Rejected: warnings that the closeout requires empty (the coordinator's suggestion). Nothing would have used them, and they would have let a lane's new art slip through until slice 19.
- **Expected failures: the 86 disabled-card placeholders** (`assets/source/roster/disabled/*.glb`, judged `--unit vehicle`). Every one fails `structure.tier_unsuffixed` (all its meshes are unsuffixed), and they add 258 `structure.tier_ratio` findings, three tier steps each, all tiers equal. They already fail the older errors too (`structure.tier_count` and `nodes.missing` on all 86, `basis.ground` on 81). They are not catalog appearances and never bake. Slice 18 rebuilds them and slice 19 requires the list empty.
- **Unsuffixed unit meshes are refused, not copied.** `build.ts` still copies an unsuffixed mesh into every tier for scenery and kits, where that is the documented convention (`blender/city/README.md`). For units and wrecks `structure.tier_unsuffixed` is an error, so such a bundle never bakes. The tier rules live in `unitArt.ts` (`tierFindings`), not in `build.ts`, which stays about building.
- **Dressing is marked by name on an empty: `dressing_*`, whose meshes hang under it.** The articulated bundle keeps empties as nodes, and mesh names are lost to the merge. Nested dressing is measured with its outermost node. It is measured at rest on tier 0, as hull fit is. It is an error from the start (`fit.dressing`, the scenery dressing code reused: same concept, so dressing can't read as cover). Thin versus bulky is judged by geometry, not by a name: whatever rises above the bulky margin must be at most `thin_m` across. So one antenna per dressing node.
- **`hull_top_m` removed entirely**, not just its two overrides. It existed only as the antenna and cupola allowance; dressing owns that now, so hull fit uses one tolerance (`hull_extent_m`) on every face. The fit hint no longer invites widening.
- **The generic tank and jeep mark their roof parts as dressing.** The parts are the antennas, plus the tank's CITV sight (0.18 m over the box) and wind-sensor mast (exactly at the tolerance edge). This is done in `tank.py`/`jeep.py` with identity `dressing_*` empties; the wreck states remove the new empties with their antennas. Blender isn't installed here, so the committed GLBs were edited to the same hierarchy (JSON only, binary chunk byte-identical). Triangle counts and bounds are unchanged, and both validate clean with no override. Their runtime bundles are stale until the coordinator's next bake: the bundles gain the empties, `asset check` reports them stale, and no outcome moves.
- **`budgetFindings` generalised.** It now takes a `Budget` (`schema.ts`: tier triangles, bundle bytes, texture layers; each enforced where set). Scenery kinds and unit classes share it. Unit bundle bytes are the encoded bundle, encoded only when a class sets the budget. Texture layers are the bundle's distinct textures. The new codes are `structure.tier_unsuffixed`, `structure.tier_ratio`, `budget.bundle_bytes` and `budget.unit_textures` (renamed at integration from `budget.texture_layers`, which slice 10 uses for the GPU array layer count).
- **An appearance drawn by types of several classes is held to each class's rules**, with identical findings reported once. A vehicle no type draws takes the default row.
- **Synthetic test art now meets the rules.** Every part names its tiers, and each tier drops parts so it reduces. The block's coarser tiers draw fewer copies of the walls. The valid-assets test still requires zero findings.
- **Engine sounds reported again from another session** (2026-10-07): roster vehicles play the default engine sound because rows are keyed `tank`/`supply`/`jeep`. That is slice 01's fix, merged on this branch (vehicle class from physics, generic rows reused), and reaches main when this branch is pushed.
