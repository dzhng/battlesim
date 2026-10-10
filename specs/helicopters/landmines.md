# Helicopters: landmines

Found by the 2026-10-10 sweep of the simulation, contract, browser, renderer, asset
and audio code. Each card names its owning slice in the [README](README.md) slice
table. Line numbers are as of that date; re-grep before you trust one.

**L1. Every hull is treated as a flat box on the ground.** *Decided: an airborne hull is not a ground body.*
- **Evidence:**
  - `vehicle_conflict` 2D OBB (`movement/mod.rs:1129`) and `drive::arc_clear` traffic;
  - `soldier::shove` (`mod.rs:473`, `soldier.rs:1039`);
  - `Threat::of` yield (`soldier.rs:180`);
  - cover from `lean::hulls` → `cover::hull_bodies`, where `take_cover.rs:217` checks only `top > muzzle`;
  - entry `apart(footprint)` (`battle.rs:530`);
  - `clear_lanes` fells forest (`battle.rs:1561`);
  - `treads()` (`battle.rs:1621`);
  - renderer track dust (`effectFrame.ts:969`) and wheel spin (`poseDriver.ts:843`).
- **Why it bites:** tanks would wait under a hovering helicopter, soldiers would be shoved from under it and take cover behind it, and it would carve lanes through forest.
- **What it changes:** one predicate, "is airborne", gates every one of these.

**L2. A dead helicopter's wreck floats, and the roll follows its nose.** *Decided: D3.*
- **Evidence:**
  - `base_z: Some(unit.position.z)` (`battle.rs:1982`);
  - `death_roll` rolls along the yaw (`drive.rs:496`);
  - `push::meet` ignores bodies already overlapped (`push.rs:57`);
  - `add_prop` does no overlap check or damage (`battle.rs:1333`);
  - renderer `cookOffs.ts:376` snaps x/y only, with 6 m reach and `REMEMBERED_S = 2` while a fall takes about 2.9 s.
- **What it changes:** the sim owns a new **falling airframe** state, which is published and digested. On impact it calls `destroy_prop` for trees, bounces off buildings, applies blast-style damage underneath and places the wreck on the ground. The renderer draws the fall from that state instead of the ground-roll cook-off.

**L3. Weapons can't tell air from ground.** *Decided: D1, D22.*
- **Evidence:** `can_damage`, `effective`, `compatible` and `preferred_kind` read armour only (`weapons.rs:391`, `423`, `1043`, `468`). Tank AP and ATGMs would target helicopters.

**L4. Helicopter hull-mounted weapons fire backwards.** *Decided: D8.*
- **Evidence:** `point_mount` (`weapons.rs:541`). The tolerance check applies only to turrets (`:1530`).
- **Also:** body yaw swings the sight cone (`sight.rs:55`). That's fine, but keep it in mind.

**L5. Facing is tied to travel direction.** *Sharp edge.*
- **Evidence:** `steer` heading, and `final_yaw` (`drive.rs:384`) gates on `tracked`, so a non-tracked unit falls into wheeled `line_up_heading` and loses its ordered facing. The `drive` helpers `expect("a vehicle has a drive")`.
- **What it changes:** helicopters need their own `step_aircraft` branch with a separate aim yaw. Don't route them through `step_vehicle`.

**L6. `moved` counts altitude.** *Decided: D10.*
- **Evidence:** `moved` (`battle.rs:1246`) is a 3D base delta. Bobbing or the roof+10 m climb would block supply and drop ATGM guidance (`battle.rs:2113` `keep = !moved[i]`).
- **What it changes:** compare XY only for helicopters, or compare the planned motion rather than the base.

**L7. Admission refuses helicopters.** *Sharp edge.*
- **Evidence:**
  - `MAX_SPEED_KMH = 130` (`contract/catalog.rs:175`);
  - `HullLimits::check` silently skips unknown mobility (`scenario.rs:618`, `_ => continue`);
  - the `admit_skirmish` match is exhaustive (`map_analysis.rs:170`).
- **What it changes:** a new `Air { cruise_kmh, … }` mobility with its own hull limits row.

**L8. Tests will break when cards become physical types.** *Sharp edge.*
- **Evidence:**
  - `disabled_model_manifest_covers_cards_without_admitting_units` (`crates/sim/tests/catalog.rs:15`): remove each converted entry from the manifest.
  - `every_unit_type_sets_up_fires_each_mount_and_moves` (`:459`): it has no altitude, and its targets are a rifle squad and a tank. Needs a `test_heli`.
  - The UI scene pins the HEL tab (`web/scenes/ui.mjs:30`).

**L9. Contacts are 2D.** *Decided: D11.*
- **Evidence:**
  - `Contact.center: V2` (`knowledge.rs:37`), `ApproximateContact.center [f64;2]` (`observation.rs:95`);
  - the web `ContactView.center: Point2` (`web/src/battle/sim/observation.ts:355`);
  - contact picking is a ground disc (`contactPick.ts`);
  - readouts are anchored to the ground (`readouts.tsx:196`).
- **What it changes:** a contract change that touches publication, the WASM packing and the digest.

**L10. Model pipeline.** *Sharp edge.*
- **Evidence:**
  - Hull-fit validation would count the rotor disc (`validate.ts:986`); exclude `rotor_*` and `blade_*`.
  - Culling bounds don't sweep the rotor (`pose.ts:256`).
  - There's no rotor articulation (`articulation.ts:102`). Reuse the `turned(...)` pattern with an accumulated angle.
  - Chin guns are static meshes with no gun or muzzle rig nodes (`apache.py:56`).
  - Gun pitch is clamped to -10°…20° (`articulation.ts:51`), but firing down from altitude needs about -22° or more.
  - `units.ts:270` treats non-tracked as wheeled.
  - **Crash:** `useBattleSession.ts:276` reads `road_kmh` from foot, tracked or wheeled mobility and throws on air.

**L11. Navigation can't read heights.** *Decided: D21.* Nav cells and bodies carry weight and stop flags, not height. `Prop::top_z()` exists (`world/props.rs:37`).

**L12. Camera can pass through a helicopter.** *Sharp edge, out of scope.* Camera obstacles are the ground and buildings only (`cameraObstacles.ts`). At about 65 m out the eye is about 49 m up, which is the helicopter band. Record it; fix it if it shows.

**L13. Shadows pop.** *Sharp edge.* `SHADOW_MARGIN_M = 3` (`modelDetail.ts:39`) culls a helicopter's offset shadow with its body.

**L14. Placement and destination ghosts sit on the ground.** *Sharp edge.* `unitGhosts.ts:79`; a helicopter's ghost shows it landed.

**L15. Budgets not measured.** *To confirm.* `asset check` caps all unit art at 512 MiB and texture layers at 2048 (`schema.ts:714,719`). There are 19 new models plus wrecks.

**Beyond the feature**
- D8 (instant non-turret aim) is a latent bug for every unit.
- O1: the comment contradicted the rule (now fixed).

## Owning slices

| Card | Slice |
|---|---|
| L1 | 01 |
| L2 | 04 (simulation), 10 (renderer) |
| L3 | 07 |
| L4 | 08 |
| L5 | 03 |
| L6 | 01 |
| L7 | 01 |
| L8 | 03, 15, 16, 17 |
| L9 | 05 |
| L10 | 06 (pipeline), 15 (rigs) |
| L11 | 03 |
| L12 | Recorded at 07; fixed at 18 only if a shot shows it |
| L13 | 06 |
| L14 | 12 |
| L15 | 15, 16 (resolved: there is headroom, see [decisions](decisions.md#confirmed-in-the-code-2026-10-10)) |
