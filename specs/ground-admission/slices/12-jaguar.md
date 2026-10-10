# 12 — EBRC Jaguar

**Unlocks:** Europe's armed wheeled recon vehicle is buyable, and it fires the autocannon
and top-attack Akeron.

Depends on 05 (recipe) and 11 (the `akeron_mp` row).

## Art first (from [slice 01 findings](01-model-triage.md#findings-2026-10-10))

Open with one art-fix commit before admission: sheet → fix → one unprimed critique.
Jaguar: make the hull a tall, flat-sided box on big wheels (not a low pointed VBCI nose); enlarge the turret and the `akeron_box` launcher pod; edge wear 0.6, then check whether the pale patches remain. Only these ranked fixes; dropped findings stay dropped.

### Art pass (2026-10-10)

Done in `jaguar.py`; evidence in [`../assets/12/`](../assets/12/).

- **Hull.** It is now a tall box with upright upper sides: the belt is at 1.45 m and the flat roof at 1.95 m, up from 1.25 m and 1.86 m. The nose is blunt, with a short glacis down from x 2.95 m. The tyres are bigger, radius 0.66 m, on the axle spacing the photos show.
- **Turret.** The turret is 0.68 m tall on the roof, 2.7 m long and as wide as the hull. The
  Akeron pod is 1.9 × 0.48 × 0.85 m and forms its left flank, standing 0.2 m above the turret roof. The RWS
  sits on a pedestal under `dressing_rws`, 0.4 m above the turret, inside the 0.3 m bulky-dressing allowance.
- **Frame.** The size stays 7.1 × 2.99 × 2.8 m, and the mounts moved to fit the art. `autocannon` pivots at
  z 1.95 m with its muzzle at (2.982, 0, 0.38). `launcher` pivots at (0, 1.28, 2.40) with its muzzle 1.25 m ahead.
- **Fit.** Measured the way `fit.hull_extents` measures, the export is 7.17 × 3.04 × 2.73 m and no face is
  more than 0.04 m off. Slice 01's 0.15 m rear gap is closed by tucking in the rear hooks and the jerrycans.
- **Edge wear.** Edge wear is now 0.6. That removed the pale washed patches on the turret and side plates,
  so the patches came from chip wear
  ([before](../assets/triage/jaguar-surface.png), [after](../assets/12/jaguar-surface-after.png)).
- **Critique.** The unprimed critique of the first fix ranked two problems: the turret was too low and flat,
  and the launcher read as a stowage bin. Both were fixed (taller, shorter turret, proud pod, raised RWS),
  and then we stopped. Dropped as below the bar: the gun is slightly short, the rear face is dark, and the
  grey smoke-discharger discs are visible only without textures.

### Admission (2026-10-10)

- **Type.** The Jaguar extends `roster_profile_wheeled_recon`, so it gets 850 m sight and recon concealment.
  It has role `recon`, hp 80, armour 50/30/20/15 (between the Fennek and the VBCI), medium weight and wreck,
  and 32/90 km/h wheeled with a 6 m turning radius. Cost stays 230. Akeron ammunition is the row's 4 rounds.
  All of these numbers are guesses for the closeout report.
- **Frame.** The hull box is 7.1 × 3.04 × 2.8 m, the width taken from the art. `autocannon` pivots at
  (0, 0, 1.95). `launcher` fires `akeron_mp` and is carried on the autocannon at (0, 1.32, 2.52), with its
  muzzle 1.25 m ahead.
- **Correction to the art-pass fit above.** The disabled exporter's `skip=("gun", …)` is a name-prefix match,
  so it also skipped `gunner_sight`. The true top was 2.92 m. The live `fit.hull_extents` caught it, and the
  gunner sight now sits lower and the commander's mast is shorter.
- **Wreck.** The fallen Akeron pod lies 1.23 m clear of the hull, so the wreck's footprint tolerance is 1.3 m.
- **Last critique.** The final art fix raised the pod 0.3 m above the turret roof on a dark arm, stood it off
  the turret, gave it rimmed tube mouths, thickened the 40 mm gun and moved the wheels outboard.
  The final unprimed critique ([live montage](../assets/12/jaguar-live.png)) found no hard defects. At the game
  camera it tells the Jaguar from the Fennek easily, but from the VBCI mostly by its 6 wheels to the VBCI's 8. It still asks for
  a taller turret. Accepted and stopped there, as in slice 08: a taller turret means growing the 2.8 m frame,
  and the real hull and turret stand about 2.9 m tall.
- **Sample** ([battle-sample.txt](../assets/12/battle-sample.txt)): from 700–840 m, unseen, the Jaguar kills a
  still T-90M with three roof dives (45°, apex 62 m) in 25 s, head-on or flank. At 400 m the tank sees it and
  wins two of three. Against the SEP v3, Trophy stops all four rounds.

## Seam

[Admission recipe](../README.md#admission-recipe):

- `extends roster_profile_wheeled_recon`; mounts `autocannon` and `launcher` firing
  `akeron_mp`, the launcher bound to the HMG rig (`jaguar.py` already rigs it that way).
- `jaguar.py` switches to `run`; the folder `jaguar/` already matches.
- Edge wear: apply slice 01's decision.
- The real launcher arm raises to fire. It stays stowed; animating it is out of scope.

## Verification

- Catalog smoke test and gate; `asset validate --type`, `bake`, `check`, `icons`.
- Shot in the `unit-roster` scene beside Fennek and VBCI.
  [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) against them,
  then an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
  as the last check.
- A quick sample: a Jaguar fires a diving Akeron at a tank from recon range.

## Delegated

Numbers from the Fennek and VBCI; Akeron ammo carried.

## Stays green

Catalog tests; existing digests.
