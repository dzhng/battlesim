# Native rendering choices

This ledger records the decisions that survive in the closed native-rendering
probe. The implementation owners are `native/godot-spike/reel.gd`,
`native/godot-spike/presentation_capture.gd`,
`web/src/battle/benchmark/presentationCapture.ts`, and
`native/godot-spike/reel_lifecycle_test.gd`.

## User constraints

- The full main-menu reel is the reference workload.
- The browser renderer is the release baseline; Godot is experimental until it
  meets the same visual target.
- Engine-specific rendering is allowed, but simulation and presentation must
  remain equivalent at the product boundary.
- The graphics report records frame-rate summary data and a frame-time chart;
  there is no hard performance threshold.
- Rust remains the authority for gameplay, and the native path must be tested
  as a renderer of that authority rather than as a second simulation.

## Needs-user

- **Keep the native path experimental after the comparison.** A user watching
  the paired cuts sees the same tank-and-city subject, but the native frame
  still lacks the browser's continuous crop texture, dust, grounding shadows,
  material richness and balanced framing. The reversible call is to keep the
  browser shipped and require a new visual-investment spec before promoting
  Godot. **Verdict:** needs-user; **confidence:** low.

- **Keep explicit presentation budgets for the probe.** The saved maps contain
  far more buildings and props than a display-backed diagnostic can instantiate
  comfortably. The renderer therefore admits the complete export but limits
  instantiated shells, props and grass as renderer LOD; removing the limits
  would turn a visual check into an unbounded memory test. The reversible call
  is to keep the current diagnostic budgets while parity is red and retune them
  only with fresh matched cuts. **Verdict:** needs-user; **confidence:** medium.

- **Treat native terrain, grass and atmosphere as approximation layers.** The
  browser's terrain material and effects are richer than the current Godot
  compatibility path. The native implementation keeps its procedural field,
  grass material and cloud shader renderer-owned so they can be replaced
  without changing map membership or visibility. The reversible call is to
  leave these approximations in the experimental path until a dedicated visual
  pass supplies a stronger material and effects contract. **Verdict:**
  needs-user; **confidence:** medium.

## Sound

- **Rust is the only simulation authority.** When Godot needs a unit pose or
  fog cell, it decodes the browser/Rust publication and renders it; it never
  advances a parallel battle or infers visibility from map geometry. This keeps
  replay, digest and side-filtering semantics owned by the existing Rust
  modules. **Verdict:** sound; **confidence:** high.

- **The comparison seam is an offline presentation capture, with GDExtension
  retained for the live binding probe.** A live native client needs an
  in-process seam, but visual comparison also needs the exact camera timeline
  and publication history that the browser displayed. The capture freezes that
  history without creating a second gameplay protocol; the GDExtension smoke
  remains a lifecycle probe. **Verdict:** sound; **confidence:** high.

- **Displayed camera frames and authority ticks have separate jobs.** Camera
  poses follow the browser's displayed-frame elapsed time, while ticks drive
  semantic units and fog. If both used a tick-derived clock, a render stall
  would move the native camera away from the browser even when the simulation
  digest matched. **Verdict:** sound; **confidence:** high.

- **Capture input is rejected at the browser boundary and checked again during
  native replay.** A malformed sample, camera pose or publication carrier is
  rejected before export; native decoding then checks packed groups, fog and
  ground bounds. The two checks protect an untrusted JSON artifact without
  moving simulation ownership into Godot. **Verdict:** sound; **confidence:** high.

- **Saved maps own placement, while renderer limits own only presentation.**
  Every exported building, surface and prop remains admitted and counted even
  when distance culling or an instance budget omits it from the current frame.
  This prevents a performance bound from silently becoming a different map or
  a different visibility rule. **Verdict:** sound; **confidence:** high.

- **Authored catalog assets own model identity.** Building kits, unit
  appearances, scenery and grass resolve through the existing checked-in
  catalogs, so browser and native refer to the same GLB sources. Primitive
  geometry remains an explicit fallback when a source is unavailable; it is
  never presented as proof of authored visual parity. **Verdict:** sound;
  **confidence:** high.

- **Configured building kits own building presentation.** When a kit is
  available, native shells are placed from the saved map and the gray primitive
  building batch is suppressed. Without a kit, the primitive batch remains the
  honest fallback. Drawing both would duplicate geometry and hide the authored
  shell. **Verdict:** sound; **confidence:** high.

- **Fog presentation follows captured visibility cells.** Native applies the
  Rust full/delta fog payload to a bounded dark-cell layer. It does not derive
  hidden cells from the map or from camera distance, so the visual overlay
  cannot become a second visibility oracle. **Verdict:** sound; **confidence:** high.

- **Display backing is an explicit readiness gate.** A headless Godot frame can
  prove lifecycle and decoder behavior but cannot prove a screenshot or a
  performance sample. The report therefore records the display server and
  keeps `comparison_ready` false unless capture validity, authored admission,
  named cuts, fog consumption, display backing and material review all hold.
  **Verdict:** sound; **confidence:** high.

- **Browser controls and native results are retained as provenance.** A native
  frame is judged against the browser control for the same workload and camera
  intent, not against another native frame. Keeping the paired images in the
  archived spec preserves the visual standard for any future promotion pass.
  **Verdict:** sound; **confidence:** high.
