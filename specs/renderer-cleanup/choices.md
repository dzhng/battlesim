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
  strides, restricted to records the browser reads. Production, test and scene
  readers are included, including dynamic scene imports. Batch field docs name
  pass and fog constants as Rust names, not browser exports. *Sound.*
- **Real nested and signed records exercise reflection.** `Environment`,
  `CastLights`, `ScarParams`, `Sun`, `Rays` and signed `WaveRun` fields exercise
  the exported schema against production WGSL without a synthetic record. *Sound.*

## 03 · Ground records

- **Keep WGSL leaf member names**, with snake-case Rust fields. Scar parameters
  nest cache placement and biome appearance into `ScarCacheParams` and `ScarLook`;
  shader accesses follow `cache` and `look` without changing field offsets,
  arithmetic or the uniform's byte size. *Sound.*
- **Name the ground field's primitive shift `KIND_SHIFT` beside its masks** and
  use it in CPU encoding/decoding and the `GROUND_KIND_SHIFT` export. Name the
  paved table bound `ROAD_LOOKS_MAX` in `tables`, derived from
  `AREA_KINDS.len()`. *Sound:*
  the encoders and their shader readers share the owners.
- **Carry typed ground records to upload.** `TerrainTables.params` holds the
  complete `TerrainParams`, including zeroed renderer diagnostic modes;
  `GrassParams` stays typed through frame updates; scar uploads supply
  `ScarCacheParams`, composed with the biome's `ScarLook` by field assignment.
  The `f4` helper rounds only after f64 arithmetic; integer fields stay integer.
  Cast at upload, retaining offset writes only for diagnostic toggles and scar
  enable. *Sound:* no truncated records, word splicing or duplicated writers.
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
- **The runtime's construction and update boundaries accept typed `OverlayGlow`.**
  The browser constructs it from its public radius and strength arguments;
  callers do not pass positional padding through the runtime API.
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

- **When:** whole-spec review. **Choice:** remove identifier aliases entirely:
  no composition reference uses them. The only substitution is `@group(GROUP)`
  from `block@N`, still parsed as `u32`. This supersedes the alias grammar above.
  Composer errors name the requesting shader or both defining files; duplicate
  shader names and index lines without `|` fail the build. The unused generated
  shader-count constant is removed. **Reach:** assembled sources are unchanged.
  **Verdict: sound. Confidence: high.**

## 04b · Remaining packed layouts

- **Presentation owns `PlotNode`, `PlotRecord`, `SurfaceRecord` and
  `GrassShapeVertex`; renderer-core owns `FogProbe`.** Their names and members
  match the existing WGSL so reflection covers the upload contracts without
  changing shader code or binding declarations.
- **Keep plot records typed until upload.** Plot and blade arithmetic stays
  f64 until each final f32 conversion; plot children remain signed integers.
  CPU surface queries consume flat float slices, so surface packing writes those
  directly using the reflected `SurfaceRecord` stride and the existing zero-fill
  and tail semantics. Empty storage bindings retain their sentinel record.
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

- **When:** whole-spec review. **Choice:** group entry points by the family they
  draw or compute: effects, fog, grass, models, scenery, terrain, water, sky and
  post. Shared stages retain only genuinely reused mesh/screen contracts and
  vertices. This supersedes retaining the extracted `passes_*` families above.
  Grass placement and constants live in `grass`; tree compression in `scenery`,
  clip-to-world reconstruction in `camera`, luminance coefficients in `math`.
  **Reach:** composition order and expressions stay unchanged. **Verdict: sound.
  Confidence: high.**

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

- **When:** whole-spec review. **Choice:** entry points name what they draw or
  compute, including sprites, grass stages and visibility probes. Registry entry
  selections change with their declarations. Block ids name their declared body;
  the grass drift endpoints both use `GRASS_MIX_DRIFT`. This supersedes preserving
  generic entry-point names above. **Reach:** token-map verification proves only
  identifier changes after composition. **Verdict: sound. Confidence: high.**

