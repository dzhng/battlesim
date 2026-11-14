# Armory: decisions

The decisions behind the plan are in [`unknowns-map.html`](unknowns-map.html):
Q1′–L14, U1–U9, D1–D2 and W09. This file records the decisions that came later.
Some were made while writing the plan; implementation adds the rest. Record
each one as a decision, the reason for it, and who made it.

## Made while planning (2026-11-12)

- **P1. Hard cutover. No compatibility layer and no migrations.** *User.*
  - Old `/?type=…` addresses stop pre-filling Skirmish, and `menuHref` emits
    `/skirmish?…`.
  - This replaces Q4's no-regression item 3. The map recommended keeping one
    redirect; the user chose the default of no compatibility.
- **P2. The yard spot is chosen by body:** a squad → `tents`, a hull with a
  flight model → `apron`, any other hull → `motor_pool`. *Agent, under the
  delegation; announced to the user.*
  - Why: two drafts measured `rec` and found 6 squads and 8 vehicles in it, so
    Q5's literal "category picks the spot" would put recon squads in the motor
    pool.
  - It keeps Q5's intent and reads a physical property.
- **P3. Depots get their own lot rule,** sized from the catalog bodies of their
  pieces. *Agent.*
  - Why: homestead yards are 8/7/8 m (`presets open_country.homesteads`) and
    can't hold a hangar. Station, spacing and even halves still come from
    `stead` (L4), through `Steading` (slice 13).
- **P4. The catalog also publishes `reach`** next to `fires_regardless`.
  *Agent.*
  - Why: the "a class limit narrows the band" rule (U7) needs
    `TargetClass::found_in`, which is owned by the contract crate. Computing it
    in the sheet would copy it.
- **P5. Idle loops are resolved in Rust,** with `idle_loops` applying
  `SoundTables::effect`. `AppAudio` plays the names it is given. *Agent.*
  - Why: the battle's sound replacements live in Rust. A TypeScript
    `effects[name] ?? name` would be a second owner. This answers "Confirm
    before coding" item 3 by construction.
- **P6. No new asset workbench.** *Agent.*
  - The existing `/labs/workbench` inspects one prop, and
    `/labs/showcase?yard=1` shows the assembled yard.
  - Why: the visual-pipeline draft proposed a workbench surface for the yard,
    but the existing lab pages already cover both views.
- **P7. Slice order.** *Agent.*
  - The picker's nine groups land before the Armory (slice 06 before 08), so the
    Armory imports a proven owner.
  - The steading refactor lands early with byte-identical output (slice 13), so
    depots reuse a proven path.
  - Physical rows land on stand-ins (14) before the art (15), so the digest
    moves and the art are judged apart.
  - The showcase spike (02–04) runs in its own lane from day one, as the kill
    switch.
- **P8. Rotor blur reads "rotors turning",** a property of the unit the
  showcase feeds in, not "airborne". *Agent.* **Superseded** by "Parked
  rotors are stopped" (below): the showcase grounds every unit, and a
  grounded type poses with still rotors and no blur.
  - Why it was chosen: in battle the two always coincide, so battle frames
    can't change.

## Alternatives the drafts proposed, and why each lost

- **Squad scatter through `WorldView.squad_spots` on an in-memory flat map**
  (fewest slices). It lost to `Showground` with `WorldGeometry::flat` (seam
  quality): that one constructor already exists twice in tests, and it carries
  the original carriers too.
- **The class restriction computed in TypeScript** from the roster's unit
  classes (fewest slices). It lost to P4.
- **U8 and the picker groups merged into the Armory page** (fewest slices). It
  lost to P7: separate seams, each proven by its first reader.
- **Military art as one slice** (seam quality). It lost to one gate per prop
  (visual pipeline), because each prop is a separate visual variable.
- **A download-headroom probe slice up front** (risk first). It was folded into
  slices 11 (measure the load line) and 14 and 15 (read the tripwire). Before
  the art lands, only today's catalog can be measured, and the Armory adds no
  every-map load.

## Recorded during implementation

Each visual checkpoint here passed on its evidence (the scene's probes,
compare-screenshots and an unprimed critique). No human replied within any
checkpoint window, so each was the agent's call under the delegation. What a
critique raised and the slice left open is in [`choices.md`](choices.md).

- **01. Menu pages became paths with no visual change.** The menu baselines
  matched the pre-slice run. The lobby draws a fresh seed each run, which can
  move its right-aligned "Ready"; that is not a regression. *Agent.*
- **04. A parked helicopter stands on its wheels.** Against the parked-AH-64D
  photo it sits on its main and tail wheels, with no float or sink. *Agent.*
