# Persistent aircraft and layered air defence

**Status:** slice 01 contract cutover is in progress. The `HighAir` layer and
authored air mobility layer are landed and Rust workspace compilation is green;
publication, sensor, weapon, and observation fields remain for this slice.
**Next pickup:** finish slice 00's audio/source provenance pass and continue
slice 01's contract/observation cutover.

This plan replaces the old one-pass jet-sortie proposal in
[`battle-foundation/slices/20-jet-sorties.md`](../battle-foundation/slices/20-jet-sorties.md).
It adds persistent fixed-wing aircraft and a weapon-authored air-defence system
to the shipped helicopter air layer. The cutover is hard: no compatibility
adapter, replay migration, or parallel sortie engine.

## Next Agent Prompt

You are implementing the persistent-aircraft and air-defence plan. Read this
README and the slice you are picking up before editing code. Start with slice 00
and record the selected YouTube source receipts in the battle-audio catalog;
use generated placeholders until a source is accepted. Then implement slices in
dependency order. Keep Rust simulation state authoritative and keep all
aircraft, radar, missile, fuel, dogfight, and refund state in the simulation;
presentation and browser code only decode observations and send typed commands.
Run the narrow check named by each slice. Every slice that produces a visual
shot runs `game-ui` where it changes player-facing air UI, uses
`compare-screenshots` when a target exists, runs `screenshot-critique` last,
and offers the shots through `preview-shots` as a non-blocking review window.
Update this handoff, the slice verdict, the choices ledger, and evidence links
before ending a pass. Do not enable the full roster before the admission slice.

## Goal and player contract

Fixed-wing aircraft are persistent HighAir units at a shared cruise height of
about 1,000 m AGL. Helicopters keep their shipped LowAir contract. Aircraft
enter from the map edge after a seeded 15–30 second traversal delay, beginning
fuel consumption on map entry. The player places the intended destination with
the ordinary purchase ghost; the simulation chooses the nearest friendly edge
entry point. Traversal cards are greyed and show a traversal state. Aircraft do
not draw ground circles; their compact info card attaches to the traversal ghost
when Space shows planned positions.

Aircraft see HighAir well and LowAir reasonably but never reveal Ground or clear
ground fog. Ground AA sees HighAir at long range and must have a physical line
of sight through terrain, buildings, and trees; AA hidden in forest is hidden but
cannot fire through the foliage. Radar is separate: every radar-capable ground
AA emits an always-on 360-degree signature, and only SEAD aircraft detect that
signature. A radar observation is an approximate area with bearing and range,
labelled as radar in the UI; it never reveals Ground fog or exact identity.

Every faction has at least one short-, medium-, and long-range AA capability.
Short AA includes a ballistic weapon that can engage Ground with reduced
accuracy, LowAir, HighAir, and incoming missiles. Medium missile AA targets
LowAir and HighAir; missile interception is false by default and authored per
weapon. Long AA targets HighAir, has 4–8 rounds, fires one at a time with a
slow reload, and may intercept missiles where its weapon row says so. Stinger
and Igla-style short missile weapons cannot intercept missiles.

SEAD is dedicated to the EA-18G Growler, Tornado ECR, and an Eastern Su-34
variant. SEAD weapons have the longest aircraft range and may target only a
visible radar emitter or a radar contact. They can fire at an emitting long-range
AA through ordinary fog. Aircraft AA missiles use authored seeker FOV (70° is
the initial value), turn rate, lifetime, and `decoyable` fields. Flares are
finite, reload slowly, and deploy automatically on seeker lock when equipped.

Aircraft carry an unlimited gun and up to four authored weapon rows. They
auto-target by legal range then value, prioritising air targets when AA missiles
are available. Explicit air orders, attack-move, ground attack, bombing and
strafing remain simulation commands. Mutual automatic fire can create an
arbitrary-size dogfight; explicit orders break it, range loss breaks it, and
neighbouring dogfights need not merge. Afterburner is an explicit toggle: +50%
speed, 2x fuel burn, and worse ground-attack accuracy. Near-empty aircraft
retreat automatically and refund health/ammunition/fuel-weighted value.

