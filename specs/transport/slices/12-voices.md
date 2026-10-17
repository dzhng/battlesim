# 12 · Voices

**Owns:** D24. **Depends on:** 11.

## Contract
New spoken lines for "board", "unload", and the "no room" refusal:
- order acknowledgements keyed in `ORDER_TRIGGERS` (`packages/battle-audio/src/voices/responses.ts:61-70`), with `JOURNEYS`/`HALTS` at `:145-147` as fits;
- the refusal in `REFUSALS` (`:79-90`).

They are generated through the Duet CLI's ElevenLabs proxy, following `packages/battle-audio/README.md` (the generator's base URL is `BASE` in `tools/generate/elevenlabs.ts`). The new takes go in `fixtures/voices.json`.

## What a human can see
The sound workbench plays the new lines for every faction voice.

## Verification
- The battle-audio package's own checks.
- One listen through in the sound workbench, judged against the existing enter-building and leave-building lines for tone and length.

## Blocker rule
If the Duet login or proxy is unavailable, record it as unfinished in [choices.md](../choices.md) and the README's Next Agent Prompt, and continue. Don't reuse the building lines as a substitute (the user chose new lines).

## Delegated
Line wording, within the existing voice style.

## Must stay green
Existing voice lines and their triggers.
