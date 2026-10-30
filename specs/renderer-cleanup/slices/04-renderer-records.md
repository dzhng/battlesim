# 04 · Renderer records, lengths, and the reverse check

**Contract.** Every struct a shader reads through a uniform or storage binding
is a checked record, and a new hand-packed layout can't slip in.

## Seam

- Records (in `presentation::records`, or a renderer-core `LAYOUTS` list for
  records only the renderer writes):
  - `Environment { world_to_view, observer, sun: Sun, aerial: Aerial }`
    replaces `SUN_AT = 80` (`environment.rs:17`) and the literal offsets at
    `:263` and `:345-346`. `Sun` is nested in WGSL too (reads become
    `….sun.direction`; `lighting.wgsl:36-44` flattens it today);
  - the `CastLights` header plus array (`environment.rs:19`, `:360-362`);
  - `WaveRun` (`water.rs:119-126`, WGSL `water.wgsl:125`). Rust is 32 bytes
    and WGSL 24 today (asserted at `pipeline_abi.rs:80-84`): add explicit WGSL
    padding members so the record matches;
  - `Rays`, nested in `View`, replacing `RAYS_BYTES` (`sky.rs:12`) and the test
    literal `48` (`pipeline_abi.rs:97`), and the test's prefix-matching helper;
  - `FogLayer` (`fog_sight.rs:22`), `GroundPaintStyle` vs `Paint`
    (`paint.rs:13-25`, mind the `grass_falloff_m` name), `OverlayGlow`.
- Lengths: `STEP_SPLATS` moves from `renderer-core/src/water.rs:112` to
  `presentation::wave` beside `WAVE`; `shader_constants` exports
  `WAVE_SPLATS_MAX = STEP_SPLATS * WAVE.max_steps`, and `water.wgsl:143` uses it
  instead of `1536`. `SKY_LUT_WIDTH` (`environment.rs:21`) reads the registry's
  `sizes().sky_lut[0]`.
- **The reverse check**, a new `pipeline_abi` test: every struct type of a
  uniform or storage global in every built-in shader is in a `LAYOUTS` list, or
  on a short allow list where each entry gives its reason.

## Verify

- Failing first: a test that the `splats` global's size equals the bytes the
  renderer allocates. Raise `WAVE.max_steps` to 7 in a scratch edit and watch it
  fail; then swap the literal for the constant and watch it pass.
- Failing first: the reverse check fails on today's tree before the records land.
- All three reels byte-identical; GPU pixel tests; browser scenes `water`, `fog`,
  `air-high`.

**Delegated:** where each record is declared (presentation or renderer-core),
by which crate writes it; the allow list's entries, each with a reason.
**Stays green:** every upload byte for byte.
