# 07 Every body generated and pinned

About $16–22 of generation.

## Contract

Every `(army, look, kind)` the roster needs has an accepted pin: 30 bodies.
The 5 from slices 00, 05 and 06 stay unless review rejects them. Exports
wait for slice 09, so they happen once, on the final code.

## The matrix

| Army | Kinds |
|---|---|
| `us_army_ocp` | rifle, recon |
| `usmc_marpat` | rifle, recon |
| `german_flecktarn` | rifle, recon |
| `eastern_emr` | rifle, recon |
| PLA | rifle |
| `french_cce` | rifle |

Each is in looks `a`, `b` and `c`. The data change that moves the LAV's crew to
`usmc_marpat` lands here, so slice 09 exports it.

## Steps

1. **Images first, for all remaining bodies** ($0.04 each).
   `skin-generate image` per key, then one contact sheet of every input image,
   laid out army by army.
   - Check each: the T-pose, feet narrow, no weapon, the look's clause, the
     kind's cue, no white in the camouflage, and skin tones varied across a
     squad.
   - Regenerate rejects, at most twice per body.
2. **Meshes.** `skin-generate generate` with Tripo, in the background, at a
   concurrency of 6, with a progress line per job and a 40-minute deadline per
   batch.
   - A body that fails Tripo twice gets Meshy once, then goes to quarantine
     and is recorded.
   - Never resubmit an unresolved request: `status` shows it, and the tool
     resumes it.
3. **Pick and pin.** Look at each candidate's preview image and run one cheap
   probe export (`rifle_squad` for the rifle kind, a scout kit for recon)
   with `asset validate`, before running `pin`. A failure to fit, weight or lie
   flat rejects the candidate: regenerate, never patch the body.
4. `skin-generate status infantry` is clean: no missing, stale or unresolved
   entries.

## What you can see

The input-image contact sheet, and each army's six probe sheets.

## Verification

- `status` is clean, and every probe export validates with no findings.
- **compare-screenshots:** per army, the probe sheets against the army's
  references and the procedural look they replace.
- **screenshot-critique** as the last check, per army: a lineup of that army's
  six bodies (contact views side by side) for cohesion, plus crops.
- **Non-blocking checkpoint**, per army or once with all six: open the shots
  with preview-shots, wait about 5 minutes, decide, record and close.

## Decisions delegated

Which candidate to pin, image regenerations within the limit, and the batch
schedule.

## Must stay green

Slice 03's gates, the pilot kits, and the ledger: every call logged, none paid
twice.

## Feedback that would change it

"This look reads wrong" regenerates that body ($0.64). "Squads look too
alike" adjusts the look clauses and skin tones, then regenerates.