- **When:** whole-spec review. **Choice:** binding variants use actual registry
  layout names (`fogLayout`, `sceneMipLayout`, `guardSeedLayout`, `waveLayout`).
  Existing `sampled` and `effects` suffixes already name real layouts and stay.
  Environment and sky uniforms use `environment` and `skyLight`; both cast-light
  bindings use `castLights`. Lattice noise/hash, sky ray lookup and scenery surface
  helpers have distinct domain names. **Reach:** resource slots, record member
  names and expression tokens stay unchanged. **Verdict: sound. Confidence: high.**

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

- **When:** whole-spec review of slice 09. **Choice:** the browser keeps only
  a scenery-request boolean; changing scenery art identity does not stale the
  canonical presentation. **Gap:** the old field survived the move of placement
  into renderer admission. **Reach:** `loadGround` reads only ground and prop
  sources; renderer installation and atomic appearance admission own scenery.
  **Verdict: sound** — no load path reads scenery art. **Confidence: high.**

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
- **When:** whole-spec review. **Choice:** fold the six display forwarding
  wrappers, environment bake/filter wrappers and unlit/PBR adapters into their
  sole callers. Resource arguments now read their declared globals; computed
  value arguments remain evaluated locals. **Reach:** structural shader change,
  isolated for the orchestrator's byte-identical plate gate. Naga and reflected
  layouts pass; plate acceptance remains pending. **Verdict: provisional.**

- **When:** whole-spec review. **Choice:** each blur radius supplies one constant
  weight array and length; one loop samples the selected kernel. Literal weight
  spellings, f32 element types, tap order and accumulation stay intact. **Reach:**
  structural shader change in its own commit; drop it if any plate changes.
  Naga and reflected layouts pass; pixel acceptance is pending. **Verdict:
  provisional.**

- **When:** whole-spec review. **Choice:** all inline RGB luminance vectors
  consume `math`'s existing f32 `LUMA` constant. **Reach:** one coefficient owner;
  expression-to-constant substitution is isolated as structural pending plates.
  Naga and ABI reflection pass. **Verdict: provisional.**

- **When:** whole-spec review. **Choice:** grass indirect arguments are plain
  u32 words: only the one-invocation prefix stage writes them, the count/build
  stages never access them, and Rust's indirect draws/readback retain the same
  12-word byte layout. The emit call discards its unused return explicitly.
  Remove the unused argument binding from grass build and cube direction from
  sprites. **Reach:** structural shader cleanup, isolated pending plates.
  Per-pipeline unused-declaration rejection is deferred: lexical name counting
  cannot reliably distinguish entry points, members and scoped references; a
  semantic reachability pass would add a WGSL parser to the build dependencies.
  The existing all-pipeline dead-block check stays. **Verdict: provisional.**

## 11 · Track side

- **When:** slice 11. **Choice:** `TrackSide` is a left/right enum and
  admitted `Track.side` stores that enum directly; serde preserves the existing
  `left: boolean` JSON contract. `Rig::track_side` reads the admitted side rather
  than converting another flag. `CatalogView` resolves the row's appearance once
  for rest palettes and track sides: prospective assets, canonical battle
  appearances, then live tooling assets. Raw `Models::from_world` callers supply
  the same fact by row and node. **Gap:** the slice delegated accessor/type names and
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

- **Terrain tables hold only material inputs they publish.** Heights, plots and
  water cells are computed by their actual consumers from the canonical ground;
  `TerrainTables` does not compute unused copies. *Sound:* one computed value per
  consumer, without retaining dead tables.

- **When:** slice 10. **Choice:** `WorldGeometry::from_triangles` accepts an
  optional `(surface bytes, blocked bit)` tint, ten-float extra faces and an
  explicit water switch. Its prebuilt-mesh entry shares the face, backdrop and
  water builders. **Gap:** the slice delegated signature details. **Reach:**
  both shells construct the same submitted layers; browser installation borrows
  canonical ground synchronously for water cells and retains completed geometry
  in Rust without a JavaScript array round trip. **Verdict: sound** — the
  failing-first seam test covers tint and extra faces, and byte-preserving
  surface/traversal probes match, including wet ground. **Confidence: high.**
