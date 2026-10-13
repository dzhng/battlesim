# Air-war research and reuse

The plan reuses shipped repository contracts before introducing new ones:

- Helicopter air state, LowAir target layers, air grids, crash publication, and
  side-scoped contacts: [`specs/done/helicopters/README.md`](../done/helicopters/README.md).
- Guided projectile lifetime, motors, turn limits, guidance support, and
  `interceptable`: [`crates/contract/src/ballistics.rs`](../../crates/contract/src/ballistics.rs),
  [`crates/contract/src/weapons.rs`](../../crates/contract/src/weapons.rs), and
  [`crates/sim/src/flight/mod.rs`](../../crates/sim/src/flight/mod.rs).
- Existing side-local optical sensing and Ground fog: [`crates/sim/src/sensing.rs`](../../crates/sim/src/sensing.rs),
  [`crates/sim/src/visibility.rs`](../../crates/sim/src/visibility.rs), and
  [`crates/sim/src/knowledge.rs`](../../crates/sim/src/knowledge.rs).
- Purchase/deployment/refund ownership: [`crates/sim/src/battle.rs`](../../crates/sim/src/battle.rs)
  and [`specs/done/unit-roster/README.md`](../done/unit-roster/README.md).
- Audio provenance, source hashes, crop frames, processing, calibration, and
  runtime preparation: [`packages/battle-audio/README.md`](../../packages/battle-audio/README.md)
  and [`assets/README.md`](../../assets/README.md).

External audio research is deliberately a first slice. Slice 00 must record
the exact YouTube URLs, attribution/rights, download tool/version, hashes,
crop frames, and processing profile before a clip is assigned. The current
catalog demonstrates this receipt shape with the public-domain UH-60 source in
`fixtures/sounds.json`; it does not authorize a new jet clip by itself.

The user supplied the gameplay behavior and no external game reference is a
required authority. Any historical platform choice only names the roster row;
it does not import real-world performance or imply a balance claim.
