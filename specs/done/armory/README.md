# Armory

The Armory is the player's unit explorer, opened from its own main-menu tile
at `/armory`. It lists every playable unit of the three factions in nine
groups, each with a full stat and weapon sheet. The open unit stands on a
live 3D stage: a military yard with the battle's own idle life and idle
sound, which the player orbits and zooms. The same work did four other
things:
- it made every menu page a URL path;
- it published two weapon facts from the rules;
- it authored a military prop set: a destructible hangar, revetments, HESCO,
  tents, a flagpole, and nets over supplies;
- it scatters rare military depots, built from those props, along country
  roads on generated maps.

This README is the record of why it is shaped this way and what must stay
true. The code holds the how. Three records sit beside it:

- [`unknowns-map.html`](unknowns-map.html): the quadrant walk the plan was
  built from (rows Q1′–L14, U1–U9, D1–D2 and W09). It is historical, and
  bannered where the build diverged.
- [`decisions.md`](decisions.md): what the user decided, and what the agents
  decided at each visual checkpoint.
- [`choices.md`](choices.md): every choice the agents made on the user's
  behalf, audited against the shipped code. Its **user-owned** group holds
  the calls still open for the user, each with its provisional answer.

## Visual provenance

- **The layout is mock A**,
  [`assets/mock/a-tank.png`](assets/mock/a-tank.png): a list panel on the
  left, a sheet panel on the right, and the yard filling the screen between
  and behind them. The user chose it over [`b-ba.png`](assets/mock/b-ba.png)
  and [`c-dossier.png`](assets/mock/c-dossier.png), all three drawn from
  [`armory-mock.html`](assets/mock/armory-mock.html). It also stands in for
  the genre's framing (WARNO, Broken Arrow), whose armory screens aren't
  freely licensed and so aren't stored.
- **The yard and the props were judged against real photos only**, in
  [`assets/references/`](assets/references/). Their sources, licences and
  what each one drove are in
  [`references.json`](assets/references/references.json): flight lines and
  hangars for the apron, a tank line and T-walls for the motor pool, tent
  cities for the camp, and one or more photos per prop. A generated image
  never judged a prop.
- **A parked helicopter** was checked against a parked AH-64D in the repo's
  own reference library (`assets/references/ah_64_apache/` at the root): on
  its wheels and tail wheel.
- **The menu tile's key art** is `assets/source/menu/armory.png`. It is
  generated in the same model and framing as the other tiles.

The art's scores against these references, and what still falls short of
them, are in `decisions.md` and the art-quality entry of `choices.md`.

## Why it works this way

**The Armory observes the battle; it never re-implements it.** A unit in the
Armory must look, move and sound exactly as it does in battle. Every one of
these comes from the owner the battle already uses:
- the idle pose, faction look and operator kit come from the presentation's
  driver, fed a synthetic unit through `Presentation::set_showcase`;
- the squad's scatter comes from `sim::arrangement::squad_spots`, through the
  wasm `Showground`;
- who carries which gun comes from `sim::weapons::operators::original_carriers`;
- the idle sound and its replacements come from `presentation::sound::idle`
  (`idle_loops`).

There is no TypeScript posing, no TypeScript rotor disc and no TypeScript
sound table. The plan's kill switch was "if the driver can't pose a unit
without a battle, stop". It never tripped.

