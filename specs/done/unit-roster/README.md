# Unit roster and first skirmish

This feature supplies the first playable faction roster and skirmish loop. The
catalog contains 150 tracked faction memberships (50 variants each for U.S.,
Europe, and the combined Eastern faction), grouped into family cards. A card can
remain visible while its variant is disabled until the corresponding capability or
model exists. Variants are the balancing and model identity; family cards are the
picker presentation.

The three factions deliberately share recognizable platforms where that is
credible. The roster is modern or near-future, with a small set of iconic
exceptions. Infantry stays role-based (standard, close-quarters, long-range,
anti-tank, and air-defense), and U.S. direct-fire TOW sits beside Javelin while
Eastern infantry uses RPG-7/RPG-29 and Kornet. Drones, aircraft, and helicopters
are catalog placeholders. Bombers, AC-130 gunships, recovery, breaching, and
fixed-wing transport remain outside the shipped scope.

## Why the shape is this way

A single catalog owns identity, family grouping, faction membership, availability,
price, weapons, and physical admission. This prevents the picker, simulation, and
asset pipeline from inventing different meanings for the same unit. Shared
platforms are shared data with separate faction membership, so Europe can field
U.S.-built equipment without manufacturing nationality-only copies.

The match starts with no deployed units. During a 60-second preparation phase each
side has 1,000 credits (five minutes at 200 credits per minute), buys units from
its faction picker, and places them one at a time through a cursor ghost. The
authority reserves the purchase, then sends the unit through the side's road-edge
spawn. A side may own at most 30 active units. Credits continue at 200 per minute;
kills, supply-truck destruction, and refunds are additional rewards. The browser
never owns a second wallet or target-priority rule.

Objectives encourage movement without making territory an economic snowball. A
seeded map has 3, 5, or 7 organic, fairly approachable sites, with one or three on
the centre line. Town sites are purpose-built squares or buildings, field sites
are open ground or farmhouse edges, and intersections are valid sites. Both sides
score in proportion to uncontested owned objectives; capture pauses under enemy
ground presence. Reaching 1,000 victory points or owning every objective ends the
match. The first opponent is a basic AI using ordinary recorded commands.

Combat rewards use causal death provenance: an ordinary kill pays 25% of the
victim's cost, adds 50% to the killer team's bounty, and pays the bounded comeback
bonus `min(B, 50%C, B*C/S)`. Supply trucks add 25% of their original cost. A unit
that physically returns to its own base refunds between 15% and 75%, weighted
between health and ammunition and decaying toward the floor over ten minutes;
blocked, cancelled, or destroyed returns do not pay. Trophy is a weapon-panel
capability with four charges, a three-second cooldown, ATGM/RPG interception, and
finite resupply from supply trucks.

Target acquisition first prefers an observed enemy that can damage the shooter at
that observed range, then purchase cost, distance, and stable handle. Existing
usable locks persist. This keeps price relevant without aim thrashing or using
hidden enemy state.

## Invariants and code pointers

- Catalog and membership: `fixtures/units/roster/*.json`, `packages/scene-assets/src/units.ts`,
  `web/tests/unitRoster.test.ts`.
- Admission and hard cutover: `crates/sim/src/catalog.rs`, replay/scenario admission,
  and `crates/sim/tests/catalog.rs`.
- Purchase, preparation, entry, 30-unit cap, and refunds: `crates/sim/src/purchase.rs`,
  `crates/sim/src/withdrawal.rs`, `crates/sim/tests/purchase.rs`, and
  `crates/sim/tests/withdrawal.rs`.
- Objectives, score, terminal referee, and seeded sites: `crates/sim/src/objectives.rs`,
  `crates/sim/src/referee.rs`, `crates/mapgen/src`, and the skirmish/objective tests.
- Settlement and bounty: `crates/sim/src/settlement.rs` and its integration tests.
- Trophy and logistics: `crates/sim/src/protection.rs`, `crates/sim/tests/protection.rs`,
  and the published observation decoder.
- Threat-first targeting: `crates/sim/src/weapons.rs` and targeting tests.
- Browser picker, ghost, cards, and weapon rows: `web/src/battle`, with focused
  coverage in `web/tests/unitRoster.test.ts`, `skirmishPurchases.test.ts`, and
  `prepareBattle.test.ts`.
- Physical appearance and runtime bindings: `assets/catalog.json`,
  `assets/runtime/catalog.json`, `packages/scene-assets`, and the family manifests
  under this directory. Runtime validation covers the baked model bundles and
  generated icons.
- Native/WebAssembly identity is intentionally hard-cut. Paired parity fixtures
  under `fixtures/parity/` are regenerated with the current engine/catalog identity;
  incompatible replays are refused rather than migrated.

The implementation is intentionally a capability slice. Future disabled cards may
remain in the picker, while unsupported air, drone, artillery, and transport
mechanics are deferred to their own specifications. No hidden compatibility layer
should be added to this feature.

## Divergences and rejected paths

The planning ladder originally described a generic encounter guard that required an
initial blue unit; the skirmish path instead admits an explicit road-edge base so a
zero-unit opening is valid. Map generation also advanced its preset revision and
compact medium depth, so parity records were regenerated at closeout. Generic wreck
classes are shared across vehicles; footprint validation therefore uses the
catalog's declared tolerance rather than requiring every hull to have a unique wreck
mesh. Earlier UI critique notes were superseded after the fog ghost, objective
labels, faction label, and disabled-card contrast were corrected.

The economy intentionally does not taper passive income, award objective credits,
add upkeep, or add a hard timer. Sophisticated AI, multiplayer reward sharing,
full drone/air mechanics, and transport passenger behavior are separate future
work. These exclusions preserve a readable first battle and avoid balance systems
that were not needed for the first playtest.

## Visual provenance

[broken-arrow-unit-picker.png](assets/broken-arrow-unit-picker.png) is the user's
Broken Arrow screenshot (attached as “Screenshot 2026-10-05 at 11.04.26 PM.png”).
It established the REC/INF/VEH/SUP/HEL/AIR category pattern, family-card grouping,
variant selection, and adjacent transport presentation.

The production scene is `web/scenes/unit-roster.mjs`. Successful captures and model
sheets are kept in ignored `throwaway/evidence/unit-roster/`; the accepted visual
review is `final-visual-critique.md`. The older `visual-critique.md` and
`startup-failures.json` are historical evidence only and are superseded by the
final capture after the visual fixes.
