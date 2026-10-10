# Off-centre turrets

Every live vehicle's turret sits where the real vehicle's does. The simulation now carries a
turret's roof gun round the turret's own ring (`placed_muzzle`, `crates/sim/src/weapons.rs`;
mirrored by `muzzleOffset`, `packages/scene-assets/src/mountMuzzle.ts`), so nothing forces a
turret onto the hull's centre line any more. Before that change every turret had been put there.

## Next Agent Prompt

**Status (2026-10-10):** the mechanism has shipped; the models have not moved yet. Five groups
are being moved in parallel (below). Each group: move the turret in the roster frame and the art,
re-export, validate, bake, compare against the references, then run one unprimed critique.

## Contract

- Only `pivot_m` changes, on a type's turret mount (`turret: true`, no `on`). Any mount `on` it
  moves by the same delta, so it keeps its place on the turret. The catalog format is unchanged.
- The roster record owns the frame (`fixtures/units/roster/*.json`, or the profile or sibling it
  `extends`). The family script builds the turret, its ring and everything on it at the frame's
  pivot. Hull features keyed to the turret also move (hatches round the ring, deck plates, the
  commander's head-out position). The hull itself, the engine deck and running gear do not.
- Battles that field a moved vehicle get new digests, because muzzles move. This is a named
  decision. Test units and test digests do not change.
- Gates per family: `asset validate --type` for every moved type (including `fit.vehicle_muzzle`
  and `fit.muzzle_arc`); the catalog bless and tests; `bake`, `icons`, `check`; a side and top
  sheet before and after against the family's side reference photo, with compare-screenshots on
  the turret position; one unprimed screenshot-critique as the last check.

## Targets

Measured from the side and top reference photos, then converted with the catalog hull length
(audit: `throwaway/turret-audit/`). x is forward from the hull centre, and every current x is 0.
The figures are ±0.1–0.15 m on clean profiles and ±0.25 m on tanks whose bustle hides the ring.
Where a group's own look at the references, with real-vehicle knowledge, clearly disagrees,
use that and record why.

| Group | Units → real ring x (m) |
|---|---|
| US wheeled | LAV-AT −1.8, LAV-25A2 −1.0, Stryker M1134 −1.4 (keeps y +0.73), Stryker Dragoon −1.05, Stryker ICV / RV +0.5, ACV-P +0.5 (low confidence: check) |
| US tracked | Bradley M2A4 / M3A3 −0.5 (check whether the turret is offset sideways), M10 Booker −0.6, Abrams SEP v2 / v2 Trophy / v3 Trophy and M1E3 +0.25 |
| Europe wheeled | Centauro II −0.95, VBCI −0.4, Boxer APC +0.3, Fennek +0.55, VBL −0.2, Jaguar +0.2 (Boxer RCT30 stays at 0) |
| Europe tracked | Puma −1.2, CV9040C / Mk IV −0.35, CV90120 −0.3, Ajax −0.4 (low: check), Leopard 2A6 / 2A7V / 2A8 +0.3, KF51 +0.3, Challenger 2 TES / Challenger 3 +0.2 |
| Eastern | T-15 −1.2, ZBL-08 −0.9, BTR-82A +0.85, T-14 −0.3, BMP-3 / BRM-3K / BMP-2M +0.3, Tigr −0.15, Type 99A −0.15 |

These stay where they are: T-72B3 and T-90M (−0.1), T-80BVM, Type 15, Leclerc, HMMWV and
Boxer RCT30.
