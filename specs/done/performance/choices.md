# Performance choices

This ledger describes the retained feature, including the earlier shipped Low
resolution, car mesh and Rust storage changes. The user's stopping rule is simple,
clearly measurable wins; there is no arbitrary percentage target. Verdicts judge
architectural tradeoffs; the rationale records verification scope and limitations.

Review these first: retaining per-cascade lists despite no clear wide-view GPU
gain; keeping conservative forest motion bounds and standing storage; leaving
wide-view walls and vegetation on their existing representations. All three are
sound with medium confidence in the tradeoff the user would choose. No unresolved
user-only decision or unsound retained choice is identified.

## Sound — medium confidence

### Keep separate lists for the four shadow images

When a tree shadows only nearby ground, it belongs in the nearby shadow image,
not every image used for farther ground. These images are called cascades. Keep
one caster list per cascade and let browser and native draw the named list. A
tree that affects several cascades has a copy of its record in each. The simpler
union-list alternative repeats every selected tree in all cascades; grouping
all possible overlap combinations would add a more complicated batching scheme.

- **When:** reopened performance work, per-cascade integration.
- **Gap:** independent camera/shadow selection did not specify cascade batching.
- **Reach:** both clients, batch-cache keys and the shared batch record. A former
  padding word carries the mask; the record stays the same size. Overlap duplicates
  instance storage and can increase buffer residency substantially.
- **Verdict:** sound for avoiding irrelevant cascade submissions while preserving
  offscreen shadows. Retention does not imply a clear wide-view GPU speedup.
- **Confidence:** medium; correctness and bounded submission justify the structure,
  but its memory cost is a meaningful tradeoff under the simple-wins preference.

### Keep forest geometry after learning that a tree fell

When a distant tree falls, retain its installed geometry and update which trees
this side knows have fallen. Search boxes already enclose possible falling poses;
selected chunks choose standing trees, falling trees and stumps from that
knowledge. Original upright bounds still control standing visibility and detail.
The alternative rebuilds the forest after each new fall, or maintains another
index and geometry copy just for fallen trees.

- **When:** immutable forest pass `51118fac1`.
- **Gap:** bounded mutation work was required, but storage ownership was unspecified.
- **Reach:** both clients retain standing storage even after trees fall. Wider
  conservative boxes may admit extra candidates. Full incoming known-fall snapshots
  still require linear equality/reconciliation work; selection is not a promise
  that every publication costs the same regardless of history.
- **Verdict:** sound; one geometry/index owner avoids whole-forest rebuilds without
  changing side knowledge or animation.
- **Confidence:** medium; extra residency and conservative selection are deliberate.

### Retain existing walls, vegetation and other prop representations

A wide camera still sees many tiny walls and hedges after culling. Keep their
existing meshes and scenery detail rules. A wall must remain a continuous visible
barrier, and a hedge must retain its coverage and shadow. Adding new image-based
representations would create more appearance and transition policies to maintain.
The same stopping rule rejects the new car representation; these classes are not
waiting for it to ship.

- **When:** final appearance-class disposition.
- **Gap:** the plan required a decision for heavy classes, not sprites for all.
- **Reach:** walls, vegetation, planters and remaining props keep existing asset
  ownership. Wide-view geometry cost remains; future changes need their own
  quality and cost justification.
- **Verdict:** sound; preserve appearance when a replacement has not earned its cost.
- **Confidence:** medium; this deliberately leaves optimization opportunities open.

### Retain simple spatial batches and conservative selection

When the player looks at one street, search a hierarchy of enclosing boxes and
skip branches outside the camera or shadow volume. Terrain, water and boundary
meshes group whole triangles into spatial cells; adjacent selected ranges share
a draw. Static populations restore their original ordering after the search.
The alternative scans every item or submits the entire map each frame. More
precise selection inside every admitted chunk would add work of its own.

