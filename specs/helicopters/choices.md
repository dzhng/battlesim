# Helicopters: choices ledger

Record every decision an implementing pass makes where the plan is silent: the
slice, the choice, the alternatives and why. Choices that change a given in
[decisions](decisions.md) need the user first.

| Date | Slice | Choice | Alternatives | Why |
|---|---|---|---|---|
| 2026-10-10 | 01 | `units::Motion { Ground(Mobility), Air(Flight) }` on `Unit.motion`. Ground-only sites read `Unit::ground()`, which panics for an aircraft, and catalog-level callers use `units::ground_mobility(t, rules) -> Option<Mobility>`. | A separate ground-only `GroundMobility` type with every navigation signature changed | Same guarantee (an aircraft can't reach ground rules) for about 40 lines instead of a signature change across navigation. The compiler forced every catalog-level caller to decide what an aircraft does. |
| 2026-10-10 | 01 | Objective eligibility stays owned by roster category (`Hel` excluded, `objectives.rs`); no `airborne()` check is added there. Its test moves to slice 17, where a roster helicopter exists. | Adding `!airborne()` to objectives | A second owner for one rule. |
| 2026-10-10 | 01 | Until slice 03, an aircraft takes move orders (any in-bounds point, facing kept) but holds where it is. The tests for forest lanes, treads and altitude-is-not-moving need a flying helicopter, so they move to slice 03. | Shipping a stub flight in slice 01 | Slice 03 owns flight. |
| 2026-10-10 | 01 | **D35 corrected:** physical units must carry at least one role (only `planned` cards have `roles: []`). Helicopters get a real role, and the AI's role picker finds them through it (slice 17). Until then, the test helicopter uses `light_vehicle`. | Category rotation entries (the old D35) | The code already picks by role. |
| 2026-10-10 | 02 | The spike's mockups are drawn over real game captures by a canvas script that read captures from the main checkout. The script isn't shipped because it only runs on that machine; the images are the record. | Shipping `compose.mjs` | A file with no durable purpose. |