The first useful checkpoint is a complete SEAD-versus-SAM exchange: radar
contact, longest-range launch, anti-missile interception, reload exposure, and
the fog boundary. The final integrated checkpoint adds mixed dogfights,
ground attacks, ingress, fuel retreat, cards, jet approach audio, and the air
alarm. Drones, passenger transport, campaign, deck building, multiplayer, and
finished cinematics remain out of scope.

## Single-owner invariants

- `contract` owns HighAir/LowAir identities, weapon fields, radar/fuel/air
  observation schemas, command payloads, and validation.
- `sim` owns aircraft motion, ingress, fuel, dogfight membership, radar
  knowledge, target selection, flight/interception, damage, and refund math.
- `presentation` owns side-filtered frame construction and audio causes; it
  never invents a target, radar contact, or fuel transition.
- `web` and renderer packages draw published contacts, ghosts, cards, and
  orders, and send commands through the existing input boundary.
- Fixtures own tier, sensor, weapon, role, fuel, flare, and model numbers;
  no unit-name branch substitutes for authored capability data.
- The old immediate fixed-entry spawn and one-pass sortie route are deleted by
  the cutover; no adapter keeps them alive.

## Slice graph

`00 → 01 → 02; 02 → (03 ∥ 04); (03 + 04) → 05; 02 → 06; (03 + 04 + 06) → 07; (03 + 05 + 07) → 08; (01 + 03–08) → 09 → 10; (00 + 01–10) → 11`

Slice 00 may prepare audio receipts in parallel with slice 01 but produces no
gameplay. Slices 03 (interception) and 04 (radar contacts) can be developed in
parallel after slice 02's contract; slice 05 (dogfights) consumes both. Slice
06's ingress state can proceed after HighAir motion, while its combat consumers
wait for the preceding weapon and contact contracts.

| Slice | Contract | Runnable proof |
|---|---|---|
| 00 | YouTube source receipts and jet/alarm audio preparation | Offline audio fixture and workbench audition |
| 01 | Hard-cut HighAir/LowAir, weapon, radar, fuel, and observation schemas | `/lab/air-contracts` |
| 02 | HighAir motion, no Ground vision, and deterministic map bounds | `/lab/high-air` |
| 03 | Projectile interception, authored seekers, decoys, FOV and LOS | `/lab/air-defense` |
| 04 | Radar signature and approximate SEAD contacts | `/lab/air-radar` |
| 05 | Arbitrary dogfight state and explicit-order/range exits | `/lab/dogfight` |
| 06 | Persistent ingress, ghost/card state, fuel, afterburner, retreat/refund | `/lab/air-ingress` |
| 07 | Aircraft guns, bombs, strafing, SEAD target restrictions | `/lab/air-ground` |
| 08 | Aspect damage, flares, splash, missile lifetime evasion | `/lab/air-survivability` |
| 09 | Faction AA matrix, SEAD profiles, roles, models, and weapon admission | `/lab/air-roster` |
| 10 | Cards, ghosts, radar glyphs, no-circle presentation, and controls | `/lab/air-hud` |
| 11 | Jet/alarm audio and integrated combined-air battle/balance | `/lab/air-audio` and `/battle/air-combined` |

## Verification policy

Each slice names a native test, a browser/WASM seam test, and a deterministic
lab scene. Replays and digests must remain equal for equal seeds and commands.
Visual slices judge one variable at a time. Run `game-ui` before changing a
player-facing air surface; use `compare-screenshots` against any approved
reference or prior candidate; run an unprimed `screenshot-critique` as the last
visual acceptance check; and offer shots through `preview-shots` without
blocking implementation. Run the full repository checks and the complete
browser scene set once slice 11 closes.

## Open evidence, not open design

The exact YouTube jet/alarm clips, crop ranges, and processing profiles are
selected in slice 00 and recorded with URL, attribution, rights, hash, frames,
and the existing audio tool's calibration recipe. A generated placeholder keeps
the slice runnable before a clip is accepted. Balance numbers are provisional
fixture data: the stated hit-count relationships are the gate, not a claim of
real-world lethality.
