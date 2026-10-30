# Renderer cleanup: choices ledger

Decisions made where the spec was silent, per slice, with verdicts
(`audit-choices` form). Consolidated when the spec closes.

## 01 · Windows cutout

- **Only the two depth-only cutout pipelines get a Windows variant.** The old
  text rewrite left the colour cutout pass (`modelCutout`) unchanged, so the
  implementer's third variant was a byte copy; the integrator dropped it.
  *Sound.*
- **Variant shaders are named `…Discard`** (`modelPrepassCutoutDiscard`,
  `modelCasterCutoutDiscard`). *Sound:* names what differs.
- **`BUILTIN_PIPELINE_COUNT` became `BUILTIN_SHADER_COUNT`** (composition lines)
  and the pipeline count comes from `pipeline_registry::pipelines()`. *Sound:*
  the two were equal only while every shader had one pipeline.
- **The discard variants keep `pixelNoise` and `coveredSamples` they no longer
  call**, so each equals the old rewrite's output exactly (the slice's proof).
  *Sound for now;* slice 07/08 may drop the unused helpers.

## 02 · Record schema

- **Nested metadata is `Option<&'static Layout>`, and one generic `[T; N]`
  impl lives in presentation** (an exported macro's callers can't implement a
  foreign trait for arrays). `bytemuck` is re-exported hidden for the macro.
  *Sound.*
- **The TypeScript generator takes two explicit lists**, field offsets and
  strides, because they have different readers; it exports 32 names (was
  114). Every production, test and scene reader is kept, including dynamic
  scene imports. *Sound.*
- **A test-only record (`Nested`) exercises the exported macro, `i32` and both
  nesting forms against inline WGSL.** *Provisional:* it tests the checker, not
  the shaders; delete it once slices 03–04 give real records those shapes.

## 03 · Ground records

- **Keep the WGSL member names**, with snake-case Rust fields, so the assembled
  shaders change only where integer literals become constant names. *Sound.*
- **Name the ground field's primitive shift `KIND_SHIFT` beside its masks** and
  use it in CPU encoding/decoding and the `GROUND_KIND_SHIFT` export. Name the
  paved table bound `ROAD_LOOKS_MAX` in `tables`, derived from
  `AREA_KINDS.len()`. *Sound:*
  the encoders and their shader readers share the owners.
- **Fill typed ground records and cast their bytes with bytemuck.** The `f4`
  helper rounds only after f64 arithmetic; integer fields stay integer. Grass
  frame updates mutate the typed record. Append packing remains only for
  variable-length tables; offsets delimit partial record publications and the
  GPU's scar-enable write. *Sound:* field placement has one owner, without a
  parallel offset-based writer.
- **The road loops and out-of-range sentinel also use `ROAD_LOOKS_MAX`**,
  covering five count uses beyond the array declaration. *Sound:* they read the
  same bounded table.
- **Prove upload identity with scratch binary captures**, including grass frame
  updates and a combined scar cache/look record, rather than committing a fixture
  that pins the summer biome's tuning. *Sound:* existing presentation vectors
  and real shader reflection remain the durable checks.

## 04 · Renderer records

- **Renderer-only uploads live in `renderer_core::records`; `Sun` and `Rays`
  stay in presentation.** The writer owns its schema. `View.rays` and
  `Environment.sun` nest those records without changing bytes. Cast lights retain
  a typed CPU image so smaller updates preserve the previous unused suffix;
  uploads cast the whole record, and diagnostics read its count.
- **`Sun` uses `direction` and `radiance` inside the nested record.** Its
  golden fixture keys follow the names; the numeric fixture values do not change.
- **Authored `Paint` remains a strict configuration type.** Its `record()`
  packs `GroundPaintStyle` with zero padding; the shader field is
  `grassFalloffM`, matching the authored metre-valued field.
- **The runtime stores typed `OverlayGlow`, admitting the shell's four floats
  at its boundary.** Overlay construction and updates accept that record.
- **Padding is excluded from member-name comparison on both sides**, while
  spans and the offsets of every non-padding field remain checked. This admits
  explicit WGSL padding for `WaveRun` without inventing semantic fields.
- **The reverse check descends through arrays and nested structs.** Its short
  exception list names existing diagnostic, GPU-generated and ground-packed
  formats and their owners. (The six slice-03 types it exempted temporarily
  were dropped at integration; slice 03 reflects them.)
