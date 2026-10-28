# Slice 03 — one runtime and direct browser presentation

**Status:** next. Depends on the shared-frame checkpoint in slice 02.

## Contract

`renderer-core` owns resource composition, world admission, preparation, encoding
and rebuilding. Native and browser provide devices/surfaces, transported asset
bytes, external geometry and application inputs. Both invoke the same runtime
against the actual canonical `presentation::Presentation`.

The browser uses `renderer-browser` as its single WASM entrypoint, linking and
reexporting the presentation/audio bindings. Keep the simulation worker separate.
A third permanent umbrella crate is unnecessary. Every participating class comes
from the same generated glue and initializer.

Rendering/install methods borrow presentation only during synchronous Rust calls;
no borrow survives an await. Keep the handle private in `ScenePresentation` and
expose narrow rendering/audio capabilities. Reacquire JavaScript typed-array views
after any allocating unified call: rendering can grow the same memory now.
Preserve publication layouts, sound behavior and UI/frame publication order.

## Implementation passes

1. Make the existing presentation bindings linkable and integrate one module/init.
   Establish explicit view lifetime and private-handle contracts before changing
   the production canvas interface.
2. Move native scene composition and preparation into the core runtime. Preserve
   its order: changed scars invalidate grass; prepare view/environment, ground,
   shadows, grass, models/scenery, scars, fog, effects and cast lights. Share the
   ground partition used for renderer selection and presentation shadow fitting.
3. Share effect-sheet decoding, premultiplication, mip generation, atlas lifetime
   and target rebinding. Both transports supply source bytes. Resolve fog edge/
   rim work from the actual typed, live-editable `FogLook` in the same owner.
4. Route the browser through synchronous direct preparation/rendering. Acquire a
   usable output before consuming per-frame knowledge updates. After device
   recreation, refresh static generation and resend current knowledge from the
   presentation; do not replay an old frame or delta.
5. Delete superseded frame/world snapshots, upload APIs, serde fog/scar transport,
   JS preparation callbacks and standalone presentation artifacts/imports. Keep
   external asset/mesh/overlay sources genuinely needed for reconstruction.

Independent owner changes may run concurrently; shared runtime/browser files
must have one integrating editor. Preserve current behavior first. An optional
experiment may pass the final output view directly to the shared encoder to
remove the browser's extra display-composite pass; measure it separately from
module unification rather than attributing combined gains to packaging.

## Acceptance

- Both targets compile and call the same runtime prepare/encode boundary.
- Canonical layouts/digests and native count/replay behavior remain unchanged.
- All native incumbent plates remain identical; the frozen browser street keeps
  its whole-image and local landmark parity against the TypeGPU control.
- Real device recreation preserves complete pixels without manual scene replay.
  Current camera/knowledge and held static generations recover correctly.
- Renderer allocation cannot leave UI/audio with detached memory views.
- Compare the actual unified GPU path with the saved separate-module incumbent:
  frozen and changing frames, CPU encoding/preparation, GPU timestamps, startup,
  recovery, payload ownership and WASM high-water memory. Packaging-only sizes
  and empty-world Node memory are feasibility evidence, not battle performance.

The scratch packaging experiment and exact evidence boundaries live in research.
Continue directly to slice 04 when these gates pass.
