# Choices ledger

Decisions the implementation made where the spec was silent. Each entry says what triggered it, what the code does, what the alternative would have done, and the verdict. User rules live in requirements.md; spec-authored resolutions in decisions.md.

## Slice 01 — stack and 3D reproduction

### Screenshots are regenerated, not committed
- **When:** slice 01.
- **The choice:** A slice's evidence images (full frames, zoomed crops, metadata) are written by its browser scene into `throwaway/evidence/<fixture-id>/`. That folder is ignored by git. The committed record is the slice verdict, which gives the numbers and each critique finding with its outcome. For example, to see slice 01's depth frames, run `bun run --cwd web scene -- foundation` and open the folder. The spec had asked for images under `specs/battle-foundation/assets/evidence/01/`, which would put every re-capture's PNGs in git history.
- **The gap:** The spec said to commit evidence images. The implementation workflow says raw captures stay out of git.
- **The reach:** Later slices follow the same rule. A reviewer who wants images must rerun the scene. Old captures are not preserved between runs unless copied aside first.
- **Verdict:** sound. Every image is reproducible from a seed-free fixture and one command. validation.md and every slice now say this.
- **Confidence:** medium.

### Lab fixtures have one registry file and one scene each
- **When:** slice 01.
- **The choice:** `apps/battle-lab/src/fixtures.json` lists each lab fixture: an id such as `foundation`, a route such as `/lab/foundation`, and a description. The React lab reads it to route pages. The browser test runner `web/scene.mjs` reads it too, and refuses to run if any fixture lacks a `web/scenes/<id>.mjs` scene or any scene lacks a fixture. The runner starts its own Vite dev server, so `bun run --cwd web scene -- foundation` works with nothing else running. The sibling repo instead needed a dev server already running and discovered scenes by walking folders.
- **The gap:** The spec asked for "one lab/scene registry" and a documented `scene -- <fixture-id>` parser, without saying what form either takes.
- **The reach:** Every later lab page (geometry, authority, movement, …) adds one JSON row and one scene file.
- **Verdict:** sound.
- **Confidence:** high.

### Browser checks run headless with the real GPU
- **When:** slice 01.
- **The choice:** The runner launches Playwright's Chromium with `channel: "chromium"` (Chrome's full headless mode) and WebGPU flags. On this Mac, that headless browser gets the hardware Apple Metal adapter, so screenshots and timing come from the real GPU. The sibling defaulted to SwiftShader, a CPU software renderer.
- **The gap:** The spec allowed a headful probe only if headless differed. It didn't say which headless mode or adapter to use.
- **The reach:** Performance slices (07, 16) measure the real GPU. A machine without a hardware adapter would fail the "hardware adapter reported" check instead of silently falling back.
- **Verdict:** sound.
- **Confidence:** high.

### The scene owns and destroys every GPU buffer itself
- **When:** slice 01.
- **The choice:** Tearing down a TypeGPU root in 0.12.5 does not free the buffers it created. The foundation test caught this: 11 buffers leaked per rebuild. `createScene` now records every allocation in an `owned` list and destroys each one in `dispose()`. The lab wraps the device's `createBuffer`/`createTexture` to count live resources, and the scene asserts the count returns to baseline after resize and rebuild cycles.
- **The gap:** The spec required cleanup to baseline but not how ownership works.
- **The reach:** Every future renderer module (fog, contacts, projectiles, rings) must use the same pattern, or the endurance slice will leak.
- **Verdict:** sound.
- **Confidence:** high.

### One pipeline draws both the world mesh and proxies
- **When:** slice 01.
- **The choice:** The renderer has a single TypeGPU pipeline. Its per-vertex input is position, normal and colour; its per-instance input is a placement (x, y, z, heading) and a tint (colour plus a highlight flag). Proxies (tank, soldier, truck, crate) are drawn instanced. The static world mesh is drawn once with an identity instance. 4× MSAA smooths edges. Lighting is one fixed sun plus ambient, with faces flipped toward the eye so hand-built wedges light correctly whatever their triangle winding.
- **The gap:** Primitive topology and scene factoring were explicitly delegated. This entry exists so later slices know the one pipeline to extend.
- **The reach:** Translucent forest canopies and fog will need a second (blended) pipeline variant.
- **Verdict:** sound (delegated discretion).
- **Confidence:** high.

### The camera uniform keeps only what the new scene reads
- **When:** slice 01.
- **The choice:** The sibling's camera uniform packed 48 floats, including focus, zoom, time and sun direction for its old environment shaders. The new one packs 40: view-projection, its inverse, eye, near plane and viewport size. A unit test checks the byte size against the WGSL struct.
- **The gap:** research.md said to port camera primitives. It didn't say whether to keep unused fields.
- **The reach:** Adding a field later means updating one packer, the struct and the size test together.
- **Verdict:** sound.
- **Confidence:** high.

### Web dependencies pinned to exact versions
- **When:** slice 01.
- **The choice:** `web/package.json` lists exact versions (`"typegpu": "0.12.5"`, not `"^0.12.5"`). They match what the sibling's lockfile resolved, and the sibling's lockfile was the starting lock. three.js, Tailwind and Radix were omitted.
- **The gap:** The spec said to preserve pinned resolved versions without saying how.
- **The reach:** Upgrading anything is now an explicit edit.
- **Verdict:** sound.
- **Confidence:** high.