- **Integrator: CPU-packed layouts are checked records**, including the remaining
  plot, surface, blade-geometry and fog-query formats (04b). Only `Count` and
  `GrassClump` (GPU-written) and `FogMaskView` (one word) remain exempt.
  *Sound against definition of done 3 for CPU-packed structs.*
- **The local light visibility variable is `sunlight`.** Composition aliases
  rename that local instead of the `sun` record member, because aliases replace
  every whole identifier token, including member accesses.
- **The synthetic nested-schema test is removed** (resolving slice 02's
  provisional entry). Real `Environment`,
  `CastLights`, `Sun`, `Rays` and signed `WaveRun` fields exercise those paths.
## Grass determinism prerequisite

The user was not asked about any of these; the integrator decided them after
finding that the base build did not render grass the same way twice.

- **Fix grass determinism instead of loosening the plate proof.** Rendering one
  parity run twice with one binary changed 0.01–0.24% of grass pixels (up to 41
  levels), and the varying pixels took more than two values, so no tolerance
  could separate noise from change. *Sound:* the cause was a real flaw (draw
  order and tier overflow left to an atomic race), and the fix restores the
  exact proof the user chose.
- **Count surviving clumps, prefix tile and lane ranges, then emit into them.**
  Authored capacities and the depth nudge stay; a full tier keeps the same
  prefix every time. No tier overflowed on the parity runs. *Sound:* bounded
  work, no global sort; wetlands GPU time 5.569 → 5.559 ms mean (noise).
  The cross-tile prefix runs on one GPU thread; *watch it* if grass windows grow
  much wider.
- **Accept the small visual change without the user's sign-off.** Against the
  old base, 41 of 54 plates change, 0.004–0.30% of pixels, at most 44 levels:
  the same size as the old build's own run-to-run noise, i.e. one of the
  orders the old code could already draw. An unprimed critique of the largest
  change saw no difference even at 3× zoom. *Sound;* the user can still revert
  it. Two renders of the fixed build: 54 of 54 plates byte-identical.

## 05 · Composition grammar

- **When:** slice 05. **Choice:** `GROUP` means a whole identifier. For
  example, a fog compute block uses `FOG_BUILD_WORKGROUP` to set how many GPU
  threads run together. It still composes without `@N`; a binding written
  `@group(GROUP)` requires the group number. A substring check would reject
  that existing compute block even though it has no group placeholder.
  **Gap:** the slice did not define what "containing GROUP" means for longer
  identifiers. **Reach:** future constants may contain those letters without
  opting into group substitution. **Verdict: sound** — it follows WGSL's
  identifier boundaries and preserves all existing shaders. **Confidence: high.**

- **When:** slice 05. **Choice:** the group number parses as a Rust `u32`,
  an unsigned 32-bit integer. A reference such as `block@2[from=to]` keeps
  aliases after the group suffix. Aliases still replace original identifier
  tokens once, then the build substitutes the literal `@group(GROUP)`
  attribute; replacement names never feed a second alias.
  **Gap:** the numeric domain and interaction with identifier aliases were
  unspecified. **Reach:** malformed or overflowing group numbers fail during
  composition; actual device group limits remain pipeline admission's job.
  **Verdict: sound** — matches the registry's group-index type without adding
  device policy to the source composer. **Confidence: high.**

- **When:** slice 05. **Choice:** the binding test requires every shader
  global to find a compatible entry, while permitting unused entries in the
  pipeline's layouts. For example, a depth-only model shader can use only the
  palette from the model layout; it need not declare that layout's material
  textures. Buffer kinds distinguish uniforms, read-only storage and writable
  storage. Textures distinguish scalar kind, dimension, array shape and
  multisampling; samplers distinguish comparison from ordinary sampling.
  **Gap:** "presence and kind" did not specify unused layout entries or the
  level of type comparison. **Reach:** adding a shader resource without its
  matching layout fails the CPU test, without making shared layouts unusable
  for smaller shaders. **Verdict: sound** — checks the shader-to-registry
  contract rather than demanding identical resource sets. **Confidence: high.**

- **When:** slice 05. **Choice:** filtering support stays outside reflection.
  WGSL's `sampler` and `texture_2d<f32>` declarations cannot say whether the
  actual sampler filters or a texture format supports filtering. The test
  therefore accepts both ordinary sampler layout modes and both float texture
  filtering modes, while still rejecting a comparison sampler or integer
  texture in their place. Unsupported resource categories fail the matcher.
  **Gap:** the requested kind check did not distinguish shader-declared facts
  from device/resource facts. **Reach:** future storage textures or binding
  arrays must extend the matcher explicitly; real GPU checks continue to own
  filtering compatibility. **Verdict: sound** — avoids claiming information
  WGSL reflection cannot supply. **Confidence: high.**

