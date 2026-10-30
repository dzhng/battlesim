# 01 · Windows cutout as its own WGSL variant

**Contract.** No shader text is rewritten at runtime. The Windows discard
variant of the cutout pipelines is authored WGSL, validated on every host.

**Why first.** `native_shader_source` (`crates/renderer-core/src/pipelines.rs:392-407`)
matches exact text (`coveredSamples(...)`, `_arg_0.material`). Any later rename
would silently undo the workaround for AMD's Windows driver, which crashes on
depth-only sample-mask pipelines (`crates/renderer-core/README.md`).

## Seam

- New blocks in `passes_models.wgsl` for the discard fragments, and composition
  lines in `pipeline_sources.txt` for the discard variants of the three cutout
  pipelines (`modelCutout`, `modelPrepassCutout`, `modelCasterCutout`; confirm
  the names in `pipeline_registry/models.rs:34-44`).
- The variant is **selected in compiled code**, in `pipeline_registry/models.rs`
  with `cfg!(windows)`. Not in `build.rs`: it runs on the build host, so
  `cfg(windows)` there tests the wrong machine.
- `build.rs` emits both variants. `BUILTIN_PIPELINE_COUNT` (`build.rs:65`) counts
  composition lines today and `native/client/tests/pipelines.rs:12` asserts it
  equals the pipeline count. Split it: a built-in **shader** count from the index,
  and the pipeline count from the registry.
- Delete `native_shader_source` and its call.

## Verify

1. Scratch check: each new discard source equals today's
   `native_shader_source(name, base_text)` output, whitespace normalised.
2. `pipeline_abi` validates both variants (they are built-in shaders).
3. The non-Windows assembled-shader dump is byte-identical to base, so no plates
   are needed.
4. `cargo test -p native-client --test pipelines`.

**Delegated:** block and shader names for the variant.
**Stays green:** every non-Windows pipeline, `pipeline_abi`.
**Feedback that changes it:** none expected; Windows isn't a verified platform
(`specs/native-gaps` item 6).
