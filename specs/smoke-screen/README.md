# Smoke screen

Selected tanks, high-end IFVs and elite infantry can **pop smoke**: a player
order that spends one charge and raises a stationary circular cloud centred on
the unit. The cloud hides like a very dense forest:
- an observer outside sees a few metres in;
- a unit inside is nearly blind;
- the enemy keeps the existing last-contact marker where the unit vanished.

The right play is to pop smoke, then reverse out. The skirmish AI plays it the
same way.

Every decision behind this plan is in [`decisions.md`](decisions.md), and the
walk that produced them is in [the map](visualizations/map.html). Choices made
where the spec was silent go in [`choices.md`](choices.md).

## Next Agent Prompt

**Status (2026-11-12):**
- **Done:** slices 01–09.
  - Slice 02's kill gate said **KEEP**.
  - The smoke lab route (`smoke`) plays the moment with real input: press H,
    watch the charges count down, see HIDDEN, see the cloud and the fog.
- **Pin moves named in `choices.md`:**
  - slice 03: the `test_smoke_*` rows shift test-type indices;
  - slice 03: dead keys were deleted from the frozen `physical-rules.json`;
  - slice 05: the layout change re-published the captures natively (the
    precedent of earlier layout changes).
- **Done:** slice 09 (26 roster carriers, matching D2; no pinned record moved)
  and the visual follow-up (seen clouds draw over the fog veil, starting at
  their first bloom step).

**Pickup:** [slice 11](slices/11-skirmish-ai.md), then [slice 10](slices/10-sound.md). Disk is tight (about 4 GB free). Before you touch
anything, read:
- `AGENTS.md`;
- `development.md`;
- this README;
- [`decisions.md`](decisions.md);
- the slice file.

**Warnings:**
- **Slice 02 is a kill gate.** If the GPU fog can't take a dynamic circle list
  inside the frame budget, stop and reslice before any catalog, UI or asset
  work. Don't fall back to grid smoke or drifting smoke: both contradict
  decisions D1 and D5.
- **Files over the size budget can't grow.** These include `battle.rs`,
  `publication.rs`, `reader.rs`, `catalog.rs`, `weapons.rs`, `fog/sight.rs` and
  `skirmish_ai/mod.rs`, and the ratchet blocks any growth. New logic goes in new
  modules. Deleting the old transient-prop expiries (slice 03) frees lines in
  `battle.rs` for its new call sites.
- **Slice 05 changes the closed publication layout.** Pull LFS first (`full run
  needs LFS`). Re-record the browser captures and re-bless the vectors in that
  slice, not later.
- **Pinned digests stay put** until slice 09. Slices 03–08 prove it.

**TODO:**
- [x] 01: cloud law and depth math ([slice](slices/01-cloud-law.md))
- [x] 02: GPU fog kill gate ([slice](slices/02-gpu-fog-gate.md))
- [x] 03: capability, order, cloud collection and the cutover, dormant ([slice](slices/03-sim-clouds.md))
- [x] 04: sight, concealment, fog sweep and cloud visibility ([slice](slices/04-sim-sight.md))
- [x] 05: observation and publication contract ([slice](slices/05-publication.md))
- [x] 06: GPU fog and the smoke lab ([slice](slices/06-gpu-fog.md))
- [x] 07: the drawn cloud ([slice](slices/07-drawn-cloud.md))
- [x] 08: player controls and panel ([slice](slices/08-controls.md))
- [x] 09: roster switch-on, the first playable skirmish ([slice](slices/09-roster.md))
- [ ] 10: sound ([slice](slices/10-sound.md))
- [ ] 11: skirmish AI ([slice](slices/11-skirmish-ai.md))
- [ ] 12: closeout ([slice](slices/12-closeout.md))

**Before you end a pass,** update this section: status, pickup, warnings and
ticks. Also append to [`choices.md`](choices.md) every decision you made that
the slice didn't.

## Slice graph

```
01 cloud law ──┬─> 02 GPU fog gate ───────────────┐
               └─> 03 sim clouds ─> 04 sim sight ─> 05 publication ─> 06 GPU fog + lab ─> 07 drawn cloud ─> 08 controls ─> 09 roster ─┬─> 10 sound
                                                                                                                                  └─> 11 skirmish AI ─> 12 closeout
```

| Slice | Contract it unlocks | What you can see or run |
|---|---|---|
| 01 | One definition of a cloud's state over time and of smoke depth along a line | Unit tests |
| 02 | The GPU fog takes a list of circles, with a WGSL ↔ Rust parity check | A probe and a frame-cost table, with a keep or kill verdict |
| 03 | `Capabilities.smoke`, `Order::PopSmoke`, the cloud collection, resupply. The old transient props are deleted | System tests; all pinned digests unchanged |
| 04 | Smoke depth in every sight path, concealment 0.1, the sweep and its cache, cloud visibility per side | End-to-end sim tests |
| 05 | Smoke clouds and charges in the observation and the publication | Codec round-trip; new captures |
| 06 | Fog closes behind clouds on screen | **Smoke lab route** (first visible) |
| 07 | A dense white-grey billow that swirls within its circle | Lab screenshots, with visual gates |
| 08 | POP SMOKE (key H), the SMOKE n row, HIDDEN | The lab, with real player input |
| 09 | Roster units carry smoke | **A skirmish where you can pop smoke** (first playable) |
| 10 | The pop and hiss, heard at a cloud you can see | Listening in the lab and in skirmish |
| 11 | The AI pops smoke on a losing unit, then reverses | A skirmish against an AI that uses smoke |
| 12 | Everything | The full check ladder and the balance report |

