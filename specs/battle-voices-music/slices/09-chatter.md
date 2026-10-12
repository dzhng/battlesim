# 09 — Chatter: arbitration, voice bank, preview

**Contract this unlocks:** one module decides which lines play. The rules are:
- at most 4 lines at once;
- priority, then distance;
- a line that can't start within 1 s is dropped;
- no speaker talks over itself;
- one unit answers for a group.

The workbench preview and the battle use that same module.

## Seam

**`packages/battle-audio/src/voices/chatter.ts` → `Chatter`.** It is pure and runs on an injected clock.
- **Input:** `VoiceRequest { tier: "response" | "own_bark" | "other_bark", speaker: string, set, trigger, intensity, position?, groupKey?, at }`.
- **Take choice:** the client's own seeded randomness, keyed by `(speaker, trigger, intensity, request count)`. It uses the `math` package's seeded RNG (load [`math`](../../../.agents/skills/math/SKILL.md)). Never use simulation RNG.
- **Voice within the set:** responses use the `speaks: responses` voice. Barks use a `speaks: barks` voice, picked by a stable hash of the speaker.
- **Output:** `VoiceSpec[]` sent to a `VoiceSink`, which plays them on the voices bus.
- **Rules:**
  - Priority runs response > own_bark > other_bark; within a tier, the request nearest the camera wins.
  - At most 4 lines play at once.
  - A request that can't start within 1 s of `at` is dropped.
  - A speaker never overlaps itself.
  - Requests with the same `groupKey` within 250 ms collapse to the first. That is the "one unit answers" rule; the caller orders by clicked unit, else highest value.
- **Numbers** live in `fixtures/game.json` → `presentation.audio.voices`.

**`packages/battle-audio/src/voices/voiceBank.ts` → `VoiceBank`:**
- `load(sets, signal)` fetches each set's `pack.json` and then its MP3s, **in the background**.
- `get(set, voice, trigger, intensity, take)` returns `AudioBuffer | null`. `null` means the line is dropped.
- It has its own load path keyed by URL; the existing bank only knows `sounds.json` clip ids (`soundBank.ts:58`).
- It never delays `BattleAudio.ready` (`battleAudio.ts:67-72`).

**Workbench preview:** `apps/sound-workbench/src/voices/chatterScript.ts` is a scripted 30 s firefight of mixed requests: an own tank under fire, a squad pinned, a seen enemy's contact, a group order. It plays through the real `Chatter` and `VoiceBank`, with a lane view showing which lines started and which were dropped.

## What the human hears
The 30 s preview, using the pilot set plus any reviewed sets.

## Verification
`web/tests/chatter.test.ts` against a fake sink and clock:
- the 4-line cap;
- the priority order;
- nearest first;
- the 1 s drop;
- no self-overlap;
- group collapse;
- deterministic take choice for a seed.

**Last check:** an unprimed [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md) of the preview lane view.

## Delegated to you
- the preview script's content;
- the lane view's look.

## Must stay green
The `battleAudio` and `soundBank` tests.

## Feedback that would change this slice
The user finds 4 lines too busy or too sparse: change the number in `game.json`.

## Checkpoint (non-blocking)
Play the preview for the user, wait about 5 minutes, then proceed.
