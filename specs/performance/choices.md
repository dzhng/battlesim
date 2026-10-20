# Performance choices

These decisions describe the reactive presentation and Metro XL work. Earlier
shipped grass and shadow-filter experiments are historical baseline behavior;
their uncertain marginal gains remain explicit in [research.md](research.md).
The user’s final rule for further work is to keep only simple, clearly measurable
wins, superseding the earlier percentage target.

## Sound

### Spend resolution on the world separately from the interface

On Low, the 3D world renders at three quarters of the canvas width and height;
HTML controls retain their ordinary resolution. The alternative spends full
pixel work on the world even on the lowest quality setting. This deliberately
trades some distant world sharpness for lower GPU work and target memory.

- **Gap:** Low had no quality-controlled resolution lever.
- **Reach:** the production Low profile; Medium and High retain full scale.
- **Verdict:** sound, backed by the measured quality comparison and visual gate.
- **Confidence:** high on the measured adapter; gains elsewhere are unmeasured.

### Reuse temporary storage without caching answers

Each frame rebuilds shadow and membership lists, but keeps their allocated
storage. A camera change likewise alternates two building-list buffers, keeping
the previous records available for comparison. Recreating those containers
would allocate repeatedly; caching their answers would risk stale visibility.
Reuse preserves record order and freshness while reducing measured CPU cost.

- **Gap:** the presentation slice specified equivalent output, not allocation ownership.
- **Reach:** browser and native consume the same unchanged presentation records.
- **Verdict:** sound; native measurements and replay parity support retention.
- **Confidence:** high. Building-list tail latency did not improve in every run.

### Give benchmark failure a terminal result

When a graphics reel stalls, the page publishes an incomplete result. Its
deadline follows the selected shots’ durations plus setup margin, so valid new
shots do not cause a fixed-duration timeout. The runner keeps a larger diagnostic
ceiling. The alternative leaves the caller waiting without a usable verdict.

- **Gap:** the original harness did not own a bounded completion outcome.
- **Reach:** graphics-reel success, failure and future shot additions.
- **Verdict:** sound; incomplete results must never count as performance passes.
- **Confidence:** high.

### Measure the player’s actual quality preference

When endurance requests Low, it writes the same saved preference a player uses.
Unsupported names fail. The former development-only URL parameter could be
ignored in production and silently benchmark the wrong profile.

- **Gap:** the harness’s quality flag lacked a reliable production mechanism.
- **Reach:** reproducibility of endurance measurements; no new renderer setting.
- **Verdict:** sound.
- **Confidence:** high.

### Keep distant detail in the existing asset owner

Thousands of visible cars occupy only a few pixels each. Their far mesh omits
unreadable details while nearer meshes, wrecks and physical geometry stay intact.
Both clients load the same baked asset. The alternative adds renderer lists or
shader variants to process excessive detail more cheaply.

- **Gap:** the spec did not choose between renderer machinery and authored detail.
- **Reach:** distant car appearance and shadows, including shadows cast by the
  next-nearer visible tier.
- **Verdict:** sound; controlled GPU improvement and reviewed images justify it.
- **Confidence:** high for measured views; not a claim about every framing.

### Evaluate visibility across gameplay views

When the camera sees most of a map, culling has little to reject. The earlier
terrain experiment measured only that wide view; it cannot settle the value of
culling when the player zooms into a street. The reopened plan measures close,
tactical and wide views separately, and holds local content fixed while remote
world grows. The rejected wall still lacks visual approval.

- **When:** reopened three-view plan.
- **Gap:** the user requested three levels without prescribing distances.
- **Reach:** use production default 65 m, existing tactical scale 260 m and prior
  stress scale 2,400 m, resolved through the player's camera rig.
- **Verdict:** sound; these cover distinct workloads without changing world density.
- **Confidence:** medium until baseline images confirm useful contact framing.

### Share far prop assets with both clients