- **When:** slice 05. **Choice:** group-only sets retain their existing
  `_primary` block when available (the sampler set keeps `_alternate`). The
  genuinely different binding variants take a layout name that uses them;
  another layout may reuse that declaration when its binding agrees. For
  example, `fog_sight_eyes_buildLayout` also serves `cullLayout`, whose eyes
  buffer has the same binding and access mode. Identifier-only differences,
  including the mangled fog uniform names, stay for slice 07. Three aliases
  whose source token was already absent are removed so the new refusal passes.
  **Gap:** the slice delegated layout names and did not name the surviving
  blocks or address existing dead aliases. **Reach:** this slice changes no
  WGSL identifier or assembled byte and creates no duplicate declaration to
  accommodate overlapping layouts. **Verdict: sound** — confines naming to the
  requested binding consolidation. **Confidence: high.**

## 04b · Remaining packed layouts

- **Presentation owns `PlotNode`, `PlotRecord`, `SurfaceRecord` and
  `GrassShapeVertex`; renderer-core owns `FogProbe`.** Their names and members
  match the existing WGSL so reflection covers the upload contracts without
  changing shader code or binding declarations.
- **Pack typed records and cast with bytemuck.** Plot and blade arithmetic stays
  f64 until each final f32 conversion; plot children remain signed integers.
  Surface inputs are already f32 and retain their existing zero fill and tail
  semantics. Public word/float buffer interfaces and empty-buffer sentinels stay
  unchanged.
- **Derive repeated record strides from the schema.** Existing public surface
  and blade float counts read their record sizes; minimum plot buffers also use
  record sizes. Input plot-tree nodes still use their own CPU source stride.
- **Use the existing reverse/forward ABI checks as the durable guard.** Removing
  the five exceptions makes the reverse test fail before the records exist.
  Scratch captures compare actual ground/blade packer output and the extracted
  fog-query packing expression before and after; no tuning-pinning fixture or
  redundant layout test is added.

## 06 · Concept moves

- **When:** slice 06. **Choice:** split road surfacing into `ground_roads`,
  including paving, shoulders, ruts, markings, slab joints and their grain and
  wandering variation. Field color, forest floor, verges and banks stay in
  `ground_material`. **Gap:** the target table permitted further concept splits
  without choosing one. **Reach:** the combined ground-material owner would be
  35,244 bytes, exceeding the 32,000-byte source budget; both resulting concepts
  fit without exemptions. **Verdict: sound** — separates road appearance from
  field/forest appearance without splitting any block. **Confidence: high.**

- **When:** slice 06. **Choice:** `ground_surface` owns height lookup adapters
  for both fog and grass, surface-field decoding and paved-field queries,
  plot-tree queries, class-distance encoding, and cubic sampling. Ground-class
  visualization and the grass shoulder response stay in `ground_material`;
  scar material changes stay in `scars`. **Gap:** the table did not distinguish
  lookup helpers from appearance rules at each boundary. **Reach:** lookup code
  follows the data it reads, while material decisions follow the appearance
  they produce. **Verdict: sound** — consumers compose the existing blocks
  without new copies or adapters. **Confidence: high.**

- **When:** slice 06. **Choice:** sky generation, lookup coordinates, environment
  filtering and roughness-to-mip policy belong to `sky`; shared PBR and aerial
  shading belong to `lighting`. The environment sun accessor belongs to
  `lighting`, screen-to-world reconstruction to `camera`, foliage surface
  shading to `scenery`, and sprite light records and calculations to `effects`.
  Paint sampling, visibility treatment and grass paint glow also belong to
  `effects`; fog display treatment belongs to `fog_masks`. **Gap:** several
  helpers crossed the listed concepts. **Reach:** each stays with the rule or
  resource it serves, rather than the pass that first used it. **Verdict:
  sound** — separates world lighting, lookups and their material consumers.
  **Confidence: high.**

- **When:** slice 06. **Choice:** retain `models_geometry` for articulation,
  card transforms and x-ray coverage, and `models_material` for authored model
  sampling, coverage and rooms. Fog owns visibility culling scratch; grass owns
  placement arguments and prop-exclusion resources. **Gap:** the table omitted
  model-only concepts and did not place every scratch binding. **Reach:** these
  existing concept owners survive after unrelated ground and stage IO blocks
  leave them. **Verdict: sound** — avoids putting model rules in an unrelated
  target module or inventing a generic binding owner. **Confidence: high.**

