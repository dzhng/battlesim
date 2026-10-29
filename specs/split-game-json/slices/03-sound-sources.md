# 03 Spike: sound sources

**Question:** do real recordings exist for every firing sound the content pass
([15](15-content.md)) needs?

**Needed:**
- firing sounds: artillery, mortar, rocket salvo (rocket pods), bomb release,
  `artillery_strike_missile`;
- burst recordings at each weapon's cadence: marksman rifle, heavy sniper, the
  AA guns (cadence from each row's fire interval and burst; see
  `sound/frame.rs:846-866`).

**Seam:** none in code. A source ledger in this file.
- Each entry: URL, author, license, the crop in seconds, and why it fits.
- Candidates go in `throwaway/sound-sources/`, fetched with `yt-dlp` as the
  catalog already does (`packages/battle-audio/tools/assets.py`).
- The repository is private, so All-Rights-Reserved sources are acceptable
  (user, 2026-10-29).

**Run / hear:** one trimmed WAV per candidate, opened for the user. This is a
non-blocking checkpoint: about 5 minutes, then pick on the evidence.

**Verify:** each burst candidate's shot spacing matches its weapon's cadence
within 5 ms per shot.

**Moves:** nothing.

**Delegated:** source search and crop choice. A recipe layered from existing
clips stands in only where no source is found; record it as a fallback.

**Would change 15:** a missing source. Note the fallback here so 15 doesn't
discover it late.
