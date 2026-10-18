# Battle voices and music

Units talk as in an RTS. A unit answers its owner when selected, ordered, arrived
or refused (responses, R1–R12). In battle it shouts what is happening to it
(barks, B1–B17), and a side hears it near the camera from its own units and
from the enemies it identifies. An unseen enemy is heard only as distant, muffled shouting
from a direction. Under it all plays one background score in the manner of
Broken Arrow's, moving between a tension mood and a combat mood. Every line and
every battle track is generated with ElevenLabs and judged in the sound
workbench; the menu bed stays the recording it was.

The [choices ledger](choices.md) is the record of every decision behind the
feature, checked against the shipped code: what the user decided, what the code
settled, and what an agent decided on the user's behalf, with the reason. It is
the user's review surface; this README explains the shape and points at the
owners. Slice numbers in the ledger name the build step where a decision was
made; the commits titled "(slice NN)" hold each step.

## Why it is shaped this way

**The simulation decides who shouts what; the client decides only how it
sounds.** A bark is a fact about the battle (a near miss, a man down, a missile
seen coming), and only the simulation knows it. So the simulation emits a
`Shout` per bark into each side's observation frame, under the same fog as every
other sense: a side hears its own units and an enemy only while it identifies
that enemy. An unseen enemy's shout reaches the other side as an anonymous
hearing cue. The client picks the take, the priority and whether a line plays at
all. The one leak the user accepted by name: a seen enemy's stressed take or
"pinned!" reveals its state.

**Shouts must never touch determinism.** The battle digest is the proof that two
runs are the same battle, and shouts change no outcome. Shout state therefore
lives in a per-side ledger beside the observations, never on a unit, never in
the digest and never drawn from simulation randomness. Adding shouts left every
battle's outcome and digest unchanged. Two changes were named decisions:
`ShoutRules` changes the rules' config digest, so old replays are refused (they
already were, by build fingerprint), and the publication bytes were re-blessed
exactly once, when the shouts group was added.

**Sound decisions live in Rust; the page plays what it is told.** The plan put
line arbitration in TypeScript. Mid-build, the native-parity work moved every
sound decision into the shared presentation crate under the rule "Rust as smart
as possible, clients as dumb as possible", so a native client plays the same
battle the browser does. Chatter was ported to Rust and held to the TypeScript
behaviour by vectors the TypeScript wrote before it was deleted; barks in battle
and unseen shouts were built in Rust from the start. Two decisions stay in
TypeScript on purpose: which answer to give (its inputs, selection and clicks,
exist only in the client; choices S7) and the score's mood director (open, N2).

**Nothing plays that nobody judged, and a judgement can't outlive the sound it
judged.** A verdict carries the hash of the bytes heard; a pack lists only takes
judged ok on their current bytes and spoken by the voice `voices.json` names
now, and the game plays only what its pack lists.
Because the user is the only reviewer, review is built for spot checks: takes
that pass the speech-to-text check may be bulk-accepted after the flagged ones
are played, which is why a misread under the threshold can ship unheard
(choices S1). Music has no verdict: the six tracks play because they are in the
catalog (open, N1).

**One voice set per country × role, not per unit type.** A set's lead voice
answers and one or two soldier voices bark. Accents follow the faction for
infantry and the building country for machines (choices S3). This keeps the
library at 56 sets and about 7,000 takes while every unit still sounds like its
crew.

**Paid-for media is kept, and stale derivatives are not shipped.** Every raw
master and design candidate stays in source; a served file is a re-renderable
derivative and is deleted when its take is regenerated (choices S35). A generation
key names the raw master so a killed run never pays twice; a processing key
decides when the served file must be rendered again, so retuning a profile
re-renders locally for free. A served file is named by its content hash, so a
new take never collides with a cached one (choices S30, S32).

**A battle pays only for the voices it can hear.** It becomes playable without
waiting for any voice. Packs load in the background: own units' sets, an enemy
kind's set from the first time it is identified, and a small bounded pool for
unseen shouts. The pool stays decoded; any other take is decoded when its line
starts. The first shipped version loaded every set both rosters speak with
and kept them decoded, about 1.3 GB of PCM; the fix brought a fresh deploy to
about 33 MB (choices S38).

## What must stay true

- **The simulation alone decides who shouts what.** No AI reads the shout
  channel. Hearing stays anonymous: an unseen shout is an ordinary hearing cue
  of the shout category (listener, sector, band); no speaker, set, trigger or
  position reaches the client.
- **Shout state is in `ShoutLedger`,** per side, with the battle's per-tick
  shout evidence beside it: all outside `Unit`, the digest and simulation
  randomness. One owner builds each unit's shouts (`shouts::build`);
  hearing only filters them (`shouts::heard`, `shouts::unheard`).
