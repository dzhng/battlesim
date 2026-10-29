# 05 Typed presentation readers

**Contract:** every presentation, audio and sound-library reader refuses
unknown fields and out-of-range values when it loads, in Rust. Numbers no code
reads are gone.

**Seam:**
- **`presentation::settings::PresentationSettings`.** One struct per
  section, with `deny_unknown_fields` at every level. `Tuning` reads
  `terrain` and `grass` from their own sections, never from the whole
  presentation object (`static_world/files.rs:67`).
- **Delete the dead shader numbers:** the 162 terrain, 23 grass and 2 glass
  numbers. The WGSL already holds them as literals, and `deny_unknown_fields`
  forces this.
  - Kept as data: `distant.plot_pixels`, `shore.*` and `water.*`, which reach
    the GPU as uniforms.
  - Rewrite `ground/tuning.rs`'s doc.
- **`AudioMix`** (renamed from `AudioPresentation`) and **`SoundCatalog`** are
  checked in Rust. The TS-only sound conventions move there:
  - firing clips are shots or launches;
  - no loops;
  - near and far share a burst;
  - effect slots are real synth slots;
  - a gain range that admits levels above 2.

  Unknown vehicle, ambience and blast sound names are refused at load.
  Today they panic in `bank.prepare` or fail on first play.
- **Light** is checked in Rust (today only `validateLight` in TS does).
  - `light.cascades.count` leaves light; it can only be 4, and becomes a
    constant in [06](06-shader-constants.md).
  - `light.backdrop.reach_m` moves to the ground extent's owner.
- **Pose:** zero `fade_s`/`out_s` and an unknown gauge class are refused.
- **View cleanups:**
  - the fog style is picked once, by `FogPresentation::selected`;
    `gameFog.ts:24` stops picking it;
  - `overlay.xray.ally` derives from `hud.ally`, and `overlay.ruler.reach`
    from `hud.accent`;
  - `camera.default.target` moves to the labs that frame with it;
  - `effects.wake.soldier_radius_m` gets a name that doesn't read as the
    physics radius;
  - fix `fog/mod.rs:18`'s citation of a removed constant.

**Run / see:** nothing changes on screen. The new behaviour is that a
misspelled key names its path in the load error.

**Verify:**
- Write first: an end-to-end test that each shipped document loads, and that
  a misspelled field in each section is refused with its path.
- The composed WGSL for every pipeline is byte-identical: this slice touches
  no shader.
- 01's probe dumps identical; native plates byte-identical, including the
  HUD, x-ray and ruler.
- Stays green: `presentation/tests/{light,fog_sight,shadow}.rs`,
  `web/tests/{fog,fogStyle,light}.test.ts`.

**Moves:** nothing. Rewrite `paris-corner-units.json`'s input text by script
if it embeds the deleted keys.

**Depends on:** [04](04-split-files.md).

**Delegated:**
- the section struct boundaries;
- whether Rust types the TS-owned sections (overlay, hud, contacts, camera)
  or skips them while each TS owner refuses unknown keys.

**Would change this slice:** a "dead" number found to be read. Keep it as
data and record why.
