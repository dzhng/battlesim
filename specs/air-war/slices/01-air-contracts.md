# 01 — Air-layer and weapon contract cutover

**Contract:** add `AltitudeLayer::HighAir` beside Ground and LowAir. Fixed-wing
aircraft use a shared HighAir cruise of about 1,000 m AGL; helicopters retain
LowAir. Add typed fields for aircraft capability, authored sensor ranges,
fuel, flares, afterburner, AA tier, radar emission/detection, weapon FOV,
seeker support, interception target class, decoyability, and air refund
weights. Hard-cut old jet-sortie assumptions; no migration or adapter.

**Seam:** `crates/contract` catalog/scenario/weapons/observation/command,
`fixtures/game.json`, TypeScript catalog and WASM codecs. Validation rejects
missing layers, invalid weight sums, more than four aircraft weapon rows,
unsupported target combinations, and a short missile row marked as an
interceptor.

**Runnable proof:** `/lab/air-contracts` prints resolved HighAir/LowAir,
sensor, weapon, radar, fuel and faction-tier records without spawning units.

**Checks:** serde/fixture validation, digest layout round-trip, native/WASM
publication decode, and planned-card hard-cut admission tests.

**Scope firewall:** no runtime movement, radar contacts, missile interception,
UI styling, or named-unit branches.
