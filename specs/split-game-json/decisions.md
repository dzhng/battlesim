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


## Slice 12

- **Directed by integrator — publish the last-fired mount weapon index.** The
  user did not answer the slice-02 choice within its window. The integrator
  selected the recommended `WeaponPose.round` mechanism on the user's behalf;
  adding it is authorized for this slice. Timing, visibility and battle digests
  remain the boundary.
- **Sound, high confidence — retain one ordered list of presentation mount rows.**
  Each row carries its weapon kind and admitted kick. Replacing the former
  first-weapon/kick pair lets effects and pose read the same published index;
  keeping a first-row alias alongside the list would create another owner.
- **Sound, high confidence — refresh captures through their owning bless tests.**
  Replaying the named deterministic battle verifies every stored digest before
  writing publication bytes. Camera records and browser count expectations stay
  untouched. This extends the existing `BLESS_PARITY` procedure rather than
  shipping a raw-layout migration script.
- **Sound, medium confidence — reconstruct before selection at the presentation boundary.**
  Both GPU capture arms run the same battle and producer. The before arm makes
  each mount's presentation rows copies of its first row, reproducing the old
  flash/kick choice. It is disclosed as reconstructed evidence; independent
  visual acceptance stays with the integrator. No production debug hook remains.


## Slice 14

- **Sound, high confidence — review sound blocks, never whole looks.** A draft
  carries one sound block for every existing look alongside its library draft.
  The server patches those blocks into the captured presentation document; the
  browser cannot submit visual changes, rename a look or choose a path. The plan
  fixed the save destination but left the wire shape open. This keeps the same
  reviewed candidate and rollback owner when library and look edits save together.
- **Sound, high confidence — shared recordings stay shared when a level saves.**
  A blast or motor without a local recording auditions the mix's shared choice;
  impacts use the mix's material levels and the look's impact scale. Changing
  only a gain preserves the omitted recording fields. The plan requested these
  auditions without specifying how to expose the mix defaults, and copying the
  displayed choices into the look would silently create new content overrides.
  Mix controls remain read only, matching the existing tool's scope.

## Slice 15

- **Directed — split artillery appearances once.** Firing identity outranks
  the content slice's no-hash promise: `artillery_howitzer` keeps `artillery`,
  `artillery_mortar` uses `mortar`, and both rocket artillery weapons use
  `rocket_artillery`. Copy the original look values before tuning sound, so
  visuals and battle outcomes remain unchanged. Re-record catalog and paired
  replay identities through their owning tests in that same assignment commit.
- **Directed by integrator — mortar split on measurement, not listening.**
  Nobody has listened to the two crops; the user was not asked. The
  integrator measured 03's audition crops with ffmpeg astats: mortar
  below-150-Hz RMS -26.9 dB, above-2-kHz RMS -36.3 dB (about 9 dB
  bass-dominant); howitzer -32.8 dB and -21.7 dB (about 11 dB
  treble-dominant). About 20 dB of spectral tilt plus about 6 dB of overall
  RMS separates them, far past an audible difference, so the mortar gets its
  own sound. A listen by the user is still the real check.
- **Directed — singles and constructed AA cadence.** Marksman has no burst
  group and heavy sniper has no magazine: use 03's single-report crops per
  observed launch. AA uses the Bofors relay fallback at 0, 0.300 and 0.600 s,
  explicitly labelled as designed timing, not a natural matching recording.
- **Directed — release and rocket-pod fallbacks.** Bomb release uses 03's
  restrained mechanical/air recipe without an explosion. Accept Apache audio
  only if its motor separates from its rotor bed on listening; otherwise use
  03's AT4 attack plus exhaust tail, once per observed rocket.
- **Cadence enforcement also exposes the LMG's inherited mismatch.** Its
  unchanged 80 ms shot interval was assigned the rifle's 100 ms recording.
  Relay the existing distinct reports at 80 ms and label that fallback;
  refusing the shipped LMG or changing its firing rule would violate the task.
- **Apache conditional source is not admitted.** Its six-second spectrogram
  contains continuous low-frequency rotor energy and broad overlapping energy
  through the crop, with no established isolated motor segment. Agent-side
  listening is unavailable, so the required separation is unverified. Use
  the explicitly authorized AT4/exhaust single-launch fallback. The spectrum
  and review boundary live in shared `evidence-15-sound/rocket-pod/`.
- **Rocket artillery uses a short ground-launch ignition fallback.** Cut the
  accepted M270 source before its long strike-missile exhaust finishes; play
  it once per rocket, not a whole salvo on every round. The source is an
  ATACMS launch, so the clip explicitly records this designed fallback.
