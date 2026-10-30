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


## Done — 2026-10-29

Typed all presentation sections in Rust, including the UI sections, and made
native, browser and workbench loads share admission. The mix is `AudioMix`;
the full sound library, metadata and routing are checked before preparation.
The dead shader settings are deleted. Ground extent, lab framing, shared UI
colours, fog selection and wake naming have their specified owners.

Proof: `after-05/` matches all 77 applicable slice-01 SHA256 entries, including
all 69 composed shaders; `cmp` matches each of the six native plates. No digest,
replay, visual or audio output moved. Recorded vectors changed only in their
retired/renamed input keys; their expected outputs are intact. The units fixture
contains no affected input keys and needed no rewrite.

Decisions: Rust also types the UI sections, rather than keeping parallel TS
admission; recipe/assignment/track gain admits [0, 16], rather than the old
ceiling of 2; backdrop reach is `ground.backdrop_reach_m`; the lab owns its
opening target. Flat puff inputs have strict flat readers because nested serde
flattening cannot enforce unknown-field rejection. Gauge classes follow the
catalog's composed physical vocabulary, rather than today's roster list.
Workbench settings admission uses its existing host-injection seam so Vite
configuration loading does not import a browser asset URL.

Regression tests were seen red before the fixes; disabling pose, sound-library
and mix-reference admission falsifies their tests again. Presentation, native
audio, focused web suites, typecheck, lint, clippy and the size ratchet passed.
The broader web run's failures were incomplete sound fakes and unfetched LFS
inputs; affected suites passed after correcting those inputs. Full-spec system
and balance gates remain at 16. As an implementation child this pass did not
spawn independent reviewers; the integrator owns that second opinion. Exact
plate bytes establish this slice's visual invariance.

**Review follow-ups (closed):**

- Dust and wake spacing must be positive in `EffectPresentation::check`, with document refusal coverage in `tests/settings.rs` (0308b96ee).
- Mist endpoints must increase after conversion to `f32` in `light/check.rs`; equal and collapsed-edge intervals are refused in `tests/settings.rs` (0308b96ee).
- `PoseFeel::check` requires positive `rest.settle_s[1]`, with a document refusal test (0308b96ee).
- Rust admits the complete HUD/overlay constraints, including callout glow up to 4; the TS fixture validators and their tests are removed (0308b96ee, 8fa8d29dd).
- Firing roles, loops and near/far burst cadence are checked by `WeaponLooks::check_sounds` / `SoundCatalog::check_shot`, covered by `tests/settings.rs` (confirmed already handled by 08/10).