- **When:** reopened terrain and shared static-bounds passes.
- **Gap:** the plan left the hierarchy layout, cell size and range policy open.
- **Reach:** Rust owns the hierarchy and clip rules for both clients. Installed
  geometry and index storage grow with the world; local selection skips remote
  branches. Horizon-visible terrain can legitimately increase the selected work.
  Crossing triangles keep their exact corners and attributes.
- **Verdict:** sound; repair world-sized traversal/submission without introducing
  geometry simplification or an invisible distance cutoff. Keep coarse batches
  where finer selection has not earned its extra work.
- **Confidence:** medium; batching balances fewer draws against conservative excess
  geometry, and does not guarantee a win at every camera distance.

### Leave duplicate terrain storage and selected-list allocation alone for now

The browser installs terrain for drawing and also gives terrain to the shared
shadow fitter. Both use the same Rust partition algorithm, but retain separate
immutable copies. Some selected record lists also allocate again when selection
changes. Sharing storage or recycling every list could reduce these costs, but
would change ownership and invalidation paths beyond the needed repair.

- **When:** final complexity review.
- **Gap:** the plan did not specify one shared browser allocation or universal reuse.
- **Reach:** extra browser CPU/WASM residency and selection allocation remain;
  native already shares its terrain partition. No new allocator or cross-boundary
  lifetime API is introduced merely to remove the duplication.
- **Verdict:** sound to defer additional machinery without a demonstrated payoff.
- **Confidence:** medium; reconsider if residency or allocation becomes limiting.

## Sound — high confidence

### Preserve seeded AI choices across native and WASM word sizes

A seeded policy chose a different reinforcement on 64-bit native than on 32-bit
WASM because the hash was reduced after casting to the platform's `usize`. The
shared contract now truncates to the WASM-width `u32` before taking the bounded
role index, so native replays the browser's existing decisions. The alternative
would preserve a platform-dependent battle stream and make parity failures look
like renderer differences.

- **When:** ally-help parity diagnosis.
- **Gap:** the rendering spec assumes simulation parity but did not define integer
  width at this AI seed seam.
- **Reach:** seeded skirmish policy construction and native/browser replay probes;
  this is a simulation portability correction separate from renderer outcomes.
- **Verdict:** sound; keep the established WASM decision stream on every target.
- **Confidence:** high.



### Spend resolution on the world separately from the interface

On Low, render the 3D world at three quarters of the canvas width and height.
HTML controls keep their ordinary resolution. The alternative spends full pixel
work on the world even at the lowest setting. This intentionally trades some
world sharpness for less GPU work and smaller render targets.

- **When:** earlier shipped Low-quality pass.
- **Gap:** Low had no quality-controlled resolution lever.
- **Reach:** production Low only; Medium and High retain full scale.
- **Verdict:** sound; use the player's explicit quality preference for this tradeoff.
- **Confidence:** high; performance magnitude still depends on the adapter/workload.

### Let a blocked rolling turn use the existing reverse recovery

A tank beginning a turn beside a wall can collide with the wall even when its
planned forward movement is tiny. The same crew should back up to make turning
room whether the rejected move was stationary or rolling. Keep the existing
checks that reversing can free the turn and that the actual reverse step clears
obstacles and traffic; remove the distinction based on zero forward displacement.
The alternative leaves the tank repeatedly trying the same blocked move.

- **When:** separate movement correction exposed by closeout verification.
- **Gap:** the performance spec did not prescribe repairing unrelated movement;
  repository closeout requires observed failures to be addressed.
- **Reach:** tracked vehicle turns against static obstacles, with ordinary
  acceleration and collision rules unchanged. Battle outcomes may change when
  a previously stuck unit reaches combat; renderer-only digest guarantees do not
  include this explicitly separate correctness change.
- **Verdict:** sound; reuse recovery rather than adding a special unit, side,
  speed override or new movement state.
- **Confidence:** high.

### Keep minimum stroke coverage in actual render pixels

