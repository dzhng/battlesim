# 05 Probe bodies: a second army, a scout, the kind cue

About $2 of generation. Settle the risks before paying for the batch.

## Contract

The generation-to-shipping path holds beyond the pilot's case (an uncovered
face, OCP, no pack). Three bodies prove it:

| Body | What it proves |
|---|---|
| `us_army_ocp a rifle`, regenerated | The rifleman's cue (assault pack and radio whip) survives a front image; the current composed prompt replaces the pilot's |
| `eastern_emr a rifle` | A high collar, a groin flap and a balaclava survive fitting and weighting; a digital green print survives the regrade; the skin split holds on a covered face |
| `usmc_marpat a recon` | A large ruck survives fitting, the tiers and prone or crawl clearance; MARPAT reads apart from OCP |

## Seam

No new code, unless a probe fails. Each fix lands in its owner, not as a
per-army case:
- fit, limbs, weights or surface: `generated_body.py`;
- prompt wording: `armies.json`.

## Steps

1. `skin-generate image` for each of the three bodies ($0.12). **Look at each
   image before paying for a mesh.** The kind cue must be visible from the
   front: pack shoulder straps and the whip above the shoulder, or the ruck's
   frame above the shoulders. Regenerate the image at most twice, then accept
   and record.
2. `skin-generate generate` with Tripo for each. Pin the best candidate. If
   Tripo fails the same body twice, use Meshy once, then quarantine and
   record.
3. Export each body on one kit: `rifle_squad a` (OCP), `rifle_squad
   eastern_emr a`, and `us_force_recon_recon_patrol a` (MARPAT). Run `asset
   validate` and `asset sheet`.

## What you can see

Three review sheets (contact views, 22 strips, red side).

## Verification

- `asset validate` has no findings (`fit.lies` included), and every export
  repeats byte for byte.
- **The cue:** the OCP rifleman's pack reads from behind and in the
  three-quarter battle view; the MARPAT ruck reads and doesn't collapse in
  tiers 1–3.
- **The EMR collar and balaclava:** neck, collar and jaw crops through
  `stand_aim`, `kneel_fire`, `prone_pinned` and `death`. No tearing, the
  collar doesn't pierce the head, and the face isn't tinted.
- **compare-screenshots:** each body against its army's procedural look
  (`sheet` of today's GLB) and its reference photos (`assets/references/<kit>/`).
  Is the identity right, and is it less wrong than before?
- **screenshot-critique** as the last check, on all three sheets with crops.
- **Non-blocking checkpoint:** open the shots with preview-shots, wait about
  5 minutes, decide, record and close.

## If a probe fails

The batch stops. Fix the owning seam and re-probe. Never multiply a failure
across 27 more bodies.

## Decisions delegated

Prompt wording within the look scheme, which candidate to pick, and the
smallest wearer fix a failure needs.

## Must stay green

Slice 03's gates and the pilot kits.

## Feedback that would change it

"The pack or whip doesn't matter" relaxes the kind cue check. "EMR looks
wrong" changes the army's prompt before slice 07.
