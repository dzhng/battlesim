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

## Lane B result (2026-11-13)

**Abrams SEPv2, SEPv2 Trophy, SEPv3 Trophy:** one script for all three,
`assets/source/skins/abrams/argon-variants-r1/build.py`, from one Argon
round on the pilot's SEPv2 script, **$1.52**. Round 2 wasn't needed (B7). `build("sep_v2")` was
proved unchanged against the pilot by a per-object geometry hash. Ten local edits (folder README): Trophy within the
roof allowance, a deterministic script, belt and stub clearances the
corrected lint found, a readable SEPv3 APU box and data link. The family
places the `trophy_aps` nodes. `abrams.py` lost its procedural body once the
M1E3 stopped importing it. Lint 0, validate clean, byte-identical twice.

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| SEPv2 (r1) | 7 | 6 | 6 | 5 | 7 | 6.2 |
| SEPv2 Trophy (r1) | 7 | 6 | 7 | 5 | 7 | 6.4 |
| SEPv3 Trophy r1 | 6 | 6 | 5 | 5 | 7 | 5.8 |
| SEPv3 Trophy r2 | 6 | 7 | 7 | 6 | 6 | 6.4 |

Below the bar, named: the paint mottling (the shared scheme recipe; R13's
matte paint lands on re-export), and the pilot's turret cheeks, which
read as a forward wedge rather than the Abrams' steep blunt faces.

**Abrams R12 sweep (2026-11-13):** the three variants' turret fronts measured
0.29–0.30 m back per metre outboard, under rule R12's floor of 0.5. The front
cheek stations sweep back about 0.75 (edit 11 in the folder README, the M1E3's
method), and the plan sweep now measures 0.64 (SEPv2) and 0.61 (both Trophy variants).
See `assets/after/abrams/<variant>-compare.png`. Lint 0, validate clean live
and wreck, byte-identical twice. Unprimed critique of the after shots:

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| SEPv2 (R12) | 7 | 6 | 7 | 5 | 7 | 6.4 |
| SEPv2 Trophy (R12) | 7 | 6 | 6 | 5 | 6 | 6.0 |
| SEPv3 Trophy (R12) | 7 | 6 | 7 | 5 | 6 | 6.2 |

The critic read "swept front cheeks" as correct. Its findings were the
shared look (olive-tinted shaded sides, which persist with textures off, so
the lighting; grey mottling on the tops; grainy shadow edges), the cheek
smoke launchers reading as flat marks at battle-near, and, at low
confidence, cheeks sharper in plan than the real turret (no top-view photo
settles it).

**M1E3:** `assets/source/skins/m1e3/argon-r2/`, two rounds, $3.85 + $4.78 =
**$8.63** (round 2 after the user's review and two replaced reference
photos). The turret runs x -2.25..+2.35, 3.5 m across, up to the 2.44 m roof.
The engine deck stands above the front deck and the nose is blunt. Both user
notes ("back needs to shift up, front too pointy"; "turret a lot longer")
read as fixed in `assets/after/m1e3/m1e3-compare.png`. Lint 0, validate clean.

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| M1E3 (r2 body) | 7 | 7 | 6 | 5 | 7 | 6.4 |

Below the bar, named: the paint (shared), dark strips on the front deck, and
the cheeks squarer than the photos'.

**M10 Booker:** `assets/source/skins/m10/argon-r2/`, two rounds, $2.31 + $0.91
= **$3.22**. The turret is set back, long and low, with a blunt front and
bustle bulk; the front is swept back from the mantlet (R12 edit). Lint 0,
validate clean, tiers inside `tracked_medium`.

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| M10 r1 | 6 | 6 | 6 | 5 | 6.5 | 5.9 |
| M10 r2 (R12 sweep) | 6 | 6 | 5 | 6 | 6 | 5.8 |

Below the bar, named. The critics want a taller turret, which contradicts the
frame's 2.5 m roof (from the photo survey) and the user's own note asking for
a lower turret, so it was kept (B13). Also named: the smoke dischargers are
flush, and there's a dark pit on the roof front. The user's note ("turret
shape looks wrong", "a small Abrams") is partly fixed: the critics now read
the turret as one blunt M10-like piece, but still say "generic".
