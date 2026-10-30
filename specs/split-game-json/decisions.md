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
- **Default recordings:** follow the [look sound contract](map.md#known-unknowns-the-decision-ledger) for mix defaults, explicit firing and per-look levels.

- **Sound, medium confidence — use one controlled battle lane for the sheet.**
  Each look is assigned to the review lab's HE round and the mount's first
  ammunition row, because hull flashes still follow that first row until 12.
  The capture overrides the development mechanics snapshot, which startup
  installs over imported fixtures; the ordinary simulation, publication,
  fielding and rendering paths produce every shot. Each launch must offer a
  cast light with the requested look at the actual review muzzle. Captures
  trigger on the mount counter, before the first projectile segment, so short
  flashes remain visible. Labels identify the controlled lane: these are
  style samples, not each weapon's trajectory or blast radius. Rejected
  blanks for unfielded looks, visibility-dependent street shots, and
  presentation-only production hooks.


## Slice 10

- **Sound, high confidence — a replay refuses a contradictory clock.** When a
  capture says it ran at a different tick rate from the encounter it names,
  native refuses it. Guessing which clock to trust could mis-time every shot
  while still drawing plausible frames. The slice did not specify this failure
  case; future captures must preserve their encounter's tick rate.
- **Sound, high confidence — absent look parts remain absent.** A crash can
  supply only its blast. Fired rounds still need explicit tracer, impact and
  kick data in addition to shot and flash, because those consumers require
  them; the fielding error names the weapon. Rejected filling missing parts
  with synthetic zero values, which would hide an incomplete authored look.
