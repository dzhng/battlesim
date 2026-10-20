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

Each frame rebuilds caster membership lists, but keeps their allocated
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

### Reject the far-car pilot when neither quality nor GPU cost passes

At wide zoom a car can occupy only a few pixels. We tried a baked image with
surface depth so those pixels could replace its distant mesh. Evaluating every
antialiasing sample preserved cars but added about 41 ms GPU time. Cheaper
sampling and then filtered surface reconstruction still made some eligible cars
vanish. Keep the shipped simplified car mesh and remove the pilot's asset
format, native path and shaders. Existing unit/corpse cards remain; their generic
resource cleanup and appearance-identity fixes survive independently.

- **When:** three-view pilot rejection and removal.
- **Gap:** the user suggested sprites; the plan allowed meshes when a card could
  not preserve appearance and earn its complexity.
- **Reach:** no new prop-atlas format or maintenance surface ships. Future work
  must demonstrate coherent subpixel coverage/depth before adding it again.
- **Verdict:** sound; reject both visible loss and the measured large regression
  under the user's simple measurable wins rule.
- **Confidence:** high in rejecting tested candidates; no claim that every
  possible sprite method would fail.

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

- **When:** per-cascade pass, integrated; performance acceptance open.
- **Gap:** independent camera/shadow selection did not specify cascade batching.
- **Reach:** the existing batch record uses a padding word for membership without
  increasing its size; both clients and batch-cache keys change together.
- **Verdict:** sound candidate; fixes measured repeated submission using existing
  fitted bounds. Four lists are simpler than grouping every combination of
  cascades, at the cost of duplicate records for overlapping casters.
- **Confidence:** medium pending measured GPU savings, upload/resource cost and
  shadow continuity across moving views. The corrected fit remains mandatory.

### Reuse an unchanged shadow receiver fit

While the camera holds still, immutable terrain and light produce the same
shadow fit. Reuse that fit until the camera changes or its inputs are replaced;
world reload also invalidates it. The alternative repeats terrain clipping every
frame for an identical answer. This does not cache animated membership lists.

- **Gap:** the plan required bounded receiver work, without prescribing reuse.
- **Reach:** the shared presentation owner; both clients receive the same fit.
- **Verdict:** sound. Invalidation and exact output checks pass, and browser
  diagnostics report zero executed receiver work during holds.
- **Confidence:** high for unchanged views; moving views still recompute and
  require their own performance measurement.


### Keep forest geometry when a tree is learned fallen

When a distant tree falls, rebuilding every standing tree makes a local view pay
for the whole forest. Keep the installed geometry and widen its spatial search
boxes once to include possible falling poses. Selected chunks check the side's
known-fall membership to choose standing or fallen records. Original standing
bounds still decide visibility and detail. This keeps dead trees' standing storage
resident and may visit extra candidates, but avoids a second geometry copy and a
separate fallen-tree index. Full incoming snapshots still require work proportional
to known falls; this is not a constant-cost publication protocol.

- **When:** immutable forest pass `51118fac1` and diagnostic forwarding.
- **Gap:** the spec required bounded mutation work without choosing storage ownership.
- **Reach:** both clients share the Rust selection; public diagnostics distinguish
  cumulative setup/reconciliation rows from last-selection membership checks.
  Those counters exclude snapshot equality/replacement work.
- **Verdict:** sound; a single immutable geometry/index owner avoids world-sized
  mutation rebuilds without changing side knowledge or fall animation.
- **Confidence:** medium; the residency and conservative-query tradeoff is explicit,
  while final moving/live production measurements remain required.


### Retain non-car appearance policies without a validated replacement

A wide camera still sees many walls and hedges after culling. Keep their current
meshes and existing scenery detail rules while validating the car pilot. The
wall simplification already lost visible contrast; cutout experiments did not
show a repeatable gain. Adding more atlas types now would add coverage and shadow
risks without a demonstrated benefit. No objects or shadow contributors disappear.

- **When:** appearance-class disposition during combined acceptance.
- **Gap:** the plan required an explicit decision per heavy class, not a card for all.
- **Reach:** walls, vegetation and other props keep existing asset ownership; future
  replacements need class-specific cost, footprint and visual evidence.
- **Verdict:** sound under the user's simple measurable wins preference; preserve
  quality where the proposed replacement has not earned its complexity.
- **Confidence:** medium; this leaves known wide-view geometry cost in place.
