# Renderer cleanup: final choices ledger

The choices the build made that the user did not. Everything in
the planning walk settled with the user (all ten items in scope with V-wakes
last, a scratch-script plate proof, the V-wake branch rule and sign-off, Codex as
implementer) is the user's and is not listed. Every entry below was decided by
the **implementer** (Codex, gpt-6.1-sol) or the **integrator** (Claude), and
each says which. This ledger was re-audited against the final branch. It replaces
the per-pass `choices.md`. Choices that a later commit superseded are dropped:
offset-write ground packers, identifier aliases in the composition grammar,
`grouped`/`fixedGroup` binding names, the TypeScript-to-Rust track bridge, the
async timing pump, the cascade loop, `WATER_BROWSER_ONLY`, the twice-triangulated
terrain, `BUILTIN_SHADER_COUNT` and the `passes_shared` home for foliage stage IO.

**Review these first:** (1) S1, TypeScript keeping its own copy of the track
rule; (2) S2, the grass order fix's small picture change accepted without the
user; (3) S0, the tree-line grass check made relative after PR #19's tint
retune.

Counts: 0 needs-user, 0 unsound, 32 sound (the V-wakes, S00, signed off by the
user), plus compressed naming discretion.

---

## Sound

### S00. V-wakes: impulses carry a heading and the water shader draws two trailing arms

- **When:** slice 15, commit `feat(water): shape directional V-wake impulses`; implementer designed it, the integrator kept it isolated until the user's sign-off. Implementer designed it. The integrator kept it isolated and droppable.
- **The choice:** Picture a rifle squad wading fast through deep wetland water.
  Each tick, presentation drops *wake impulses* along the squad's path. An
  impulse is a small push into the *wave field*, a texture the GPU steps forward
  like a ripple tank. Each impulse is tagged with a `kind`, either ring or V.
  Before this commit the shader ignored `kind` and drew every impulse as a round
  ring. An unprimed critique of the baseline found no V in the picture. So the
  build took the slice's "shape it" branch rather than its "delete `kind`" branch.
  - **Heading.** Each impulse now carries a *heading*: the direction the unit
    actually moved this step, computed from its displacement. It is not the
    direction the unit faces, so a unit backing up still leaves its wake behind
    its motion. The heading fills the two padding floats of the GPU splat record
    (`WaveSplat`), so the record stays the same size and the splat count, step
    count and wake budgets are unchanged.
  - **Arm shape.** In the shader, a V impulse becomes two crests trailing behind
    the source. Each arm is `across ± 0.5·back`, about 27° either side of the
    path. Each crest has a dip beside it, so repeated pushes never pile water
    into a mound. An arm fades in over 2 texels behind the source and fades out
    by max(6×radius, 16) texels. It is at least 1.4 texels wide so infantry
    register, and it pushes at a quarter of the ring's strength.
  - **Unchanged.** Slow or shallow movers keep the ring. Sim and digests do not
    change.
  - **Test.** `crates/renderer-core/tests/water_wake.rs` runs the production
    wave-step pipeline on a calm field. It asserts wake energy behind travel,
    both arms present, and a symmetric ring control; removing the V shaping
    fails it. (An earlier browser check that re-assembled the shader in
    JavaScript was replaced: it was a second owner of shader composition.)
  - **Open concerns.** The final critique still flags arms that sit apart from
    the legs and ridges that look too regular and sharp. Stills don't show
    motion or vehicle wakes. The arms are wider than a real boat wake (about
    19.5° per side). The tuning numbers are named constants beside the wave step.
- **The gap:** the slice delegated the shape. The map makes the picture the
  user's.
- **The reach:** `WaveSplat.heading` and the V branch of `waveStep` become what
  later water work builds on. Native's shorter wave history
  (`specs/water-native`) will show these arms truncated.
- **Verdict:** sound: the look is the user's call, and the user signed off on
  the after crops (`assets/v-wake-before.png` vs `assets/v-wake-after.png`) on
  2026-10-30. The open concerns above stand as known, accepted limits.
- **Confidence:** high (signed off).

---

### S0. The tree-line grass check compares the verge against a control, not a fixed share

- **When:** commit `1b2ff647b` (integrator), a separate commit because the
  failure predates this branch.
- **The choice:** The ground lab frames a row of trees along a field edge. The
  ground under the row is the plots' *verge*, a grassy strip, so it should show
  grass rather than a wood's bare brown floor. The scene renders the strip with
  grass off and on, and counts strip pixels that change by more than 12 colour
  levels.
  - **The old rule:** grass must change at least 5% of the strip.
  - **The failure:** `staging` itself reads 4.0%. A bisect of `staging`'s
    history pins the drop to PR #19 (`cf1c125ca`, "Tint near grass against a
    tip-weighted mean"): 11.1% before it, 4.0% after; the one-renderer merge
    moved it by 0.0002. PR #19 deliberately tinted near grass to average out to
    the ground beneath it (verge blade tips went from about 2.5× the ground's
    brightness to 1.08×); blade count, geometry, tiers and culling are
    unchanged, and the bare shots are pixel-identical.
  - **The new rule:** a third shot draws the same strip with verge growth set
    to zero, as a control for grass spilling in from neighbouring ground
    (about 1.1%). The check passes when verge grass changes more than twice
    the control (now 4.0% vs 1.1%; a deliberate break measuring 0% fails).
  - **The title:** the commit says "against a wood floor"; the control is the
    suppressed verge, not a wood.