- **`fixtures/voices.json` is the only voice authority.** `sounds.json` never
  lists a voice clip, unit rows carry no voice field, and Rust never reads
  `voices.json`.
- **One writer per generated record.** `voices.json` is written only through
  the fixture-publication save path: by people in the workbench, and by
  `generate pick` when it saves a voice. The generation tool alone writes
  `voices.lock.json`, the packs and the raw masters, has `assets.py` render the
  served files, and writes the generated music entries of `sounds.json` (whose
  other entries belong to the workbench's sounds editor and `assets.py`). The
  workbench reaches the tool through one queue. Credits are spent only through
  the tool; the browser never calls ElevenLabs.
- **`assets.py` alone owns ffmpeg and encoding.** The tool calls it.
- **Rust's `Chatter` is the only place line limits are enforced,** for battle
  and for the workbench's chatter preview alike. Its rules, including a line's
  level, are data in `presentation.audio.voices`. The player only carries out
  its start commands; the one line it drops itself is one whose take decodes
  later than `wait_s` after its start (choices S38).
- **`MusicPlayer` is the only music path,** menu bed included. Music streams
  and is not a mixer bus.
- **One provenance owner** (`catalog.ts`, with its `Generated` kind) **and one
  levels owner** (`SoundSettings`, shown by one `SoundControls` in both menus).
- **No new places.** Generated audio lives under `assets/source/audio/` (raw)
  and `assets/runtime/audio/` (served, content-hashed names); music lives in the
  existing sound catalog. The art bake owns `assets/runtime/`'s top-level hash
  directories, so audio never uses one.

## Where the build diverged from the plan

- **Decisions moved to Rust** (above). The plan's TypeScript Chatter shipped,
  then was ported and deleted; `cue_voices`, the old murmur for heard-but-unseen
  infantry, was deleted from Rust's synth rather than TypeScript's.
- **Voices are saved permanently.** The discovery walk planned to save a
  designed voice only while its lines generated and to clone a reference clip
  for later retakes. The user chose permanent Voice Design voices instead, on a
  Pro plan; premade and cloned voices were rejected.
- **There is no music bus.** Music streams through a media element, which an
  offline context can't host, so the mixer's buses are units, effects, ambience
  and voices (choices S27).
- **Talk time is 2.5 s per bark,** not the plan's 1.5 s, because the generated
  barks run about 2–2.7 s (choices S5).
- **A line's level became data** (`gain` in `presentation.audio.voices`) after an
  audit found it as a constant left behind by the move to Rust (choices X1).
- **Eight sets have a female lead,** against the plan's "about six"
  (choices S2).
- **The library is far larger than planned:** about 60 MB of served voices and
  300 MB of raw voice masters in Git LFS, a consequence of keeping everything (choices S12).
- **Lines a voice misread twice were reworded,** not retried, and voices with
  over 30% of takes flagged were designed again (choices S4, S11).

## Dead ends

- A voice set per unit type; premade or cloned voices; any language but
  English.
- A heavier radio treatment with soft clipping; the user asked for less, and the
  light radio, earpiece and intercom chains were approved.
- The first round of music: foreground styles with melody and hooks were
  rejected for the Pulse underscore.
- A top-level `assets/generated/` with its own Vite plugin, a separate
  `music.json`, a review file and a pack index (choices S36).
- A `voice` field on unit rows: `UnitType` refuses unknown fields and shared rows
  need a per-faction answer (choices S16).
- The `cue_voices` murmur, which sounded "like footsteps or heavy breathing".
- Loading and decoding every roster voice per battle (choices S38).

## Audio provenance

These recordings are the standard the library was judged against. They live in
[`assets/`](assets/).

- **`assets/saved-voices.json`** holds the four reference voices the user
  approved during the discovery walk: their saved ElevenLabs `voice_id`s and
  descriptions. They seeded the US tank crew, Russian rifle squad, US recon and
  British pilot sets, and every later voice description follows their register.
- **`assets/reference-voices/`** holds those voices' auditions
  (`us_tank`, `ru_rifle`, `us_recon`, `uk_pilot`) and the treatments the user
  approved by ear: the tank on the light radio (`us_tank-radio-light`), recon on
  the earpiece and the pilot on the rotor intercom (the `candidateA` renders).
  They drove the processing profiles in `assets.py` and the role scripts' style.
- **`assets/reference-music/`** holds the 30 s excerpts of the direction the user
  chose: Pulse for combat (`u_combat_pulse`, with a `ctx26_` context mix of it
  from the same round), Pulse sparse and Pulse dark for tension. They drove the
  prompts of the six generated tracks.
- **Broken Arrow's score** (Evgeny Shchukin and Alexander Tochilkin, analog synth
  and orchestra on an adaptive-intensity engine) was the named inspiration:
  [Steam OST](https://store.steampowered.com/app/3693780/Broken_Arrow_Soundtrack/),
  [Apple Music](https://music.apple.com/us/album/broken-arrow-vol-1-original-videogame-soundtrack/1818789596).
- **[`visualizations/discovery-map.html`](visualizations/discovery-map.html)**
  is the 2026-10-11 discovery walk: the known unknowns, the decision ledger and
  the account findings. It is superseded wherever `choices.md` differs.
- **[`visualizations/voice-sets.html`](visualizations/voice-sets.html)** is the
  roster → set table and role scripts the user could veto before generation
  began, frozen as reviewed. `bun run generate plan --html` renders the current
  one into the scratch folder.

ElevenLabs terms behind the account choice: [voice slots per plan](https://help.elevenlabs.io/hc/en-us/articles/24351056337937-How-many-voice-slots-do-I-get-per-tier-and-how-can-I-increase-it),
[pricing](https://elevenlabs.io/pricing), [output rights](https://terms.law/ai-output-rights/elevenlabs/).

## Where the contracts live

The [audio principles](../../../packages/battle-audio/README.md) (Generated
voices, Music playback) and the [workbench principles](../../../apps/sound-workbench/README.md)
(the Voices and Music tabs) describe the mechanics. Entry points:

- **Simulation:** `Shout`, `ShoutSpeaker`, `ShoutTrigger` in
  [`crates/contract/src/observation.rs`](../../../crates/contract/src/observation.rs);
  `ShoutRules` in [`crates/contract/src/scenario.rs`](../../../crates/contract/src/scenario.rs);
  `ShoutLedger`, `PRIORITY`, `under_fire`, `incoming`, `build`, `heard`,
  `unheard` in [`crates/sim/src/shouts.rs`](../../../crates/sim/src/shouts.rs).
  Pinned by [`crates/sim/tests/shouts.rs`](../../../crates/sim/tests/shouts.rs)
  (including that AI sources never read shouts), `menu_reel.rs` for an unchanged
  menu reel (its recorded event logs) and `publication.rs` for the shouts group.
- **Decisions:** `Chatter`, `ChatterRules` and `Speakers` in
  [`crates/presentation/src/sound/voices.rs`](../../../crates/presentation/src/sound/voices.rs);
  the unseen-shout cue in `sound/frame.rs`. Pinned by
  [`crates/presentation/tests/voices.rs`](../../../crates/presentation/tests/voices.rs)
  against the recorded chatter vectors.
- **Browser:** `BattleAudio` in
  [`battleAudio.ts`](../../../packages/battle-audio/src/battleAudio.ts), which
  decides which sets a battle loads; the Rust binding `chatter.ts`,
  `BattleVoices`, `VoiceBank`, `ShoutPool`, `responses.ts` and
  `voiceCatalog.ts` in
  [`packages/battle-audio/src/voices/`](../../../packages/battle-audio/src/voices/);
  `MusicPlayer`, `MusicDirector`, `BattleMusic` and `musicCatalog.ts` in
  [`packages/battle-audio/src/music/`](../../../packages/battle-audio/src/music/).
  Their tests are under `web/tests/` (`battleAudio` for which sets load,
  `chatter`, `voiceBank`, `shoutPool`, `responses`, `unitResponses`,
  `voiceCatalog`, `musicDirector`, `musicPlayer`, `battleMusic`).
- **Generation:** the `bun run generate` tool in
  [`packages/battle-audio/tools/generate/`](../../../packages/battle-audio/tools/generate/)
  (proxy client `elevenlabs.ts`, `lock.ts`, `packs.ts`, `transcript.ts`,
  `music.ts`) and `VOICE_PROFILES` in
  [`assets.py`](../../../packages/battle-audio/tools/assets.py); the workbench
  reaches it through `apps/sound-workbench/generation.ts`. Pinned by
  `web/tests/voiceGenerator.test.ts`, `musicGenerator.test.ts`,
  `voicesTab.test.tsx` and `musicTab.test.tsx`.
- **Data:** [`fixtures/voices.json`](../../../fixtures/voices.json),
  `fixtures/voices.lock.json`, the `music` section of
  [`fixtures/sounds.json`](../../../fixtures/sounds.json), and
  `presentation.audio.{voices,music,cues.shout}` in `fixtures/game.json`.
