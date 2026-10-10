# Off-centre turrets

Every live vehicle's turret sits where the real vehicle's does. A gun riding on a turret
(`on` in its mount row) swings round that turret's own ring, not round the hull's middle.

## Why

A mount on a turret used to turn about the hull's middle. So a turret whose ring sits off the
middle (a front-engined vehicle's, behind it) would carry its roof gun on the wrong circle.
To dodge that, every roster turret had been put on the centre line, some by more than a metre
(LAV-AT, Stryker M1134, T-15). The fix is in the geometry, not the data:
`placed_muzzle` (`crates/sim/src/weapons.rs`) turns a mount's pivot round its carrier's ring
(`MountSpec::ring`, the carrier's pivot on the ground), and `muzzleOffset`
(`packages/scene-assets/src/mountMuzzle.ts`) mirrors it for effects and the asset validator.
A hull mount's ring is the hull's middle, so a centred turret behaves exactly as before.

## Invariants

- `pivot_m` is in the hull's frame at rest, for every mount. The roster record owns it, and the
  family script builds the turret and its ring at it. Hatch and deck clearances round a turret
  were placed by hand in some scripts (Bradley, LAV, T-15, ZBL-08, M10, ACV), so moving a pivot
  again means checking them.
- One level of carriage: a mount `on` a turret must sit on a turret that is itself on the hull
  (refused at load otherwise), because the ring turns with the hull.
- A mount `on` a turret keeps its place on the turret: moving a turret moves its carried mounts
  by the same delta.
- The simulation and its mirror agree. Tests:
  `a_roof_hmg_on_an_off_centre_turret_rides_the_turret_round_its_ring`
  (`crates/sim/tests/weapons.rs`) and `web/tests/mountMuzzle.test.ts`. The validator's
  `fit.muzzle_arc` holds every model to the same rule.

## Where they sit

Measured from each family's side and top reference photos (`assets/references/<family>/`),
with real-vehicle layout as a check (front-engined vehicles carry their turrets behind
centre). x is forward from the hull's middle; error is about ±0.15 m on clean profiles and
±0.25 m on tanks, where the bustle hides the ring.

| Group | Ring x (m) |
|---|---|
| US wheeled | LAV-25A2 −1.0, LAV-AT −1.3, Stryker M1134 −1.4 (y +0.73), Dragoon −1.05, Stryker ICV / RV +0.5, ACV-P −0.9 |
| US tracked | Bradley M2A4 / M3A3 −0.5 and y −0.2 (offset right), M10 −0.3, Abrams SEP v2 / v2 Trophy / v3 Trophy / M1E3 +0.25 |
| Europe wheeled | Centauro II −0.95, VBCI −0.4, Boxer APC +0.3, Fennek +0.55, VBL −0.2, Jaguar +0.2 |
| Europe tracked | Puma −0.45, CV9040C / Mk IV −0.35, CV90120 −0.3, Ajax −0.4, Leopard 2A6 / 2A7V / 2A8 +0.3, KF51 +0.3, Challenger 2 TES / 3 +0.2 |
| Eastern | T-15 −1.2, ZBL-08 −0.9, BTR-82A +0.85, T-14 −0.3, BMP-3 / BRM-3K / BMP-2M +0.3, Tigr −0.15, Type 99A −0.15 |
| On the middle | T-72B3, T-90M, T-80BVM, Type 15, Leclerc, HMMWV, Boxer RCT30 |

Before/after crops against the reference photos are in
[`../assets/off-centre-turrets/`](../assets/off-centre-turrets/).

## Decisions

- **Where the audit's figure and a closer look disagreed, the photo won.**
  - LAV-AT: −1.3, not −1.8. Four measurements ran from −0.95 to −1.45.
  - ACV-P: −0.9, not +0.5. The near-orthographic beach photo puts the station between
    axles 3 and 4.
  - M10: −0.3, not −0.6. The turret box sits 0.35 m behind its pivot.
  - Puma: −0.45, not −1.2. The square-on photo and the road-wheel count agree; the −1.2 came
    from a foreshortened front-quarter shot.
  - The ZBL-08 keeps −0.9 against a critique that read it 0.8 m too far back: both side photos
    put the turret at the third axle.
- **Bradley's turret is offset 0.2 m right.** The head-on photo and published descriptions
  agree.
- **Hull features yield to the turret.**
  - Hatches the turret now covers move clear or are left off: the Stryker's squad hatches under
    a weapon base, and the T-15 and ZBL-08 troop hatches.
  - Crewmen in hatches follow the pivot.
  - Some wrecks turn their turret off the bow (Leopard 2, KF51, Challenger 2) or droop the gun
    (Abrams), so a forward-moved gun stays inside the wreck's footprint allowance. The VBCI
    wreck loses its gunner's sight.
- **The 2A8 inherits its frame from the 2A7V,** overriding only its cannon's rounds, so the
  turret position has one owner.
- **Battles that field a moved vehicle have new digests,** because the muzzles moved. Test units
  and every recorded publication battle are unchanged.

## Left as found

- Several turrets are lower and simpler than their photos. The Centauro II's, the clearest
  case, has since been rebuilt to its photos' size, on a frame raised to the real vehicle's
  height ([before, after and reference](../assets/turret-shapes/centauro/)).
- The T-14's barrel is short, and its turret front is about 0.3 m long.
- The ICV station's base cuts about 14 cm into the commander's ring, as it did before the move.