When Low draws the world at reduced resolution, a line authored to cover one
screen pixel can land on less than one rendered pixel and become faint. Keep
ordinary line widths and unit body sizes in screen coordinates, but enforce the
minimum line coverage using the canvas's actual resolution. Pass that ratio
through the existing viewport callbacks instead of giving each overlay its own
quality-setting lookup. The alternative makes every mark larger or adds a
special case for one distant aircraft stem.

- **When:** closeout contact-stroke correction.
- **Gap:** the earlier resolution change did not define the minimum coverage unit.
- **Reach:** world-rendered contact and order marks share one width rule. Thin rims
  and keylines can cover more pixels; the unit body radius stays unchanged. HTML
  controls are unaffected. Existing pointer caches invalidate when the ratio changes.
- **Verdict:** sound; repair the coordinate contract across consumers without a
  new quality owner or additional draws.
- **Confidence:** high.

### Reuse temporary Rust storage without caching changing answers

When another frame needs temporary lists, reuse their allocated storage where
existing owners support it. When a building selection changes, alternate scratch
lists so the previous answer remains available for comparison. Recreating these
containers wastes allocations; reusing an old visibility answer would risk stale
objects. Recompute changing answers while retaining suitable storage.

- **When:** earlier shipped presentation-reuse pass.
- **Gap:** equivalent presentation output did not prescribe allocation ownership.
- **Reach:** shared browser/native presentation; this is selective reuse, not a
  claim that every static selector allocates nothing.
- **Verdict:** sound; preserves order and freshness with less temporary churn.
- **Confidence:** high.

### Keep distant car detail in the existing asset owner

When a car is a few pixels across, use its already-shipped simplified distant
mesh. Nearer cars, wreck identity and physical bodies keep their own geometry.
Both clients load the same asset. The alternative adds rendering machinery to
process details too small to read.

- **When:** earlier shipped car-mesh pass.
- **Gap:** renderer machinery versus authored detail was unspecified.
- **Reach:** distant car appearance and the mesh tier used for its shadows.
- **Verdict:** sound; a small asset change keeps runtime ownership simple.
- **Confidence:** high.

### Ship no new far-car image representation

A tiny car could be drawn as a textured rectangle, called an impostor. The new
pilot required surface reconstruction to sit on the ground correctly. Shading
each antialiasing sample made that path expensive; cheaper sampling could lose
cars. The final filtered candidate no longer disappeared in its expanded camera
states, but softness and temporal appearance remained uncertain; it was not qualified. Keep the simplified
mesh instead of shipping an unresolved appearance policy and new asset format.

- **When:** final pilot removal `8ce8d4d96` and disposition.
- **Gap:** the sprite suggestion did not require accepting a visually or
  computationally costly method.
- **Reach:** no prepared prop atlas, native prop-card path, depth channel or
  sample-frequency shader ships. Existing unit/corpse cards remain. A future
  approach can be reconsidered without inheriting this rejected machinery.
- **Verdict:** sound under the user's simple-wins preference. Filtering's earlier
  disappearance verdict was invalidated by byte decoding; the final disposition
  is unqualified appearance, not a claim that filtered sprites cannot work.
- **Confidence:** high in retaining meshes until a simpler replacement qualifies.

### Preserve general card transforms, replacement lifetime and asset identity

When an existing card represents a scaled model, transform its position and
lighting consistently with that scale. When new card textures replace old ones,
finish creating the replacement before releasing the working set; failure frees
only the unfinished replacement. When two appearances share asset bytes, keep
their logical names separate rather than returning the first name for both.
Malformed byte ranges are rejected by the existing asset reader.

- **When:** retained corrections from the card work and byte-decoding cleanup.
- **Gap:** these existing-owner failure cases were exposed during the pilot.
- **Reach:** current unit/corpse cards, workbench models and appearance loading;
  no prop-card policy or new container format is required.
- **Verdict:** sound; preserve general correctness fixes without the rejected pilot.
- **Confidence:** high.

### Select static casters using the shadow cameras actually rendered

A car outside the player's view can still cast a shadow onto visible ground.
Select it against the fitted light-camera volumes, independently of camera
visibility. A car below sea level follows the same rule as one above it. The
alternative derives a second approximate shadow reach from absolute world height,
which can omit shadows or admit unrelated objects.