- **04. The battle's rotor blur and idle stance stay the battle's.** A fresh
  eye read the blur as "frozen blades over a glass plate" and the squad's
  idle as combat-ready. Both are `fixtures/presentation.json` data that every
  battle reads, so neither was retuned; both went to the user. Since parked
  rotors stopped (below), the Armory shows no blur at all. *Agent.*
- **06. The search stays a field at every width,** and the nine-tab strip
  ships as is: the tabs fit beside the search at 1280 px with room to spare,
  and every label reads whole at 2×. *Agent.*
- **08. Layout A is accepted.** Left to the user: each bar is a magnitude
  scaled against the same figure across the roster (U9), so a slow reload has
  the long bar; a tile shows its family's cheapest price. *Agent.*
- **08. The menu changes only inside the Armory tile's band.** Above and below
  it, the new menu baseline matches the old one with no pixel differing.
  *Agent.*
- **08. The tile's key art comes from the Duet CLI, same model, second
  prompt.** The first prompt's wide shot was the odd tile out among the
  set's close framings. *Agent.*
- **10. The placeholder yard's composition is accepted:** each body stands on
  its spot with its own dressing, and no prop stands between a unit and its
  camera. *Agent.*
- **11. No `holdBackdrop`.** With the stage live, the menu backdrop still
  held the frame budget, measured once. *Agent.*
- **11. Switch time (S7) and the load line (S5) are loose tripwires,**
  measured once. A switch takes about a second at most; the first one after
  the menu shows is far slower, because the menu's background prefetch
  competes with it. That first open is worth a user look. *Agent.*
- **11. Every group stands grounded.** A unit of each of the nine groups
  stands on its spot with every model on the ground; a parked helicopter on
  its main and tail wheels. *Agent.*
- **12. The camera's extremes keep the eye clear** of the unit, the props and
  the ground. Keys, the screen's edges and drags on the panels never move it.
  The critique read the first pitch ceiling as a flat map view, so the
  ceiling came down. The far framing and each spot's opening angle are data
  for the user to tune. *Agent.*
- **L1 lifted (slice 14's gate).** `tracked-armour-models` closed and gave up
  `fixtures/props/**`, the catalog and the runtime bundles, so slices 14–17
  ran. *Agent.*
- **14. The hangar's destruction reads:** HE brings the standing hangar down
  to a low slab of the same plan. The ruin's top then drew as unseen in plain
  sight of infantry; that was a fog gap, fixed at closeout (below). *Agent.*
- **15. Each military prop passed its own gate against its real photos,**
  after three critiques each. Only the hangar meets the repo's model bar; the
  scores and the open notes are in `choices.md`'s art-quality entry. *Agent.*
- **16. The yard reads as a military yard.** Each spot has its own setting
  (an apron before hangars, a motor pool between revetments, a tented camp),
  all inside a HESCO perimeter and matched against the references.
  "Lived-in" would also want people and parked vehicles, which no prop is;
  that stays open. *Agent.*
- **16. Trees and ground variety come from the battle's backdrop.** The user
  asked for trees and livelier terrain. Past the yard's own ground the
  presentation draws the battle's biome patchwork, hedgerows and trees, and
  the yard's ground is the battle's ground material, so there is no new tree
  art or tree list. *User asked; agent chose how.*
- **16. One page crash during an `armory` run was treated as transient:** it
  did not repeat on an unchanged rerun. *Agent.*
- **17. Depots read as compounds.** On skirmish maps each depot reads as one
  compound on paved yards beside a road or track, and the plan picture rings
  each one. Raised to the user: the density (`open_country.depots.per_road_km`)
  and the lobby's map picture, which draws no props. *Agent.*
- **Parked rotors are stopped.** In the Armory a parked helicopter's rotors
  stand still, its real blades drawn crisp with no blur disc, and a parked
  aircraft makes no rotor or engine sound. This replaces P8's "rotors
  turning" on the ground. *User.*
- **Relief beyond the yard.** The user asked for more exciting terrain. Hills
  rise behind the treelines from each spot's opening camera, through the
  backdrop's relief past the map; the yard, units and panels did not move. A
  critique saw nothing behind the nearer hills, so the far ridge was raised,
  data only. *User asked; agent chose how.*
- **A top just under an eye takes the fog's roof rule** (closeout,
  `3a48f3df7`). The ruin's top failed both the face test and the roof rule,
  which was gated on the surface standing above the eye. The rule now gates
  on the probe point, so the two split up faces at one height. *Agent.*