- **When:** slice 06. **Choice:** preserve the existing `passes_*` entry-point
  families and move their stage IO into those families. Reusable full-screen
  and mesh IO stays in `passes_shared`; all color/fog output copies stay in
  `fog`. Water-processing fragments and the scenery caster vertex move into
  existing pass modules. **Gap:** the table did not place reusable IO or entry
  points authored outside pass modules. **Reach:** no stage identifier, block
  suffix or composition order changes; duplicate bodies remain for slice 08.
  **Verdict: sound** — keeps stage contracts distinct from algorithm records.
  **Confidence: high.**

- **When:** slice 06. **Choice:** bindings used by several concepts have one
  resource/algorithm owner: sampled color inputs live in `post`, environment
  and material lookup samplers in `sky`, and model-card resources in
  `models_geometry`. Resource-free general functions live in `math`; domain
  constants and policy stay with their concepts. **Gap:** the table did not
  assign generic texture/sampler declarations or every pure domain helper.
  **Reach:** composition reuses each existing declaration without a new
  shared-binding module or merged variants. **Verdict: sound** — makes ownership
  explicit while preserving every assembled byte. **Confidence: high.**

## 07 · Names

- **When:** slice 07 probe. **Choice:** author fog's uniform as `params`,
  removing the 72 `params_1=params` aliases present after slice 06 (the slice
  text anticipated 44). Where grass or ground also binds parameters, use
  `fogParams` through explicit aliases until the domain naming pass.
  **Gap:** the probe did not specify how to avoid those existing global-name
  collisions. **Reach:** three assembled pipelines rename `params_1` to
  `fogParams`; every other assembled token stays unchanged. **Verdict: sound**
  — names the resource without merging bindings or changing layouts.
  **Confidence: high.**

- **When:** slice 07 fog and sight. **Choice:** name the fog uniform
  `fogParams` in every caller, keep `layer` for the layer selector, name
  binding variants by layout or read access, and name color/fog records and
  stage inputs by their material family. Cross-module declarations and
  consumers change together; duplicate bodies remain for slice 08.
  **Gap:** the slice delegated vocabulary and did not define how to name
  equivalent records before consolidation. **Reach:** shader composition has
  explicit domain dependencies without changing entry points or reflected
  records. **Verdict: sound** — names explain purpose rather than extraction
  order, and no new compatibility mechanism survives. **Confidence: high.**

- **When:** slice 07 models. **Choice:** use material-family stage types,
  `materialSample` and `baseColor` for sampled materials, and `worldNormal`
  and `worldTangent` after geometry transforms. The atlas direction helper
  takes `localNormal`. Entry-point names and both Windows discard bodies stay
  intact. **Gap:** the slice delegated the semantic names. **Reach:** these
  names distinguish textures, material results and coordinate spaces without
  changing any reflected field. **Verdict: sound. Confidence: high.**

- **When:** slice 07 models. **Choice:** the scratch token-map proof resolves
  lexical bindings when one spelling names unrelated variables. Its map is
  consistent per pipeline and declaration: all uses must resolve to the same
  declaration before and after. Member tokens, builtins, punctuation, literals
  and token counts must stay identical. The probe and fog passes also pass
  the stricter spelling-only map. **Gap:** the slice did not specify whether
  identifier identity means spelling or lexical binding. **Reach:** removing
  local-mangling aliases can give independent scopes different descriptive
  names; for example, a transformed normal and a lighting normal need not
  share a name. **Verdict: sound** — checks binding identity as well as token
  identity, with Naga and record reflection independently checking the result.
  **Confidence: high.**

- **When:** slice 07 ground, grass and scenery. **Choice:** ground and grass
  bind `terrainParams` and `grassParams`; a local ground-record pointer is
  `terrain`. Forest litter color, floor weights and moss lookup positions have
  distinct names. Scenery keeps `cam` as its camera binding, uses `viewCamera`
  for its local borrow, and distinguishes clump shading from the `shade`
  function. **Gap:** the slice delegated how to separate domain parameters
  and local lookups. **Reach:** consumers in water and grass placement change
  with the resource owner; ground alias chains and camera/function mangling
  aliases disappear. **Verdict: sound** — one binding vocabulary per domain,
  without new adapters or changes to arithmetic. **Confidence: high.**

