# 01: cloud law and depth math

## Contract

There is one pure definition of a cloud, which everything downstream reads.
That definition covers:
- **its state at any tick:** stepped bloom, full strength, then a stepped fade;
- **the smoke depth along a sight line:** the sum, over the live clouds, of each
  cloud's chord length times its density, counted only where the line is below
  the cloud's top.

The simulation's sensing (04), the fog sweep (04), the GPU fog (02/06), the
drawn cloud (07) and the sound (10) all read this definition. None of them has
its own copy.

## Seam

The seam is a new module, `crates/contract/src/smoke.rs`. It is pure, with no
world access, like `contract::swamp`:

```rust
pub struct SmokeCloudSpec {          // authored on the capability (slice 03)
    pub radius_m: f64, pub height_m: f64,
    pub life_s: f64, pub bloom_s: f64, pub fade_s: f64,
    pub bloom_steps: u8, pub fade_steps: u8,
    pub depth_per_m: f64,            // 0.2 (D1)
    pub concealment: f64,            // 0.1 (D1)
    pub look: String,                // names a presentation smoke_screen_looks row (07)
}
pub struct SmokeCloud { pub id: u32, pub centre: [f64; 3], pub popped: Tick, pub spec: SmokeCloudSpec }
pub struct CloudState { pub phase: u8, pub radius_m: f64, pub depth_per_m: f64, pub concealment: f64 }

/// None once the cloud has expired.
pub fn state(c: &SmokeCloud, tick: Tick, tick_hz: u32) -> Option<CloudState>;
/// Exact length of segment a→b inside the circle (centre, r), in plan view.
pub fn circle_chord(centre: [f64; 2], r: f64, a: [f64; 2], b: [f64; 2]) -> f64;
/// Smoke depth of one sample piece: Σ over the clouds whose top
/// (ground_z + height_m) is above sight_z, of the piece's chord × depth_per_m.
pub fn piece_depth(clouds: &[(SmokeCloud, CloudState)], a: [f64; 2], b: [f64; 2],
                   sight_z: f64, ground_z: f64) -> f64;
```

- **Steps:** bloom moves the radius through `bloom_steps` equal steps; fade moves
  `depth_per_m` and the concealment shortfall `1 − concealment` together through
  `fade_steps` steps down to nothing (choices: stepped law). The spec's starting
  values are 4 bloom steps and 5 fade steps; they are data.
- **Height:** this is the forest canopy rule, applied at each sample, so callers
  sample along the line the way `foliage_depth` does (`FOLIAGE_SAMPLE_M`). The
  chord stays exact in plan view.
- **Overlaps add together** (choices).

## What you can run

`cargo test -p contract smoke`.

## Tests

These are unit tests, because this is the complex logic AGENTS.md allows them
for. They read every value from a spec they build themselves.
- The chord for a miss, a tangent, a line through the centre, both ends inside,
  one end inside, and zero length.
- The phase never decreases. There are at most `bloom_steps + fade_steps + 1`
  distinct states. After `life_s` the state is `None`.
- Depth is 0 when the line is above the cloud's top.
- Two overlapping clouds add.

## Delegated

You may choose internal helper names, whether `Tick` is imported or passed as
`u64`, and how the test tables are laid out.

## Must stay green

The `contract` crate's tests. No other crate changes in this slice.

## Feedback that would change this slice

If the user wants a smooth fade, the stepped law becomes per-tick. Slice 02's
rebuild budget then has to be re-measured.
