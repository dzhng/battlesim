# 05 — Voice design

**Contract this unlocks:** every voice in `voices.json` has a permanent, user-picked `voice_id`.

## Seam

**`generate design <set>/<voice>`** calls `POST /v1/text-to-voice/design` with:
- the voice's `description`;
- a fixed audition text per role: at least 100 characters, steady → stressed → desperate, as in the walk. Use the same text for every voice of a role, so candidates are comparable.

It returns 3 previews.

**Previews:**
- They are kept as raw masters under `assets/source/audio/voices/design/<set>/<voice>/`, because preview ids are temporary.
- They are recorded as candidates in the lock.

**In the Voices tab**, a voice with `voice_id: null` shows its 3 candidates, **played through the set's processing profile**.
- **Pick** runs `generate pick`. That calls `POST /v1/text-to-voice` with the generated id, and the returned `voice_id` is written into `voices.json` through the workbench save path.
- **None of these** bumps a design retake, or takes an edited description and redesigns.

## What the human hears
3 candidates for each of about 51 voices. The four T1 voices are already picked.

## Verification
- Tool tests with a fake fetch: design → pick → id saved.
- After the run, every voice in `voices.json` has a non-null `voice_id`, which the validator checks once the slice closes.

## Gate
Non-blocking (the user: "don't let me block you here").
1. Open the candidates for the user and wait about 5 minutes.
2. If there's no reply, pick per voice on the evidence: the closest to the T1 reference in register and accent, with a clean speech-to-text transcript of the audition. Record each pick in `choices.md`.
3. The user can re-pick any voice later in the Voices tab. That saves the new voice and regenerates only that set.
4. A set's batch (slice 06) starts as soon as its voices are picked.

## Delegated to you
- the audition texts per role, in the walk's register;
- design retake mechanics.

## Feedback that would change this slice
The user rejects all candidates for a voice repeatedly: rewrite its description, for example age, timbre or delivery.
