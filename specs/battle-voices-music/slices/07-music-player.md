# 07 — Music player, buses and levels

**Contract this unlocks:**
- All music, the menu included, streams through one `MusicPlayer` on a `music` bus.
- A `voices` bus exists.
- Players set master, music and voices levels in both menus.
- The decode-whole-file path for music is gone.

## First acceptance check (risk-first)

Before building the rest, prove the media path. Put the existing menu bed, re-encoded as a 128 kbps MP3, under `assets/runtime/audio/music/`. Then confirm that it:
- streams through `HTMLMediaElement` → `MediaElementAudioSourceNode` → the music bus;
- loops;
- crossfades over 4 s to a second element;
- does all of this under the site's COOP/COEP headers (`vercel.json`) in Chrome, Safari and Firefox (production build, `bun run --cwd web preview`).

Check, as well:
- range requests work;
- there is no whole-file decode;
- autoplay resumes after the first gesture, the same way `AppAudio` does today.

**If this fails in any WebGPU browser, stop and update the spec** before going further. Voice slices don't depend on this one.

## Seam

**`packages/battle-audio/src/music/musicPlayer.ts` → `MusicPlayer`:**
- `play(trackId, {fade})`, `crossfadeTo(trackId, seconds)`, `duck(depth, attack, release)`, `stop(fade)`.
- It is used by `AppAudio` (menu and loading screens) and, in slice 14, by battle.
- It stays **outside** `WebAudioSink`, because `MediaElementAudioSourceNode` doesn't exist on `OfflineAudioContext`.

**Buses:** `Bus` (`audioPresentation.ts:9`) gains `"music"` and `"voices"`.
- Neither bus sends to the distance reverb (`webAudioSink.ts:73-80`).
- The music bus ducks while any line plays on the voices bus. The depth and release are in `fixtures/game.json` → `presentation.audio.music`.

**Catalog:** `fixtures/sounds.json` gains a `music` section: `moods { menu, tension, combat } → [track ids]` and `tracks { id → { source, sha256, gain } }`.
- The source is the existing third-party record for the menu bed (EA/DICE).
- Slice 08 adds `Generated` sources for the new tracks.
- `catalog.ts` validates the section.

**Cutover:**
- Delete `menu_music` from `MENU_BED` (`appAudio.ts:10-13`) and from the bank's prepare call (`appAudio.ts:178`).
- Delete the `battlefield2-menu` WAV clip and its recipe from `sounds.json`, and the WAV file itself.
- The `countryside` ambience bed stays on the existing path.

**Levels:**
- `SoundSettings` (`settings.ts:4-11`) becomes `{ muted, volume, music, voices }`, with defaults of 0.8 / 1 / 1. Unknown values are clamped the same way as `volume`.
- The music bus gain is master × music. The voices bus is master × voices.
- **`apps/battle-lab/src/SoundControls.tsx`** gains two sliders beside master. It is the one control that `MainMenu` and `PauseMenu` both render.
- Load [`game-ui`](../../../.agents/skills/game-ui/SKILL.md) first. These are player-facing.

**Docs:**
- Add the Vercel project's Git LFS dashboard setting to `web/README.md` as a sharp edge (card L7 in `choices.md`).
- Update `packages/battle-audio/README.md` for the music path.

## What the human hears and sees
- The menu bed streaming from the menu.
- The sliders in the main menu and the pause menu.

## Verification
- **Tests:**
  - `appAudio` tests: the menu bed plays through `MusicPlayer`, and no WAV decode happens.
  - A settings test: the new levels persist and clamp.
  - A `MusicPlayer` test against a fake media element: the crossfade and duck curves.
- The browser matrix above.
- **Visual:**
  - Before and after shots of both menus, judged with [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md).
  - **Last check:** an unprimed [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md) of the sliders. Slice variable: the sound controls only; the rest of the menu is out of scope.

## Delegated to you
- `MusicPlayer` internals;
- element pooling;
- slider labels, within game-ui.

## Must stay green
- `web/scenes/sound.mjs`;
- menu audio lifecycle tests.

## Feedback that would change this slice
- A browser can't stream under COEP: fall back to decoded buffers for music only, with tracks cut to about 90 s loops. Record this as a named decision.

## Checkpoint (non-blocking)
Open the menu shots and play the menu for the user. Wait about 5 minutes, then proceed.