- **When:** slice 07 remaining domains. **Choice:** environment, material and
  scene samplers have distinct resource names. Duplicate declarations that
  differ by group substitution are named `grouped` or `fixedGroup`; declarations
  with different binding layouts retain an actual caller's layout name.
  Lighting normals and diffuse albedo are named locally, without composition
  aliases. **Gap:** the slice delegated shared-resource vocabulary and names
  for copies awaiting slice 08. **Reach:** every existing caller binds the
  same authored names, so no identifier aliases remain. **Verdict: sound** —
  removes name repair without merging resources or inventing layouts.
  **Confidence: high.**

- **When:** slice 07 remaining domains. **Choice:** Gaussian blur blocks name
  their actual radius (5, 9, 13, 17 and 21 texels), and shared stage blocks
  name the operation they call. Water separates macro normals, bed depth,
  sun direction and sunlight; display, sprite and effect IO names identify
  their stage family. Numeric octave/weight constants retain their semantic
  indices. **Gap:** the slice delegated variant vocabulary and the distinction
  between mangling and meaningful numeric indices. **Reach:** all extraction
  suffixes and compiler names disappear while kernels, literals, record
  members, entry points and discard behavior remain unchanged. **Verdict:
  sound. Confidence: high.**

## 09 · Scenery tier mesh

- **When:** slice 09. **Choice:** `renderer_core::scenery::place_scenery`
  admits initial scenery for both shells; its finest-tier extraction is also
  used by the existing atomic replacement transaction. Reconstruction keeps
  admitted placement rather than resetting learned felling. **Gap:** the slice
  delegated the function's home and mutable presentation access. **Reach:**
  renderer-browser borrows presentation synchronously during installation;
  replacement remains prospective until commit. **Verdict: sound** — geometry
  has one owner and replacement keeps its atomicity. **Confidence: high.**

- **When:** slice 09. **Choice:** placement registers canonical scenery rows
  immediately, before the tooling suffix, and renderer installation returns a
  refreshed world header. **Gap:** placement now follows world loading, which
  previously registered scenery rows. **Reach:** late scenery remains canonical
  when tooling art is replaced; browser row readers refresh after admission.
  **Verdict: sound** — the failing-first late-admission regression proves the
  missing stump row and checks replacement geometry. **Confidence: high.**

- **When:** slice 09. **Choice:** browser installation explicitly carries
  whether the scene requests scenery, and source selection includes ground
  scenery kinds only for those scenes. **Gap:** ground can exist without scenery
  in geometry and model labs. **Reach:** those labs keep their deliberately bare
  ground; enabled scenes load bundles before placement. **Verdict: sound** —
  preserves the existing scene input instead of inferring intent from available
  bundles. **Confidence: high.**

- **When:** slice 09. **Choice:** `sceneryAppearances` retains catalog/category
  lookup but no longer checks the summer state. **Gap:** the slice asked whether
  the lookup still needed the deleted state constant. **Reach:** the authored
  asset loader validates required states; Rust tier admission names a missing
  summer state before placement. **Verdict: sound** — state selection has one
  runtime owner. **Confidence: high.**

## 08 · Collapse copies

- **When:** slice 08 fog unpack. **Choice:** all horizon readers call
  `fogUnpack`, including the horizon builder. **Gap:** slice 07 retained
  separate build/read names for the same packed format. **Reach:** one body in
  `fog` owns decoding; no compatibility wrapper or composition alias remains.
  **Verdict: sound** — the assembled builder changes only identifier tokens;
  Naga validation and record reflection pass. **Confidence: high.**

- **When:** slice 08 height lookup. **Choice:** name the shared integer-grid
  lookup `heightSample` in `ground_surface`; fog and grass retain their own
  coordinate adapters. **Gap:** the slice delegated the merged name. **Reach:**
  the paged height decoder still reads through `readWord`; bindings and all
  interpolation expressions are unchanged. **Verdict: sound** — only function
  identifiers change in assembled shaders; Naga and reflection pass.
  **Confidence: high.**

- **When:** slice 08 color-fog. **Choice:** The shared world-material output is `ColorFog` in `fog`. The merged name was delegated. All nine consumers keep the same color/fog locations and types; the optional trailing struct semicolon has no effect on WGSL. **Gap:** the slice did not enumerate this contract or its merged vocabulary. **Verdict: sound** — assembled stage types differ only by their names; Naga validation and record reflection pass. **Confidence: high.**

