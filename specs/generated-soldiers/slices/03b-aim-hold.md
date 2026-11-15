# 03b The aim hold: stock in the shoulder, head on the stock

Added during slice 05, from the user's review of the probe renders: "gun is
wayyyy too high for aiming, look at some reference images". It changes every
soldier's aimed clips, the procedural ones too, so it is its own slice.

## Contract

A shouldered rifle sits the way the reference photos show it
(`assets/references/rifle_squad/us_army_ocp-three_quarter_rear.jpg`): butt in
the right shoulder pocket, rifle level, head brought onto the stock, the eye
behind the optic and looking forward. The validator's stand-aim fit still
holds: height, eye and muzzle against the simulation's 1.70, 1.60 and 1.40 m.
The simulation's numbers are not changed.

## Seam

`shouldering.py` (new) owns the hold; `clips_infantry.py` calls it and
`weapons.py` carries each weapon's butt.

- **One shouldering rule (`shouldered`).** A weapon with a butt
  (`hold["butt"]`, renamed from `prone_butt` because standing uses it now)
  rests it in the shoulder pocket, `POCKET` up and `PRONE_POCKET` lying. A
  launcher, which has no butt, keeps its sight 8 cm ahead of the eye.
- **`weld_cheek`.** The neck cranes forward by up to `NECK_CRANE_DEG` (24°)
  and the head tips back by as much, so the face keeps its direction and only
  the eye comes down toward the sight. Whatever gap the crane can't close is
  taken up by the stock rising in the shoulder: the modern heads-up hold.
  - The same crane is keyed on every frame of the clip, so a walk's bob or a
    prone flinch keeps its motion.
  - Callers: kneel-fire, stand-aim, prone and its crawls, and the aimed and
    crouched walks.
- **The face-up corpse's raised knee lolls outward** (`corpse`, rolled about
  the body's long axis), as a dead man's does. Every face-up corpse lies well
  under `fit.lies`.

## Evidence

- 30° of crane put the eye at 1.466 m and the helmet at 1.63 m, which failed
  `fit.eye` and `fit.soldier_height`. At 12° the unprimed critic still saw
  the stock at the jaw. 24° passes once stand-aim no longer drops the pelvis
  and the trigger elbow hangs lower, on every soldier tried: the three
  probes, procedural `rifle_squad` look `b`, the machine gunner, and the
  Javelin, TOW and Stinger kits.
- Before the crane counter-rotated the head, the weld nodded the whole head:
  the user asked "why are their heads looking down".
- Face-up corpses: 0.54–0.56 m, against 0.586–0.602 before.

## Verification

- `asset validate` has no findings on those kits.
- Side and three-quarter renders of `stand_aim` and `kneel_fire` are compared
  against the reference photo (compare-screenshots); slice 05's critique
  covers them.

## Decisions delegated

None left: the pocket offsets are the prone clip's existing method, and the
crane limit is set by the validator's eye bound.