- **When:** slice 10, revised after whole-spec review. **Choice:** browser
  installation passes its existing Rust-generated terrain mesh, explicit skirt
  positions and `extraFaces` to `WorldGeometry::from_mesh`. Native's physical
  triangles use `from_triangles`, which delegates to the same builders. Empty
  water cells mean no water; there is no second runtime switch. The browser's
  water request only selects those cells from its synchronous presentation borrow.
  **Gap:** the initial boundary carried both mesh and triangles and triangulated
  twice; edited lab meshes needed an easy-to-miss source override.
  **Reach:** picking, shadow fitting and submitted terrain use the same mesh;
  skirt and backdrop overrides remain explicit for disconnected lab geometry.
  Runtime storage and diagnostics retain `skirt_and_props` for the assembled
  skirt plus extra faces. **Verdict: sound** — deletes the duplicate source and
  keeps one constructor family and one face builder. **Confidence: high.**

## 12 · Frame timing

- **When:** slice 12, revised after whole-spec review. **Choice:** `begin`
  captures the reset generation, without a caller frame index. Only
  `AfterCompletion` returns an owned, single-use `PendingFrame`; `wait` returns
  optional milliseconds. Native retains its report-row index alongside the
  ticket. `InFrame` maps after submission and publishes rolling stats directly.
  **Gap:** the initial API round-tripped frame identity and built browser tickets
  that production immediately dropped. **Reach:** invalid counters and failed
  maps remain missing samples; reset excludes old samples without cancelling
  their ticket results. Delayed heavy/light duration checks prove attribution,
  and `stats()` proves reset membership. **Verdict: sound** — only production
  completion paths remain. **Confidence: high.**

- **When:** whole-spec review of slice 12. **Choice:** callers size the bounded
  readback pool explicitly. Native uses `IN_FLIGHT + 1` because it reserves the
  next frame's slot before retiring the oldest submission; the browser retains
  its three-slot asynchronous window. **Gap:** the old hidden pool size silently
  had to exceed native's in-flight limit. **Reach:** pacing and saturation rules
  stay unchanged while full native coverage no longer depends on two unrelated
  constants agreeing. **Verdict: sound. Confidence: high.**

- **When:** slice 12 review. **Choice:** share the query pair for `InFrame`,
  whose marked encoder copies its counters before another frame can overwrite
  them. Retain per-slot query sets only for native's delayed `AfterCompletion`
  resolves. Both modes share one resolve buffer: each resolve and readback copy
  executes together on the queue, so later resolves cannot overwrite an earlier
  readback. **Gap:** the initial seam reserved a query set and resolve buffer
  per slot in both modes. **Reach:** outstanding native queries remain isolated;
  browser resource ownership follows its in-frame copy contract.
  **Verdict: sound** — delayed heavy/light workloads check native query isolation
  and reused-slot durations, while both modes keep independent readbacks until
  mapping completes. **Confidence: high.**

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
  fragment workloads, draining render submissions while deferring native counter
  resolves and ticket reads until after subsequent frames and reset. If a later
  frame overwrites an earlier native query pair, the heavy frame comes back with the
  light frame's duration; sharing native query sets fails that assertion.
  A later light frame also checks that reusing the old heavy frame's slot does
  not retain its duration. **Gap:** the slice required attribution coverage
  without prescribing a GPU fixture. **Reach:** independently created devices
  isolate the two modes; native's known-stale `InFrame` path checks asynchronous statistics and
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

- **Choice:** workbench race tests retain the real route and fake its source
  loading and GPU viewport boundaries only. The real catalog provider, stage,
  pose and sheet code stay loaded. The scene additionally checks a corrupt
  GLB's rejected automation request and displayed error through the real
  loader. **Gap:** the slice delegated the test boundary. **Reach:** every
  original race still passes through the File input or automation API;
  internal UI/geometry refactors no longer require coordinated mocks.
  **Verdict: sound.**

- **Choice (revised after whole-spec review):** fake-unit ruler tests own
  equal-range grouping, single-row versus multi-row captions, deduplication,
  nonpositive reach and 3D muzzle-to-aim distance. Synthetic physics and raised
  cursors distinguish height errors from a ground-distance approximation.
  The scene checks sorted unique reaches, distance/`along_m` consistency and
  matching DOM ticks. **Gap:** the scene previously re-derived grouping from
  loadouts and near-flat distance checks could not detect a 2D calculation.
  **Reach:** retuning content cannot break the oracle. **Verdict: sound** —
  ten deliberate production mutations failed the added assertions.