At wide zoom a car may cover only two pixels. A textured card can represent it
cheaply, but the current browser body cards do not support all prop transforms
and native substitutes meshes. The plan extends shared asset data and both
consumers so a browser-only shortcut cannot masquerade as an engine improvement.

- **When:** reopened three-view plan.
- **Gap:** the user suggested sprites without prescribing storage or client scope.
- **Reach:** asset validation, browser/native rendering and independent shadows.
- **Verdict:** sound; prototype opaque cars first and retain meshes where cards
  cannot preserve appearance, rather than committing every class to sprites.
- **Confidence:** medium; required material/depth channels await the visual pilot.

### Invalidate old timing results without waiting for the GPU

After moving the camera, a readback from the previous view can finish late.
Reset now marks a new measurement generation: the old readback still releases
its buffer normally but cannot enter the new view's statistics. Blocking until
every old readback completes would unnecessarily stall the rendering thread.

- **When:** timer reset pass, `79cdb00cf`.
- **Gap:** the plan required clean windows without prescribing synchronization.
- **Reach:** explicit measurement resets and existing renderer reset paths.
- **Verdict:** sound; generation is captured when a frame begins, covering resets
  between begin, submission and collection as well as delayed promises.
- **Confidence:** high; delayed and rejected readbacks are regression tested.

### Reuse the fitted shadow volume for static casters

When a car stands below sea level, its shadow still depends on its height above
nearby receivers. The first candidate instead extended its box by its absolute
world elevation. That can omit a shadow below zero and over-admit geometry high
above zero. Static selection must use the same fitted light-space volume that
the shadow pass renders, including offscreen casters, rather than inventing a
second receiver approximation.

- **When:** independent prop caster candidate, `75489f4a8`; replaced by fitted volumes in `2c6e202d7`.
- **Gap:** the plan required independent selection without specifying its query.
- **Reach:** props, buildings and scenery share the corrected query.
- **Verdict:** original approximation unsound; the implemented replacement uses fitted caster volumes.
- **Confidence:** high; translating the existing offscreen-caster case below zero
  exposes a missed caster. Camera lists must remain independent.

### Group terrain triangles without changing their shape

When the camera looks at one street, the renderer can draw only nearby terrain
batches. The candidate groups whole triangles into spatial cells and stores a
binary tree of their enclosing boxes. A query skips a whole branch when its box
cannot meet the camera or a shadow volume. Crossing triangles keep all their
original corners and attributes; the cells never cut or simplify the surface.
Adjacent selected ranges combine into one draw to reduce call overhead. The
shared bounds index stores where each branch ends, allowing a query to jump
past rejected regions without allocating a traversal stack. Populations retain
their own ordering and visibility rules.

- **When:** terrain candidate, `5dc40aebd`.
- **Gap:** the spec delegated the index layout and range-coalescing policy.
- **Reach:** one Rust partition drives both clients; it retains reordered CPU
  vertices and the tree alongside the uploaded GPU mesh.
- **Verdict:** sound candidate; bounded traversal and exact geometry conservation
  justify testing it, while GPU, image and whole-frame acceptance remain required.
- **Confidence:** medium until the all-view production comparisons finish.


### Keep caster membership for each shadow cascade

A nearby tree may affect only one shadow image. Combining all selected trees
into one list and drawing that list into every shadow image repeats geometry
that the GPU will later discard. The next pass keeps four independent lists and
publishes which cascade each list belongs to. Browser and native draw the same
membership. Objects spanning multiple cascades appear in each required list.

- **When:** per-cascade pass, implementation in progress.
- **Gap:** independent camera/shadow selection did not specify cascade batching.
- **Reach:** the existing batch record uses a padding word for membership without
  increasing its size; both clients and batch-cache keys change together.
- **Verdict:** sound candidate; fixes measured repeated submission using existing
  fitted bounds. Four lists are simpler than grouping every combination of
  cascades, at the cost of duplicate records for overlapping casters.
- **Confidence:** medium pending measured GPU savings, upload/resource cost and
  shadow continuity across moving views. The corrected fit remains mandatory.
