# Battle audio

Sound observes the same side-visible causes as effects. It never changes firing
cadence, physical impacts or simulation hearing. Anonymous hearing cues stay anonymous;
an exact unit assignment applies only to an observed launch.
The [combat audio rationale](../../specs/done/combat-audio/README.md) records the
constraints behind the library and workbench.

The [sound catalog](../../fixtures/sounds.json) separates provenance, clean clips,
reusable recipes and assignments. Each recording in a recipe is an alternative,
not another shot. Optional synthesis supplies restrained body inside that same
buffer, under each of its shots, so one cause still costs one voice.

Automatic fire is heard as recorded bursts, not one report repeated per round.
A burst clip re-lays several source shots at its gun's shot interval, so one
voice sounds the gun's next rounds too while they land on its shots; a round
off that cadence, or past its last shot, starts the next recording. Rounds stay
aligned with their muzzle flashes and the same report never machine-guns. A
recipe's alternatives, and a firing choice's near and far sounds, cover the
same rounds; a burst never covers more than the gun's own burst. Cannons,
grenades and launchers keep single reports.

A round's contact sound follows the round as well as the surface: a hit by
its material, a glance by its ricochet, so a shell strikes and deflects off
armour with weight a rifle round lacks. Unassigned rounds fall back to the
contact's default, then the shared slot. Heavier rounds sound louder by their
impact scale.

A battle prepares only what it can play: the synthesized baselines and every
assigned or replacing recipe. The rest of the library is stored for audition
and downloads only when auditioned, so growing it costs a battle nothing. Baseline synthesized recipes remain
selectable under their original IDs. A recorded recipe has a distinct ID.
The bank measures each prepared clip or recipe once and keeps a separate source
calibration gain. Audition and battle playback apply that gain before their authored
volume controls. The [loudness owner](src/loudness.ts) uses K-weighted maximum short
windows, so short reports and long quiet tails share a useful reference. This is a
library calibration, not a measurement of the final battle mix. Source samples,
synthetic waveforms, decay and stereo balance remain intact; only playback gain
changes. Recipe gain and a synthesis-only recipe's support level remain intentional
volume adjustments. Silence stays silent.
Mixed loops repeat each layer at its own period and crossfade the resulting seam;
a shorter layer never creates a silent gap in the longer loop.

Assignments identify an exact unit type and an authored mount name. Equivalent
physical mounts share their named equipment choice. The unit catalog owns the
roster; the audio catalog does not resolve inheritance. Hull launch evidence
identifies the mount rather than its currently selected ammunition.

The [sound workbench](../../apps/sound-workbench/README.md) auditions the complete
library, including unused reloads and mechanical actions. Storing a clip does not
create a gameplay event. Audio saves affect a new or explicitly restarted page;
an active battle retains its accepted audio generation.

## Recording preparation

Pinned inputs live under `assets/third-party/audio`, and mono PCM WAVs under
`assets/runtime/audio`. Both use Git LFS; pull these paths in a fresh checkout.
The source hash, frame range and processing profile in the catalog are the rebuild
recipe. [The audio asset tool](tools/assets.py) owns processing and checks prepared
bytes against that recipe. Run its help for commands; it needs Python 3, ffmpeg
and ffprobe, with no additional Python packages.
The frozen [gunfire source review](tools/gunfire-regions.json) retains the original
numbered take boundaries and user-directed splits and exclusions. Listening estimates
such as “halfway” do not replace the catalog's exact retained crop frames.

A clip is a crop of one recording, a burst of its shots, or a design that
layers crops of several recordings, each placed at an offset with its own gain,
pitch and optional low-pass. Pitch shifts by resampling, so a lower layer is
also slower, as a bigger body would sound; each layer fades out rather than
stopping mid-rumble. Designs keep every source's provenance, as crops do.

Sources are public domain, CC0 or Creative Commons with attribution, which the
source row's author and URL supply. Non-commercial recordings are marked in
their notes and must not ship in a commercial release.

The tool averages stereo channels, keeps the source crop in its original frame
coordinates, applies the selected tonal profile and fades, and normalizes peak
level with headroom. Loop clips use a crossfade. A burst clip names each shot's
source frames and overlap-adds them at its interval: a shot's retained tail ends
before its next source attack, so a recording faster than the gun leaves a short
gap between shots rather than a doubled attack. It admits all pinned sources and
all crop ranges before replacing any outputs. Browser Web Audio handles decoding
and sample-rate conversion; audio loading never happens per projectile.

Family labels describe perceived character rather than an unverified weapon
model. Measurements prove onset, clipping and reproducibility; timbre choices
remain reviewable in the workbench. Source quality and designed reuses are explicit
in each clip's provenance.

The [sound frame](src/soundFrame.ts) owns timing, distance and voice allocation.
Only implicit presentation slots follow shared effect replacements; an explicit
recipe selection keeps its identity. The [bank](src/soundBank.ts) prepares live
and offline buffers through the same asynchronous path. Disposal aborts loading
and prevents late admission.

The [page audio owner](src/appAudio.ts) retains one context and bank across
client navigation. Its menu bed attempts playback early; when browser permission
blocks that attempt, a qualifying input resumes the same context. The bed stays
through loading and fades only when prepared battle audio updates. Source
calibration and the persisted sound settings govern both mixes.

Each [battle observer](src/battleAudio.ts) owns fresh side-visible evidence and
a disposable mixer. Leaving a battle disconnects every voice and the whole
mixer output, including reverb tails; it does not cancel shared bank preparation.
Publications arriving before readiness or while muted are dropped so preparation
and unmuting cannot replay old combat.

Real document departure closes the page engine and cancels bank preparation.
A document restored from the browser's back-button cache remains closed and
requires the app's manual Reload recovery; destroyed handles are never revived.
