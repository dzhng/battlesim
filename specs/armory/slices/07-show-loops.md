# 07 · Show loops on `AppAudio`, and quiet tiles

Map row L9, plus P5. No dependencies.

## Contract

K7. Non-battle loops get their own voices in the one audio context. They sit
under the existing unlock, mute and volume, beside the menu bed, which keeps
playing. Sound names arrive already resolved (slice 03), so `AppAudio` never
resolves replacements. Elements marked quiet click but never clack on hover.

## Seam

In `packages/battle-audio/src/appAudio.ts`:

```ts
export interface ShowLoop { sound: string; gain: number; rate: number }
/** The non-battle loops now playing; [] fades them all out. */
showLoops(loops: readonly ShowLoop[]): void;
```

**Voices:**
- They come from a reserved id range on the menu `WebAudioSink`, on bus
  `units`. The bed (slot 1) and the cues (slot 3) are untouched (`appAudio.ts:171-172,299`).
- A sound that stays keeps its voice; its gain and rate are updated.
- `setScreen("loading" | "other")` and mute clear the voices. They are **not**
  cleared by the bed's restart. Today `graph.menu.clear()` (`:285,295`) would
  kill them, so their clear path is separate.

**What it leaves alone:**
- The battle paths (`createBattle`, `battleHeard`).
- No second `AudioContext` exists.

**The cue rule.** The existing menu cue listener (`:62-81`) skips the hover cue
for anything under `[data-cue-hover="off"]`. Clicks still sound.

## The human runs or sees

Nothing until slice 11.

## Verification

Write these first, in `web/tests/appAudio.test.ts`:
- `show loops play beside the menu bed and stop with the screen`;
- `replacing the show loops keeps shared voices and fades the rest`;
- `muted, show loops start nothing and resume on unmute`;
- `a bed restart leaves the show loops playing`;
- `a quiet tile clicks but never clacks on hover`.

## Delegated to the implementer

- The fade length.
- The size of the id range.

## Must stay green

- `appAudio.test.ts` and `battleAudio.test.ts`.
- The `_appJourney.mjs` probes: one context, running, and the music.

## Feedback that would change it

If the engine sounds too loud or too quiet under the bed: the gain is the
battle's own idle gain (`fixtures/audio.json`). Changing it for the Armory alone
needs a named decision from the user.
