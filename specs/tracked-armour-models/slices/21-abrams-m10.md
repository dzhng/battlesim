# 21: Abrams and M10

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane B, after 20. Branches from the rollout plan commit.

**Models:** Abrams SEP v2 Trophy and SEP v3 Trophy (`abrams.py`), M1E3 (`m1e3.py`), M10 Booker (`m10.py`). The SEP v2 is done (the Argon pilot).

## The user's notes (the done list)

- M1E3 Abrams: "back needs to shift up, front too pointy".
- M10 Booker: "turret shape looks wrong".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- **The Abrams variants (P12):** one new session gets the SEP v2's `argon-r2/build.py` and adds `build(variant)` for the SEP v2 Trophy and SEP v3 Trophy: Trophy housings in place of the bustle's side stowage and rails; the SEP v3's UAPU box, CROWS-LP and data link. The SEP v2's own geometry must come back unchanged. Its rounds archive in `skins/abrams/argon-variants-r{1,2}/`, and the family then wears the one script for all three.
- M1E3: its own session; the user's note in the brief as numbers (rear deck line raised, nose blunter) against the frame.
- M10 (Q8c): its own session. The turret shape from the drawing, and a nose and turret length-to-height ratio that doesn't read as a small Abrams. Judge it in one sheet beside the SEP v2.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.

## Lane B log (2026-11-12)

**Sessions** (all created 12:51–12:53; Bento's dispatch was down workspace-wide,
`lastDeliveredInputSeq` 0, so no round 1 had started; cost so far $0):
- Abrams variants: `jn7ad1dwaez1skassvj1sz4ckn8j30mv`. The SEP v2's
  `argon-r2/build.py` extended with `build(variant)` for `sep_v2`,
  `sep_v2_trophy` and `sep_v3_trophy`, the SEP v2's geometry unchanged (the
  brief asks for a per-object vertex hash as proof; we diff it ourselves too).
- M1E3: `jn78bm1wsgt4afa91snv1pd7718j34q7`. Example script: the SEP v2's (B3).
- M10: `jn70jg8d7mdznefq10mm645v998j2dwf`.

**Frame targets:** the Abrams and M1E3 hull 7.93 × 3.66 × 2.44, ring (0.25, 0),
muzzle (5.805, 0, 1.903); the M1E3's RWS pivot (-0.20, +0.30, 2.42). The M10:
hull 7.00 × 3.40, box top 2.60 (sight dome to 2.89 as dressing), ring
(-0.30, 0), muzzle (5.45, 0, 2.10), roof gun pivot (-0.40, 0.62, 2.80).

**Before (measured):** M1E3 hull top 1.70 m only at the rear, and a wedge nose
(the note); the M10 turret runs x -2.34..+1.09 and z 1.60..2.68 (1.08 m tall
sides, 3.42 m long) against the photos' 0.7 m and 3.7 m.
