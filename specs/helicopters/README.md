# Helicopters

**Status:** in progress. The simulation is done through slice 09, with 14 too; the browser slices are running. **Updated:** 2026-10-10.

## Next Agent Prompt

You are implementing helicopters. Read [decisions](decisions.md) first. Its D1–D43 are givens, and you don't reopen them inside a slice. Then read [landmines](landmines.md) and the slice you're picking up. Load [tweak-mechanics](../../.agents/skills/tweak-mechanics/SKILL.md) before any rule change, [write-tests](../../.agents/skills/write-tests/SKILL.md) before any behaviour change, and [renderer](../../.agents/skills/renderer/SKILL.md) or [game-ui](../../.agents/skills/game-ui/SKILL.md) for slices that draw.

**Next pickup, in priority order:**
1. Integrate slice 16's worktree (the other 18 helicopters). Everything else through 17b is merged. Merge each, rerun its narrow checks on the merged tree, and remove its worktree.
2. Slice 15 (the Apache on real art). It depends on 07, 09 and 10.
3. Slice 13 (the contact sign), after 12. Slice 16, after 15. Slice 17, after 16. Slice 18 last.

**What the simulation now owns:**
- **Motion:** `units::Motion { Ground, Air }`. `Unit::airborne()` and `Unit::layer()`. `ground_footprint` is `None` in the air, and ground-only sites call `Unit::ground()`.
- **Flight:** `movement::air` (`step_aircraft`, `low_hover`) and `navigation::air::AirGrid`, one per side.
- **Crashes:** `sim::crash`, plus `damage::detonate` and the `helicopter_crash` row. The browser does not see the falling airframe until slice 10.
- **Fire:** `weapons::reaches` reads `targets`, `weapons::hull_fixed` together with `Reach.face` aims hull guns, and the `guidance` property is required on guided rows.
- **Contacts:** they carry `z` and `layer`. A heard mask spans two 24-bit words, so up to 48 weapon rows.

The record of each slice lives in its slice file and in [choices](choices.md).

**Warnings:**
- **Harmless plinking is a feature.** Rifles and MGs fire at helicopters they can't hurt (D2). Never gate it.
- **Balance is out of scope.** The numbers are first-pass and must not be "clearly unbalanced". Don't tune them; tuning every unit is a later spec.
- **The user doesn't read code.** Confirm code facts yourself and ask only about intent or taste.
- Fetch only the LFS files a slice needs. The helicopter models are under `assets/source/roster/disabled/`.
- Format TypeScript with oxfmt (`bun run fmt`), never Prettier.
- Adding a weapon row renumbers the rows sorted after it. `combat.json`'s digests then move with no change in behaviour; prove that by renaming the new row to sort last before you re-record.

**Before you end each pass,** update this section: the status, the next pickup, the checklist below, and any decision you made where the plan was silent (recorded in [choices](choices.md)).

### Checklist
- [x] [01 — An airborne hull](slices/01-air-hull.md)
- [x] [02 — Design spike: the airborne lost-contact sign](slices/02-contact-sign-spike.md)
- [x] [03 — Flight: route, altitude, separation](slices/03-flight.md)
- [x] [04 — Falling airframe (simulation)](slices/04-falling-airframe.md)
- [x] [05 — Contacts carry height](slices/05-contact-height.md)
- [x] [06 — Test airframe art and rotor pipeline](slices/06-airframe-art.md)
- [x] [07 — First browser checkpoint: air-aware weapons](slices/07-first-checkpoint.md)
- [x] [08 — Hull-mount facing for every unit](slices/08-hull-mount-facing.md)
- [x] [09 — Helicopter missile and rocket rows](slices/09-heli-weapon-rows.md)
- [x] [10 — Drawing the fall](slices/10-fall-draw.md)
- [x] [11 — Damage smoke trail](slices/11-damage-smoke.md)
- [x] [12 — Drop line, ground ring and ghosts](slices/12-drop-line-ring.md)
- [x] [13 — The airborne contact sign, built](slices/13-contact-sign.md)
- [x] [14 — Resupply sink](slices/14-resupply-sink.md)
- [x] [15 — The Apache, on real art](slices/15-apache.md)
- [ ] [16 — The other 18 helicopters](slices/16-roster.md)
- [x] [17 — Skirmish: entry and AI](slices/17-skirmish.md)
- [x] [17b — Rotor sound](slices/17b-rotor-sound.md)
- [ ] [18 — Closing scene (D14) and closeout](slices/18-closing-scene.md)

