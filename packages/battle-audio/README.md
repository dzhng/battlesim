# Battle audio

Sound observes the same side-visible causes as effects. It never changes firing
cadence, physical impacts or simulation hearing. Anonymous hearing cues stay anonymous;
an exact unit assignment applies only to an observed launch.
The [combat audio rationale](../../specs/done/combat-audio/README.md) records the
constraints behind the library and workbench.

The [sound catalog](../../fixtures/sounds.json) separates provenance, clean clips,
reusable recipes. Its [Rust reader](../../crates/presentation/src/sound/catalog.rs)
admits both player loads and workbench drafts, including source metadata and
burst/loop compatibility. The [audio mix](../../fixtures/audio.json), [weapon looks](../../fixtures/presentation.json)
and resolved weapon rows are admitted together before a bank prepares voices
or the workbench previews or saves a library draft. Each recording in a recipe is an alternative,
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
armour with weight a rifle round lacks. Its weapon look carries contact overrides and loudness; absent recording overrides
use the mix's default for that part.

A battle prepares only what it can play: the synthesized baselines and every
assigned or replacing recipe. The rest of the library is stored for audition
and downloads only when auditioned, so growing it costs a battle nothing. A synthesized baseline is
selected by its sound's name; a recorded recipe has an ID of its own.
The bank measures each prepared clip or recipe once and keeps a separate source
calibration gain. Audition and battle playback apply that gain before their authored
volume controls. The calibration ([`sound::bank`](../../crates/presentation/src/sound/bank.rs)) uses K-weighted maximum short
windows, so short reports and long quiet tails share a useful reference. This is a
library calibration, not a measurement of the final battle mix. Source samples,
synthetic waveforms, decay and stereo balance remain intact; only playback gain
changes. Recipe gain and a synthesis-only recipe's support level remain intentional
volume adjustments. Silence stays silent.
Mixed loops repeat each layer at its own period and crossfade the resulting seam;
a shorter layer never creates a silent gap in the longer loop.

Weapon rows name one appearance, inherited by the contract's catalog resolver.
[The look reader](../../crates/presentation/src/looks.rs) is the authority for
both clients: firing has an explicit recording, while other parts may use the
mix's defaults. Changing look values never changes battle rules or outcomes.
A hull's launch currently follows its mount's first weapon row.
Vehicle loops are keyed by vehicle class
(the catalog view's `vehicle_class`, which `contract::catalog::UnitType::vehicle_class`
derives from mobility, hull weight and the logistics role).

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