- **Choice:** `_leaks.mjs` publishes one `__gpuResources` observer: weak buffer
  and texture identities, `counts(before)` filtered by a monotonic `created()`
  mark, all-device queue draining, validation messages, and total devices
  requested as `requestedDevices`. `__resources` retains live page-owner counts. Installation is
  idempotent; explicit destroy and collected handles both disappear from
  allocation counts. **Gap:** the slice delegated the observer API.
  **Reach:** app journeys can distinguish departed allocations from the new
  backdrop's, while terrain probes use the same observer without another
  `requestDevice` wrapper. Fake WebGPU boundaries prove destroy, checkpoint,
  multi-device drain and validation behaviour; the scenes prove weak-handle
  release on the browser. **Verdict: sound.**

- **Choice:** ghost subjects are the first installed skinned, articulated and
  static bundles with positive height; a static subject and pose must have
  tier-zero geometry. Burial depth follows the subject's bounds. **Gap:** the
  slice named subject properties but not eligibility or state selection.
  **Reach:** catalog names, ordering of empty static states, and model height
  can change without invalidating alpha/depth coverage. **Verdict: sound.**

- **Choice:** ghost probes create detached scenes through the existing
  `SheetRenderer`, one subject at a time on one GPU. Its normal tile size,
  world and rendering settings replace the probe's duplicated scene recipe;
  normal clear/fogged tiles also supply the foreground mask, rather than a
  second no-bloom scene. Existing alpha, occlusion and fog thresholds remain.
  **Gap:** the slice did not specify a rendering seam. **Reach:** production
  workbench setup is exercised without a new production or test-only API;
  the harness retains only loader, GPU, sheet and camera publication entry
  points. **Verdict: sound.**

## 13 · Move tests

- **Choice:** headless test admission lives in `renderer-core/tests/support`,
  uses the shared device descriptor and native's adapter options, and retains
  only the device and queue. Native backend features and `pollster` are
  dev-dependencies. **Gap:** the slice delegated helper names and scope.
  **Reach:** core GPU tests need no native shell dependency; production WASM
  keeps its existing backend feature selection. **Verdict: sound** — the
  pipeline probe runs through core's built-in constructor.

- **Choice:** move the grass determinism and shared frame-timer tests with the
  named renderer suites. Keep replay-backed model/scenery/prop and runtime
  integration tests in native: their fixtures require native's `Replay`,
  filesystem `Library`, or effect-sheet loader. **Gap:** the slice's original
  roster predates the grass and timer tests and leaves ownership to inspection.
  **Reach:** renderer behaviour runs without the native crate; shell integration
  keeps its real services. **Verdict: sound.**

- **Choice:** share typed submit/map/poll/copy/unmap readback among the four
  tests using the same protocol. Keep texture sampling, multisample resolve,
  row-padding removal and scalar scar sampling with their test fixtures.
  **Gap:** the slice delegated readback sharing. **Reach:** one test helper
  owns the repeated buffer protocol without generalizing unlike sampling paths.
  **Verdict: sound.**

- **Choice (revised after whole-spec review):** `support::runtime_library`
  loads bundles and their catalog-referenced textures into core's byte library
  for both model-geometry and grass checks.
  **Gap:** the suggested grass fixture uses texture-free bundles, while the
  geometry fixture has textured materials. **Reach:** the test keeps the same
  authored asset, material admission and geometry assertions without depending
  on native's filesystem library. **Verdict: sound** — omitting the textures
  reproduced the decoder's missing-texture refusal before restoring full input.

- **Choice:** use `libm::pow` for the impostor test's gamma oracle after moving
  it under the crates' transcendental-method lint. **Gap:** the move exposes a
  lint that native's folder did not inherit. **Reach:** the input, expected
  albedo bytes and every assertion stay unchanged; a scratch probe compared
  both calculations and got identical `[148, 202, 243]` bytes. **Verdict: sound.**

- **Choice:** remove the unused `gpu::{bind, buffer}` re-export alongside the
  named library, bundle and pipeline shims. Bundle's filesystem `Library` and
  native's surface/headless GPU admission remain production shell services;
  their consumers import shared renderer types directly. **Gap:** the slice
  named three shim sites, while inspection found this fourth unused export.
  **Reach:** no renderer-core re-export remains in native. **Verdict: sound** —
  the consumer search found no dependency on the extra forwarding names.
