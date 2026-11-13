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
  showcase feeds in, not "airborne". *Agent.*
  - Why: in battle the two always coincide, so battle frames can't change. A
    parked helicopter with its rotors turning then blurs with no role check.
  - The fewest-slices draft proposed the same rule as "rotor travel advanced".

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

- **Slice 01's visual gate passed on identity,** so there was no human
  checkpoint. *Agent.*
  - On the final tree, `menu`, `menu-skirmish`, `menu-tutorial` and
    `menu-settings` are byte-identical to the pre-slice run on the same
    machine.
  - One intermediate run's `menu-skirmish` differed by 277 px in the status
    line. The lobby draws a fresh random seed each run, and the seed's hidden
    width moves the right-aligned "Ready". That run still matched the approved
    baseline pixel for pixel.
- **06. The search stays a field at every width;** the nine tabs fit beside it
  at 1280 px (the `ui` scene's tab-row and containment checks), so no
  collapse to an icon. *Agent, under the delegation.*
- **06. The nine-tab strip is accepted on the evidence:** it fits with room to
  spare, every label reads whole at 2×, and the unprimed critique found no
  defect in its fit. Its notes on the selected tab's contrast and the
  search's margins describe the existing style, unchanged here and left to 08.
  No human reply came in the checkpoint window. *Agent.*