Each source row records attribution and rights alongside its pinned bytes.
User-supplied recordings retain an explicit receipt, including reserved-rights
material chosen for a personal project. Downloaded candidates may instead live
in the catalog's provenance section with a pinned hash and an explicit
rights-unassigned status; they are not playable sources until accepted. A sound the library has no clean recording of
is a generated sound effect: `bun run generate effects` buys its candidates once
(their prompts, in [the tool](tools/generate/effects.ts), rule out voices and
crowds), keeps each raw master under `assets/source/audio/effects/` as a
`Generated` source, and the asset tool cuts it into clips like a recording. Music is
the exception to this preparation: it streams instead (see [Music playback](#music-playback)).

The tool averages stereo channels, keeps the source crop in its original frame
coordinates, applies the selected tonal profile and fades, and normalizes peak
level with headroom. Loop clips use a crossfade. A one-shot clip never ends on a cut:
one still ringing at its end (a siren mid-cycle, a hit cut at 1.6 s, a generated master
stopping at its requested length) fades out over up to half a second by how loud it
still is there, and one starting inside a sound already going fades in over a few
milliseconds, never over an attack. A crop under 400 ms is a transient and keeps
the click guard. A burst clip names each shot's
source frames and overlap-adds them at its interval: a shot's retained tail ends
before its next source attack, so a recording faster than the gun leaves a short
gap between shots rather than a doubled attack. It admits all pinned sources and
all crop ranges before replacing any outputs. Browser Web Audio handles decoding
and sample-rate conversion; audio loading never happens per projectile.

Family labels describe perceived character rather than an unverified weapon
model. Measurements prove onset, clipping and reproducibility; timbre choices
remain reviewable in the workbench. Source quality and designed reuses are explicit
in each clip's provenance.

## Generated voices

Unit voices are generated with ElevenLabs and never hand-edited. [`voices.json`](../../fixtures/voices.json)
is the only authority on what is said, by which voice and through which profile;
[the voice catalog](src/voices/voiceCatalog.ts) validates it and names a take
(`set/voice/trigger/intensity/take`) and its served URL. Spending credits, and
writing `fixtures/voices.lock.json` and every voice audio file, belong to
[the generation tool](tools/generate/cli.ts) alone, run as `bun run generate`; its
bare usage lists the commands. It reaches ElevenLabs only through Duet's proxy with
this machine's Duet login.

Audio has two provenance kinds, both owned by [the catalog](src/catalog.ts): a
recording names its licence, author and pinned bytes; a generated sound (`Generated`)
names the generator, model, the text or prompt and, for a voice, the voice id and
description it was spoken with. Every voice take and every battle track is generated.

A take is two cached steps. Its **raw master** (`assets/source/audio/voices/`) is a
function of the expanded line, voice, model, settings and retake count, so it is paid
for once; bumping a retake in `voices.json` is how a judged-bad take is bought again.
Its **served file** (`assets/runtime/audio/voices/<set>/`, content-hashed) is a
function of the raw bytes, the profile's whole definition, its bed and the encoder,
so tuning a profile re-renders locally for free. A killed run is simply rerun: raw
masters on disk are trusted by name, and the lock is written atomically as it goes.
Every take is transcribed and compared with its script after stripping audio tags
and reading numbers as digits; a lost keyword (the callsign, "sabot", "gunner"), a
high word error or a take longer than its trigger's talk time flags it for review.
`check` proves the lock by hash and never re-encodes, since MP3 bytes differ between
ffmpeg builds.

A voice is bought before its takes. `design` asks Voice Design for three candidates
from the voice's description, each speaking its role's audition: the role's own
lines, steady to desperate, the same for every voice of the role so candidates
compare. A candidate's generated id is temporary, so its preview is kept as a raw
master (`assets/source/audio/voices/design/`), transcribed, and recorded in the
lock's `designs`; a design is bought again only when the description, audition or
the voice's design retake (`<set>/<voice>/design` in `retakes`) changes. `pick` saves
one candidate as a permanent voice and writes its `voice_id` into `voices.json`
through the workbench's fixture publication. Candidates are heard through the set's
profile, rendered locally into the scratch folder.

[`assets.py voice`](tools/assets.py) is the treatment: where the speaker sits
decides it (dry in the open, a light radio over the tank's engine, an earpiece for
recon, an intercom over the rotor). Each take is mixed with its profile's hiss and
bed, normalised by integrated loudness rather than peak, and encoded as mono MP3 at
about 40 kbps. `assets.py voice-profiles` prints the definitions the tool hashes.

A bed is the game's own sound for that machine, so a sound has one owner: the
profile names a clip in [`sounds.json`](../../fixtures/sounds.json) (the intercom's
rotor is `rotor-uh60`, the loop every helicopter plays) and a filter that places it,
such as the cockpit's muffling. Only a machine the game has no recording of gets a
generated bed, a raw master the tool's `beds` command buys once and the lock records;
the tank's engine is one, since the game synthesises engines. Either way the bed's
sha256 enters the processing key, so a changed clip or master re-renders its takes.

Which lines play is decided in one place, in Rust
([`presentation::sound::voices`](../../crates/presentation/src/sound/voices.rs),
bound by [the chatter](src/voices/chatter.ts)): answers before own shouts before a
seen enemy's, nearest the listener first (a battle's is its sound frame's own), a
few lines at once (`audio.voices`), a line dropped when it can't start
soon, a unit never over itself, one answer per group. It picks the take with the
client's own seeded randomness, never the simulation's, and gives start commands
on the voices bus that the page only carries out. Its takes come from
[the voice bank](src/voices/voiceBank.ts), which loads a battle's sets from their
`pack.json` in the background and tells the chatter each set's voices and each
take as it arrives, with the length its pack records; a take not yet loaded is a
dropped line, never a wait. A loaded take stays as fetched, MP3 about a twentieth
of its decoded size, and is decoded when a line starts it; the last few played
stay decoded. A line plays when its take is decoded, or is dropped if that comes
later than the chatter's wait after its start. The sound workbench's chatter
preview runs the same two.

A unit answers its owner ([responses](src/voices/responses.ts)) when selected,
when an order is accepted or refused for a reason the player caused (a unit an
order refuses on its own, such as a squad left without a seat, answers with its
refusal, and the rest answer the order; a refusal on arrival answers when the
side observes it, once), and when an ordered move (or a purchase's drive in)
ends after a real journey; each queued journey arrives on its own, starting when
the unit stops after the one before. One unit answers for a group: the one
clicked, else the highest-value. Only the owner hears an answer, dry and centred
on the voices bus.

A shout the side hears in the open, its own unit's or an identified enemy's, is
a bark: Rust's sound frame asks the chatter for it in the set its kind speaks
with for its side, and the line is spoken where the shout was, placed as every
unit sound is (distance, the air's low-pass and reverb send). The page hands
Rust each unit kind's set for its own faction and the enemy's
(`BattleAudio.speakers`, from `voices.json`'s assignments), once; Rust never
reads `voices.json`. Outside a skirmish the factions are unknown and shouts are
silent. Once its sounds are ready, a battle loads the sets of its own units and
pending purchases, and an enemy kind's set from the first time the side
identifies one, each in the background as it appears; a set without a pack is
silent. The page's line player is the mixer's (`WebAudioSink.line`), so a bark
has the graph every unit sound has. `generate pack` writes a set's `pack.json`
from the takes judged ok on the bytes served and spoken by the voice
`voices.json` names now, so a replaced voice's takes leave the pack even before
regeneration finishes; before review, `--unreviewed` also lists unjudged takes
the transcript check passed.

An unseen enemy's shout reaches the side only as an anonymous hearing cue, so
it plays a stressed bark from a generic pool of the enemy faction's sets, never
the speaker's own voice, muffled past understanding. [The shout pool](src/voices/shoutPool.ts)
only loads those takes, a few per shouting voice from each pack that exists,
and keeps them decoded because a cue starts at once (`takes_per_voice` bounds
it). Rust picks the take and places it. With no pack loaded a
shout is silent.

What a battle hears, when, where and how loud (timing, distance, the air's
colour, the voice budget and stealing, loops, footsteps, cues) is decided in
Rust, once for both clients ([`presentation::sound`](../../crates/presentation/src/sound/));
the [sound frame](src/soundFrame.ts) only carries its commands out on the
page's mixer, and native carries the same commands out on `kira`. Only
implicit presentation slots follow shared effect replacements; an explicit
recipe selection keeps its identity. The [bank](src/soundBank.ts) prepares live
and offline buffers through the same asynchronous path: it decodes the
recordings, and Rust synthesizes, composes and calibrates each recipe
([`sound::synth`](../../crates/presentation/src/sound/synth.rs),
[`sound::bank`](../../crates/presentation/src/sound/bank.rs)), so both clients
play the same samples. The synthesized sounds' names are generated from Rust
(`synthSounds.gen.ts`). Disposal aborts loading
and prevents late admission.

The bed keeps its level between shots and makes room under a loud one, as a
film mix does: a sound at least `audio.ambience.duck_over` loud
at the listener (its level there, as the voice budget ranks it, never a list
of weapons) dips the ambience bus by the bed's `duck`, which recovers once the
dip is reached. Distant fire never reaches the threshold, so the bed does not
pump under it. Both players dip the ambience through a gain of its own after
the bus, reverb tail included, so the dip never overwrites the bus level; the
music's duck under voice lines is the same command on the music bus.

The [page audio owner](src/appAudio.ts) retains one context and bank across
client navigation. Its menu bed attempts playback early; when browser permission
blocks that attempt, a qualifying input resumes the same context. The bed stays
through loading and fades only when prepared battle audio updates. Short mechanical actions give menu controls a quiet hover and a stronger activation.
They share the prepared bank, keep one transient voice and respect the persisted
sound settings. Source calibration governs both mixes.

Each [battle observer](src/battleAudio.ts) owns fresh side-visible evidence and
a disposable mixer. Leaving a battle disconnects every voice and the whole
mixer output, including reverb tails; it does not cancel shared bank preparation.
Publications arriving before readiness or while muted are dropped so preparation
and unmuting cannot replay old combat.

Real document departure closes the page engine and cancels bank preparation.
A document restored from the browser's back-button cache remains closed and
requires the app's manual Reload recovery; destroyed handles are never revived.

## Music playback

All music, the menu bed included, streams through the [music player](src/music/musicPlayer.ts):
a media element feeds the Web Audio graph, so a track is never decoded whole
into a buffer. It sits outside the voice mixer because offline contexts have no
media-element source.

The catalog's `music` section maps each mood (`menu`, `tension`, `combat`) to
track ids. A track names the `sources` record it was encoded from, the hash of
the served file, and a gain. The served file is
`assets/runtime/audio/music/<track>.<sha8>.mp3`, a mono 128 kbps encode written
by the asset tool's `music` command; the hash in the name keeps a re-encoded
track from meeting a cached one. A track's gain sets its loudness in the mix.

The battle score is generated, so its sources record how (`Generated`: model,
prompt, length) instead of a licence, and a music master is never cropped into
a clip. `bun run generate music` owns those tracks: their table and prompts live
in [the tool](tools/generate/music.ts), each raw master is paid for once and kept
under `assets/source/audio/music/`, and each served encode is given the gain that
sets it to −26 LUFS. `--track` buys a track again.

Which mood plays, which track comes next and when the music dips is decided in
Rust, once for both clients ([`presentation::sound::music`](../../crates/presentation/src/sound/music.rs)),
from the side's own barks and whether a voice line sounds. Its numbers live in
`audio.music`. Each mood's tracks shuffle by the score's seed, every
track once a shuffle and never one back to back. Rust names a track by its
catalog id and emits music commands with the rest: start a track fading in, stop
every track fading out (a crossfade is the two over one span), and duck. In a
battle, [the score](src/music/battleMusic.ts) opens on `tension` as the menu's
music fades, its seed drawn by the page, and only carries the commands out on the
player, which resolves an id to its served file. The menu's music is the app's,
not a battle decision.

Music feeds the same master as the rest of the menu mix, so master volume and
mute cover it. Inside the player, a level gain carries the player's music
setting and a separate duck gain dips under voice lines, so neither overwrites
the other. The `voices` bus carries the voices setting.
