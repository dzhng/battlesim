# Ground admission and top-attack missiles

Bring every disabled roster card that needs no new movement layer into play — its
behaviour and a model that reads well in battle beside its live peers — and add the
one mechanic three of them need: a guided missile that climbs, then dives onto the
roof.

## Next Agent Prompt

**Status (2026-10-10):** in progress. Wave 1 is being built in parallel: 01, 02+03, 05
and 10. **Next pickup:** whichever of those has no commit on main yet; then wave 2 (04,
06–09, 11) once 05 has corrected the recipe and 03 has landed.

You are implementing this spec. Read this README, then the slice you pick up, then the
readmes of the folders it touches. Follow the [admission recipe](#admission-recipe) for
every unit slice. Record any decision the slice left open in [choices.md](choices.md).
Push each green slice to main. Before ending your pass, update this section: status,
date, the next pickup point, ticks in the checklist, and any blocker.

Warnings:
- `fixtures/catalog.json`, `assets/catalog.json`, the runtime catalog, generated icons and
  `fixtures/units/model-manifest.json` are shared generated or registry files. Admission
  commits are serialised: never hand-merge them; rebase, then regenerate.
- Models are Git LFS objects. Fetch only the families your slice touches (`asset pull`).
- Run the full check, browser verify and the balance report once, in slice 13 — not per
  slice.

Checklist:
- [ ] 01 model triage — ranked fix list and three art decisions ([slice](slices/01-model-triage.md))
- [ ] 02 top-attack flight — row property, admission, dive steering ([slice](slices/02-top-attack-flight.md))
- [ ] 03 top attack in battle — sight, release, APS, roof payoff, overhead launch ([slice](slices/03-top-attack-in-battle.md))
- [x] 04 top-attack view — side-on capture of the arc ([slice](slices/04-top-attack-view.md))
- [ ] 05 pilot: Challenger 3 + the two new profiles; writes the recipe's evidence ([slice](slices/05-pilot-challenger-3.md))
- [ ] 06 T-14 and T-15 Armata ([slice](slices/06-armata.md))
- [x] 07 BRM-3K and Type 15 ([slice](slices/07-brm-type15.md))
- [x] 08 M10 Booker, CV90120, Centauro II ([slice](slices/08-m10-cv90120-centauro.md))
- [x] 09 M1E3 with Trophy hardware ([slice](slices/09-m1e3.md))
- [x] 10 Javelin and Akeron infantry kits ([slice](slices/10-infantry-kits.md))
- [x] 11 Javelin and Akeron MP teams ([slice](slices/11-at-teams.md))
- [ ] 12 EBRC Jaguar ([slice](slices/12-jaguar.md))
- [ ] 13 closeout ([slice](slices/13-closeout.md))

## Slice graph

```
01 model triage ──────────────┬─► 05 pilot Challenger 3 + profiles ─┬─► 06 Armata
                              │                                     ├─► 07 BRM-3K, Type 15
                              │                                     ├─► 08 M10, CV90120, Centauro II
                              │                                     └─► 09 M1E3 (Trophy art)
02 top-attack flight ─► 03 in battle ─► 04 view ─┐
10 infantry kits ────────────────────────────────┴─► 11 Javelin, Akeron teams ─► 12 Jaguar (+05)
                                                                     all ─► 13 closeout
```

01, 02 and 10 start in parallel. 06–09 run in parallel once 05 has fixed the recipe, but
their admission commits land one at a time (shared generated files). The missile chain
02 → 03 → 04 is sequential.

## Scope

In: the twelve cards whose `planned.reason` is "Ground profile and model admission
pending" —

| Card | Profile | Slice |
|---|---|---|
| T-14 Armata | `advanced_mbt` | 06 |
| T-15 Armata heavy IFV | `heavy_ifv` | 06 |
| BRM-3K | `tracked_recon` | 07 |
| Type 15 (ZTQ-15) | new `light_tank` | 07 |
| M10 Booker | new `light_tank` | 08 |
| CV90120 | new `light_tank` | 08 |
| Centauro II | new `wheeled_tank_destroyer` | 08 |
| Challenger 3 | `advanced_mbt` + `trophy_aps` | 05 |
| M1E3 Abrams | `advanced_mbt` + `trophy_aps` | 09 |
| Javelin team | `at_team` + top attack | 11 |
| Akeron MP team | `at_team` + top attack | 11 |
| EBRC Jaguar | `wheeled_recon` + Akeron top attack | 12 |

Out (firewalls): air, helicopters, drones, artillery and air defence stay disabled;
no fire-and-forget, seeker lock or in-flight retargeting; no change to what APS can
intercept; no passenger carriage (carrier descriptions keep saying it is not
implemented); no top attack for TOW, Kornet or Spike; no model iteration past the
"good enough" bar (match live peers, one critique pass, fix what it ranks, stop); no
new routes or harnesses where an existing lab or scene can show the thing.

## Decisions

Made by the user (2026-10-10):
- **Guidance is unchanged.** A top-attack missile is still sight-supported: the launcher
  renews its aim point from its side's sighting; moving, Stop or lost sight releases it
  to a fixed point a coast ahead. Only the path changes. Fire-and-forget was offered and
  declined (a second mechanic, and much stronger AT teams).
- **APS still intercepts a diving missile.** Top attack wins by striking thin roof armour,
  not by beating Trophy. An elevation-arc APS was offered and declined (a third mechanic
  that rebalances every Trophy unit).
- **No compatibility, no migration.** Digests change where rules change; that is a named
  decision recorded in the slice that changes them.
- **Scope** is all twelve no-new-mechanic cards plus top attack; the light tanks, Trophy
  variants and top-attack carriers are all in.

Made in planning, from the evidence (reversible; the alternative is recorded):
- **Top attack is a weapon-row property, steered statelessly.** See
  [the contract](#contract-top-attack). Three of four drafts converged on this; the risk-first
  draft wanted explicit phase state (climb → cruise → dive) on the projectile. Rejected:
  a phase machine adds digest state and transitions for the same picture; the stateless aim
  point already yields climb, pitch-over and dive and degrades to a flatter shot at short
  range. Revisit only if slice 02's traces show the stateless path cannot reach a steep dive.
- **The roof is struck by geometry, not by rule.** Hull face selection
  (`face_toward` / `hull_face_at`, `crates/sim/src/units.rs`) already picks Roof for an
  impact from above. Missiles detonate on contact (`meet_hull`, `crates/sim/src/damage.rs`)
  and are judged by `pierces` against the struck face, so no roof special case exists or
  is added.
- **Top attack must pay off.** Today `atgm` pierces 300 and the thickest roster front is
  260, so a dive would change nothing. The Javelin and Akeron rows therefore sit below
  improved/advanced MBT fronts and far above every roof (slice 11 owns the numbers; starting
  guess 200). The war-film read: a Javelin kills a T-90 by diving onto it, not head-on.
- **Real profiles are starting guesses, not targets.** Javelin climbs to about 150 m in
  top-attack mode (about 60 m in direct mode), minimum range about 150 m top-attack /
  65 m direct ([Wikipedia](https://en.wikipedia.org/wiki/FGM-148_Javelin),
  [GlobalSecurity](https://www.globalsecurity.org/military/systems/munitions/javelin-design.htm),
  [CSIS Missile Threat](https://missilethreat.csis.org/missile/fgm-148-javelin/)). Akeron MP
  advertises a selectable high top-attack path to 4–5 km with no published apex
  ([Wikipedia](https://en.wikipedia.org/wiki/Akeron_MP),
  [MBDA datasheet](https://mbdainc.com/wp-content/uploads/2022/07/Datasheet_2022-Akeron-MP.pdf)).
  Game ranges are compressed (`atgm` reaches 900 m), so the loft is tuned to read on screen at
  play camera — likely 40–80 m — not copied.
- **Order:** Challenger 3 is the pilot because it shares the Challenger 2 TES frame and
  already draws Trophy nodes; it proves the recipe with the fewest unknowns. Top attack runs
  in parallel because nothing in the vehicle slices depends on it.

## Contract: top attack

```
WeaponBallistics.top_attack: Option<TopAttack>   // crates/contract/src/ballistics.rs
TopAttack { loft_m: f64, dive_deg: f64 }          // deny_unknown_fields
```

- **Admission:** only `FlightConfig::profile` (`crates/sim/src/flight/mod.rs`). It refuses
  `top_attack` on a row without guidance (`turn_deg_s`), a non-finite or non-positive
  `loft_m`, and `dive_deg` outside (0, 90). It resolves to `LaunchProfile.loft:
  Option<Loft { height_m, slope }>` with `slope = tan(dive)` computed once.
- **Steering:** `Guidance` gains `loft: Option<Loft>`. One pure method,
  `Guidance::aim(position) -> V3`, owns the geometry: while supported and lofted, aim at the
  loft height one climb's run (`height_m / slope`) ahead, never past the point — so the missile
  shoots up at the dive angle, holds the loft, and dives once the commanded point lies at least
  `dive_deg` below its horizon (`position.z − point.z ≥ slope · horizontal distance`). A
  released missile ignores the loft. `Flight::acceleration` calls `aim`, so flight and the APS
  look-ahead see the same path. (Slice 04 replaced the first form, aiming at the loft above the
  far target, which climbed on a ~7° ramp that read as a flat shot.)
- **Unchanged:** `Battle::guide` still renews `point`; `guidance_clear`, `release`,
  `release_coast_s`, APS, damage and face selection are untouched. The publication layout
  does not change: `GuidedMissile.point` stays the commanded point (its doc says
  "commanded", not "steering to"); the climb is visible only in the flown path the renderer
  already draws.
- **Digest:** the loft is hashed only when present, so every battle without a top-attack
  row keeps its digest. Battles that fire one get new digests — the named decision.

## Admission recipe

Every unit slice moves a card from planned to live the same way. Slice 05 runs it first
and corrects this section with what it learnt.

1. **Frame.** Copy the family script's `DIMENSIONS`/`MOUNTS` exactly into the roster
   record, then tune. Once admitted, the roster record owns the frame; the exporter reads it
   through `catalog_frames.family_variants` (`packages/scene-assets/blender/vehicle_export.py`
   `run`) and its own `DIMENSIONS`/`MOUNTS` are deleted.
2. **Type.** In `fixtures/units/roster/<faction>.json`: drop `planned`; add `extends` (a
   profile in `fixtures/units/ground/profiles.json` or a live sibling), roles, hull (size,
   eye, armour, weight/push class, wreck), mobility, mounts (pivots and muzzles), parts.
3. **Source.** The source folder must equal the family name `run(family)` reads: move
   sources out of `assets/source/roster/disabled/`, and split or rename scripts so one script
   maps to one folder. Switch the script from `run_disabled` to `run`; delete its local frame.
4. **Appearance.** Add the appearance and wreck entries to `assets/catalog.json`
   (mount→node map), following the nearest live peer.
5. **Registry.** Delete the card's entry from `fixtures/units/model-manifest.json` (the
   catalog test refuses a manifest entry that is not a disabled card).
6. **Regenerate.** Re-export live then wreck (`asset blender`), `asset validate <glb>
   --type <id>`, `bake`, `check`, `icons`; regenerate `fixtures/catalog.json` through the
   catalog gate in `crates/sim/tests/catalog.rs`.
7. **Prove.** The catalog smoke test (instantiate, move, fire every mount); one sheet and
   one `unit-roster` scene shot beside its live peer; [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md)
   against the peer and its references; an unprimed
   [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md) as the last check.

## Single owners

| Concept | Owner |
|---|---|
| A unit's physical frame (hull, eye, mounts) | its roster record; exporters read it |
| Profile defaults (sensors, sound) | `fixtures/units/ground/profiles.json` |
| Appearance, wreck art binding | `assets/catalog.json` |
| Disabled-card model registry | `model-manifest.json` — admitted cards leave it |
| Top-attack authoring | `WeaponBallistics.top_attack` only |
| Top-attack admission | `FlightConfig::profile` only |
| Dive geometry | `Guidance::aim` only |
| Roof vs front | `face_toward` + `pierces`; no `if top_attack` in battle, damage, protection, publication or renderer |
| Shared exporter helpers (`vehicle_parts.py`, `vehicle_export.py`, `catalog_frames.py`) | the coordinator; a helper edit re-exports every family using it |

The end state reads as designed today: no `run_disabled` caller left for an admitted
card, no `disabled/` source for one, no exporter-local frame for a live card, no stale
"Trophy blocked" or "top attack deferred" row in
[the admission audit](../done/unit-roster/capability-admission.md).

## Verification gates

- Narrowest check per slice; the slice names it. Full `check`, browser `verify` and the
  balance report run once, in slice 13.
- Every visual shot: [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md)
  is the last check before acceptance (unprimed). Where a peer or reference exists,
  [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md) judges candidate
  against target. Human review is non-blocking: open shots with
  [preview-shots](../../.agents/skills/preview-shots/SKILL.md), wait about five minutes,
  then decide on the evidence, record it in [choices.md](choices.md), close the shots and go on.
- Behaviour: invoke [write-tests](../../.agents/skills/write-tests/SKILL.md) before changing
  rules; invoke [tweak-mechanics](../../.agents/skills/tweak-mechanics/SKILL.md) before slice 02.
- Visible UI (the guidance line during a climb): invoke
  [game-ui](../../.agents/skills/game-ui/SKILL.md).

## Risks and unknowns

| Risk | Where it is settled |
|---|---|
| Fit tolerance: disabled exports are checked to 6% (0.52 m on the T-14); live types to 0.1 m (`fit.hull_extents`). Some models may need art bends. | 01 measures; each unit slice fixes |
| Turn radius: 200 m/s at 60°/s is about 190 m, so a full pitch-over may not fit at short range or low loft. | 02 traces; tune loft/turn on the row |
| A shallow dive may strike the upper side, not the roof, because the aim point sits inside the hull at aim height. | 02 roof test |
| A climbing missile leaves the launch line that `first_obstruction` checks, so it may hit a ceiling, facade or canopy above a launcher. | 03 overhead-launch test |
| Lifetime admission is checked along a straight line; a lofted path is longer. | 02 max-range arrival test |
| APS look-ahead uses start-of-tick acceleration; a tight dive bends inside that window. | 03 APS test |
| An enemy sees a round only over ground it sees; a missile high in its climb may flicker. | 04 capture (inference, unverified) |
| The mechanics editor may need the new field in its validator. | 02 (inference, unverified) |
| M1E3 draws `aps_panel_*`, not the `trophy_radar_*`/`trophy_launcher_*` nodes `trophy_aps` requires. | 09 |
| Javelin has only `active_a`/`carried_a`; Akeron only `active_a`; the Javelin is fielded by Europe but has only a US look. | 10 |
| Thirteen new runtime models raise the game page's download (unmeasured). | 13 measures against `asset check`'s catalog budget |

## Files

- [choices.md](choices.md) — decisions made where a slice was silent.
- [slices/](slices/) — one file per slice.
- `throwaway/ground-admission/` (ignored) — sheets, critiques and traces while working.
  Evidence that decides something is copied into `assets/` here.
