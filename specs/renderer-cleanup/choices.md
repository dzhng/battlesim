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
