# Prior reactive frame performance

Historical result, reopened after the user identified missing zoom coverage.
The terrain/caster conclusions below apply only to the measured wide view;
they do not reject culling at close or normal zoom. [The active plan](README.md)
supersedes the earlier completion boundary.

The browser and native client share presentation output and baked art. This
work reduces repeated presentation allocations and excessive detail in dense
Metro XL views without changing simulation outcomes. Low quality also renders
the 3D world at reduced resolution while HTML controls keep their CSS size.

The user accepted the market-town reel at roughly 39 FPS 1% low and set the
final rule for further XL work: **keep only simple, clearly measurable wins**.
That supersedes the original percentage-reduction target. This is not a claim
that every large battle now meets a fixed FPS floor.

## Why these changes

Temporary presentation lists need fresh contents, not fresh allocations.
Reusing their storage preserves iteration order and visibility semantics;
retaining a second cache of derived answers would require more invalidation
rules. The native replay provides evidence for CPU cost while shared records
and count parity constrain correctness.

A correctly culled view can still contain too much detail. The measured XL
opening admitted about 100,000 props, with 99.6% of selected bounds intersecting
the actual camera. Many parked cars occupied only a few render pixels. Reducing
their existing far mesh from 300 to 172 triangles saved about 3.9 ms GPU time
(12%) in interleaved comparisons with equal instance counts. Nearer car meshes
and wreck generation remain unchanged; no new renderer machinery is retained.

Low quality’s internal render scale is 0.75. The tradeoff is world-image
sharpness; the HTML interface remains at ordinary resolution. It is a separate
quality lever, not an explanation for every CPU or GPU bottleneck.

## Invariants

- Simulation publications and deterministic battle outcomes remain authoritative.
- Browser and native consume the same presentation records and asset identities.
- Quality measurements select the real production preference and confirm render
  dimensions. Development-only overrides are not production evidence.
- Interleaved GPU samples must settle after each change. Marginal pass omissions
  are diagnostic and cannot be added into a CPU/GPU percentage breakdown.
- Draw counts do not prove a performance win or a correct image. Keep the image,
  resource lifetime and whole-frame measurements beside the cost claim.

## Rejected approaches

Caster-list filtering, additional vertex-stage variants and cutout changes
showed no consistent GPU benefit. Simplifying courtyard walls lost distant
visual definition. Shared spatial terrain selection preserved geometry but
saved only about 0.22 ms within observed variation, insufficient for its
cross-client maintenance cost; the entire candidate was removed.

The early 27 ms “backbone” attribution mixed states in a rolling timing window
and was withdrawn. Terrain submission remains map-wide, but it was not the
material off-view bottleneck in this stress scene. Infinite-far views also admit
more world toward the horizon; strict map-size-independent work needs an
explicit visibility contract as well as spatial selection.

## Owners and evidence

[Presentation](../../crates/presentation/README.md) owns frame-local storage
and static building selection. [Scene assets](../../packages/scene-assets/README.md)
owns distant meshes, validation and baking.
[Graphics settings](../../apps/battle-lab/src/graphicsSettings.ts) owns quality
profiles; [browser verification](../../web/README.md) owns the graphics reel
and production endurance runner.

[Research](research.md) preserves workload-specific measurements, rejected
experiments and validation evidence. Earlier grass and shadow-filter changes
are explicitly labelled previously shipped baseline behavior with uncertain
marginal gains, not newly proven improvements. [Choices](choices.md) records
the durable ownership and acceptance decisions.

Visual acceptance of the distant-car change used matched before/after sheets
from the existing model,
with the far tier confirmed active and an unprimed review. These were generated
comparison evidence, not a new design reference. The original art and changed
art are recoverable from Git; scratch capture paths and measurement limitations
are recorded in the research ledger.
