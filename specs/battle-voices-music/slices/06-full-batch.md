# 06 — Full batch and packs

**Contract this unlocks:** every set has reviewed takes, and a published pack the battle can load.

## Run

**Generation:**
- Run `generate voices` for every set whose voices are picked. It runs in the background with visible progress (one line per take), and stops on quota or 401 errors.
- Order sets by how often skirmishes field them, using the roster and the skirmish AI's purchase lists, so the most-heard sets are ready first.
- Don't wait on the run. Start it in the background and keep working.

**Review:** the user reviews in the Voices tab.
- The default gate (`choices.md` → Open) is: play every flagged take, spot-check, then "Accept unflagged in set".
- Takes marked bad are regenerated with `R`.

**Packs:** `generate pack` writes `assets/runtime/audio/voices/<set>/pack.json`. Each pack lists:
- the set's **accepted** takes (trigger, intensity, voice, take, URL, duration);
- the speaker role of each voice (`responses`/`barks`).

A set with no accepted take for some trigger simply has no entry for it, and the battle stays silent there.

## Verification
- `generate check`: every take listed in a pack exists, its sha matches, and its verdict is ok.
- `generate report` gives the final totals: clips, characters, LFS bytes.
- Voices total about 20 MB, the L2 estimate. If they come out well over, report it before committing.

## Gate
The user's review. Nothing plays in battle before that review is in a pack.

## Delegated to you
- batch ordering;
- pack field names.

## Feedback that would change this slice
- A set sounds wrong as a whole: redesign its voice (slice 05) and regenerate that set only.