- **When:** fitted caster-volume replacement and per-cascade integration.
- **Gap:** independent caster selection did not specify the query geometry.
- **Reach:** props, buildings and scenery share fitted-volume selection. Correct
  offscreen inclusion may increase work relative to the old incomplete lists.
- **Verdict:** sound; one shadow-volume authority for selection and rendering.
- **Confidence:** high.

### Fit shadows from local terrain receivers and reuse unchanged fits

When a remote mountain is added outside the camera, it should not stretch the
shadow depth range over the street the player sees. Query local terrain and clip
conservative triangle boxes to the camera. Retain the existing object headroom
and shadow reach. If camera, terrain and light stay unchanged, reuse the fit;
changing installed inputs or loading a world invalidates it. Animated caster
membership remains separate.

- **When:** local receiver fitting and receiver-cache passes.
- **Gap:** the plan required local fitting without specifying clipping or reuse.
- **Reach:** shared browser/native fitting. Visible horizon terrain still counts;
  fixed headroom/reach remain the existing renderer policy. Camera motion reruns
  local clipping rather than using a stale fit.
- **Verdict:** sound; eliminate remote elevation dependence and repeated identical work.
- **Confidence:** high.

### Keep the browser building buffer's all-or-unchanged publication contract

When one building or cascade list changes, publish the complete current building
record set into the browser's packed buffer. Its offsets then describe one
consistent publication. Sending only changed lists would require another scheme
to preserve offsets and old contents across replacements.

- **When:** per-cascade building publication integration.
- **Gap:** more caster lists raised the question of partial uploads.
- **Reach:** one population generation invalidates all its packed building records;
  the broader upload trades bandwidth for a simpler existing buffer contract.
- **Verdict:** sound; preserve the current ownership model rather than adding a
  partial-update allocator.
- **Confidence:** high.

### Use three production camera scales and distinguish kinds of work

When a street view and a whole-town view behave differently, report them
separately. Use the production camera at 65, 260 and 2,400 metres, with its normal
clearance rules. Keep moving windows separate from stationary holds. Count actual
submitted triangle-list draws, not allocated capacity; label omitted passes.
Report receiver work separately from cached fits and snapshot processing.

- **When:** reopened measurement contract.
- **Gap:** the user requested three zoom levels without exact distances or counter
  boundaries; the earlier wide-only framing could not settle local-view behavior.
- **Reach:** future comparisons must preserve camera, render size and local content.
  Partial submission counters are not a complete GPU workload or unique-object count.
- **Verdict:** sound; exposes different bottlenecks without averaging away regressions.
- **Confidence:** high.

### Reset timing windows without waiting for old GPU readbacks

After the camera moves, an old timing result can arrive late. Starting a new
window increments a generation number; a result is accepted only if its frame
began in that generation. Old reads still finish and release their buffers.
Waiting for them before rendering would create an unnecessary stall.

- **When:** timing-epoch pass `79cdb00cf`.
- **Gap:** clean windows were required without prescribing synchronization.
- **Reach:** asynchronous timing collection and renderer measurement resets.
- **Verdict:** sound; excludes prior-view work without blocking forward progress.
- **Confidence:** high.

### Give measurements a truthful quality setting and terminal failure

When endurance asks for Low, set the same saved preference a player uses; reject
unknown quality names. When a graphics reel stalls, publish an incomplete result
within a deadline derived from its shots and setup allowance. An ignored debug
parameter or an indefinitely waiting runner would leave the caller interpreting
the wrong workload or waiting without an outcome.

- **When:** earlier production-quality and reel-completion corrections.
- **Gap:** quality selection and bounded failure outcomes were not reliable contracts.
- **Reach:** production measurement setup and future reel additions. Incomplete
  results cannot count as successful measurements.
- **Verdict:** sound; the instrument must state what it ran and whether it finished.
- **Confidence:** high.