- **When:** slice 08 mesh-input. **Choice:** Identical overlay, world, terrain, water and ground-backdrop fragment inputs use `MeshFragmentInput` in `passes_shared`, beside the shared mesh vertex output. The additional-copy scan found this cross-family contract; centroid interpolation and every stage field remain intact. **Gap:** the slice did not enumerate this contract or its merged vocabulary. **Verdict: sound** — assembled stage types differ only by their names; Naga validation and record reflection pass. **Confidence: high.**

- **When:** slice 08 paint-output. **Choice:** Marching and still paint use `PaintVertexOutput` in `passes_effects`. The additional-copy scan found identical stage fields; the vertex algorithms remain separate because they compute different positions. **Gap:** the slice did not enumerate this contract or its merged vocabulary. **Verdict: sound** — assembled stage types differ only by their names; Naga validation and record reflection pass. **Confidence: high.**

- **When:** slice 08 foliage-input. **Choice:** Foliage and scenery-backdrop fragments use `FoliageFragmentInput` in `passes_shared`. The additional-copy scan found the same world, normal, color, local and heart contract; shared stage IO belongs beside its shared vertex output rather than in a fog entry-point family. **Gap:** the slice did not enumerate this contract or its merged vocabulary. **Verdict: sound** — assembled stage types differ only by their names; Naga validation and record reflection pass. **Confidence: high.**

- **When:** slice 08 model-input. **Choice:** Impostor, model, lit-model and both x-ray fragments use `ModelFragmentInput` in `passes_models`. The additional-copy scan found identical member names, types, locations and interpolation attributes; their material and visibility algorithms remain independent consumers. **Gap:** the slice did not enumerate this contract or its merged vocabulary. **Verdict: sound** — assembled stage types differ only by their names; Naga validation and record reflection pass. **Confidence: high.**

- **When:** slice 08 cutout-input. **Choice:** Depth and shadow-caster cutouts use `CutoutFragmentInput` in `passes_models`, including both Windows discard variants. The additional-copy scan found the same UV/material input; the name describes the shared fragment contract without preferring either consumer. **Gap:** the slice did not enumerate this contract or its merged vocabulary. **Verdict: sound** — assembled stage types differ only by their names; Naga validation and record reflection pass. **Confidence: high.**

- **When:** slice 08 foliage-output. **Choice:** Dressing, felled scenery and scenery casters use `FoliageVertexOutput` in `passes_shared`, alongside standing scenery. The additional-copy scan found identical output fields, including invariant clip position; geometry transforms remain authored by their separate algorithms. **Gap:** the slice did not enumerate this contract or its merged vocabulary. **Verdict: sound** — assembled stage types differ only by their names; Naga validation and record reflection pass. **Confidence: high.**

- **When:** slice 08 fog layer. **Choice:** one `fog_layer` binding block
  takes the group through the existing composition grammar. **Gap:** the
  additional-copy scan found a fixed-group declaration beside the same grouped
  declaration. **Reach:** every binding number, resource name and type stays
  unchanged; other layouts retain their own declarations. **Verdict: sound** —
  assembled shaders are byte-identical; Naga validation and record reflection
  pass. **Confidence: high.**

- **When:** slice 08 fog parameters. **Choice:** one `fog_params` binding
  block takes the group through the existing composition grammar. **Gap:** the
  additional-copy scan found a fixed-group declaration beside the same grouped
  declaration. **Reach:** every binding number, resource name and type stays
  unchanged; other layouts retain their own declarations. **Verdict: sound** —
  assembled shaders are byte-identical; Naga validation and record reflection
  pass. **Confidence: high.**

- **When:** slice 08 environment sampler. **Choice:** one
  `sky_environmentSampler` binding block takes the group through the existing
  composition grammar. **Gap:** the additional-copy scan found a fixed-group
  declaration beside the same grouped declaration. **Reach:** every binding
  number, resource name and type stays unchanged; other layouts retain their
  own declarations. **Verdict: sound** — assembled shaders are byte-identical;
  Naga validation and record reflection pass. **Confidence: high.**

- **When:** slice 08 Windows discard cleanup. **Choice:** the depth discard
  pipeline omits unused `pixelNoise` and `coveredSamples`; the caster discard
  pipeline retains `pixelNoise`, which its stochastic cutoff still calls.
  **Gap:** slice 01 deferred unused-helper deletion; the current bodies settle
  which helpers are actually unused. **Reach:** sample-mask variants and all
  discard entry points remain intact. **Verdict: sound** — only unused depth
  discard definitions disappear; Naga and reflection pass. **Confidence: high.**