**A parked aircraft stands still and silent** (the user's call). The
showcase stands every unit on the ground (`FeedUnit::grounded`). A flying
type that is grounded poses as a ground vehicle: no rotor travel, blur or
flight lean. Its idle loop is silence, because a running rotor reads as
about to take off. Battles never set the flag, so their frames don't move.

**The yard spot follows the body, not the category.** A squad stands at the
tents, a hull with a flight model on the apron, and any other hull in the
motor pool (`yard.ts` `spotFor`). The recon category holds both squads and
vehicles, so a category rule would have put recon squads in a motor pool.

**The yard is presentation data the simulation never admits.**
`fixtures/armory/yard.json` names its spots, its props by kind, its paved
pads and the relief beyond it. Art names come from the runtime catalog. Past
its own ground, the presentation draws the battle's own backdrop: the
biome's patchwork, hedgerows and trees, rising over the yard's relief.
Relief past the yard is relief past a map's edge, so the backdrop owns it
(`BackdropRelief`). The yard and every spot stay at height 0.

**Each show stands on a presentation of its own.** A pick builds a fresh
presentation and scene and frees the old pair once the new one stands, as a
battle never reuses a presentation across scenes. Reusing one let a cache
outlive its scene, and the GPU refused the frame. Every switch therefore
places the ground and scenery again, and a held, blurred last frame covers
the wait.

**The Armory's art never touches the battle's.** `ArmoryAppearances` gives
each selection its own appearance library. It downloads the yard and catalog
once and drops the last unit's bytes on the next switch. A superseded pick
stops fetching, and a failure stays on the stage behind its own boundary.
The page itself loads lazily with its visit, so nothing of the Armory
reaches the menu until a player opens it.

**The weapon facts are the rules', published.** "Fires at what it cannot
hurt" (`fires_regardless`) and its reach per altitude band, including where
a target-class limit narrows the band, are `contract::WeaponDefinition`
methods. They are published under `derived` on the catalog's weapon rows,
in the browser through the wasm `publish_weapons`. The sheet and the
mechanics editor read them and never recompute them. The rules identity and
battle digests didn't move. Harmless plinking with a default gun is
intentional and cinematic.

**Military props are rows, not rules.** A hangar, a revetment or a tent does
what its body row says: blocking, stopping rounds, occluding, cover tier,
weight, `hp` and what it falls into (`fixtures/props/military/`). No rule
anywhere names a hangar, a net or a tent. A net covers only supplies,
because the only depot piece that names a net is the supply stack.

**Depots reuse the homestead walk with their own lots.** A depot stands at a
station on a country road, through the same station walk, spacing and
even-halves rule as homesteads (`Steads<P>`, read through `Pieced`). Its
lots are sized from its pieces' boxes, because a homestead lot can't hold a
hangar. A depot is one compound: its yards may touch, all of it stands on
one side of its road, and it stands whole or not at all. Depots are placed
after homes on their own random stream, so a map with no depot station is
byte-identical to one generated before depots existed. A paired balance
report showed depots change the ground, not who wins.

**Every contract change was a hard cutover** (the user's call). Old
`/?type=…` addresses open the bare menu, and nothing is migrated.

## Contracts it set

- **Menu addresses:** `/`, `/skirmish[?…]`, `/tutorial`, `/settings`,
  `/developer`, `/armory[?unit=<card id>]`. `menuHref` emits `/skirmish?…`,
  and unknown paths show the menu. Tutorial media moved to `/tutorial-media/`
  so `/tutorial` is a route only.
- **`Showground`** (game-wasm): `new Showground(rules_json)`, and
  `.squad(kind, seed)` returns a squad at rest with its slots, offsets and
  each mount's original carrier.
- **The showcase** (presentation): `set_showcase(fielded, input)` shows a
  unit in place of the last, and refuses while a battle is loaded. The wasm
  `idle_loops(setup, kind)` returns the loops a stationary battle vehicle
  starts, with replacements applied, and none for a type that flies.
  `FeedUnit::grounded` is the driver's one "on the ground" field.
- **Weapon rows:** a `derived: { fires_regardless, reach[] }` beside the
  unchanged serde fields. An authored `derived` is refused.
- **Unit groups:** `UNIT_GROUPS`, `unitGroup` and `familyGroups` in
  `web/src/battle/present/purchaseFilter.ts`. They are presentation only:
  `Category` stays the game rule.
- **Non-battle audio:** `AppAudio.showLoops(loops)` on the one audio
  context, and `[data-cue-hover="off"]` silences the hover cue.
- **The yard document:** `fixtures/armory/yard.json`, admitted by `admitYard`
  (`apps/battle-lab/src/armory/yard.ts`). Its one documented home is
  `fixtures/README.md`.
- **The Armory camera:** `camera_armory` in `fixtures/presentation.json`,
  admitted in Rust (`settings/ui.rs` `ArmoryCamera`). In TypeScript,
  `CameraPresentation` (renderer-core's `cameraController.ts`) carries each
  camera's own `pitch_limits`.
- **The backdrop relief:** `Presentation.set_ground(…, relief?)` and the wasm
  `BackdropRelief(json, map)` with `.height(x, y)`. `Relief::footprint()` is
  the contract's one owner of a feature's box.
- **The prop catalog** gained the military rows. **Map presets** gained
  `open_country.depots`, at `layout-presets-21`. Both moved the generated
  maps, the prop ranks and the digests that embed them, as named moves.
  Battle state digests and replays moved nowhere else.

## Single-owner invariants

| Concept | Owner | Never |
|---|---|---|
| Menu paths, titles, tiles | `apps/battle-lab/src/menu/pages.ts` `MENU_PAGES`; contents in `menu/MenuVisit.tsx` `CONTENTS` | a page list elsewhere, a path literal in tests |
| Backdrop, detection, warm-up, cover | `MenuShell` above the page | a page mounting the backdrop or running detection |
| Skirmish address | `battleLinks.ts` `menuHref`, `askedChoice` | `/?type=` anywhere; a redirect |
| Idle poses, rotors, faction looks, operator kits | `crates/presentation` showcase, on the battle's driver and `posing.rs` | TypeScript posing or a TypeScript rotor disc for a shown unit |
| "On the ground" | `FeedUnit::grounded` | a rotor-only flag; a type check in the showcase |
| Squad scatter, original carriers | `sim::arrangement::squad_spots`, `sim::weapons::operators::original_carriers` | a TypeScript scatter; a second first-claim search (spawn reads the same owner) |
| Idle sound and its replacements | `presentation::sound` (`idle_loops`, `SoundTables::effect`) | `effects[name] ?? name` in TypeScript. (The sound workbench's `Movement.tsx` predates this and still keeps such a copy.) |
| Non-battle voices | `AppAudio.showLoops` | a second `AudioContext`; `createBattle` for the Armory |
| Fires-regardless; reach per band | `contract::WeaponDefinition`, published under `derived` | `default && unlimited` anywhere; a recomputation in the sheet |
| Unit groups, a family's group | `purchaseFilter.ts` `UNIT_GROUPS`, `unitGroup`, `familyGroups` | a group read by Rust, a rule or the AI; a second tab list |
| Mount names | `panelRows.ts` `equipmentName` | sheet-local naming |
| Sheet stats and their words | `unitSheet.ts` `SHEET_STATS` | labels in the page |
| Armory art residency | `ArmoryAppearances` | Armory units in the shared game library; `appResources.refuse` from Armory code |
| A live single-unit stage | `web/src/battle/render/showcaseStage.ts` `ShowcaseStage` | a second live stage class. (`SheetRenderer` stays the workbench's one-shot readback: a different job.) |
| What a stage is built and judged from | `armory/stageInputs.ts`, `armory/stageProbes.ts` | per-page copies in the Armory and the showcase lab |
| Yard layout and spots | `fixtures/armory/yard.json` | coordinates in TypeScript; a saved map |
| Height past the yard | `BackdropRelief` | a height grid for the yard; a second relief reader |
| The world's scenery art | `worldMesh.ts` `worldSceneryNames`, `worldScenery` | a tree list in the yard |
| A flat map | `MapDefinition::flat`; `WorldGeometry::flat` builds on it | a test helper's own flat map |
| Groups of pieces at road stations | `open_country/rules.rs` `Steads<P>` and `Pieced`; the station walk in `open_country/steads.rs` | a `depot()` that copies the station walk; a variant taken back out under `unreachable!` |
| A depot's lot id | `open_country/rules.rs` `Pieces::depot_of` | a second parser |
| Prop physics | the prop catalog's rows | any rule naming a hangar, net or tent |

One known break: placed-prop boxes. Depots read them from
`street_props.bodies`, but the homes' and field cover's rows still carry
their own copies (`choices.md`, the unsound entry).

## Where to look

- **Page and stage:** `apps/battle-lab/src/armory/` (`ArmoryPage`,
  `ArmorySheet`, `ArmoryStage`, `armoryAppearances`, `armoryCamera`, `yard`).
  The sheet model is `web/src/battle/present/unitSheet.ts`.
- **Showcase:** `crates/presentation/src/presentation/showcase.rs`,
  `crates/presentation-wasm/src/showcase.rs`, `crates/game-wasm/src/showground.rs`.
  The lab is `/labs/showcase` (`apps/battle-lab/src/routes/showcase.tsx`).
- **Military art:** `packages/scene-assets/blender/military.py` and its
  family modules. The rows are in `fixtures/props/military/`.
- **Depots:** `crates/mapgen/src/open_country/steads.rs` and `rules.rs`. The
  numbers are in `fixtures/map-presets.json` `open_country.depots`.
- **Fog on low tops:** `fog.wgsl` `fogSeenSurface`. A top just under an eye
  takes the roof rule.
- **Tests that pin it:** `web/tests/armory*.test.ts(x)`, `unitSheet.test.ts`,
  `purchaseFilter.test.ts`, `appAudio.test.ts`;
  `crates/game-wasm/tests/showground.rs`; `crates/sim/tests/shelter.rs`;
  `crates/mapgen/tests/open_country/`; `renderer-core/tests/probes.rs`; the
  browser scenes `armory`, `showcase` and the `street` tour's round trip.

## Dead ends

- **Posing in TypeScript.** It was ruled out up front. The driver posed from
  a synthetic feed, so nothing was needed.
- **Rotors turning on a parked helicopter.** The first build blurred a
  parked helicopter's rotors, read as "rotors turning". It looked about to
  take off, and the user stopped them.
- **One presentation for every show.** A cache outlived its scene and the
  GPU refused the frame. This is why each show gets its own.
- **The yard as a real height grid.** Holding the hills needed a 3–4 km map,
  which pushed every treeline past the hills and compiled a million-sample
  world on each open.
- **A tree list in the yard.** It would have been a second placement of the
  battle's trees. The backdrop draws them.
- **A bounding sphere for framing.** A helicopter's rotor diagonal sized it
  loosely. Framing fits the rest-pose box per view angle.
- **Resource timing as the load line.** It counted the menu's background
  prefetch, 200–300 MB a switch. The load line is what the Armory holds.
- **`holdBackdrop`.** It was planned in case the backdrop and stage blew the
  frame budget. Measured, they didn't, so it was never built.
- **Placeholder-yard dressing.** The heras fence and dragon's teeth read as a
  building site, and the hangar ruin as a derelict airfield.
- **A bare earth-berm revetment** read as "a loaf". The revetment is a steel
  bin of earth, as in every reference. **Geometric ribs on the hangar** read
  as a ribbed barrel, so the corrugation is in the texture. **A 30 × 20 m
  hangar** rarely found a straight enough road.
- **Depots copying homes' per-piece road side** read as things dropped along
  a road, and **unpaved depot yards** read as boxes in a crop field.
- **Grouping per card.** A family could split across two tabs.
  `familyGroups` groups per family for both browsers.

## Open at close

- The calls in `choices.md`'s user-owned group: art quality, depot density,
  props on the lobby's map picture, the battle-wide rotor blur and idle
  stance, a jet's 1 m ground sight, and the sheet's bar direction.
- The distant-impostors receipt (`specs/distant-impostors/baseline.json`)
  predates depots and wasn't re-recorded. It needs the browser benchmark.
- **Out of scope:** splitting the `default` weapon flag's three jobs (W09b)
  is its own placeholder spec,
  [`default-weapon-cleanup`](../../default-weapon-cleanup/README.md), which
  builds on the published facts.
