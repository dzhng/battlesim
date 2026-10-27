# One-renderer choices ledger

## Sound

### Shared shadow resources own cascade attachment and ground-caster order

The browser had allocated four shadow layers and their camera records but did
not draw anything into them. The shared shadow owner now clears each fitted
cascade, binds its camera, and draws the shell-supplied packed ground caster
geometry. The native and browser shells can therefore use one cascade order and
depth policy; only the geometry source and platform resource setup remain at the
boundary. The alternative would have kept a browser-only shadow loop beside the
native pass.

**Gap:** the spec required one shadow owner but did not say whether attachment
clearing belonged to the owner or the shell. **Reach:** model/scenery caster
routing and receiver sampling must extend this same pass boundary. **Verdict:**
sound; the scene records four cascades per frame and the shared owner contains
the order. **Confidence:** high.

### Shared core owns post and overlay composition order

The browser used to copy the lit HDR target to the canvas and draw a diagnostic
overlay directly on the surface. That bypassed the production tone map, fog rim,
depth-tested overlay target, and halo blur. The shared core now owns the same
target sequence as the production frame: post writes the display target, the
rim overlays it, then overlay geometry resolves into its transparent target and
the halo/composite pass draws over the surface. The alternative would have kept
another browser-only compositor that could drift from native.

**Gap:** the spec required one pass owner but did not prescribe where the
surface composite should happen. **Reach:** native and browser must use this
ordering as later caster, model, and fog parity work lands. **Verdict:** sound;
the browser scene passes and the shared target owner is reused. **Confidence:**
high.

### Shared Rust owns attachment policy and pass ordering (fog-mask pass)

When the native client created the fog distance, compose, and rim passes, it
also decided which textures were cleared, resolved, or loaded. The shared core
now owns those choices, while native supplies only the presentation look bytes
and the browser shell can use the same resource owner. The alternative would
be two implementations that can silently disagree about which image is shown.

**Gap:** the original extraction plan named the owner but did not prescribe the
fog-mask API. **Reach:** future fog, post, and browser integration must reuse
this target/pass contract. **Verdict:** sound; the GPU work and order are
unchanged. **Confidence:** high.

### Keep filesystem and DOM asset loading outside the renderer core

Native reads effect sheets and runtime bundles from disk; browser fetches them.
The renderer core receives device-ready resources and records instead of
opening files or touching the DOM. This keeps one renderer while allowing two
platform shells. **Gap:** the spec allowed shell asset policy without naming
the exact handoff. **Reach:** effects and post extraction follow the same seam.
**Verdict:** sound. **Confidence:** high.

## Needs-user (provisional calls)

### Keep TypeGPU as a control until matched production parity

The implementation preserves the TypeGPU renderer while Rust gains production
passes, so screenshots and timings have a trustworthy control. The provisional
call is to delete it only after the shared Rust path owns every required pass
and lifecycle gate; this is reversible until that deletion commit.

**Gap:** the user delegated the migration decision but not the exact deletion
checkpoint. **Reach:** determines when browser source files and the dependency
can be removed. **Verdict:** needs-user, provisionally keep until parity.
**Confidence:** medium.

## Sound

### Rebuild target-bound shared resources together on resize

When the browser changes from 640×360 to 320×180, the target textures change.
The fog-mask and post bind groups point at those textures, so keeping the old
groups would submit work against stale attachments. The browser now swaps the
target owner and rebuilds those shared groups in the same operation. The
alternative would be a target swap with hidden old-resource ownership.

**Gap:** the spec required resize recovery but did not say which dependent
resources must be rebuilt atomically. **Reach:** every future size-dependent
pass must join this rebuild boundary. **Verdict:** sound. **Confidence:** high.

### Keep device-loss recreation as an explicit remaining gate

The browser shell can reconfigure an outdated or lost surface, and the scene
now exercises that call. A true device loss invalidates the device and every
GPU resource; the current shell reports the condition but does not recreate the
device and all owners. We keep that as an open gate rather than claiming the
surface recovery proves device recovery. **Gap:** the existing spike had no
device-loss injection contract. **Reach:** blocks final TypeGPU deletion until
the shell can rebuild the device/resource set or the product explicitly owns a
browser reload policy. **Verdict:** sound to leave open. **Confidence:** high.