- **Integrator: the shadow cascade blend stays unrolled.** A loop over the
  middle cascades (first and last explicit) assembled back to the original
  tokens, but it changed one wetlands plate (`plate-840`) by one level at one
  pixel, reproducibly; without it all 54 plates are byte-identical. The loop
  index replaces literal cascade indices, which lets the compiler fold
  differently. Per map unknown 1, the original expression stays. *Sound.*
## 11 · Track side

- **When:** slice 11. **Choice:** `TrackSide` is a left/right enum and
  `Rig::track_side` derives the per-node answer from admitted tracks rather
  than storing another node flag. `CatalogView::track_side` reads prospective
  assets before live assets; raw `Models::from_world` callers supply the same
  fact by row and node. **Gap:** the slice delegated accessor/type names and
  the raw-world seam. **Reach:** both platform shells and staged replacements
  use presentation's pitch-qualified rig; synthetic renderer fixtures explicitly
  supply no tracks. **Verdict: sound. Confidence: high.**

- **Integrator: TypeScript keeps one track rule, `articulationRig`, for the
  bake gate and the workbench poser.** The implementer bridged the poser to
  Rust through a JSON wasm call with a per-bundle cache, behind an optional
  parameter on `articulationRig`. That added transport and a mode switch while
  the TS rule stayed for the bake validator, which must reject files before Rust
  sees them, so it removed no copy. Dropped; this revises map unknown 9 (also
  the integrator's call). Rust now has one owner (presentation's rig), read by
  renderer admission; TS has one (the bake rule), which the validator keeps
  consistent with Rust by rejecting pitchless tracks. *Sound.*

- **When:** slice 11. **Choice:** the failing-first regression exercises the
  CPU admission output actually uploaded as model vertices, without a GPU.
  **Gap:** the slice prescribed the malformed-node case but not the test seam.
  **Reach:** missing, zero and negative pitch, non-track names, both valid sides
  and prospective rig replacement are checked on packed scroll selectors.
  Native plate identity remains the orchestrator's verification gate.
  **Verdict: sound. Confidence: high.**
## 10 · World geometry

- **When:** slice 10. **Choice:** `WorldGeometry::from_triangles` accepts an
  optional `(surface bytes, blocked bit)` tint, ten-float extra faces and an
  explicit water switch. Its prebuilt-mesh entry shares the face, backdrop and
  water builders. **Gap:** the slice delegated signature details. **Reach:**
  both shells construct the same submitted layers; browser installation borrows
  canonical ground synchronously for water cells and retains completed geometry
  in Rust without a JavaScript array round trip. **Verdict: sound** — the
  failing-first seam test covers tint and extra faces, and byte-preserving
  surface/traversal probes match, including wet ground. **Confidence: high.**

- **When:** slice 10. **Choice:** retain the browser's early Rust-generated
  terrain mesh for CPU shadow fitting and picking; carry source triangles for
  submitted construction. A lab that edits its mesh clears that triangle source
  and uses the direct entry. Separate skirt positions preserve the local edge
  when geometry scaling adds disconnected terrain; the existing backdrop
  override stays explicit. **Gap:** the slice named prebuilt labs but did not
  spell out early CPU consumers or the river lab's edited tint. **Reach:**
  installation triangulates ordinary terrain again in Rust; river tint, bare
  labs and scaling keep their existing geometry. **Verdict: sound** — avoids
  changing receiver fitting or inferring source ownership from mesh contents.
  **Confidence: high.**

- **When:** slice 10. **Choice:** `WorldLayers.skirt_and_props` carries only
  extra faces; Rust adds the skirt from supplied boundary positions at initial
  admission and live face replacement. Runtime storage and submitted diagnostics
  use `skirt_and_props` (`skirtAndProps` in camel-case JSON). **Gap:** the slice
  required a slot rename but did not specify the browser input after assembly
  moved. **Reach:** traversal boxes and moving workbench figures keep one face
  builder, and replacing extras cannot discard the skirt. Unused browser mesh
  assembly bindings are removed. **Verdict: sound** — inputs and completed
  renderer layers each have one owner. **Confidence: high.**

- **Integrator: the browser triangulates terrain twice at install**, once early
  (wasm helper, for CPU shadow fitting and picking) and once in the shared Rust
  constructor. Both are Rust code, so the concept has one owner; the cost is one
  extra triangulation per map load, not per frame. *Sound; watch it* if map
  load time matters, and pass the early mesh to the prebuilt entry then.
## 12 · Frame timing

