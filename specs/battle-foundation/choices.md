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

## Slice 02 — world geometry

### Traversability is judged per triangle, at its centroid
- **When:** slice 02.
- **The choice:** Whether ground units may stand somewhere depends on the slope of the ground triangle under them, plus whether it is water. The export tags every terrain triangle once, by querying the surface at the triangle's centroid. The centroid lies inside that triangle, so its slope is exactly that triangle's. The first attempt tagged grid vertices instead. A vertex touches up to six triangles, so its tag came from whichever triangle the lookup happened to pick, and colours blurred across cells.
- **The gap:** The spec fixed the 35° rule and the triangulation, but not how traversability is sampled for export.
- **The reach:** Navigation in slice 04 should use the same per-triangle rule, or the overlay and the router will disagree.
- **Verdict:** sound.
- **Confidence:** high.

### Trunks and bridge decks do not block ground movement
- **When:** slice 02.
- **The choice:** `PropKind::blocks_movement()` returns false for forest trunks and bridge decks, and true for buildings, walls, crates, wrecks and ruins. Trunks still stop bullets and sight rays, since they are solid for `raycast`. A tank can therefore drive through a forest (slower, per M02) without routing around every trunk. A bridge deck is something you drive on, not around.
- **The gap:** The spec made trunks projectile colliders and forests traversable, but never said whether trunks block movement.
- **The reach:** Navigation, and later wreck and ruin behaviour, use this single rule.
- **Verdict:** sound; M02 requires it.
- **Confidence:** high.

### Plateau ("mesa") relief replaced the ramp primitive
- **When:** slice 02.
- **The choice:** Map relief has two shapes: `ridge` (the village formula) and `mesa`, a flat-topped rectangle whose sides fall at a set angle. The first version had a one-sided `ramp`. It dropped to zero at its sides, which made accidental cliffs that read as broken geometry. The lab now shows the slope limit with a 20° mesa (passable) and a 45° mesa (blocked).
- **The gap:** The spec asked for "hill, slope threshold" without a shape vocabulary.
- **The reach:** Later maps are authored with ridges and mesas; new shapes are added to one enum.
- **Verdict:** sound.
- **Confidence:** medium.

### Water surfaces and bridge decks
- **When:** slice 02.
- **The choice:** Water is a rectangle whose ground is lowered to `bed_z`, drawn as a translucent sheet at `surface_z`. The sheet is not solid, so a ray passes through it to the bed. A bridge is an oriented deck prop, solid for rays and bullets, whose top `surface_at` reports as walkable. The ground underneath is still water.
- **The gap:** The spec required "an explicit traversable top surface over impassable water", but not whether water stops rays or bullets.
- **The reach:** Rounds fired at a river strike its bed. If water should stop bullets or sight, that needs a new rule.
- **Verdict:** sound for now.
- **Confidence:** medium.

## Slice 03 — authority, transport and replay

### A malformed command still uses up its sequence number
- **When:** slice 03.
- **The choice:** Every command carries a per-side sequence number (`seq`) that must be exactly the next one: 1, 2, 3, … If the number is wrong (a skipped or repeated command), the command is refused and the counter does not move. If the number is right but the content is bad (it names someone else's unit, a unit that does not exist, or a point off the map), the command is refused, but its number is used up and it is written into the replay log. So "move unit 99" as seq 2 is rejected, and the next command must be seq 3. A replay re-submits it and gets the same rejection. The alternative, logging only valid commands, would let replay acknowledgements drift from what the player saw.
- **The gap:** The spec asked for ordered acks and a replay of "accepted commands", but didn't define whether an invalid one counts as accepted.
- **The reach:** Networking and replay export will rely on this numbering.
- **Verdict:** sound.
- **Confidence:** medium.

### The main thread builds its own copy of the static map
- **When:** slice 03.
- **The choice:** The battle authority runs in a web worker. The page also builds a `WorldView`, which is Rust's world-geometry code compiled to WASM, from the same map JSON. It uses that view to draw the terrain and to find the ground point under a right-click. Only the fixed starting map is copied, which every side knows from the start. Anything that changes later, such as wrecks or ruins, must arrive through that side's observations. The alternative was asking the worker for every ground pick, which is an asynchronous round trip on each click.
- **The gap:** The spec said picks use "the authoritative surface representation" and that static geometry is public, but not where the pick runs.
- **The reach:** Slice 09's wrecks and slice 11's ruins must update this view from observed prop changes, never from hidden truth.
- **Verdict:** sound.
- **Confidence:** medium.

### Worker and in-thread parity is proven by replay
- **When:** slice 03.
- **The choice:** The same authority code runs in a worker (production) or in the page thread ("direct"). The lab's "Check replay" button asks the worker for its replay log, plays it back in the page thread, and compares the state digest (a fingerprint of the full battle state) at every tick. Identical digests prove same-build replay and worker/direct equivalence in one check. The in-thread copy really detaches transferred buffers, just like a worker, so a bug that reuses a buffer after handing it over fails here too.
- **The gap:** The spec asked for "direct versus worker parity" without saying how.
- **The reach:** Later slices keep this check green with no new wiring, since the digest covers all authoritative state.
- **Verdict:** sound, provided every new piece of authoritative state is added to `Battle::digest`.
- **Confidence:** high.

### The simulation stops at four catch-up ticks and says it is slow
- **When:** slice 03.
- **The choice:** If the page falls behind (a stall or slow machine), each wake-up runs at most 4 ticks. Then it drops the remaining wall-clock backlog and reports "running slow". It never skips ticks. The battle simply runs slower than real time. A consumer that stops returning buffers stops the ticking entirely, shown as "waiting-consumer". A hidden tab suspends. Resuming restarts the clock from now rather than bursting through the missed time.
- **The gap:** The spec required bounded catch-up and explicit slowdown; 4 is a chosen number.
- **The reach:** Slice 16's endurance run will show whether 4 is right.
- **Verdict:** sound.
- **Confidence:** medium.
