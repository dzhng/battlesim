# 04 — Pilot

**Contract this unlocks:** a measured cost and quality for one full set, then the batch proceeds. The user said "don't let me block you here": the pilot doesn't wait for anyone.

## Run

1. Run `generate voices --set us_tank`. This set's voice is already saved; it uses all 29 triggers × 2 intensities × 2 takes, about 116 text-to-speech calls plus 116 transcriptions.
2. The proxy can't read the credit balance (`user_read` is denied). Measure cost from characters sent and any cost header the proxy returns. Don't wait for a dashboard reading.
3. `generate report` prints:
   - calls;
   - characters sent;
   - retries and error codes;
   - wall time;
   - the mismatch rate and the flagged takes;
   - any cost header the proxy passes through.
4. `generate plan` projects the full batch: clips × characters × the measured cost per character.

## What the human hears and reads
- The pilot set in the Voices tab.
- The report and the projection.

## Verification
- Every take is in the lock, with a transcript and a match score.
- `check` passes.
- The served files play in the tab.

## Gate: none (user, 2026-10-11)
Report the numbers to the user, record them in `choices.md`, and go straight on to slices 05 and 06.

Stop only when one of these happens:
- the proxy returns a quota or 401 error;
- the pilot shows a quality problem the feedback below must fix first (a mismatch rate above about 10%, or audible drift).

## Feedback that would change this slice
- **Credits are too high:** cut takes to 1 per intensity for barks, or drop the stressed intensity for some responses. That is a data change in `voices.json`.
- **Mismatches are common:** tighten the scripts and avoid hard words.
- **v3 drifts between lines:** try voice settings (stability, style) in the generation key.

## Delegated to you
How the report is laid out.