- **The gap:** the spec says nothing about this scene; the failure surfaced
  mid-run, and the integrator first attributed it to PR #19 without a bisect.
  The bisect (run before close) confirmed it.
- **The reach:** the check no longer breaks when the grass tint is retuned;
  it still fails if the verge loses its grass. It is weaker than "5%" by
  design: it tests that verge grass exists, not how much contrast it has.
- **Verdict:** sound. The old threshold pinned PR #19's predecessor's tuning;
  the drop is a deliberate look change, proven by bisect.
- **Confidence:** medium (the user might prefer a contrast floor too).

Ranked least confident first.

### S1. TypeScript keeps its own track rule; Rust has one owner

- **When:** slice 11. The integrator dropped the implementer's bridge, which revised map unknown 9 (also an agent decision).
- **The choice:** A tank model has nodes named `track_L` and `track_R`. A node
  counts as a track only if it also carries a positive `link_pitch_m`, the
  spacing between track links that drives how fast the tread texture scrolls.
  In Rust, presentation now decides this once (`TrackSide`), and renderer
  admission reads that answer.
  - **The rejected bridge:** the implementer also routed the TypeScript posers
    (the model workbench and the sheet renderer's `modelInstances`) to Rust
    through a JSON wasm call with a per-bundle cache.
  - **Why it was dropped:** TypeScript still needs the same rule for the bake
    validator, which must reject a bad file before Rust ever sees it. So the
    bridge added transport and a mode switch and removed no copy.
  - **Today:** `articulationRig` in `packages/scene-assets` checks name and
    pitch exactly as Rust does. The validator rejects pitchless tracks, so on
    any baked file the two always agree.
- **The gap:** the definition of done (user-accepted) says the track side has
  *one owner*. Rust can't serve the bake gate.
- **The reach:** "one owner" means one per language here. A change to the track
  rule must be made in both `presentation/src/units/articulation.rs` and
  `scene-assets/src/articulation.ts`. A divergence would show only on unbaked
  files.
- **Verdict:** sound. The TypeScript copy is forced by the bake gate, and a
  bridge would add machinery without removing it. The user should know that
  definition-of-done item 4 is met per language, not globally.
- **Confidence:** low to medium.

### S2. The grass determinism fix's small picture change was accepted without the user

- **When:** prerequisite commit `f1af9b94c` (integrator).
- **The choice:** The base build never drew the same grass twice: rendering one
  run twice with one binary changed 0.01–0.24% of pixels by up to 41 levels.
  - **The change:** after the fix, compared with the old base, 41 of 54 plates
    change on 0.004–0.30% of pixels, by at most 44 levels. That is the same
    size as the old build's own run-to-run noise. In effect it is one of the
    pictures the old code could already draw on any given run.
  - **Critique:** an unprimed critique of the worst plate saw nothing, even at
    3× zoom.
  - **The baseline:** the integrator re-based every later plate proof on the
    grass-fixed build. The user's proof was byte-identity against the base,
    which the base itself could not meet.
- **The gap:** the user's rule "slices 01–14 leave every plate byte-identical"
  assumed a deterministic base.
- **The reach:** every "byte-identical" claim on this branch means identical to
  the grass-fixed build. Reverting the fix restores the old flicker and breaks
  the plate proof.
- **Verdict:** sound, not needs-user. The change is not visible: a re-roll of
  existing noise does not alter the look the user owns. The user's "nothing the
  player sees changes" holds. It restores the exact proof the user chose rather
  than loosening it. The caveat sits in S3: the overflow path, which the plates
  never exercised, did change behaviour.
- **Confidence:** medium.

### S3. Grass placement counts, then prefixes, then emits, so draw order and overflow are fixed

- **When:** prerequisite commit `f1af9b94c` (integrator).
- **The choice:** Every frame the GPU scatters grass *clumps* (a few blades
  each) over a window of 4 m tiles, and 64 GPU threads (*lanes*) grow each tile.
  - **Before:** each clump took its slot with an atomic counter. Which thread
    won each slot, and so the draw order and which clumps survived when a LOD
    tier's buffer filled up, depended on GPU scheduling.
  - **Now, in three passes:**
    1. `grassCount` grows every tile without writing, and records each lane's
       count and the tile total.
    2. `grassFinish` is one GPU thread that walks all tiles and turns their
       totals into starting offsets.
    3. `grassBuild` grows every tile again and writes each clump at its fixed
       offset.
  - **Overflow:** a full tier keeps the same leading clumps every time. Because
    tiles are numbered row by row, the tiles late in that order lose their
    grass first. Before, losses were scattered and changed from frame to frame.
  - **Cost:** placement now runs twice per frame (count, then build). Wetlands
    GPU time moved 5.569 → 5.559 ms, which is noise.
  - **Kept:** authored capacities and the per-blade depth nudge stay.
    `GRASS_BUILD_LANES` and `GRASS_TILE_RANGES` are owned by presentation's
    `ground/grass.rs`.
- **The gap:** nothing specified how to make grass deterministic.
- **The reach:**
  - The single-thread prefix scales with tile count. Watch it if grass windows
    grow much wider.
  - When a tier overflows, grass vanishes from a contiguous run of tiles rather
    than thinning everywhere. No parity run overflowed, so no plate shows this.
  - Future grass work must keep growing deterministic, because placement runs
    twice and both passes must find the same clumps.
- **Verdict:** sound. It is bounded work with no global sort, and it fixes a
  real flaw. The spatial bias on overflow is the one behaviour to know.
- **Confidence:** medium.

### S4. The browser keeps resolving GPU timestamps in-frame

- **When:** slice 12 (implementer, closing map unknown 6, an agent decision).
- **The choice:** The renderer times each frame with GPU timestamp counters.
  - **The worry:** on Metal, a counter read in the same frame can be *stale*.
    The end mark may be written before the frame's work really finishes, which
    would under-report GPU time.
  - **The two modes:** native's bench resolves after the frame completes
    (`Resolve::AfterCompletion`). The map's plan was to measure the browser and
    switch it if it was stale.
  - **The measurement:** on Apple M6/Metal, Chromium 148, 1280×720, three Paris
    reel runs per mode gave the same answer. Medians were 9.86 ms in-frame and
    9.90 ms after completion, and both agreed with a native replay of the same
    capture.
  - **The result:** the browser stays on `Resolve::InFrame`, and native keeps
    `AfterCompletion`.
- **The gap:** whether browser counters were stale was unknown.
- **The reach:** two timing modes stay in the core timer. The evidence covers
  only Chromium on Metal; Safari and other adapters are unmeasured.
- **Verdict:** sound. Measured, not assumed, and it keeps the cheaper
  non-blocking path where it is valid.
- **Confidence:** medium.

### S5. Whole-spec review folded shader wrappers and shared repeated arithmetic, blur kernels included

- **When:** whole-spec review fix commits `a612fc4ec`, `067a16a9b`, `7e801b28e`, `2b1fcbaaf`, `f7219cd6b`, `ef15e8d96`, `be41ac671` (implementer, each family in its own commit).
- **The choice:** After the slices, several shader helpers were thin wrappers
  with one caller, such as six display forwarding functions and the environment
  bake and filter adapters. Each was folded into its caller. Repeated maths was
  given one body:
  - **Gaussian blurs:** each radius (5, 9, 13, 17, 21 texels) now supplies a
    weight array and its length, and one loop samples the kernel. The old code
    had an unrolled body per radius.
  - **Luminance:** every inline RGB-to-luminance vector reads `math`'s existing
    `LUMA` constant.
  - **Overlay glow, placed positions and eye-facing normals** each got one shared
    body, and grass's vertex result uses its stage-output struct.
  - **The risk:** the blur loop is the same kind of change that was rejected for
    shadow cascades (S6). A loop index in place of literal offsets can let the
    shader compiler round differently.
- **The gap:** the spec's "one body per algorithm" did not settle wrappers and
  repeated inline expressions.
- **The reach:**
  - The shaders hold fewer tiny functions to keep in sync.
  - These commits rest on the same rule as every slice: if any plate moves, the
    family's commit is dropped, not re-tuned.
- **Verdict:** sound, provided plates stay byte-identical. Each family is
  isolated, so a pixel change drops exactly one commit.
- **Confidence:** medium.

### S6. The shadow cascade blend stays unrolled

- **When:** slice 08 (integrator).
- **The choice:** The sun's shadow uses four *cascades*: nested shadow maps,
  each covering a farther band of distance, blended at their seams. The blend is
  written out four times in `shadow.wgsl`.
  - **The attempt:** the slice planned a loop for the middle cascades, keeping
    the first and last explicit. It assembled cleanly.
  - **What it did:** it changed one wetlands plate (`plate-840`) by one level on
    one pixel, and did so reproducibly. Replacing literal cascade indices with a
    loop index let the compiler fold the maths differently.
  - **The decision:** per map unknown 1 (any byte difference fails until traced;
    put the original back unless the maths change is deliberate), the unrolled
    form stays.
- **The gap:** the map assumed the loop would stay byte-identical.
- **The reach:** four near-copies remain in one place. A future change to
  cascade blending edits four blocks.
- **Verdict:** sound. It holds the user's byte-identity rule, and the
  duplication is local.
- **Confidence:** medium to high.

### S7. Road surfacing is its own shader module, split for the file-size budget

- **When:** slice 06 (implementer).
- **The choice:** Moving each ground function to its concept's module would
  have made one `ground_material.wgsl` of 35 KB, over the repo's 32 KB
  per-file budget. The implementer split out `ground_roads`, which holds
  paving, shoulders, ruts, markings, slab joints and road variation. Field
  colour, forest floor, verges and banks stay in `ground_material`. Lookups
  such as height pages, the surface field, plot queries and cubic sampling went
  to `ground_surface`, even when fog or grass is the caller.
- **The gap:** the target table allowed further splits without naming one.
- **The reach:** future road appearance work lands in `ground_roads`. The split
  line is "road vs field", partly chosen by size. The shaders README says so.
- **Verdict:** sound. It is a real seam (roads vs open ground) rather than an
  arbitrary cut, and no block was split.
- **Confidence:** medium.

### S8. The layout check ignores fields whose names start with `pad`

- **When:** slice 04 (implementer).
- **The choice:** The ABI test (`pipeline_abi.rs`) checks each Rust record
  against the WGSL struct of the same name: member names, offsets, array counts,
  nested records and total size.
  - **The problem:** `WaveRun` is 32 bytes in Rust but was 24 in WGSL, and
    filler fields have no meaningful names.
  - **The fix:** the WGSL struct got explicit padding, and the test drops every
    field or member whose name starts with `pad` from the name and offset
    comparison, on both sides. The total size is still compared.
- **The gap:** the spec didn't say how padding is matched.
- **The reach:** filler never needs a semantic name. But a real field that
  happens to start with "pad" (say `paddle_m`) would escape name and offset
  checks, and only the size check would catch a misplacement.
- **Verdict:** sound. It is simple and covers every current record. The prefix
  convention is the thing to remember.
- **Confidence:** medium.

### S9. Grass indirect-draw arguments became plain words, and there is no per-pipeline unused-declaration check

- **When:** whole-spec review, commit `f5f389d21` (implementer).
- **The choice:** Grass draws with *indirect arguments*, a 12-word GPU buffer
  that tells the draw call how many clumps to draw.
  - **The words:** after S3, only the one-thread prefix pass writes it, so its
    atomics became plain `u32`s. Unused bindings were removed from grass build
    and sprites.
  - **The deferral:** a check that rejects any declaration a pipeline doesn't
    use was not added. Counting names by text can't tell entry points, struct
    members and scoped locals apart. A real reachability pass would add a WGSL
    parser to the build.
  - **What still guards:** the build refuses any block that no pipeline
    references at all.
- **The gap:** the map noted "nothing checks for unused WGSL blocks" but asked
  for no fix.
- **The reach:** a dead declaration inside a used block can still creep back in.
- **Verdict:** sound. It is the cheap, reliable guard, and the costly one is
  explicitly deferred.
- **Confidence:** medium.

### S10. Scenery placement moved into renderer-core and registers rows at once

- **When:** slice 09 (implementer), trimmed in the review (`766d4f052`, later fixes).
- **The choice:** Before, the browser built the scenery *tier mesh*, the
  finest-LOD tree geometry that sets tree scale, felling and stumps, in
  TypeScript, while native built it in Rust.
  - **One owner:** both shells now call `renderer_core::scenery::place_scenery`,
    which reads decoded bundles.
  - **Registration:** placement registers canonical scenery rows immediately,
    ahead of any tooling art.
  - **The header:** browser installation returns a refreshed world header.
  - **Preserved state:** rebuilding the renderer keeps admitted placement and
    learned felling, so a felled tree stays felled.
  - **Explicit request:** the browser passes an explicit "this scene wants
    scenery" boolean, so geometry and model labs keep their bare ground.
  - **Summer state:** `sceneryAppearances` no longer checks for the summer state.
    Rust admission names a missing one.
  - **Deleted:** the TypeScript `tierMesh`, `SCENERY_STATE` and
    `FOLIAGE_MATERIAL`.
- **The gap:** the slice delegated the function's home and how mutable
  presentation access works.
- **The reach:** browser installation borrows presentation synchronously.
  Placement now follows bundle loading, so scenery kinds join source selection
  the way grass does.
- **Verdict:** sound. A failing-first test proved a late-admission stump row
  was missing, and replacement keeps its atomic commit.
- **Confidence:** high.

### S11. World geometry: two Rust constructors over one face builder, with no water switch

- **When:** slice 10, revised in review `dd88c9221` (implementer).
- **The choice:** The world layers are terrain, skirt plus extra faces, backdrop
  and water.
  - **Native** passes physical triangles to `WorldGeometry::from_triangles`,
    with an optional per-triangle tint for traversal views and extra faces.
  - **The browser** already has a Rust-generated terrain mesh, so it calls
    `from_mesh` with that mesh, explicit skirt positions and extra faces.
    `from_triangles` itself just builds the mesh and calls `from_mesh`.
  - **Water:** empty water cells mean no water, with no second switch in the
    runtime. The browser's `water` flag only chooses whether to read cells from
    its synchronous presentation borrow.
  - **Retired:** `sceneGeometry.ts` is gone. The slot is `skirt_and_props`
    everywhere. `TerrainTables` stops computing heights, plots and water copies
    that nobody read.
  - **The first version:** it carried both the mesh and triangles, so terrain
    was triangulated twice, and lab edits needed an override that was easy to
    miss.
- **The gap:** the slice delegated signature details.
- **The reach:** picking, shadow fitting and the drawn terrain use the same
  mesh. Labs with disconnected geometry pass explicit skirt and backdrop
  inputs. The wasm entry keeps seven arguments behind a lint exemption whose
  comment names what blocks a split.
- **Verdict:** sound. One constructor family, one face builder, no duplicate
  source.
- **Confidence:** high.

### S12. The track side is an enum stored on the admitted track, resolved once per catalog row

- **When:** slice 11 (implementer), refined in review `36a7e8db2`.
- **The choice:** `TrackSide` is `Left | Right`, and an admitted `Track` stores
  it directly.
  - **JSON:** serde keeps the existing `left: boolean` contract, so no file
    changes.
  - **Lookup order:** `CatalogView` resolves a row's appearance once, for rest
    palettes and track sides alike. It checks prospective assets (mid-swap)
    first, then canonical battle appearances, then live tooling assets.
  - **Raw callers:** a caller of `Models::from_world` supplies the same fact per
    row and node. Synthetic renderer fixtures say explicitly "no tracks".
  - **The test:** a GPU-free regression checks the packed scroll selectors
    actually uploaded. It covers missing, zero and negative pitch, non-track
    names, both sides, and a replacement rig.
- **The gap:** the slice delegated type names and the raw-world seam.
- **The reach:** both shells and staged replacements read one pitch-qualified
  rig. The lookup order is the contract for asset swaps.
- **Verdict:** sound.
- **Confidence:** high.

### S13. Timing API: a ticket only for delayed resolves, a pool sized by the caller, a shared query pair in-frame

- **When:** slice 12, revised in review `1753f16dc` (implementer).
- **The choice:** `renderer_core::timing::FrameTimer` is the only GPU frame
  timer; native's bench lost its own.
  - **Frame calls:** `begin` records which reset generation a frame belongs to.
    No caller frame index round-trips.
  - **Tickets:** only `AfterCompletion` returns a `PendingFrame`, a single-use
    ticket whose `wait` gives optional milliseconds. Native keeps its own
    report-row index beside the ticket. `InFrame` maps after submit and
    publishes rolling stats itself, so the browser gets no ticket it would
    throw away.
  - **Pool size:** the caller sizes the readback pool. Native asks for
    `IN_FLIGHT + 1`, because it reserves the next slot before retiring the
    oldest. The browser asks for 3.
  - **Query sets:** `InFrame` shares one query pair, since its counters are
    copied before another frame can overwrite them. `AfterCompletion` keeps a
    query set per slot.
  - **Failures:** invalid counters and failed maps become missing samples, not
    errors.
- **The gap:** the slice delegated the API. The first draft round-tripped frame
  identity and hid a pool size that silently had to exceed native's in-flight
  limit.
- **The reach:** a new timing caller must pick a mode and a pool size. Reset
  excludes old samples without cancelling tickets.
- **Verdict:** sound. Only paths production uses remain, and no hidden constant
  coupling.
- **Confidence:** high.

### S14. Timing is proven with heavy and light frames read back late

- **When:** slice 12 (implementer); the test moved to `renderer-core/tests/frame_timing.rs` in slice 13.
- **The choice:** The test submits a heavy fragment workload, then a light one.
  It defers native counter resolves and ticket reads until after later frames
  and a reset.
  - **What it catches:** if a later frame overwrote an earlier frame's query
    pair, the heavy frame would come back with the light frame's duration, and
    the assertion fails. A later light frame also checks that reusing the heavy
    frame's slot doesn't keep its old duration.
  - **Isolation:** each mode gets its own device. Concurrent workloads aren't
    used as a control, because GPU stages overlap.
- **The gap:** the slice asked for attribution coverage without naming a
  fixture.
- **The reach:** future timer changes are judged on attribution, not on timing
  numbers.
- **Verdict:** sound.
- **Confidence:** high.

### S15. Composition grammar: `shader|block,block@N`, nothing else

- **When:** slice 05, final form in review `6a11b7942` (implementer).
- **The choice:** `pipeline_sources.txt` lists each pipeline as its shader name,
  a bar, and ordered block ids.
  - **Groups:** a binding shared across bind groups is written once as
    `@group(GROUP)`. A reference `block@2` substitutes the number, parsed as
    `u32`. A block containing the *whole identifier* `GROUP` requires `@N`, and
    one without it refuses `@N`. So `FOG_BUILD_WORKGROUP`, a workgroup-size
    constant that merely contains those letters, needs no group.
  - **No aliases:** the earlier identifier-alias syntax (`[from=to]`) is gone,
    since no reference used it once the names were fixed. Nothing rewrites
    shader text except that one attribute.
  - **Refusals:** the build refuses duplicate block ids (naming both files),
    duplicate shader names, lines without `|`, missing blocks (naming the
    requesting shader) and blocks no pipeline references.
  - **Shader list:** the shader list is generated, and the pipeline count comes
    from `pipeline_registry::pipelines()`.
- **The gap:** the slice left identifier boundaries, the number domain and the
  error reporting open.
- **The reach:** this grammar is the contract every future shader edit follows.
  Device group limits stay with pipeline admission, not the composer.
- **Verdict:** sound.
- **Confidence:** high.

### S16. The binding test requires every shader resource to match its layout and allows unused layout entries

- **When:** slice 05, extended in review `3dde7f65e` (implementer).
- **The choice:** A CPU test walks every pipeline. Each global a shader declares
  must find a compatible entry in the pipeline's bind-group layouts.
  - **What counts:** buffer kind (uniform, read-only or writable storage),
    texture sample type, dimension, arrays and multisampling, and comparison
    versus ordinary samplers.
  - **Spare entries:** a layout may hold entries the shader doesn't use. A
    depth-only model shader reads only the palette from the model layout.
  - **Filtering:** whether a sampler filters, or a texture format supports
    filtering, is left to real GPU checks, because WGSL can't say.
  - **Windows variants:** the review made the test cover both cutout variants
    on every host. A bad binding planted in the unselected Windows body proved
    the earlier gap.
- **The gap:** "presence and kind" didn't settle spare entries or type depth.
- **The reach:** adding a shader resource without its layout entry fails on
  CPU. New resource kinds, like storage textures, must extend the matcher.
- **Verdict:** sound.
- **Confidence:** high.

### S17. One block per binding when bindings differ only by group; true variants are named by a layout

- **When:** slices 05 and 08, naming in review `26ff67782` (implementer).
- **The choice:** About 50 binding declarations differed only in their
  `@group(n)`. Each set became one block. Fog layer, fog parameters and the
  environment sampler joined them in slice 08.
  - **True variants:** declarations that differ in binding number, access mode
    or type stay separate. Each is named for a real registry layout that uses
    it (`fogLayout`, `sceneMipLayout`, `guardSeedLayout`, `waveLayout`).
  - **Reuse:** another layout reuses a declaration when its binding agrees. For
    example, the fog "eyes" buffer declaration serves both build and cull.
- **The gap:** the slice delegated which survivor keeps which name.
- **The reach:** a new layout either reuses a declaration exactly or adds one
  named for itself, never a positional `_alternate`.
- **Verdict:** sound.
- **Confidence:** high.

### S18. Module ownership map: concept modules for rules, `passes_*` families for entry points

- **When:** slice 06, final form in review `a4c816c74` (implementer).
- **The choice:** Blocks live in 29 modules (`crates/renderer-core/src/shaders/README.md` holds the table).
  - **Concept modules** own records and algorithms: `camera`, `fog`,
    `lighting`, `sky`, `shadow`, the four `ground_*` modules, `water`, `grass`,
    `scars`, `scenery`, `effects`, `post`, `math`, `models_geometry` and
    `models_material`.
  - **Entry points** and their stage IO sit in a family per thing drawn or
    computed (`passes_effects`, `_fog`, `_grass`, `_models`, `_scenery`,
    `_terrain`, `_water`, `_sky`, `_post`).
  - **`passes_shared`** keeps only the full-screen triangle, the placed-mesh
    vertex and genuinely shared mesh and screen IO.
  - **Placement rules:**
    - A binding used by several concepts belongs to the rule or resource it
      serves. Sampled colour inputs go to `post`, environment and material
      samplers to `sky`, model-card resources to `models_geometry`.
    - `math` holds only resource-free functions, and domain constants stay with
      their domain.
    - The colour-and-fog output struct (`ColorFog`) belongs to `fog`, since it
      is the visibility contract.
- **The gap:** the target table didn't place cross-concept helpers, generic
  samplers, or stage IO written outside pass modules.
- **The reach:** every future shader helper has an obvious home. The README
  table is the map future agents read first.
- **Verdict:** sound.
- **Confidence:** high.

### S19. Collapse only identical copies; algorithms with different maths stay separate

- **When:** slice 08 (implementer).
- **The choice:** A scan for repeated bodies merged only byte-identical
  contracts:
  - one horizon decoder `fogUnpack`, since builder and readers decoded the same
    packing;
  - one integer-grid height lookup `heightSample`, while fog and grass keep
    their own coordinate adapters;
  - seven shared stage structs, for example `ModelFragmentInput` for impostor,
    model, lit and x-ray fragments.
  - **Kept apart:** marching and still paint share an output struct but keep
    separate vertex code, because they compute positions differently.
  - **Windows bodies:** the depth discard variant drops two helpers it no longer
    calls, and the caster variant keeps `pixelNoise`, which it still uses.
- **The gap:** the slice delegated merged names and the extent of the extra
  scan.
- **The reach:** a change to a shared contract now reaches every consumer at
  once, which is the point.
- **Verdict:** sound.
- **Confidence:** high.

### S20. Record schema mechanics: nested layouts by reference, one generic array impl, hidden `bytemuck`

- **When:** slice 02 (implementer).
- **The choice:** The `records!` macro (presentation) describes each GPU record:
  its fields, offsets and size, which make up its `Layout`.
  - **Nesting:** a field that is itself a record carries
    `Option<&'static Layout>`, so the schema nests (`Environment.sun: Sun`,
    `View.rays: Rays`) and checks recurse.
  - **Arrays:** one generic `[T; N]` implementation lives in presentation,
    because a crate using an exported macro can't implement a foreign trait for
    arrays.
  - **`bytemuck`:** re-exported but hidden, so renderer-core can invoke the
    macro without its own dependency.
  - **Integers:** signed fields (`i32`) are supported for `WaveRun.shift`.
  - **Proof:** the real nested and signed records exercise the schema; no
    synthetic test record was added.
- **The gap:** the slice named the need, not the representation.
- **The reach:** any crate can declare checked GPU records with the same macro.
- **Verdict:** sound.
- **Confidence:** high.

### S21. The writer owns the record: presentation's records, renderer-core's records, typed until upload

- **When:** slices 03, 04 and 04b, final form in review `01f5c0c3d` (implementer; integrator required the 04b coverage).
- **The choice:** Ownership follows whoever writes a record.
  - **Presentation** owns the records presentation computes: terrain, grass,
    scar (`ScarParams` nesting `ScarCacheParams` and `ScarLook`), plot, surface
    and blade shape, `Sun`, `Rays` and wave splats.
  - **`renderer_core::records`** owns what the renderer writes: `Environment`,
    `CastLights`, `WaveRun`, `FogLayer`, `GroundPaintStyle`, `OverlayGlow` and
    `FogProbe`.
  - **Typed until upload:** records stay typed structs until upload, then are
    cast whole. Before, code wrote numbers at hand-counted word offsets.
  - **Toggles:** they rewrite their few words at `offset_of!` positions, for
    example the terrain diagnostic view words and the scar enable word.
  - **Precision:** float maths stays `f64` until the final conversion; integer
    fields stay integers.
  - **Typed callers:** authored `Paint` stays a strict config type with a
    `record()` packer. The runtime takes a typed `OverlayGlow`, so callers no
    longer pass positional padding.
  - **Cast lights:** they keep a typed CPU copy, so a shorter update preserves
    the old unused tail.
- **The gap:** the spec named the records to reflect, not their home crate or
  the packing style.
- **The reach:** a new GPU record goes in the writer's crate with the macro.
  Packers never hand-count words.
- **Verdict:** sound.
- **Confidence:** high.

### S22. The reverse check covers every CPU-packed struct, with three named exemptions

- **When:** slice 04 and 04b (implementer, scope raised by the integrator).
- **The choice:** The forward ABI check asks whether each Rust record matches
  WGSL. The reverse check asks whether each WGSL struct a shader reads has a
  Rust record, and it descends into arrays and nested structs. Exactly three
  structs are exempt, each with its owner named:
  - `Count`: GPU atomic counters read as words by a probe.
  - `GrassClump`: GPU-written; a separate test checks its stride.
  - `FogMaskView`: a single enum word.
- **The gap:** the spec said "every buffer layout a shader reads", and the first
  pass left six ground records and five packed layouts exempt.
- **The reach:** a new shader struct without a Rust record fails the test unless
  someone adds a named exemption.
- **Verdict:** sound. It meets definition-of-done 3 for CPU-packed data.
- **Confidence:** high.

### S23. The generated TypeScript records hold two explicit lists of what something reads

- **When:** slice 02 (implementer, executing map unknown 3).
- **The choice:** `records.gen.ts` had 114 exports, 82 of which nothing read.
  Now the generator takes two explicit lists, field offsets and strides.
  - **What qualifies:** records some browser production code, test or scene
    reads, including dynamic scene imports. That leaves 32 exports.
  - **Constants:** a named subset of shader constants is exported. Pass and fog
    constants in docs are named as Rust names, not browser exports.
- **The gap:** "only what something reads" didn't say whether tests and scenes
  count.
- **The reach:** a browser file that starts reading a new record must add it to
  the list.
- **Verdict:** sound.
- **Confidence:** high.

### S24. The Windows cutout: only the two depth-only pipelines get a `…Discard` variant

- **When:** slice 01 (integrator trimmed the implementer's third variant).
- **The choice:** On Windows, two depth-only cutout pipelines discard pixels
  instead of writing a sample mask. These are the model prepass and the shadow
  caster, and their variants are `modelPrepassCutoutDiscard` and
  `modelCasterCutoutDiscard`.
  - **Before:** a runtime text replace produced them.
  - **Now:** they are authored WGSL entries, chosen by `cfg!(windows)` in
    `pipeline_registry/models.rs` (the map's decision).
  - **The third variant:** the implementer also made one for the colour cutout
    pass. The old rewrite never touched that pass, so the copy was byte-identical
    and was dropped.
- **The gap:** the slice didn't list which pipelines needed variants.
- **The reach:** binding checks cover both variants on every host (S16).
- **Verdict:** sound.
- **Confidence:** high.

### S25. Renderer tests live in core with one device helper; shell-bound tests stay in native

- **When:** slice 13 (implementer, review revised the library helper).
- **The choice:** The high-air, impostor, x-ray, probe, marks, fog-view,
  frame-view, scar, model-geometry, pipeline and water-sampling tests moved to
  `renderer-core/tests`, along with the grass determinism and frame-timing tests.
  - **Shared support:** `tests/support` holds headless device admission, using
    native's adapter options. It also holds one typed readback helper shared by
    four tests and `runtime_library`, which loads bundles and their textures
    into core's byte library.
  - **Dev-dependencies:** native backends and `pollster` are dev-only; the wasm
    build is untouched.
  - **Stayed in native:** model, scenery, prop and runtime tests, because they
    need native's replay, filesystem `Library` or effect-sheet loader.
  - **Shims removed:** native's re-exports (`lib`, `pipelines`, `bundle`, and an
    unused `gpu::{bind, buffer}` the slice hadn't named).
  - **Lint:** the impostor gamma oracle uses `libm::pow` under core's lint; it
    gives the same bytes.
- **The gap:** the slice delegated helper names and pre-dated the grass and
  timer tests.
- **The reach:** `cargo test -p renderer-core` is the renderer's GPU suite.
  Native keeps only shell integration.
- **Verdict:** sound.
- **Confidence:** high.

### S26. Brittle tests feed controlled inputs at real boundaries

- **When:** slice 14, ruler revised in review `2fd419933` (implementer).
- **The choice:**
  - **Proxy colours.** The test once copied the gamma formula. Now it feeds
    white primitive art at the asset-data boundary and compares known linear
    RGB to a decimal tolerance.
  - **Workbench races.** The test once mocked six modules. Now it keeps the
    real route, provider, stage, pose and sheet code, and fakes only source
    loading and the GPU viewport. A scene also checks that a corrupt GLB is
    refused with a visible error.
  - **Range ruler.** Unit tests with fake units own range grouping, captions,
    de-duplication, nonpositive reach, and true 3D muzzle-to-aim distance;
    raised cursors catch a flat 2D shortcut. The scene checks only sorted
    unique reaches, distance consistency and matching tick marks. It no longer
    re-derives grouping from a real loadout.
- **The gap:** the slice delegated each seam.
- **The reach:** retuning art, loadouts or UI internals no longer breaks these
  tests. Each test fails on the behaviour it names.
- **Verdict:** sound.
- **Confidence:** high.

### S27. One GPU allocation observer, in the leak tracker

- **When:** slice 14 (implementer).
- **The choice:** Two scenes (`_appJourney`, `_terrainQuery`) each wrapped
  `requestDevice` to watch buffer and texture creation, on top of `_leaks.mjs`'s
  own wrapper.
  - **One observer:** `_leaks.mjs` now publishes `window.__gpuResources`, which
    holds weak handles to every buffer and texture.
  - **Counting:** a `created()` mark, then `counts(before)`, counts only what
    was allocated since and is still alive. Explicit destroy and garbage
    collection both remove a handle.
  - **Also published:** all-device queue draining, validation messages, and
    `requestedDevices`.
  - **Install:** installation is idempotent.
- **The gap:** the slice delegated the API.
- **The reach:** an app journey can tell the departed screen's allocations from
  the new backdrop's. Any future scene measures GPU allocations through this
  one hook.
- **Verdict:** sound.
- **Confidence:** high.

### S28. The ghost probe picks subjects by property and renders through `SheetRenderer`

- **When:** slice 14 (implementer).
- **The choice:** The model-ghost scene checks that half-buried and fogged
  models draw with the right transparency and depth.
  - **Subjects:** it once pinned `basketball_court` and rebuilt a scene from
    about 12 internals. Now it takes the first installed skinned, articulated
    and static bundles with positive height. A static subject must have tier-0
    geometry. Burial depth follows the subject's bounds.
  - **Rendering:** each subject goes through the workbench's real
    `SheetRenderer`, one at a time on one GPU, at its normal tile size and
    settings. Normal clear and fogged tiles supply the foreground mask, so no
    second no-bloom scene is needed.
  - **Thresholds:** the alpha, occlusion and fog thresholds are unchanged.
- **The gap:** the slice named subject properties but not eligibility or the
  render seam.
- **The reach:** the catalog can rename, reorder or resize models without
  breaking the probe. The production sheet path is what it tests.
- **Verdict:** sound.
- **Confidence:** high.

### S29. Fix grass determinism rather than loosen the plate proof

- **When:** prerequisite (integrator), before slice 03's plates.
- **The choice:** The first determinism probe, the same binary rendering the
  same run twice, differed in grass pixels. The varying pixels took more than
  two values, so no tolerance could separate noise from a real change. The
  integrator fixed the cause (S3) instead of adding a tolerance. The fix landed
  as its own `fix(renderer)` commit, per the firewall that a latent bug gets its
  own commit.
- **The gap:** the user's proof assumed a deterministic base.
- **The reach:** all fifteen slices could be proven byte for byte. Grass is now
  reproducible for any future plate or snapshot comparison.
- **Verdict:** sound. It restores the exact proof the user chose.
- **Confidence:** high.

### S30. Renames are proven by lexical binding, not just spelling

- **When:** slice 07 (implementer).
- **The choice:** Removing compiler-mangled names (`params_1`, `_arg_0`) can give
  two unrelated locals that shared one spelling different descriptive names. A
  transformed normal and a lighting normal, for example, needn't share a name.
  - **The proof:** the scratch token map checks that every use resolves to the
    same declaration before and after. Member names, builtins, literals,
    punctuation and token counts must stay identical.
  - **Where it was stricter:** fog and probe passes also passed a spelling-only
    map.
- **The gap:** the slice said "token-map check" without defining identity.
- **The reach:** future rename-only shader changes can be proven the same way
  without a GPU.
- **Verdict:** sound.
- **Confidence:** high.

---

## Compressed discretion (naming and cosmetic calls, all sound)

- **Shader names (slice 07 and review):** about 20 calls.
  - Uniforms: `fogParams`, `terrainParams`, `grassParams`, `environment`,
    `skyLight`, `castLights`.
  - Locals: `terrain`, `viewCamera`, `sunlight`.
  - Model values: `materialSample`, `baseColor`, `worldNormal`/`worldTangent`,
    `localNormal`.
  - Blur blocks are named by radius in texels.
  - Entry points are named for what they draw or compute.
  - Block ids carry their module's prefix, a convention the build doesn't
    enforce.
  - Both grass drift endpoints use `GRASS_MIX_DRIFT`.
- **Merged struct names (slice 08):** 7 structs.
  - `ColorFog` (in `fog`) and `MeshFragmentInput` (in `passes_shared`).
  - `ModelFragmentInput` and `CutoutFragmentInput` (in `passes_models`).
  - `FoliageFragmentInput` and `FoliageVertexOutput` (in `passes_scenery`).
  - `PaintVertexOutput` (in `passes_effects`).
- **Record fields and constants:** 8 calls.
  - Fields: `Sun.direction`/`radiance`, with golden fixture keys renamed and
    values unchanged; `grassFalloffM`.
  - Masks and bounds: `KIND_SHIFT` beside the ground masks; `ROAD_LOOKS_MAX`
    derived from `AREA_KINDS.len()`.
  - Wave and fog: `wave::SPLATS_MAX`; the unused `FOG_HORIZON_SECTORS` export
    removed; the named fog-mask distance cap.
  - WGSL member names kept with snake-case Rust fields.
- **Small structural trims:** 3 calls.
  - Empty `discard` statements removed.
  - Grass build bind-group construction shared.
  - Composer error messages name the shader and both files.
- **Docs:** 2 stale statements corrected: the shaders README rationale, and
  "native draws no wakes" in the water-and-wetlands README.

- **Closeout (integrator):** the grass indirect-argument array length
  (`GRASS_ARGS_WORDS`, now owned beside the other grass GPU constants in
  presentation) and the scar tile side (`SCAR_TILE`, re-exported from the scar
  page) join the shader constant prelude, found by the closing audit. Sound.

- **Closeout, after the Codex review (integrator):** the grass determinism fix
  had added a storage buffer to the grass build, so one compute stage bound
  nine storage buffers; WebGPU guarantees eight, and the device requests only
  what the adapter reports, so an eight-buffer adapter would fail to start the
  renderer even with grass off. The count and prefix passes now bind only what
  they read (eight and two), and a CPU-only registry test checks every
  pipeline's per-stage counts against WebGPU's defaults. Found by an
  independent review, not by any GPU here (this machine reports higher limits).
  Sound.
- **Closeout (implementer, from the final review):** the wake heading lives on
  `WakeKind::Vee { heading }` (rings carry none; the GPU record keeps zeros);
  the grass indirect-argument sizes and the raw-count offset derive from the
  tier count and are exported to the shader; the Windows cutout pairs are one
  list the registry and its test share. Sound.
