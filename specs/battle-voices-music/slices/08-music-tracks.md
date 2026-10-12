# 08 — Music tracks, Music tab and mood director

**Contract this unlocks:**
- Six reviewed tracks: 3 combat, 3 tension.
- A pure `MusicDirector` that decides the mood.
- A workbench tab to audition both.

## Generation

`generate music` uses slice 02's client and cache. It calls `POST /v1/music` with `music_v1`, about 150 s per track, at most 2 concurrent calls, retrying the occasional 500. It writes:
- raw masters to `assets/source/audio/music/`;
- 128 kbps served files to `assets/runtime/audio/music/<track>.<sha8>.mp3`;
- tracks with `Generated` provenance to the `sounds.json` `music` section.

**Prompts.** Every prompt starts with this constant, which the user approved:

```
STYLE = "Background underscore for a modern military real-time strategy game, in the style of hybrid
analog-synth and orchestral game scores. It must sit underneath gunfire, explosions and radio dialogue:
no lead melody, no hooks, no solos, no big hits or drops, consistent low-mid energy throughout, dark and
wide. Instrumental."
PULSE = "Steady pulsing analog synth bass ostinato in D minor, low sustained strings, distant brass
swells, same palette as a 125 BPM hybrid combat score."
```

| Mood | Direction | Prompt after `STYLE` (and `PULSE` for tension) |
|---|---|---|
| combat ×3 | "Pulse" (`assets/reference-music/u_combat_pulse.mp3`) | "Steady pulsing analog synth bass ostinato in D minor, subdued electronic drums and low taiko kept far back in the mix, soft sustained low strings, distant brass swells. 125 BPM." Vary it per track: a different section focus or intensity curve, still within the brief. |
| tension ×2 | "Pulse, sparse" (`u_tension_pulse_sparse.mp3`) | "Calm reconnaissance version of that score: the bass ostinato at half intensity, filtered and soft, no drums at all, only a quiet ticking pulse, long low string pads. 125 BPM." |
| tension ×1 | "Pulse, dark" (`u_tension_pulse_dark.mp3`) | "Brooding pre-battle version of that score: a slow low synth pulse and sub-bass heartbeat, faint distant taiko every few bars, dark pads and occasional low brass. 125 BPM." |

The default level is **−26 LUFS** relative to the battle mix (T12). Set each track's `gain` so they match.

## Seam

**`packages/battle-audio/src/music/musicDirector.ts` → `MusicDirector`:**
- `note(trigger, t)` takes own barks.
- `mood(t)` returns `tension | combat`.
- Any own B5–B9 within 20 s switches to combat; 45 s without one returns to tension.
- It shuffles a mood's tracks without repeating one back to back.
- It is pure and runs on an injected clock. The numbers live in `fixtures/game.json` → `presentation.audio.music`.

**Workbench Music tab** (`apps/sound-workbench/src/music/MusicTab.tsx`):
- tracks grouped by mood;
- a manual mood switch that drives `MusicDirector` and `MusicPlayer`, so the user hears the real 4 s crossfade;
- a regenerate button per track, which spawns `generate music --track`.

## What the human hears
All six tracks, and the crossfade between moods.

## Verification
- A director test on a fake clock: 20 s in, 45 s out, no back-to-back repeat.
- Tool test with a fake fetch for `music`.
- **Last check:** an unprimed [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md) of the tab.

## Delegated to you
- per-track prompt variations within the table;
- track length between 120 and 180 s.

## Gate
The user listens; this is taste. It is non-blocking for other lanes, but **a track plays in battle only after the user has heard it** or the 5-minute window has passed with the decision recorded.

## Feedback that would change this slice
A track is too foreground: regenerate it with a stronger restraint clause, or lower its gain.