## One owner per concept

| Concept | Owner |
|---|---|
| A cloud's law over time (bloom, steps, fade) and smoke depth along a line | `contract::smoke`, new and pure, like `contract::swamp` |
| The smoke capability's schema | `contract::catalog`: `Capabilities.smoke` |
| Live clouds | `sim::world::smoke::SmokeClouds`, held by the world geometry |
| Popping smoke | `Order::PopSmoke`, applied only by `sim::smoke::pop`, called from `Battle::apply` |
| Sight depth (foliage plus smoke) | One `WorldGeometry::sight_depth`. `foliage_depth` stays the tree-crowns-only reader for launch checks and map analysis |
| Ground concealment | `sensing::ground`: the strongest of forest, shelter and smoke |
| Which clouds a side sees | `sim::smoke::seen_by`, reading the side's simulation fog field |
| Smoke in the observation | `ObservationFrame.smoke_clouds`, `OwnUnit.smoke` |
| GPU smoke depth | A single WGSL `fogSmokeStep`, held to `contract::smoke` by a probe parity test |
| The look and sound of a cloud | A `smoke_looks.<name>` row in `fixtures/presentation.json`, named by the capability (the weapon-look pattern), whose `sound` names the clip |

When this is done, the code must read as if smoke had been designed in from
the start:
- **No parallel concepts.** There is no second "smoke screen" concept, no
  transient-prop path, no grid smoke, and no compatibility decoding.
- **No hard-coded rules.** No rule reads a unit's role, family or name; who
  smokes is data on each row.

## Contracts that change

| Contract | Change | Slice |
|---|---|---|
| Catalog schema | `Capabilities.smoke` is added; `PropBody.lifetime_s` and its validation are deleted | 03 |
| Commands | `Order::PopSmoke { units }` is added (Rust in 03; `protocol.ts` in 08) | 03, 08 |
| Battle digest | Clouds take over the expiry slot, so a battle with no clouds hashes identically. Unit smoke state and `smoke_s` resupply progress fold only when the unit carries smoke. The roster shift is named in `choices.md` | 03, 09 |
| Observation | `smoke_clouds` (no popping unit, no kind) and `OwnUnit.smoke` | 05 |
| Publication layout | Group `smokeClouds`, own-unit field `smokeCharges`, look names list `smokeLooks`. Captures and vectors are redone once | 05 |
| Fog upload | A smoke-circle buffer and `fogSmokeStep` | 02, 06 |
| Presentation fixture | `smoke_looks`, `smoke_screen_budget` | 07 |
| `skirmish-ai.json` | `smoke_loss_share`, `smoke_window_s`, `smoke_reverse_m` | 11 |
| `sounds.json` and `audio.json` | Smoke pop clips and the look's sound slot | 10 |

## Firewalls

**What must stay unchanged:**
- **Pinned digests:** pinned test-unit and capture-unit digests through every
  slice; roster and skirmish digests through slice 08.
- **What smoke doesn't touch:**
  - `launch_clear`, `map_analysis` and mapgen read tree crowns only;
  - smoke blocks no movement, stops no rounds and gives no cover;
  - navigation and air planning: a pop bumps no obstacle revision;
  - helicopter clearance;
  - the hearing cues.
- **Files left alone:**
  - `fixtures/parity/map-layout/physical-rules.json` stays frozen;
  - wreck smoke keeps its look, drift and 4096-puff budget;
  - the TS and native foliage builders (`useBattleSession.ts:397`, `native/client/src/capture.rs:578`) stay untouched.
- **Behaviour:**
  - player units never pop smoke on their own;
  - the AI makes no extra random draws;
  - the observation never leaks hidden state.

**Out of scope:**
- wind and drift;
- smoke shells from artillery or mortars;
- smoke on aircraft;
- thermal sights that see through smoke;
- a smoke artillery mission;
- smoke that blocks movement or rounds.

## Review gates

**Visual slices (06, 07, 08):**
- Before any change the player sees, load `game-ui`; before renderer work, load
  `renderer`.
- Judge every shot before and after, or against the [mock](visualizations/mock.html)
  where it applies, with `compare-screenshots`.
- Run an unprimed `screenshot-critique` as the **last** check before accepting a
  visual slice.
- Open the shots for the user with `preview-shots`. This checkpoint is
  non-blocking: allow about 5 minutes; if the user is silent, decide on the
  evidence, record the decision in `choices.md`, close the shots and carry on.

**Rule slices (03, 04, 11):**
- Write the tests first.
- Test behaviour, not values: a test reads its thresholds from the fixture's own
  spec and never pins a roster value.
- Run a quick sample battle while iterating. The full balance report runs once,
  in slice 12.

**Every slice:** climb the check ladder only as far as the slice's question
needs. Run everything once, in slice 12.

## Still open

1. **Firing from inside smoke (D9).** A unit in smoke may fire at targets its
   side has spotted, under the existing shared-spotting rule. This is a default
   the user may overturn. If they do, slice 04 gains one rule: a mount whose eye
   is in smoke may not engage, and that rule must be expressed through data.
2. **The Centauro II (D2).** It is a wheeled tank destroyer with the
   `light_vehicle` role, not on the user's list, so it gets no smoke unless the
   user says so.
