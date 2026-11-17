# Supply helicopter

**Status:** slices 01–07 done; slice 08 next. **Updated:** 2026-11-17.

One supply helicopter per faction. It lands to resupply the units around it and
takes off again when it packs. It costs the same as the supply truck, carries
half its stock, and is bought from the LOG tab. While landed it is a ground
vehicle that tanks can kill. The skirmish AI flies one in when a fighting group
is running dry far from its truck.

## Next Agent Prompt

Pick up at **slice 08**. Browser scenes need `git lfs pull --include="assets/runtime/**"` and `--include="assets/third-party/**"`, and `bun run build:wasm`. Build serially: the disk was at 26 GB free on
2026-11-17, so check `df` first. Read
[`decisions.md`](decisions.md) before any slice; every row there is a given.
Load `tweak-mechanics` before a simulation slice, and `game-ui` and `renderer`
before slices 06–08. Write each slice's test first. Each slice's "Digest" line
says which pinned bytes it may move. A slice that moves any other pinned bytes
has a bug, not a re-bless. Ground battles must stay byte-identical through
slices 01–04 and 06–09. Two traps from slices 01–03:
- **The file-size ratchet never lets an over-budget source file grow in any
  commit.** A split committed first doesn't create room. The commit that
  needs bytes must also move an equal concept out of that file.
  `crates/sim/src/battle.rs`, `crates/contract/src/catalog.rs`,
  `crates/sim/src/skirmish_ai/mod.rs` and `crates/contract/tests/catalog.rs` are
  all over budget.
- **After any catalog rule change, run the sim lib tests** (`cargo test -p sim --lib`).
  Test decks patched in-test are validated too.
- **The ratchet script checks staged blobs.** Run it as
  `git add -A … && bun .agents/skills/ratchet-files/scripts/check-file-size.mjs $(git diff --cached --name-only)`.
- **After a digest change, also run** `cargo test -p presentation` and the web
  capture tests (`web/tests/captureObservations.test.ts`).
- **Scenes:** close a page before opening the next; the browser stalls after
  about six open WebGPU pages. Props added by encounter events don't render in
  the lab.

Before you end a pass, update this section: status, the next pickup point, and
the TODO below.

### TODO

- [x] [01](slices/01-ai-standing-supplier-drives.md) The AI's standing supplier drives (no outcome change)
- [x] [02](slices/02-lands-and-serves.md) A supply heli lands and serves (first playable)
- [x] [03](slices/03-landed-is-a-ground-vehicle.md) Landed, it is a ground vehicle
- [x] [04](slices/04-clear-landing-spot.md) Nearest clear spot, or NO LANDING ZONE
- [x] [05](slices/05-enemies-see-it-land.md) Enemies see it land (knowledge and publication contract)
- [x] [06](slices/06-rotor-winds-down.md) The rotor winds down and up (visual: rotor)
- [x] [07](slices/07-ground-mark.md) Landed, it wears the ground mark (visual: marker)
- [ ] [08](slices/08-rotor-sound.md) The rotor sound fades with the rotor (audio)
- [ ] [09](slices/09-ai-emergency-airlift.md) The AI flies one in for an emergency
- [ ] [10](slices/10-roster-cards.md) Three roster cards, shop order, voices, tutorial
- [ ] [11](slices/11-closeout.md) Closeout: full run, review, close-spec

## Contracts this feature changes

| Contract | Change | Slice |
|---|---|---|
| Catalog `capabilities.deploy` | gains `drop_seconds`; required on an aircraft that deploys, refused on a ground unit (P2) | 02 |
| Catalog `Mobility::Air` | gains `rotor_radius_m`; required on an aircraft that deploys (P6) | 04 |
| Rules `air.rotor` | gains `landing_reach_m` = 40 (P7) | 04 |
| Sim `Unit` | `flies()` / `airborne()` / `landed()` / `push_class()` / `target_class(rules)` (P1, P3–P5) | 02, 03 |
| Low hover (`movement/air.rs`) | clears every type that can land (D5) | 02 |
| Own observation | `MoveState::NoLandingSpot` (P9); `grounded` (P12) | 04, 05 |
| `IdentifiedUnit` + `IDENTIFIED_FIELDS` + TS decoders | `deployment: Option<DeploymentState>`, `grounded` (Q7, P12); D5's "never timers" note names the exception | 05 |
| Knowledge digest | tracks refresh layer and class at each sighting and fold both in (Q7) | 05 |
| Presentation emitter record | gains rotor speed (Q6) | 08 |
| `fixtures/audio.json` | `air_light_logistics` row | 08 |
| Shop order (`purchaseFilter.ts::shopOrder`) | ground movers before aircraft within a tab (P13) | 10 |
| Roster rows | abstract `<id>_airframe` parents, three supply rows (P14) | 10 |
| Model exporter (`catalog_frames.py::_variant`) | several types may draw one appearance if hull and mounts agree (P15) | 10 |

