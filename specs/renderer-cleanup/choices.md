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
