# Armory

A player-facing unit explorer, reached from a new main-menu tile. It shows every
playable unit of the three factions in nine groups, with a full stat and weapon
sheet. The selected unit stands on a static 3D stage: a military yard, with
idle life, that you can orbit and zoom. The same work turns every menu page
into a URL path. It authors a military prop set (destructible hangar,
revetments, HESCO, tents, flagpole, nets over supplies), and scatters rare
military depots across generated maps.

[`unknowns-map.html`](unknowns-map.html) is the record of how the plan was
reached: settled ground, interview, landmine sweep. Its decision ledger is the
**given** this plan builds on: Q1′–L14, U1–U9, D1–D2, W09. Where this README
and the map disagree, this README wins. The disagreements are listed in
[What changed after the map](#what-changed-after-the-map).

## Next Agent Prompt

**Status (2026-11-13):** slices 01–13 are merged; the Armory is playable on the placeholder yard into
`t3/unit-explorer-armory`, and staging is merged in (`736eff7d8`). The
integrated tree is green on the cross-lane checks:
- contract weapons, showground, showcase;
- sim digest, replay, parity, operator and catalog;
- typecheck and the focused vitest files.

**The kill switch did not trip.** The pose driver runs from its own feed frame.

**L1 has lifted:** `tracked-armour-models` closed into `specs/done/` on staging,
so slices 14–17 may start. Run them one lane at a time while disk is tight.

**Pickup:**
1. The `armory/mil` lane is running slices 14 and 17. Merge it when it lands.
2. Then **15** (military art) and **16** (the yard swapped to it). Slice 16
   also owns the flat sand ground and its tan-on-tan contrast.
3. Then **18**. Add these to its run:
   - the `street` app-flow tour, which now includes an Armory round trip;
   - the first open of `/armory`, which takes 15–19 s while the menu's
     background prefetch downloads every runtime file. Judge it against the
     load line;
   - slice 08's sheet at 1280, cut off with no scroll cue;
   - a jet's "Sight · ground 1 m". The contract requires `ground_m > 0`
     (`contract/src/catalog.rs` `check` on sensors), so the air-war profile
     writes 1 m for "blind to the ground". Allowing 0 is a contract change for
     the user. Until then, the sheet shows the true number.

The lanes worked from
[`throwaway/armory/LANE-BRIEF.md`](../../throwaway/armory/LANE-BRIEF.md), an
ignored scratch file. Its rules are restated here: one worktree per lane, and
the orchestrator merges.

**Integration notes for the next slices:**
- **08:**
  - Add the `armory` row to `MENU_PAGES` (plain data) and to `CONTENTS` in
    `menu/MenuVisit.tsx`.
  - Import `menuPageOf` from `router.tsx`.
  - Read `catalog.weapons[id].derived` (`WeaponRow`/`Reach` in
    `packages/scene-assets/src/units.ts`).
  - The group is decided per card. Decide whether a family takes its group from
    its first typed variant.
- **11:**
  - `MenuShell`'s `<LoadingTasks>` must wrap the plate's children before the
    stage can report `renderer`.
  - Construct `new ArmoryAppearances(units)`, and pass `art.runtime`, not
    `gameRuntime`, to `fieldedBattle`.
  - An `AbortError` from `show` is a normal switch.
  - Call `showLoops([])` on leave.
  - The showcase needs a presentation with no battle loaded, and the renderer's
    tables must be reinstalled after `setShowcase`.
- **11 (from slice 10):**
  - `ArmorySelection.scenery` is `yardScenery(yard, units.view.props,
    runtimeCatalog)`. Fetch `/catalog.json` through `ArmoryAppearances`' own
    fetch so `library.load` reuses the bytes.
  - Move the showcase lab's yard mode onto `ArmoryAppearances`.
  - A sound setup passes `catalog.rules.weapons`, never `catalog.weapons`.
  - `plainWorld` lives in `workbench/benchWorld.ts`, which awaits
    `loadPresentation()` at top level. Move it to a neutral module if the
    Armory's bundle cares.
  - The ground's edge shows as a band on the horizon; settle it with the
    backdrop.
- **12:** spot cameras are fixed per spot and sized for the largest body, so a
  helicopter on the apron looks small. Framing per unit belongs to 12.
- **16:** a concrete apron needs a per-spot ground in the yard schema.

**Raised to the user, not retuned (one owner, the battle's data):**
- The rotor-blur look reads as "frozen blades over a glass plate".
- The idle squad reads as combat-ready.

Both are the battle's own looks (`presentation.json`), so changing either is a
battle-wide decision for the user.

**Known failures found along the way**, each to fix in its own commit:
- **Four `unit-roster` baselines** (`journey-zoom`, `purchase-ghost`,
  `journey-placed-facing`, `journey-space-orders`) differ only in tree foliage.
  The leaves were reworked after those pictures were approved on 2026-10-29.
  Re-approve them after looking.
- **The `street` app-flow tour couldn't run in a lane checkout:** the menu
  never showed. On the merged tree the cause was an unpulled effect flipbook
  (`assets/third-party/effects/*.png`). Its refusal now names the pull
  (`c0198f3dd`). Before any browser scene, pull `assets/third-party/**`,
  `assets/runtime/**` and the menu art.

**Blockers and warnings:**
- **Generated files:** slice 05 re-blesses `fixtures/catalog.json`. If it
  conflicts on merge, re-bless it; never merge it by hand.
- **Stale branch state:** other sessions merge and delete checkouts, and
  `staging` can be force-pushed. After pushing, verify with
  `git merge-base --is-ancestor`.
- **Disk:** the machine ran out on 2026-11-12, with five lanes building.
  Run at most two Rust-building lanes at once, with `CARGO_INCREMENTAL=0`.
  TypeScript-only lanes copy the base wasm (same sources) instead of building
  it. Remove each lane's worktree as soon as it is merged.
- **Before you ask the user:** read the map. Most questions are already
  answered there.

**TODO:**
- [x] [01](slices/01-menu-routes.md) Menu routes: shell above `PageVisit`,
  paths, hard cutover.
- [x] [02](slices/02-showground.md) Showground: the squad at rest and its
  original carriers, exported from the simulation.
- [x] [03](slices/03-showcase-entry.md) Showcase entry in the presentation
  crate, and `idle_loops`. **This is the kill switch.**
- [x] [04](slices/04-showcase-lab.md) Showcase lab: `ShowcaseStage` and
  `/labs/showcase`. The spike is verdict-visible here.
- [x] [05](slices/05-fires-regardless.md) `fires_regardless` and `reach`
  published, W09 amended.
- [x] [06](slices/06-unit-groups.md) Nine unit groups in the Reinforcements
  picker and the tutorial.
- [x] [07](slices/07-show-loops.md) `AppAudio.showLoops` and quiet tiles.
- [x] [08](slices/08-armory-page.md) Armory page: the sheet model, layout A,
  the tile and its key art.
- [x] [09](slices/09-armory-appearances.md) The Armory's own appearance
  library.
- [x] [10](slices/10-yard-document.md) The yard document and spots, with a
  placeholder yard.
- [x] [11](slices/11-armory-stage.md) The Armory stage: composition, loading,
  detection, errors, backdrop.
- [x] [12](slices/12-armory-camera.md) The Armory camera.
- [x] [13](slices/13-steading-refactor.md) Steading refactor: zero output
  change.
- [ ] [14](slices/14-military-rows.md) Military prop rows and
  mechanics, on stand-ins.
- [ ] [15](slices/15-military-art.md) Military art: one prop per
  gate, bake.
- [ ] [16](slices/16-military-yard.md) The yard swapped to the
  military set.
- [ ] [17](slices/17-depots.md) Depots in open country.
- [ ] [18](slices/18-closeout.md) Closeout: full run, balance, shots,
  `close-spec`.

**Before you end every pass,** update this section: the status, the date, the
pickup point, the blockers and the TODO. Record any decision this plan left to
you in [`decisions.md`](decisions.md).

## Contracts this spec changes

| # | Contract | After | Slice |
|---|---|---|---|
| K1 | Menu addresses | `/`, `/skirmish[?…]`, `/tutorial`, `/settings`, `/developer`, `/armory[?unit=<card id>]`. `menuHref` emits `/skirmish?…`. `/?type=…` opens the bare menu (hard cutover, no redirect). Unknown paths show the menu. | 01, 08 |
| K2 | Tutorial media URL | `/tutorial-media/<name>`. The folder `assets/runtime/tutorial/` is renamed so `/tutorial` is a route only. | 01 |
| K3 | game-wasm `Showground` | `new Showground(rules_json)`. `.squad(kind, seed)` returns `SquadAtRest` JSON (soldier slots and positions, the original carrier of each mount). | 02 |
| K4 | Presentation showcase | `Presentation::set_showcase(fielded, input)` and `clear_showcase()`. The free fn `idle_loops(tables, vehicle_class)` returns loops with the battle's replacements applied. All of these go through presentation-wasm. | 03 |
| K5 | Catalog-view weapon rows | The serde fields are unchanged. A new `derived: { fires_regardless, reach[] }` comes from contract methods. The rules identity, `config_digest` and battle digests don't move. | 05 |
| K6 | Unit groups | `UNIT_GROUPS` and `unitGroup()` in `purchaseFilter.ts`, read by the picker and the Armory. `Category` stays the game rule. | 06 |
| K7 | `AppAudio` non-battle loops | `appAudio.showLoops(loops)` on the one audio context. `[data-cue-hover="off"]` silences the hover cue. | 07 |
| K8 | Yard document | `fixtures/armory/yard.json`, typed by `YardDocument`. A presentation document, never admitted by the simulation. | 10 |
| K9 | Prop catalog | Gains the military rows. `PropKind` ranks renumber: the native counts test, the publication-reader parity, the street-lab digest and `config_digest` move. These are named moves. | 14 |
| K10 | Map presets | `open_country.depots`, a `Steads` block with prop-body pieces. `revision` goes to `layout-presets-21`. Generated maps change. | 13, 17 |

**Unchanged:** the battle state digest and replays, apart from the moves named
for K9 and K10; the publication layout; the render records; the bundle formats;
the native client. Nothing is backward-compatible and nothing is migrated.
Every change above is a hard cutover (decided by the user,
[decisions.md](decisions.md) P1).

## Slice graph

```
 can start now                                                  after L1 (lifted)
 ────────────────────────────────────────────────────────────   ──────────────────────────
 01 menu routes ─────────────────────────────┐
 06 unit groups ─────────────────────────────┤
 05 fires regardless ────────────────────────┴─► 08 armory page ─┐
 02 showground ─┐                                                │
 03 showcase ───┴─► 04 showcase lab ─► 10 yard document ─────────┼─► 11 stage ─► 12 camera
 07 show loops ──────────────────────────────────────────────────┤
 09 armory appearances ──────────────────────────────────────────┘
 13 steading refactor (byte-identical) ─────────────────────────────────┐
                                        [L1] 14 military rows ─► 15 military art ─► 16 military yard
                                                       └────────────────┴─► 17 depots
 everything ─► 18 closeout
```

- **Lanes from day one:**
  - menu: 01;
  - presentation: 02 → 03 → 04;
  - catalog: 05;
  - independent: 06, 07, 09, 13.
- **08** needs 01, 05 and 06. **10** needs 04. **11** joins 07, 08, 09 and
  10. **12** is last on the stage.
- **The Armory ships to players at 12, on the placeholder yard.** Slices 14–17
  upgrade it; L1, their gate, lifted on 2026-11-13.

## Review map

| Slice | The human runs or sees | The one verdict |
|---|---|---|
| 01 | `/settings` directly; Back and Forward; refresh `/skirmish?…` | Menu screenshots are byte-identical; the backdrop never restarts |
| 04 | `/labs/showcase`: a tank, a grounded helicopter, a squad | Idle life reads as alive and grounded |
| 06 | The skirmish picker's nine tabs at 1280 | The tab strip reads and fits |
| 08 | The menu tile, then `/armory`, `/armory?unit=…` | Layout A's list and sheet legibility |
| 10 | `/labs/showcase?yard=1` | The composition at each spot |
| 11 | `/armory` with the 3D stage live | The unit is seated in its spot |
| 12 | Drag and wheel on `/armory` | Framing at the extremes |
| 15 | The workbench, one prop at a time | Each prop's silhouette (one gate per prop) |
| 16 | `/armory` on the military yard | "A lived-in military yard" |
| 17 | `/labs/map-picture` on large seeds | A depot reads as a military compound, sited by a road |

**Standing visual gate for every slice that produces a shot:**
1. Use `game-ui` before any player-facing work, and `renderer` before renderer
   work.
2. Judge the shot with
   [`compare-screenshots`](../../.agents/skills/compare-screenshots/SKILL.md)
   whenever there is a prior look or a reference.
3. Run [`screenshot-critique`](../../.agents/skills/screenshot-critique/SKILL.md)
   as the **last** check on every shot, unprimed.
4. The human checkpoint is **non-blocking**:
   - open the shots with `preview-shots`;
   - wait about 5 minutes;
   - if the user is silent, decide on the evidence and record the decision in
     [`decisions.md`](decisions.md);
   - close the shots and proceed.

**References** live in [`assets/references/`](assets/references/). Gather them
in slice 10 (the yard) and slice 15 (the props). Record the source URL, the
licence or provenance, and whether each image is a photo or generated in
`assets/references/references.json`. Never judge a prop from a generated image
alone. The mock the user chose (layout A) is in
[`assets/mock/`](assets/mock/), with its shots.

## Single-owner invariants

| Concept | Owner | Never |
|---|---|---|
| Menu paths, titles, tiles | `apps/battle-lab/src/menu/pages.ts` `MENU_PAGES` | a page list in `MainMenu`, a `screenForPath` case list, a path literal in tests |
| Backdrop, detection, warm-up, cover | `MenuShell` above `PageVisit` | a page mounting `MenuBackdrop` or calling `runAutoDetect` |
| Skirmish address | `battleLinks.ts` `menuHref`, `askedChoice` | `/?type=` anywhere; a redirect |
| Idle poses, rotor spin and blur, faction looks, operator kits | `crates/presentation` showcase, which reuses the driver, `Drawn` and `Fielding` | TypeScript posing (`posedPalette` or `sampleClip` in Armory code), a TypeScript rotor disc |
| Squad scatter, original carriers | `sim::arrangement::squad_spots`, `sim::weapons::original_carriers`, via `Showground` | a TypeScript disc scatter, as in the mock |
| Idle sound and its replacements | `presentation::sound` (`vehicle_loops`, `SoundTables::effect`) | `effects[name] ?? name` in TypeScript |
| Non-battle voices | `AppAudio.showLoops` | a second `AudioContext`; using `createBattle` for the Armory |
| Fires-regardless; reach per band | `contract::WeaponDefinition` methods, published under `derived` | `default && unlimited` anywhere; a sim-private copy |
| Unit groups | `purchaseFilter.ts` `UNIT_GROUPS`, `unitGroup` | a group read by Rust, a rule or the AI; a second tab list |
| Mount names | `panelRows.ts` `equipmentName` (exported) | sheet-local naming |
| Armory art residency | `ArmoryAppearances` | Armory units in the shared game library; `appResources.refuse` from Armory code |
| A live single-unit stage | `web/src/battle/render/showcaseStage.ts` `ShowcaseStage` | a second live stage class. (`SheetRenderer` stays the workbench's one-shot sheet readback, with page-posed models. It is a different job, not a duplicate.) |
| Yard layout and spots | `fixtures/armory/yard.json` | coordinates in TypeScript; a saved map |
| Flat world geometry | `WorldGeometry::flat` (promoted from the test helper) | the two test-helper copies, which become calls to it |
| Groups of pieces at road stations, even halves | `open_country` `stead` with `Steading` | a `depot()` that copies the station walk |
| Prop physics | prop catalog rows | any rule naming a hangar, net or tent |

The end state must read as if designed today, not as the old shape with the
feature bolted on. Code that each new owner replaces is deleted in the slice
that replaces it:
- `PAGES` in `MainMenu`;
- `LabPage`'s fallback to the menu;
- the picker's `CATEGORIES`;
- the private `fires_regardless`;
- the test-helper flat worlds;
- the `Homesteads` type, which becomes `Steads`.

## What changed after the map

- **P1. Hard cutover.** Old `/?type=…` addresses stop pre-filling Skirmish.
  This replaces Q4's no-regression item 3. The other five items stand.
- **P2. The yard spot is chosen from the unit's body**, not its category:
  - a squad → the infantry area;
  - a hull with a flight model → the apron;
  - any other hull → the motor pool.

  `rec` holds 6 squads and 8 vehicles, so Q5's literal "category picks the
  spot" would put recon squads in the motor pool. This keeps Q5's intent and
  reads a physical property.
- **P3. Depots get their own lot rule**, sized from the catalog bodies of their
  pieces. Homestead lots are 8/7/8 m and can't hold a hangar. Placement still
  uses the homestead station walk and its even-halves rule (L4).
- **P4. The catalog publishes `reach`** (per band: range, the class limit only
  where it narrows the band, intercepts) next to `fires_regardless`. The
  "narrows a band" judgement then stays in the contract crate with
  `TargetClass::found_in`, and isn't copied into the sheet.
- **P5. Idle loops are resolved in Rust** (`idle_loops` applies
  `SoundTables::effect`), and `AppAudio` plays names it is given. This answers
  "Confirm before coding" item 3 by construction.
- **P6. No new asset workbench.** The existing `/labs/workbench` inspects a
  single prop; `/labs/showcase?yard=1` shows the assembled yard.

[`decisions.md`](decisions.md) records each of these with its reason, and the
alternatives the drafts proposed and why each lost.

**Out of scope:** splitting the `default` weapon flag's three jobs (W09b). It is
its own placeholder spec, [`default-weapon-cleanup`](../default-weapon-cleanup/README.md),
which builds on slice 05. Plinking with an unlimited default gun is intentional
and cinematic; never remove it.

## Known unknowns that a slice settles

| Unknown | Settled by | If it fails |
|---|---|---|
| Can the driver pose from a synthetic feed, without a publication? | 03 | Stop and escalate to the user. Never use TypeScript posing. |
| Is the rotor-disc emission separable from the battle's effect state? | 03 | Reslice 03 into its soldier, rotor and sound parts. |
| Do parked aircraft show stores through the ground? | 04 (probe, needs LFS) | Name the model to the user. The art fix goes to that model's next pass. |
| Do both scenes (backdrop and stage) hold the frame budget? | 11 (measured) | `holdBackdrop`: the backdrop stops presenting but keeps its clock and worker. |
| Do nine tabs plus search fit at 1280? | 06 | Shorten the labels (a table edit), or collapse the search to an icon. |
| Switch time and download headroom (S5, S7) | 11, then 14 | Record the measurement. A tripwire breach is a named decision, never a silent raise. |
| Can a hangar lot fit at any road station? | 17 | Leave the depot out (never refuse). Adjust the lot or the group mix. |