No compatibility shims and no migrations (P21).

## Slice graph

```
01 AI standing supplier drives ─────────────────────────────────────────────┐
02 lands & serves ─► 03 ground vehicle ─► 04 clear spot ─► 09 AI airlift ──┤
                         └─► 05 enemies see it ─► 06 rotor ─► 07 ground mark ┤
                                                    └─► 08 rotor sound ─────┤
                                                                             └─► 10 roster cards ─► 11 closeout
```

- 01 and 02 can run in parallel; 05 can run beside 04.
- **01 must land before 10.** Without it, the id tie-break makes the AI's
  opening buy a helicopter and park it as its truck.

## One owner per concept

These are the invariants every slice inherits. The finished code reads as
designed today, not tacked on.

| Concept | Owner | Never |
|---|---|---|
| On the ground or not | `deployment.rs` latch, read through `Unit::landed()` (P1) | inferred from motion, id, height, or a client timer |
| Flies vs in the air now | `Unit::flies()` / `Unit::airborne()` (P3) | one predicate doing both jobs |
| Layer and target class of a unit | `Unit::layer()` / `Unit::target_class(rules)` (P5) | weapons or knowledge reading `UnitType::target_class` |
| Height while landing | one height law in `movement/air.rs`, which also sets `air.velocity.z` (D3) | a second clamp, or `position.z` edited elsewhere |
| Nearest clear spot | one ring search shared with crash wreck placement (P8) | a second scan of props |
| Rotor speed drawn and heard | one presentation helper from (grounded, progress, target, `drop_seconds`) (P12) | a second calculator in TS, or a sim field named rotor |
| Standing vs flying supplier | the AI's one `drives` predicate on mobility (D11) | id, label or role special cases |
| Emergency trigger | one AI module with pure trigger functions (P17) | spread through `purchase.rs` |

## Gates every slice inherits

- **Test first, end to end.** Simulation tests are `crates/sim/tests/` battles on
  test units (P18). Tests read numbers from data (deploy times, reach, thresholds,
  low hover) and assert relations, so a retune leaves them green. No test pins a
  roster row, a stat, or a class name string.
- **Domain tests only** until closeout (`development.md` check ladder).
- **Visual slices** (06, 07, 10): run `compare-screenshots` against the before
  shot, and make an unprimed `screenshot-critique` the last check on every shot.
  Show the shots to the user inline. The user's review is never a gate: give
  ~5 minutes, then decide on the evidence and record the call in the slice file.
- **Digest discipline.** Each slice names the bytes it may move. Before
  re-blessing, prove that only the named cause moved them.
- **Review** (`review` skill) and an append to `choices.md` at the end of every
  pass.

## Firewalls

- The simulation never names an animation; presentation never decides physics,
  targeting, set-up, clear spots or AI urgency.
- Tests and labs field only `test_supply_heli`; the roster cards are game content
  (slice 10).
- `test_heli` and `test_transport_heli` gain nothing.
- No other helicopter lands; no new art; no truck retune.

## Review surface

- Battle lab supply route, `/labs/supply?variant=…` (`apps/battle-lab/src/routes/supply.tsx`,
  scene `web/scenes/supply.mjs`). The existing truck encounter stays the default
  variant. Each slice adds a variant: `heli`, `heli-fire`, `heli-ground`,
  `heli-shift`, `heli-crowded`, `heli-truck`.
- AI lab `/labs/supply-ai` (slice 09).
- Shop LOG tab and Armory (slice 10).

## Evidence

- [Decision map](visualizations/map.html) and [landing mock](visualizations/landing-mock.html)
  from the planning walk.
- Planning drafts and sweep notes live outside the repo in `throwaway/supply-heli/`
  and are not needed to build.