## Goal

The 19 roster helicopters are playable for the player and the skirmish AI, and they feel like Broken Arrow and WARNO:
- They cruise nap-of-the-earth at 20 m and pop up over roofs.
- They fly around towers.
- They strafe, turning the airframe toward the target.
- They fire on the move.
- They resupply at a truck by sinking to a low hover.
- Rifles spark off their armour. Autocannons bring them down.
- A downed helicopter falls in an arc and flattens the trees it lands on.

**Done** is [D14](decisions.md): buy an Apache in a skirmish; it flies in, pops over a village and kills a tank; an IFV shoots it down; and the wreck flattens trees.

**Non-goals:**
- **Anti-air units.** Future AA, and the `high_air` layer (D43), slot into the `targets` column this plan adds.
- **Jets.**
- **Transport.** It's [its own placeholder](../transport/README.md).
- **Rotor wash.**
- **Countermeasures and radar.**
- **Balance tuning.**
- **Backward compatibility or data migrations.** Every change is a hard cutover.

It replaces [battle-foundation slice 18](../battle-foundation/slices/18-air-movement.md), which had two flight modes.

## Contracts (what changes at the interfaces)

| Contract | Change | Slice |
|---|---|---|
| Unit catalog (`contract::catalog`) | `Mobility::Air { cruise_kmh, turn_deg_s, climb_mps }`, its speed cap, and `HullLimits.air`. Hull limits become exhaustive. | 01 |
| Rules (`fixtures/game.json`) | A battle-wide `air` block: cruise, clearance, ceiling, obstacle, separation. | 01, 03 |
| Weapon rows | `targets: [ground \| low_air]` (`high_air` comes with jets or AA, D43), authored on root rows and inherited through `extends`. | 07 |
| Weapon rows | New `heli_atgm`, `rocket_pod` and `helicopter_crash`. | 09, 04 |
| Unit digest | `Unit.air: Option<AirState { velocity, agl_target, aim_yaw }>`. | 03 |
| Battle digest and publication | Falling airframes are digested (04) and published as a new `crashes` group (10). | 04, 10 |
| Observation and publication | Contacts gain `z` and `layer`. **The digest layout changes; parity is re-recorded.** | 05 |
| Observation and publication | Identified (and own) units gain a coarse `smoking` bit. | 11 |
| Sim rule, every unit | `moved` compares XY only. Ground digests are unchanged. | 01 |
| Sim rule, every unit | Hull-mounted weapons fire only when the body faces the target. Ground digests are unchanged. | 08 |
| Weapon rows | Guided rows state `guidance: "stationary" \| "on_the_move"` (D38). Today's ATGMs author `"stationary"`, so their digests are unchanged. | 09 |

## One owner per concept

Every slice must keep these invariants. A slice that adds a second owner is wrong.

| Concept | The one owner |
|---|---|
| Altitude layer of a unit | Its type's mobility (`Mobility::layer()`) |
| What a weapon can target | The weapon row's `targets`, read only through `weapons::reaches` |
| "Is this hull on the ground?" | `Unit::ground_footprint()`, which is `None` when airborne. No site tests "is vehicle" for ground rules. |
| Air movement | `sim::movement::air::step_aircraft`. Air units never reach ground navigation or `drive`. The `Motion` type split enforces this. |
| Air routes | `sim::navigation::air::AirGrid`, one per side, built from that side's knowledge |
| Air heights | The `air` block in `fixtures/game.json`. The low hover is derived from the catalog (D27). |
| Crashing airframe | `sim::crash`. The renderer only draws the published `crashes`. |
| Crash damage | The `helicopter_crash` weapon row through `damage::blast`, not a new damage path |
| Low hover | `air::low_hover`, shared by resupply now and transport later |
| Contact height | `knowledge::Contact`. The renderer and picking read the published z. |
| Whether a contact floats | The published `aloft` (D33), decided by the sim from the contact's layer, z and the low hover. The sign, its panel's anchor and picking all read it. |

**Removed, not wrapped:**
- the flat `units::Mobility`, replaced by `Motion`;
- `hull_box()` as a ground test, replaced by `ground_footprint()`;
- the 2D contact centre;
- the global `PITCH_LIMITS`, replaced by per-rig limits in slice 15.

The finished code should read as if helicopters were designed in from the start.

## Slice graph

