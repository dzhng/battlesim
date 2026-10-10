# 11 — Javelin and Akeron MP teams

**Unlocks:** both AT team cards are buyable and fire top-attack missiles. The first roster
rows use slice 02's property.

Depends on 02, 03 (and 04's starting loft) and 10.

## Seam

- `fixtures/game.json`: `javelin` and `akeron_mp` rows extending `atgm`, with `top_attack`,
  `interceptable`, finite ammo and supply round prices. Descriptions say "top attack; the
  gunner must keep sight" — never "fire-and-forget".
- **Penetration:** below improved/advanced MBT fronts (190–260 today) and far above every
  roof and IFV side, so the payoff is real (README decision). Starting guess 200; the row's
  number is a guess the balance report tunes. TOW, Kornet and Spike are untouched.
- Range and loft: start from slice 04's accepted values. Akeron reaches further than
  Javelin (it does in service), within the game's compressed ranges.
- Soldier kinds: Javelin and Akeron gunners in `fixtures/units/ground/profiles.json`
  (extending the AT rifleman), with operator appearances in `infantry-appearances.json`,
  following the RPG and TOW gunner pattern. Pose: shoulder-fired (`stand_aim`, like the RPG
  team), not TOW's `kneel_fire`.
- Roster records in `shared.json` (Javelin, US and Europe) and `europe.json` (Akeron):
  drop `planned`; squad slots; remove the manifest entries.

## Verification

- Catalog smoke test and gate.
- `crates/sim/tests/weapons.rs`-style handoff check: when the gunner falls, the launcher
  passes to a survivor and a missile already in flight keeps its guidance.
- A quick battle sample: a Javelin team ambushes a T-90M from the flank at mid range.
  Expect roof kills, and expect the same team to fail head-on against Trophy at a higher rate
  than against a tank without APS.
- Shots: the team firing (launch and apex) in the `unit-roster` scene or the slice 04 lab.
  [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) against the TOW
  team firing, then an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
  as the last check.

## Delegated

Ammo counts, costs, ranges and loft within the decisions above; slot layout (follow the
TOW team).

## Stays green

Guidance and protection tests; every digest of a battle without these rows.

## Would change this slice

A battle sample in which the Javelin never dives (all shots short-range or overhead). Then
the loft or the minimum range is wrong; tune the row, not the rule.