- **When:** slice 12. **Choice:** `begin` takes the caller's frame index;
  `submitted` returns an optional owned `PendingFrame`, whose explicit native
  wait returns that frame's identity and optional milliseconds. Invalid counters
  and failed maps remain missing samples. Reset invalidates old rolling-window
  membership without relabelling or cancelling an already issued frame result.
  For example, frame 17's readback may finish after frame 19 and after a reset: its ticket
  still returns frame 17, while the reset window receives neither old frame.
  Reading a single latest-value field instead could file frame 19's time in
  frame 17's row. **Gap:** the slice delegated the result API and reset/completion plumbing.
  **Reach:** native fills the original report row; browser may discard tickets
  and consume the same bounded rolling window. **Verdict: sound** — delayed or
  reordered completions cannot become the current frame's sample. Synthetic
  attribution/reset regressions were each falsified, and the public GPU path
  exercises both modes with delayed reads and reset between encoding/submission.
  **Confidence: high.**

- **When:** slice 12. **Choice:** queue-completion callbacks only set an atomic
  readiness flag. Browser frame work calls `resolve_completed` without waiting;
  native resolves through its explicit ticket wait, outside timed CPU phases.
  Each reserved slot owns queries, resolve buffer and readback until mapping
  completes. If the GPU finishes frame 18 between browser draws, its callback
  sets the flag; the next draw submits its counter copy and a later map frees
  the slot. It never waits inside the page's drawing call. **Gap:** the slice
  delegated non-blocking completion handling;
  wgpu's completion callback requires `Send` even though WebGPU device handles
  are thread-local. **Reach:** no browser-specific dependency, unsafe handle
  transfer or blocking poll enters the core's asynchronous path. Native keeps
  its render wait followed by its own telemetry-submission wait and its
  two-render pacing. **Verdict: sound** — native GPU tests cover the non-blocking
  resolve pump independently of the ticket wait; the WebGPU build validates the
  callback boundary. **Confidence: high.**

- **When:** slice 12. **Choice:** retain `InFrame` for the browser. A stale end
  counter could omit a frame's actual GPU work; delaying resolve until completion
  would recover that missing time. On Apple M6/Metal, Chromium 148 production,
  AC power, high quality and 1280×720, three Paris reel runs per mode gave GPU
  means of 9.640–9.913 ms (`InFrame`, median 9.858) and 9.508–9.974 ms
  (`AfterCompletion`, median 9.899); median reported p95s were 12.385 and
  12.399 ms. All six recordings ran at 60.002 FPS and passed the FPS-counter and
  no-GPU-contention checks. Native replay of the exact first browser capture
  from each mode measured 9.321/9.328 ms means and 12.476/12.459 ms p95s, with
  all 2,400 reportable frames measured in each. **Gap:** map unknown 6 left the
  browser's Metal counter validity open. **Reach:** the browser keeps one
  default; the temporary compiled-mode switch and capture-export hook stay in
  scratch. These measurements cover this Chromium/Metal path, not Safari or
  another adapter. **Verdict: sound** — completion resolve recovers no missing
  GPU duration, its spread overlaps in-frame timing, and both agree with the
  scale and tail of the exact-input native calibration. **Confidence: high.**

- **When:** slice 12. **Choice:** prove GPU attribution with heavy and light
  fragment workloads, draining render submissions while deferring every counter
  resolve/readback until after subsequent frames and reset. If a later frame
  overwrites an earlier query pair, the heavy frame comes back with the light
  frame's duration; deliberately sharing all query sets fails that assertion.
  A later light frame also checks that reusing the old heavy frame's slot does
  not retain its duration. **Gap:** the slice required attribution coverage
  without prescribing a GPU fixture. **Reach:** independently created devices
  isolate the two modes; native's known-stale `InFrame` path checks identity and
  reset bookkeeping, while `AfterCompletion` also checks workload durations.
  GPU stages can overlap, so concurrent workload cost is not a reliable control
  for a timestamp span; production pacing is untouched. The fixture reuses
  native's existing GPU admission, and belongs with the renderer GPU tests when
  slice 13 moves them. **Verdict: sound** — controls overlap and startup state
  while preserving delayed readback, slot retention, reuse and reset coverage.
  **Confidence: high.**

## 14 · Brittle tests

- **Choice:** proxy colour checks supply white primitive art at the asset-data
  boundary and compare known linear RGB output with decimal precision, rather
  than reproducing gamma arithmetic and float rounding. **Gap:** the slice did
  not choose the input seam or output tolerance. **Reach:** art retuning does
  not invalidate the conversion proof. **Verdict: sound.**
