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
- **Use one `write_vec4` helper with byte offsets** for the ground parameter
  packers; retain the append helper for variable-length tables. *Sound:* the
  original f64-to-f32 conversion stays unchanged while the records own placement.
- **The road loops and out-of-range sentinel also use `ROAD_LOOKS_MAX`**,
  covering five count uses beyond the array declaration. *Sound:* they read the
  same bounded table.
- **Prove upload identity with scratch binary captures**, including grass frame
  updates and a combined scar cache/look record, rather than committing a fixture
  that pins the summer biome's tuning. *Sound:* existing presentation vectors
  and real shader reflection remain the durable checks.
