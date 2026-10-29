# Implementation decisions

## Slice 08

- **Sound, medium confidence — keep look assignments read only until 14.**
  The library editor still edits recipes and auditions looks and movement.
  Assignment writes need presentation/audio publication, which slice 14 owns.
  Rejected retaining the old library assignment fields or translating them back
  into looks: both would preserve a second owner and a compatibility format.
- **Sound, high confidence — preserve the inherited burst-cadence bugs.**
  Resolving all looks exposes AA/marksman recordings that the old test skipped
  because it checked only explicit catalog assignments. Slice 08 checks that
  near and far cover the same burst; the existing playback tests still check
  scheduling. Weapon-cadence enforcement waits for 15's corrected recordings,
  as required by the exact-output contract. Rejected silently changing those
  sounds or refusing today's shipped rows.
- **Sound, high confidence — cook-off fireballs take their own scale.**
  They pass the former default scale directly to the blast constructor, rather
  than inventing a weapon or an otherwise unused look for `cook_off`.
- **Sound, high confidence — admit defaults before checking look overrides.**
  The mix owns each part's default recording, and the look reader fills absent
  recording overrides once at setup. Firing and every level remain explicit.
  Rejected resolving fallback weapon rows during playback.
