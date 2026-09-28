# 27f — Unit types are catalog data

**Status:** planned. **Depends on:** the trench removal and 27e merging (both change the fixture and contract). **Lane:** contract, sim and presentation (one owner per concept). **Given:** the user's direction of 2026-09-27: hundreds of unit types across several factions, including tanks with many variants (the M1 Abrams family is the example), other vehicles, infantry, and later helicopters. "Make sure our architecture accounts for this from the get-go." This slice also closes the whole-spec review's findings 6–9.

## The concept without one owner

"What a unit type is" is spread across:
- a **closed enum**, `contract::UnitKind`, with six variants;
- **22 per-kind named rule fields** (`physics.tank_half_extents_m`, `movement.jeep_mps`, `health.tank`, `rifle_squad_size`, `sensors.recon_ground_m`, `sight_shape.<kind>`, …);
- **about 85 `UnitKind::` matches** across 10 sim files, several of them picking numbers;
- **five hand-written TS hull maps**, **six TS kind lists**, and **"is this a vehicle" decided five different ways**.

Adding one kind today means about 15 edits in two languages. At hundreds of types that doesn't scale, and every miss is silent: a unit with no pick box, never drawn, or sighting along its hull.

## The end state

1. **A unit type is a catalog entry, addressed by id.** The id is a string in data (`"m1a2_sep"`) and a dense index at runtime. No Rust or TS enum lists unit types. Scenarios, commands, publications and replays name types by id; the publication already sends a `unitKinds` table, which becomes the catalog's id list.
2. **Variants inherit.** An entry may `extends` another, and gives only what differs: `m1a2 extends m1a1 extends m1`, overriding armour, a mount's weapon, sight range or cost. Inheritance resolves **once, at load**, into flat, fully specified records. Nothing downstream ever sees `extends`. Abstract bases (`abstract: true`) exist only to be extended. Cycles, unknown parents and incomplete leaves fail at load with a named error.
3. **Behaviour comes from components, not types.** A resolved type has these parts. The sim branches on a component's presence or variant, never on a type id or name:
   - `faction`, `family` (for UI grouping and balance) and `cost`;
   - `body`, one of:
     - `squad { size, soldier: <infantry body params> }`;
     - `hull { half_extents_m, eye_m, armor { front, side, rear, roof }, hp, weight_class, push_class, wreck }`;
   - `mobility`: `foot { mps, road_multiplier } | tracked { mps, road_mps, turn_deg_s, reverse_fraction, … } | wheeled { mps, road_mps, turning_radius_m, reverse_fraction, … }`. There's room for `rotor` or `air` later as new variants, but they aren't built now;
   - `sensors { ground_m, sight_shape, eyes }`;
   - `mounts [ { name, weapons, turret, on, pivot_m, muzzle_m } ]`, as today, moved from the rules;
   - `capabilities`: optional parts such as `deploy { … }`, `supply { stock, … }` and `garrison`, present only when the type has them;
   - `sound { profile, loudness_m }`;
   - `appearance`: the catalog appearance id. **A variant may have its own model while inheriting its parent's mount geometry** (user: an M1A2 may use a different 3D model from the M1A1, with turret and gun positions probably the same). So:
     - geometry the sim uses (hull extents, eye, mounts' pivots and muzzles) belongs to the **type** and is inherited;
     - the **model** is per type and may be shared or replaced;
     - each type's own model is fit-checked against its own resolved numbers. A new M1A2 model must put its turret and gun where the inherited mounts say, or the variant overrides those numbers explicitly.
     
     That keeps the sim and every drawn variant in agreement, without re-authoring shared geometry.
4. **One owner per derived question,** answered from components:
   - "is it a vehicle": it has a `hull` body;
   - "is it infantry": it has a `squad` body;
   - "does it have a turret": the first `turret` mount;
   - "what is its footprint": from its body.
   
   These helpers live once in the sim and once in TS (scene-assets), with the TS side generated from or checked against the same schema.
5. **Catalog files:** `fixtures/units/<faction>/<family>.json` (or one file per type; choose at implementation and record it), loaded and resolved by one loader, `sim::catalog`, exposed through the contract. The village fixture keeps its rules and map, and references unit types by id. Weapons stay their own rows, referenced by mounts.
6. **The guards scale with the catalog.** A generated test runs **for every resolved type**:
   - its components are complete and in range;
   - it has an appearance that validates and fits its numbers (the `fit.*` checks);
   - a short smoke battle spawns it, moves it, and has it fire each mount.
   
   Type number 300 gets the same checks as type number 6, with no hand-written test.

## Constraints

- **Constrain to what exists.** Build the components the current six types need. Leave room for air and rotor mobility (the enum is open to a new variant, and heights are not assumed zero in identity), but write no air code, no unused fields and no speculative abstractions. Factions and families are fields; no faction logic yet.
- **Behaviour must survive, and digests must not change.** This is a pure data move: every battle digest, replay and village report must be byte-identical before and after. The digest and replay tests plus `village_report -- --quick --compare main` prove it. Where a digest has to change (for example if kind ids enter it differently), make that a named, justified exception.
- **No compatibility layer.** Delete `UnitKind`, the 22 named fields, the matches, the TS hull maps and the kind lists in the same pass. No aliases.
- Update `fixtures/README.md` ("Adding a unit kind") to describe adding a type as **one catalog entry**, and make adding a variant as small as an `extends` plus overrides.

## Verification

- A worked example proving scale, as a test fixture only, never shipped: three M1-like variants extending one base: one differs in armour, one swaps a mount's weapon, and one has **its own model while inheriting the mount geometry**. The fit check passes for the good model, and fails for a model whose turret sits elsewhere. They resolve correctly, pass the generated per-type checks, and one spawns in a test battle.
- Every digest and replay unchanged; `bun run check`; `bun run verify`.
- A size report: lines added and deleted in production, tests and docs. This slice should delete about as much as it adds.

## Decision budget

- **Delegated:** file layout of the catalog, and the exact component field names within this shape.
- **Anything else:** record it in `choices.md`.

## Decisions from the unknowns walk (2026-09-27)

- **Factions and sides (user):** each side picks one faction, and mirrors are allowed. The faction decides the roster and the look (camo); the side colour lives only in the UI and markers.
- **Roles (user):** every type carries inherited role tags (`mbt`, `recon`, `at`, `infantry`, `logistics`, …) from a role registry in the fixture. AI and scripts select by role, never by type id. Today's six kinds map one-to-one onto roles, so the digests survive.
- **Mixed types (user):** a mixed selection takes the same orders, and each unit executes with its own numbers. The command bar shows the union of the selection's capabilities, and the unit card lists each unit's type.
- **Select similar (user):** double-click selects the same type; a second double-click (or Ctrl + double-click) widens to the same role.
- **Technical calls (the user delegated these; the most elegant option was chosen):**
  - the sim owns resolution, and web and tools get resolved records through wasm (one owner);
  - the first cut keeps today's six ids;
  - objects deep-merge, named lists (mounts) merge by name, other lists are replaced;
  - weapons use the same `extends` inheritance;
  - an infantry appearance is a set, picked per soldier;
  - the layout is `fixtures/units/<faction>/<family>.json` plus the role registry.
- **Squad composition (user):** a squad type lists soldier **slots**, and each soldier can be different, with his own weapon (mount) and appearance. When a soldier falls, his slot's weapon is lost from the squad. Squad sizes vary by type: an ATGM team is 2–3 soldiers at most. Picking up the weapons of the fallen is deferred.
  - The first cut expresses today's squads as slots with identical behaviour (the digests survive). The AT team keeps today's size in this pass; resizing it is a later, named data change, not part of the refactor.
- **Catalog fields (provisional, while the user was away; reversible):** only fields with a consumer ship:
  - kept: `cost` (the AI's costliest-target choice) and a per-type `sound` profile;
  - deferred until a consumer exists: era or service years, crew count, and reusable upgrade parts;
  - an independent commander sight (the M1A2 CITV) is a real mechanic that deserves its own slice.
  
  Authoring is by the user and agents, in plain JSON, one family per file. Real vehicle names are fine; branded liveries and markings are not. Armies come from a fixed scenario roster for now; decks or points come later, and `cost` is ready for them.
- **User follow-ups (2026-09-27):**
  - **Special weapons transfer.** A weapon (mount) may be flagged `special` (an ATGM, a squad MG). When its carrier falls, it passes to the next living soldier, so the squad keeps it while anyone remains. Unflagged weapons are lost with their soldier.
  - **Every type has a human-readable `name` and `description`.** Era or service years are not needed.
  - **Every weapon and ammunition row has a display `name`, a `description` and an `icon`.** Icons are code-generated SVGs (a script, like the rest of the procedural art) under `assets/icons/`, and the validator requires every weapon's icon to exist.
  - **Vision per variant:** already covered by per-type `sensors` (range and sight shape; 360° is 1/1/1). An independent commander sight (a second eye with its own bearing, the hunter-killer loop) is a new sensor component and stays out of this refactor unless the user asks.
  - **Upgrades are enabled as reusable parts.** A part is a named bundle of overrides and additions (armour, capabilities, mounts). A type lists `parts: [...]`, applied at resolution after inheritance (`m1a2_sepv3` = `m1a2_sepv2` + `trophy_aps`). Parts that need a behaviour not built yet (e.g. APS interception) are refused by the validator until their capability exists. Parts are fixed per type for now; per-unit loadouts chosen at deployment can build on the same parts later.