| # | Slice | Depends on | What a human can judge |
|---|---|---|---|
| 01 | An airborne hull | — | Tests: a helicopter is not a ground body |
| 02 | Contact sign spike | — | Variant sheet; **the user picks** |
| 03 | Flight | 01 | Tests: altitude, pop-up, tower detour |
| 04 | Falling airframe (simulation) | 01, 03 | Tests: arc, trees felled, wreck on the ground |
| 05 | Contacts carry height | 01 | Tests: air contact z and layer through WASM |
| 06 | Test airframe art | 01 | `/lab/air-hover`: spinning rotors at height |
| 07 | **First browser checkpoint** | 03, 06 | `/lab/air`: flight, rifle sparks, autocannon damage |
| 08 | Hull-mount facing | 03, 07 | Tests: strafing; no backwards fire |
| 09 | Helicopter weapon rows | 07, 08 | Tests: missiles and rockets on the move |
| 10 | Drawing the fall | 04, 06 | Fall contact sheet |
| 11 | Damage smoke | 04, 06 | Smoke crop |
| 12 | Drop line and ring | 03, 06 | Readability crop; **checkpoint** |
| 13 | Contact sign, built | 02, 05, 12 | Floating sign over fog |
| 14 | Resupply sink | 03 | Tests plus a sink crop |
| 15 | The Apache | 07, 09, 10 | Real art, mounts, chin gun pitch |
| 16 | Other 18 helicopters | 15 | Roster sheet |
| 17 | Skirmish: entry and AI | 14, 16 | A real skirmish |
| 17b | Rotor sound | 06 | Audition in the sound workbench |
| 18 | **Closing scene (D14)** | all | Full beat; full check, verify and balance report once |

**Order.** Slices 01–05 are headless and can kill the plan within the first week, before any art:
- 01: ground-only code;
- 03: the air grid;
- 04: the fall state;
- 05: the publication change.

Slice 06 proves the rotor pipeline on test art. Slice 15 proves it on one real model before the other 18.

## Verification standards

- **Narrowest check per slice.** The full `check`, `verify` and the balance report run once, in slice 18.
- **Visual slices** name one visual variable and a crop. Each ends with an unprimed [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md). Where there is a before shot or a reference, also run [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md).
- **Human checkpoints** (slices 02, 07, 12 and 18) are non-blocking: [preview-shots](../../.agents/skills/preview-shots/SKILL.md), about 5 minutes, then decide on the evidence and record the decision.
- **Rule changes keep ground digests unchanged.** Slices 01, 08 and 09 change rules for every unit, and each carries a test proving ground battles are unaffected. The one named digest change is slice 05's contact layout.
- **Scratch evidence** goes in `throwaway/evidence/<fixture-id>/`. Evidence the user judged goes in `assets/`.

## Research

- [Broken Arrow dev diary #2: Helicopters](https://forum.slitherine.com/viewtopic.php?p=936391). Helicopters are "cavalry": fast and hard-hitting but fragile. They fly nap-of-the-earth to hide behind terrain.
- [Broken Arrow: how to win (Matrix Games)](https://www.matrixgames.com/news/mission-accomplished-how-to-win-in-broken-arrow). Missiles favour high targets and guns favour low ones. SHORAD is what kills helicopters.
- [Broken Arrow on NamuWiki](https://en.namu.wiki/w/Broken%20Arrow(%EA%B2%8C%EC%9E%84)):
  - Low-flying helicopters keep moving and rise slightly over obstacles; this is D4.
  - Heavy air defence can't attack helicopters; this is D1's layer split.
- [Helicopters on the WARNO wiki](https://warno.fandom.com/wiki/Helicopters) and the Steam threads on [observations](https://steamcommunity.com/app/1611600/discussions/0/3192490350127985739/) and [controls](https://steamcommunity.com/app/1611600/discussions/0/3198117312276031789/). Nap-of-the-earth at low altitude; unmask to fire, then drop back. These are community sources from early access.

## Planning notes

- **The map behind this plan:** the explore-unknowns walk with the user on 2026-10-10. Its decisions, landmines and code facts are carried in full in [decisions](decisions.md) and [landmines](landmines.md).
- **How it was drafted:** three independent drafts, each with a bias (fewest slices, risk first, seam quality), were merged into this plan.
  - From the risk-first draft: the ordering.
  - From the seam-quality draft: the `Motion` type split and `ground_footprint`.
  - From the fewest-slices draft: the gap list.

  All three ran on Claude. The Codex draft failed because its model was at capacity, so the model-family mix the write-spec skill asks for didn't happen.
- [Transport](../transport/README.md) reuses `air::low_hover` (slice 14) for landing.
