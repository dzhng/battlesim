# Armory: choices

Decisions the implementing agents made where the spec was silent. Each gives
the choice, why, the alternative, and a verdict (sound / provisional /
needs-user).

## Slice 02

- **The flat map's owner is `MapDefinition::flat(size)` in `contract`;
  `WorldGeometry::flat(size, rules)` builds on it.** Why: both test helpers
  take extra map fields (props, forests, buildings), so what they duplicated
  is the map, not the geometry. The helper in `sim/tests/common` merges its
  extra fields over `MapDefinition::flat`, and `world_geometry.rs`'s copy
  calls that helper. Alternative: `WorldGeometry::flat` alone, leaving the
  helpers' JSON literal as a second owner of the defaults. *Sound.*
- **Only the two named copies moved.** About 30 other test files write the
  same `fog_cell_m: 8, height_grid_m: 4, slope_cutoff_deg: 35` literal inside
  whole scenario JSON (maps with props, roads, encounters). Folding those in
  is a test-wide refactor outside this slice. *Provisional:* a later
  test-cleanup pass can point them at `MapDefinition::flat`.
- **`original_carriers` lives in `weapons/operators.rs`** beside
  `assign_operators`, which moved there out of the over-budget `weapons.rs`
  in its own commit first. It takes each mount's spec (`None` for a gun not
  wanted, such as an empty one) and each member's slot (`None` once fallen),
  and returns member indices. `assign_operators` keeps its
  handoff and spare passes. Alternative: the spec's `(specs, slots)` with the
  ammo filter left in `assign_operators`, which would have needed a second
  filtered copy of the specs. *Sound.*
- **The showground's ground is 400 m square and the squad stands about its
  middle,** reported as offsets from it, so `SquadAtRest` is about (0, 0).
  Soldier ids are `slot + 1` and the arrangement draws as battle unit 0. Why:
  the largest squad spread fits with room to spare; flat ground scatters the
  same anywhere, so the test compares against `squad_spots` on its own flat
  world within 1e-9. *Sound.*
- **`Showground` keeps Rust-typed `admit` and `at_rest` beside the wasm
  `new` and `squad`.** Why: `JsError` cannot be built natively, so a native
  test of a refusal needs the `String` error. *Sound.*

## Slice 03

- **A presentation shows a battle or a showcase, never both.** `set_showcase`
  errs while a battle is loaded and `load_world` with a battle errs while a
  unit is shown. Why: the showcase's effect batch and the battle's are each
  the frame's one `effects()`; holding both would need a merge nobody needs.
  A world of statics alone (no battle) still loads beside a showcase.
  Alternative: draw both and concatenate batches. *Sound.*
- **The shown unit's rows are the presentation's own (canonical), after the
  world's and before the lab's replaceable suffix.** Why: the browser loads
  art for canonical rows (`BrowserScene`'s `sourceNames`), and when those
  rows change it rebuilds its scene, the one rule a battle's reload already
  follows; tooling rows would never have their bundles loaded. The world's
  own rows never move; the lab's shift and are reinstalled with the
  renderer's tables. Alternative (my first cut): rows in the tooling suffix,
  which would need a second art path in the browser. *Sound.*
- **Both wasm calls return the world header's address**
  (`set_showcase(fielded, input) -> u32`, `clear_showcase() -> u32`), not
  `()` as the slice sketched. Why: rows change, and `install_lab_assets`
  already hands the renderer the new header this way. *Sound.*
- **The wasm `set_showcase` reads `{units, effects}`** (what
  `Fielding::resolve` returned, minus its table names) and takes the clip
  tables staged with `add_clip_table`, as `load_world` does. *Sound.*
- **The clearing test is two tests:** clearing leaves the lab's rows and
  model as they were, and a showcase and a battle refuse to share a
  presentation. Why: one asserts rows, the other an exclusion. *Sound.*
- **The idle test allows a watcher to stand still.** The driver makes one
  soldier in `rest.watch_every` a watcher, weapon up, holding; the test asserts
  more than one soldier idles and no two share a pose, without naming which
  soldier watches. *Sound.*
- **`idle_loops_of(setup, kind)` parses the whole `SoundFrame` setup** (one
  `Setup` struct, admitted by one `setup::admit` that `Sound::from_json` also
  uses) and finds the kind's `vehicle_class` in its `unit_catalog`. Why: one
  setup document, one admission; the browser already builds it. Alternative:
  a smaller idle-only setup, a second parser of the same fields. *Sound.*
- **Posing has one owner, `presentation/posing.rs`:** the battle's
  `Loaded::frame` and the showcase draw soldiers and vehicles through
  `UnitArt` (which now carries the frame's view and detail), share a
  `Scratch` of node transforms, and build models with `Model::unit` plus
  struct update. Why: removes the battle's nine-argument model closure
  without adding lint exemptions. The showcase's rotor blur calls the
  battle's own `draw_rotors`. *Sound.*
- **No "rotors turning" feed field.** The slice planned one, but the driver
  already turns and blurs rotors by the type's `airborne` (a property of the
  type, not of where it stands), so a parked rotorcraft's rotors turn with no
  driver change and battle frames are untouched. The showcase draws its unit
  with no aircraft shadow, in the view pass, as a grounded vehicle is drawn.
  Alternative: the planned feed property, which would add a second switch for
  one behaviour. *Sound.*

## Slice 04

- **`ShowcaseStage.show` is async** (`Promise<IdleLoop[]>`), not the sketch's
  synchronous `IdleLoop[]`. Why: a shown kind is fielded from runtime files
  the page fetches. A show another supersedes resolves with none. *Sound.*
- **Showing a unit rebuilds the stage's `BrowserScene`.** The shown unit's
  rows are canonical (slice 03), so the scene's `worldCurrent()` turns false
  and the stage builds a new scene, the one rule a battle reload follows.
  Alternative: an in-place row reinstall, a second admission path.
  *Provisional:* slice 11 should time unit switches; if a rebuild is slow,
  the fix belongs to `BrowserScene` for every caller.
- **`StageOptions` adds `look` (scene settings, detail, overlay glow) and
  `failed`.** Why: `web/src` does not import the lab's `game*` settings from
  `apps/`; the page that mounts the stage owns its look. *Sound.*
- **Fielding has one owner, `web/src/battle/sim/fielding.ts` (`fieldUnits`,
  `RuntimeFiles`)**; the lab's `fieldedBattle` builds on it. *Sound.*
- **`idleLoops(options, kind)` lives in `battle-audio/soundFrame.ts`** beside
  `vehicleLoops`, reading the one `setupJson` the battle's `SoundFrame` reads.
  *Sound.*
- **The lab shows one `ShowcaseStage` per unit, three canvases side by side.**
  Why: a presentation shows one unit; three panels also give each unit its
  own framing for the gate's crops. It drives its own orbit camera (drag,
  wheel) rather than `LabViewport`, which builds its own scene. Fixture
  `showcase` uses the `game` catalog, `loading: false`. *Sound.*
- **The page loads unit art with no scenery** (`scenery: null`); the probe's
  aircraft load asks only for the aircraft's art. *Sound.*
- **Grounded is judged from each drawn model's placement plus its far-pose
  bounds** (`farPoseBounds`), not the idle clip's frame. *Provisional.*
- **Parked aircraft: gear is the `wheel_*`/`track_*` subtree,** the
  validator's own rule; a skid or belly rests on the node that carries it.
  *Sound.*
- **Reference photo:** `assets/references/ah_64_apache/ah_64d_apache-parked_rotors_turning.jpg`,
  a public-domain US Army hot-refuel photo (AH-64D, rotors turning). *Sound.*
