# 03b The aim hold: butt in the shoulder, eye at the top of the sight

Added during slice 05, from the user's review of the probe renders: "gun is
wayyyy too high for aiming", "why are their heads looking down", and then:
"make the eye look at the absolute top of the gun (top of scope). I rather have
it be too low than too high … the butt of the gun needs to be aligned with the
shoulder". It changes every soldier's aimed clips, the procedural ones too, so
it is its own slice.

## Contract

A shouldered weapon sits the way the reference photos show it
(`assets/references/rifle_squad/us_army_ocp-three_quarter_rear.jpg`):

- the butt in the right shoulder pocket;
- the head facing level along the bore at the target, never down at the weapon;
- the eye level with the very top of the weapon (a scope's top), or above it,
  so the weapon errs low;
- both hands on the weapon.

The validator now checks two of these on every soldier, in his skeleton's aim
reference pose:

- `fit.facing`: the head faces within `AIM_FACING_DEG` (8°) of level;
- `fit.cheek`: the eye is at least the skeleton's `sight_over_bore_m` over the
  muzzle. The rifle skeleton declares 0.05; the launcher skeleton declares
  none, because a launcher's sight sits beside its tube.

The simulation's numbers don't change. The stand-aim fit (height, eye and muzzle
against 1.70, 1.60 and 1.40 m) still holds.

## Seam

- **`shouldering.py`** owns the hold.
  - `shouldered` rests a butt in `POCKET` (`PRONE_POCKET` lying), and puts a
    launcher's eyepiece `EYE_RELIEF` ahead of the eye.
  - `lean_in` sets the upper body of every aimed pose: the support shoulder
    swings forward to the handguard, and the head rolls onto the stock.
  - `weld_cheek` turns the head to look along the bore. The neck then cranes,
    forward or back as far as `NECK_CRANE_DEG`, until the eye reaches the
    sight's height. Past that, the butt slides up or down the shoulder, at
    most `POCKET_SLIDE`; any remaining gap leaves the eye above the sight. A
    launcher only turns its head, and is then placed at the eye.
- **`weapons.py`:** a rifle hold's `sight` is the top of the M68 optic.
- **`infantry_rig.py`:**
  - `EYE_IN_HEAD` sits at the generated heads' pupil, 1.5 cm below spike 03's
    brow;
  - `within_reach` slides the support hand back along the weapon until the
    left arm reaches it, for every hold;
  - each authored clip releases its IK constraints once baked, since muted IK
    stacked on a bone misplaces the live chain.
- **Validator:** `soldierFit.ts` (`facingPitch`, the two checks), and
  `AimReference` in `schema.ts`.

## Evidence

- **Before:**
  - the aim poses bent the head 22° down;
  - `weld_cheek` could only raise the weapon, so it lifted the butt 9–10 cm
    off the shoulder to reach the eye;
  - the support hand fell 18–22 cm short of the handguard, so it hung in the
    air;
  - the stand-aim right hand missed its target by 5.4 cm, because muted IK was
    stacked on its bone.
- **After, on the three probes:**
  - the eye is level with or above the top of the gun: +0.2 to +1.0 cm on the
    M4s, +3 cm on the AK-12 (the hold is shared, and the AK has no optic);
  - the head faces 0° from level;
  - the butt stays within 4 cm of the pocket;
  - every hand is within 1 mm of its target. The exception is the walk and run
    low-ready support hand, 1.7 cm short (5.3 before) because the walk swings
    the shoulder.
- **Launcher (Javelin):** the tube rests on the shoulder; its underside sits at
  the shoulder's skin top. The eyepiece is at the eye.
- **Sweep:** `fit.facing` and `fit.cheek` are clean on all 126 roster infantry
  GLBs. Before this fix the sweep flagged 120 `fit.cheek` and 33 `fit.facing`.

## Verification

- `asset validate` across the roster infantry: only the pre-existing `fit.lies`
  and `fit.soldier_height` findings on procedural carried kits (decisions
  ledger).
- Unprimed critique of side, three-quarter, kneeling, prone and walking frames
  against the reference photos.

## Decisions delegated

The slide and crane limits (`POCKET_SLIDE`, `NECK_CRANE_DEG`), the shoulder
swing, and the rifle skeleton's 5 cm sight-over-bore floor (iron sights; optics
sit higher).
