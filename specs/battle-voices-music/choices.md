# Battle voices and music: choices

Every entry below is a given for whoever builds this. Each one says who closed it:

- **user**;
- **territory**, meaning the code or a live probe answered it;
- **agent**, meaning I answered on the user's behalf under the delegation recorded below, with the preference and reason.

Append build-time decisions under "Build log" at the end, attributed the same way. The walk happened on 2026-10-11. Its full map is [`visualizations/discovery-map.html`](visualizations/discovery-map.html), and this file supersedes it wherever they differ.

## Delegation

The user confirmed four preferences:

1. Take the recommendation when it's grounded in the code.
2. Atmosphere over leanness.
3. The simulation is the authority.
4. Numbers are guesses to tune.

The user opted in to agent answers for **structural** questions: workbench layout, file formats and naming, and catalog records. **Not delegated:** taste (voices, accents, treatment, music), and account decisions. Spending on this feature's generation was delegated later; see "Closed after the plan".

## Access and account

- **D1 (territory):**
  - ElevenLabs is reached through Duet's service proxy, `POST https://ctl.duet.so/v1/services/elevenlabs/<ElevenLabs path>?workspace=<slug>`, with the Duet bearer token from `~/.duet/config.json`.
  - The workspace must go in the query parameter. Headers are refused with `workspace_ambiguous`.
  - The proxy strips `xi-api-key` and injects Duet's key.
  - Allowed: text-to-speech (v3 with audio tags), dialogue, voice design and create, speech-to-speech, speech-to-text (`scribe_v1`, multipart, verified), sound effects, music (`music_v1`).
  - Denied: `voices_read`, `models_read`, `user_read`, so voices, models and the balance can't be listed.
  - The route isn't in Duet's public reference. Keep the base URL in one constant.
- **Account (user, 2026-10-11):** the user upgraded the account to **Pro**: 160 custom-voice slots, 8 concurrent TTS calls verified. Music allows about 2 concurrent calls and returned an occasional 500. This supersedes the map's Starter/Creator analysis (card L1) and A6.
- **Voices (user):** every voice is a **Voice Design voice, saved permanently**.
  - Rejected, premade voices with accent tags: the user compared them side by side and said the designed voice "sounds much better than the others".
  - Rejected, cloning or re-cloning from a reference clip: unnecessary with permanent voices.
  - The four T1 voices were saved on 2026-10-11, with ids in [`assets/saved-voices.json`](assets/saved-voices.json):
    - us_tank `z0wdO1PMjmqh0usMe4hA`
    - ru_rifle `Jqx4K9GHzgAqONoiSwhb`
    - us_recon `HDZsRSSxHYdXkV0AKe32`
    - uk_pilot `DqHWHfkdp2tajWvTPtM7`
- **Licence (territory):** paid ElevenLabs plans allow commercial use in games. ElevenLabs keeps a training licence over the content. Provenance records "made under Duet's ElevenLabs plan".

## What talks, and who hears it

- **D2 (user):**
  - **Responses** are R1–R12, heard by the owner only.
  - **Barks** are B1–B17. They happen in the world, are heard near the camera, and play for every side.
  - The user's reason: "in our hollywood realism battle you would be hearing people shouting all the time".
- **D3 (user):** every line is in English. Non-English nations speak with their accent.
- **D4 (user):** one voice set per **country × role**, about 40 sets. Squads get 2–3 voices: the leader answers responses and soldiers shout barks.
  - Rejected: a set per unit type, about 90 sets (option B), and a later per-unit override (option C).
- **D5 (user):** **the simulation emits shouts.**
  - A side hears all of its own units' shouts, and an enemy's only while that enemy is seen.
  - Picking the take is the client's, using its own seeded randomness.
  - Rejected: the client deriving shouts (B), and unseen shouts at their true position (C).
  - **The leak is accepted by name:** a seen enemy's _stressed_ take or a pinned bark reveals its state. The comment on `IdentifiedUnit` at `observation.rs:14-15` ("never health…") is amended in slice 13 to name this audible exception.
- **D6 (user):** all 29 triggers, B17 included even though the agent recommended cutting it. Each has steady and stressed intensity × 2 takes.
  - **Stressed** means health ≤ ½ or suppressed.
  - **Agent:** for squads, "health" is living members ÷ starting members, because `OwnUnit.hp` is 0 for infantry (`observation.rs:457`). Preference 1; this resolves an undefined term.
- **D7 (user):**
  - In the simulation: an 8 s cooldown per unit per trigger, and a unit never talks over itself.
  - In the client: at most 4 lines at once. Owner responses, then own barks, then others, nearest the camera first. A bark that can't start within 1 s is dropped. One unit answers for a group.
  - Rejected: denser (B) and sparse radio-style (C).
- **Callsigns (user):** one callsign per voice set, spoken in R1 and R11 only. Every other line is callsign-free, so B17 lines are generic ("We lost one!").
  - Rejected: callsigns per unit type (about 700 more clips) and no callsigns at all.
- **European generic infantry accent (user):** German. This covers the rifle, assault, marksman and recon-patrol squads. The agent had recommended British.
- **Swearing (user):** allowed in stressed lines.
- **Female voices (agent; A7, T5):** about 6 sets: pilots, plus one vehicle crew per faction. The user can veto any in slice 01's review. Preference: T8, cheap to change in review.

## The three physical barks (agent, under tweak-mechanics; the user approved the slice list)

These are presentation only. They change no outcome, and none adds state to `Unit` or draws RNG.

- **B5, under fire:** a unit hears an **enemy** round. That means either:
  - a `FlightEvent::NearMiss` within the weapon's `suppression_radius_m`; or
  - an impact or ricochet on the unit, or within that radius of it.

  This is the evidence suppression already scores (`damage.rs:380-500`, `near_miss` at `:667-672`), applied to every unit, hulls included. Details:
  - Own-side rounds never count; a war film would show "check fire!", not "taking fire!".
  - A squad that is already Suppressed or Pinned stays silent on B5, because B6 and B7 own that moment.
  - The shooter need not be seen.
  - Data check: only the MANPADS rows (Stinger, Igla-S, FN-6) have a zero radius. That's correct, because their threat to helicopters is B15's.

- **B15, incoming missile:** an enemy guided round counts if both:
  - its commanded point (`Guidance.point`, `flight/mod.rs:364-370`) lies within the unit's footprint (hull extents, or the squad area's radius);
  - the unit sees it: `sensing::sees_point(world, eye(unit), missile, sight::of(unit), 1.0, sensors)` (`sensing.rs:102`).

  A missile behind a ridge stays silent until it crests. A missile aimed at the neighbouring tank makes only the neighbour shout.

- **B17, friendly destroyed nearby:** when a unit is destroyed (a vehicle wrecked, or a squad's last soldier dead), only the **nearest** living friendly that sees its position shouts. It is one witness, never a chorus; this mirrors `hearing.rs:41-46`'s nearest listener. A death nobody saw gets no shout.

## Unseen enemies (user, reverses map T6)

The user found the `cue_voices` murmur sounds "like footsteps or heavy breathing" and accepted the recommendation to replace it:

- `cue_voices` is deleted.
- An **unseen** enemy's shout reaches the other side through hearing as an **anonymous** cue: listener, 8-way sector, near/far band, and nothing else.
- The client plays a random bark from a **generic pool**, never the speaker's own set, under heavy muffling (about 600 Hz low-pass plus reverb), so the words can't be made out.
- Unseen infantry that isn't shouting is silent when idle, with the existing footsteps cue when moving.
- **Agent:** using a generic pool and publishing no trigger or set keeps timbre and state from leaking. That follows the D5 fog rule and preference 3.

## Taste (user, from generated samples)

- **T1:** the reference voices are US tank **C**, Russian rifle **B**, US recon **B** and UK pilot **C**. Audio is in [`assets/reference-voices/`](assets/reference-voices/); the descriptions are in `saved-voices.json`. The same-role sets borrow their description style.
- **T2:** treatment follows where the speaker sits.

  | Speaker              | Treatment                                                                                                                                                   |
  | -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
  | Tank crew            | **light** radio (high-pass ~250 Hz, low-pass ~3.8 kHz, gentle compression, faint hiss) over a faint engine bed. "Processed, but less" than the first radio. |
  | Infantry in the open | **dry** (slapback was rejected)                                                                                                                             |
  | Recon                | quiet earpiece                                                                                                                                              |
  | Pilots               | intercom over a rotor bed                                                                                                                                   |

  References: `us_tank-radio-light.mp3`, `us_recon-earpiece-candidateA.mp3`, `uk_pilot-intercom-rotor-candidateA.mp3`.

- **T7:** one score for every faction.
- **T8:** the user is the only reviewer, so the workbench is built for fast spot checks and mark-bad → regenerate.
- **T9–T12, music:** a Pulse-family **background underscore**:
  - D minor, 125 BPM, a synth bass ostinato, drums and taiko far back, low strings, distant brass;
  - no melody, hooks, solos or drops.

  Moods:
  - **Combat** ×3 (the "Pulse" direction).
  - **Tension** ×3, mixing the "Pulse, sparse" direction (no drums) with "Pulse, dark" (sub-bass heartbeat, faint taiko).

  Level: **−26 LUFS** under the battle mix by default.
  - Rejected in round 1 as too foreground: hybrid, industrial, orchestral and rock.
  - Rejected in round 2: arpeggio, muted drums, and a separate drone tension.

  The prompt template is the `STYLE` constant in [`slices/08-music-tracks.md`](slices/08-music-tracks.md). The chosen sketches are in [`assets/reference-music/`](assets/reference-music/).

- **D8 (user):** two moods. Any own B5–B9 bark within 20 s switches to combat; 45 s without one returns to tension; the crossfade is 4 s. Music plays on a music bus that dips under voices.
  - Rejected: one playlist (B) and three moods (C).
- **T13 (user):** players get **music** and **voices** levels next to master. The user's reason: "this way we don't need to over-analyze it". The level owner is `settings.ts`, and the control is `SoundControls.tsx` in both menus.

## Structure (agent on the user's behalf unless marked; preference in brackets)

- **A1 [1]:**
  - The workbench gets a Voices tab: one set at a time, a grid of triggers × intensity × take, with flagged takes first, and a keyboard review queue.
  - It gets a Music tab with a manual mood switch.
  - It gets a 30 s chatter preview through the real `Chatter`.
  - None of these reuse `Library.tsx`.
- **A2 [3]:** `fixtures/voices.json` is the only voice authority. **Assignments live there, not on unit rows.**
  - `UnitType` rejects unknown fields (`catalog.rs:113`).
  - A row field would enter `Rules` and the contract for data the simulation never reads; the client chooses the speaker within a set.
  - Shared rows (`rifle_squad`, `assault_squad`, Javelin, Stinger, AH-64E) need a per-faction assignment anyway.
  - This supersedes D4's "each unit row gets `voice`".
- **A3 [3]:** the `Shout` contract is as in the README.
  - `kind` and `position` ride on the shout, so B16 can speak for a unit that has just died. `OwnUnit` drops dead units, and an `ObservedTargetId` can retire.
  - `ShoutRules.talk_s` is a **nominal** talk time per trigger. The simulation never reads generated clip durations, and the generation tool flags any take that runs longer than it.
- **A4 [1]:** the `Generated` provenance kind goes in `catalog.ts`, the existing owner, and voices import it.
- **A5 [1]:** the generation tool is TypeScript in `packages/battle-audio/tools/generate/`.
  - It has an injected `fetch` so it can be tested offline.
  - It caches in two levels: the generation key is the hash of (text after callsign expansion, voice_id, model, settings, retake), giving a **raw master**; the processing key is the hash of (raw sha, profile, bed, encoder), giving the **served MP3**. Tuning processing therefore re-renders locally for free.
  - It is resumable, runs at most 8 TTS calls at once (music 2), retries with backoff and jitter, and stops on 401 or quota errors.
  - It prints one progress line per take.
  - `assets.py` remains the only ffmpeg owner, with a `voice` subcommand. It gets loudness targets in place of fixed-peak normalisation and a second input for beds.
- **A6 (superseded):** voices are saved permanently; see Access and account.
- **Asset layout (agent; refined after the user said to "make sure the folder structure makes sense as a whole … don't just keep tacking on extra folders"):**
  - Generated audio follows `assets/source/menu` → `assets/runtime/menu`.
  - Raw masters go in `assets/source/audio/{voices,music}/`.
  - Served files go in `assets/runtime/audio/{voices,music}/`, with a content hash in each filename.
  - **Rejected:** a top-level `assets/generated/`, served at `/<sha>/` through a new Vite plugin and immutable cache rule. That is a new top-level kind plus serving machinery, and the bake's top-level sha namespace (`web/asset.mjs:272-279`) belongs to art.
  - **Rejected:** separate `music.json`, `voices.review.json` and `voice-packs.json`. Music joins the sound catalog that already owns `menu_music`. Verdicts and pack data live in the tool's lock. Packs sit at fixed names and are revalidated by ETag.
  - The `*.mp3` LFS rule is added in `.gitattributes`.
- **Accents per unit (agent; the user may veto in slice 01):**
  - Infantry speaks its **faction's default nationality**: US for US, German for Europe, Russian for Eastern. National teams keep their own nation: Akeron is French, the FN-6 team is Chinese.
  - Vehicles and aircraft follow the equipment's **country of origin**.
  - Where several nations built the equipment: NH90 → German, Tiger HAD → French, Tiger UHT → German, Boxer → German, CV90 family → Swedish, Centauro → Italian.
  - Preference 1. This follows the user's "German for generic European infantry" pattern and resolves a gap the drafts found.
- **Loading (agent, card L2):**
  - Voice packs load in the background **after** `BattleAudio` `ready`, and only for the sets in this battle.
  - A bark whose clip hasn't loaded is dropped.
  - Music streams through `HTMLMediaElement` → `MediaElementAudioSourceNode`, outside `WebAudioSink`.
  - Voice MP3 is mono at about 40 kbps; music is 128 kbps.
- **L9 (agent):** every take is transcribed and compared with its script, after stripping tags and normalising numbers. A dropped keyword or an overlong take is flagged. This was found when "Gunner" came out as "Gutter".
- **L10 (agent):** the menu bed moves to the music path, re-encoded to MP3. The WAV clip and the decode-whole-file path are deleted.
- **L7 (territory):** the live deployment serves real LFS media, so the Vercel project's Git LFS setting is on. Documenting it in `web/README.md` is part of slice 07.
- **Compatibility (default, user silent):** no backward compatibility and no migrations. Hard cutovers only. `ShoutRules` changes `config_digest`, so old replays are refused; they already are by build fingerprint.

## Merging the drafts (agent, 2026-10-11)

There were four drafts: A fewest-slices (Claude), B risk-first (Codex), C seam-quality (Claude), D asset-pipeline (Claude).

**Agreed by all four, so firm:**

- Shout state is per side and off `Unit`, with no RNG.
- One named re-bless.
- The library and review come before any battle playback.
- A pilot gate comes before the batch.
- `Chatter` and the voice bank are pure, separately tested modules.

**Splits and the call made:**

- **Slice count.** A had 4, C 21, D 12. Kept: 16, with 3 parallel lanes.
  - A's merged "voices and music in battle" slice hides three seams: arbitration, music streaming and simulation publication.
  - C's split of the simulation into ledger → physical barks → publish is kept, so the re-bless happens once, at the end.
- **The browser media spike** (B's slice 2: MP3 streaming under COEP in every WebGPU browser). It is folded into slice 07 as its first acceptance check, ahead of music generation, so a failure stops music early without blocking voices.
- **Where generated files go:** see "Asset layout" above.
- **Assignments:** C put them on unit rows; A and the agent chose `voices.json` (A2).
- **Where the response trigger comes from:** R11 "arrived" is taken from `OwnUnit.goal` going from `Some` to `None`, not from an ack. R12 is taken from `CommandAck.error` (`command.rs:259`).

## Closed after the plan (user, 2026-10-11)

- **Review gate (slice 06):** yes. After playing every flagged take and spot-checking, bulk-accepting a set's takes that passed speech-to-text counts as reviewed.
- **Spend (slice 04):** the user delegated it: "don't let me block you here". The pilot reports cost and then the batch proceeds. This supersedes "spending is not delegated" in the Delegation section for this feature's generation budget.

## Build log

<!-- Append: date, slice, decision, who closed it, and why. -->

**2026-10-12, slice 01 (agent; Codex implemented, orchestrator reviewed):**

- **Sets:** 56, more than the planned ~40. Every country × role that the roster actually fields gets its own set, so no accent is borrowed.
- **Role per unit:** taken from the unit row's own data (roster category, declared roles, squad slot bodies). This puts the Centauro II with tank crews, the Tiger HAD/UHT with attack pilots, the FN-6 team in a Chinese set, and squad-bodied recon rows with recon infantry.
- **Voices per set:**
  - Squads of 6 or more get 3 voices.
  - Sniper, ATGM, MANPADS and small recon teams get 2.
  - **Vehicle crews get 2:** a commander who speaks responses and a crew member who shouts barks. The spec hadn't said; one voice per crew would sound flat in battle.
- **Female voices:** 8 sets have a female response voice, covering the attack and transport pilots plus one IFV crew per faction. The user can veto any of them in the Voices tab.
- **Callsigns:** a national word plus a number ("Dragon 1", "Adler 6"), unique per set.
- **The four saved T1 voices** sit on `us_tank_crew`, `ru_rifle_squad`, `us_recon_infantry` and `uk_attack_helicopter_pilot`. The alias sets the first pass created were deleted.
- **Scripts were rewritten by hand** after review found Codex's version mechanical (`[urgent]` stuck on the steady text, duplicated tags):
  - steady lines carry no tag;
  - stressed lines carry one tag and different words;
  - about 1 in 7 stressed lines swear;
  - sniper teams whisper like recon.
- **The CLI** is `packages/battle-audio/tools/generate/cli.ts`, run as `bun run generate plan`. It totals 7,108 clips and about 181k characters.

**2026-10-12, slice 11 (agent; Codex implemented, a Claude worker wrote the tests during a Codex capacity outage, orchestrator reviewed):**

- **Splitting a side's shouts:** `shouts::build` emits a side's **own** shouts. `shouts::heard` copies an enemy's own shouts into the other side's frame as `Seen(handle)`, only while that side identifies the speaker that tick. One owner builds each unit's shouts; hearing only filters them.
- **Tie-break order** is data (`PRIORITY`): B16 > B10 > B9 > B8 > B7 > B6 > B14 > B13 > B12 > B11 > B4. A contact call speaks first and blocks that unit's other barks for its talk time.
- **Transitions:**
  - Contact fires on a **newly** identified enemy, spoken by the nearest unit that sees it.
  - Altitude decides an air contact before body does, so helicopters count as air.
  - B4 fires on a **new** target only.
  - B6 and B7 fire when the tier **rises**.
  - B12 is infantry-only, judged per weapon mount.
  - B13 fires once when a weapon runs dry.
  - Dead and withdrawn units never speak again.
  - Writing the tests found 13 bugs of exactly this kind in the first implementation.
- **`talk_s`** starts at 1.5 s for every trigger, a neutral value within the slice's 1–3 s.
- **No compatibility shim.** `ShoutRules` serializes like every other rule. The two parity scenarios that embed rules gained the `shouts` block as data. Every pinned `digest` and `publication_sha256` was compared value by value and is unchanged.
- **Tests stay in the one sim test binary** (`cargo test -p sim --test sim shouts::`). The spec's `--test shouts` wording was the orchestrator's mistake.
- **Follow-up for slice 12:** evidence compares a full copy of every unit (`previous_units`) made each tick. Replace it with a minimal per-unit snapshot holding only what barks read (alive, health, member hp, tier, mount reasons, ammo), and check the cost on the benchmark.
  **2026-10-12, slice 02 (agent; preference 1 unless noted):**
- **Command names:** the generating command is `voices` (`bun run generate voices`), as slices 03, 04 and 06 already call it; `generate generate` read badly. `beds` generates the two beds. Selection adds `--trigger` and `--intensity` beside `--set`, `--take` and `--only`, because slice 02's own human check ("`--only R2`") needed a trigger filter that `--only missing|bad|flagged` can't express. Set names are the real ids (`us_tank_crew`), not the walk's short names.
- **`process`** is `voices` without a client: it re-renders only takes already in the lock and never calls.
- **Cache by file:** a raw master is trusted by its generation-key file name, so a kill before the lock flush never pays twice. A superseded take's old raw master and served file are deleted with it (one take, one of each).
- **Speech-to-text reads the raw master,** so retuning a profile never re-transcribes. The overlong check uses the served duration.
- **Processing key hashes the profile's whole definition** (from `assets.py voice-profiles`), not only its name, so tuning a constant re-renders. `assets.py` stays the owner of those definitions.
- **Thresholds:** word error above 0.4 flags (one wrong word in a 3-word line is 0.33); talk time defaults to 3.5 s for every trigger until slice 11's `talk_s`. Keywords are the callsign plus "sabot" and "gunner".
- **Numbers:** "two-one", "two one", "twenty one" and "21" all read as 21; "fifty five" as 55.
- **Loudness:** −16 LUFS integrated for dry, radio-light and intercom; −19 for the earpiece ("quiet earpiece", and the approved earpiece reference measured −18.8). Linear gain plus a −1.5 dBFS limiter, so dynamics stay. Outputs land about 0.4 LU under target after MP3 encoding.
- **Encoder:** mono, 40 kbps, 24 kHz (at 40 kbps a 44.1 kHz stream only wastes bits on bandwidth the radio profiles cut anyway). Bitexact flags and no metadata, so a take renders alike on one ffmpeg build.
- **Intercom hiss amplitude** 0.015, as the radio's; the walk named only its band and weight.
- **Beds:** the walk's prompts, regenerated through the tool so their provenance is recorded (`eleven_text_to_sound_v2`, `loop: true`, 12 s). A bed starts at an offset and the hiss is seeded from the take's bytes, so neighbouring lines don't open on the same engine note.
- **v3 settings:** `stability: 0.5` (v3 accepts only 0, 0.5 and 1).
- **Human check, by the brief:** the four saved voices spoke R2 `move` steady and stressed, take 0, through their own set's profile only: 8 text-to-speech and 8 speech-to-text calls, 232 characters, all matched at 1.00. The other profiles on the review page are local re-renders of the same raw masters, with no credits spent.
- **Size note for slice 06:** these takes average about 12 KB (1–3 s at 40 kbps). At 7,108 clips the library would be about 85 MB, well over L2's 20 MB estimate. Slice 06 must report it before committing, as it already says.

**2026-10-12, slice 02 review (orchestrator):**

- **One owner for talk time.** Barks are held to the simulation's `ShoutRules.talk_s` in `fixtures/game.json`; the generation tool reads that file. Responses never reach the simulation, so the tool keeps one named ceiling for them, `RESPONSE_TALK_S` = 4 s. The tool's interim 3.5 s default is deleted.
- **`talk_s` raised from 1.5 s to 2.5 s for every bark.** The tested takes run about 2–2.7 s, and at 1.5 s the simulation would start a unit's next shout while its last one is still playing. Digests are unaffected: shouts aren't digested, and the parity scenarios embed their own copy of the rules.
- **Library size estimate: about 85 MB, not 20 MB.** At 40 kbps a take averages about 12 KB. That is acceptable: it lives in LFS, and a battle loads only its own sets, in the background after `ready`. Slice 06 reports the real figure before committing the batch.

**2026-10-12, slice 03 (agent; a Claude worker implemented during the Codex capacity outage, orchestrator reviewed):**

- **Verdicts:**
  - A verdict carries the sha256 of the bytes the reviewer heard. `generate verdict` refuses any take whose bytes have changed since. One function, `verdictOf`, owns "a verdict holds only while the sha matches".
  - The server runs every generation-tool command one at a time, so `voices.lock.json` keeps one writer.
  - **Accept unflagged in set** is a single tool run.
- **Regeneration:** `generate voices` gains `--voice`, so R regenerates exactly one take and never pays for a set's other missing takes.
- **Snapshots** send a short per-take summary, and the server alone computes counts. The snapshot revision hashes only `voices.json`, so a verdict never makes a pending edit stale.
- **Review flow:**
  - Opening the tab is silent; only a key or a click plays audio.
  - The queue is fixed when a set opens.
  - A, X and R advance to the next take and play it.
  - Line edits and R save without a review diff, to keep the keyboard flow fast.
- **Not built:** a description editor per voice (the server supports it; there is no UI yet). Regenerating a description is slice 05's design loop.

**2026-10-12, beds (user):**

- **The helicopter bed is the game's own `rotor-uh60` loop,** the recording every helicopter already plays in battle. It is US Army UH-60 B-roll, public domain. For the intercom it is low-passed to 900 Hz ("cockpit-muffled"; the user chose option B in an A/B). It replaces the ElevenLabs-generated rotor bed.
- **Rule:** a bed comes from the game's existing sound for that machine, so a sound has one owner across the game. The generation tool must take beds from catalog clips, not from `/v1/sound-generation`.
- **Tank bed:** the game has no recorded engine (engines are synthesised; only the track rattle is recorded), so the tank bed stays generated until the user hears an alternative.
- **Implementation is due before slice 05:** the generation tool's `beds` command, the `voice-intercom` profile and re-processing of the existing takes.
**2026-10-12, slice 07 (agent; Codex implemented in part before its capacity outage, a Claude worker finished the review fixes, orchestrator reviewed):**
- **Music is not a mixer bus.** It streams through `MusicPlayer`, which feeds the same master as the rest of the mix: element → track gain → level (`buses.music` × the music setting) → duck → master. The mixer's `Bus` list no longer has `music`, because a bus nothing plays on was dead structure. The voices bus keeps its distance-reverb send: barks are positional battle sounds (slices 14/15), and slice 07's "neither bus sends to the reverb" was wrong for voices.
- **Level and ducking are two separate nodes,** so neither overwrites the other.
- **Loudness:** the menu bed's track gain is 0.16, which matches the old WAV mix at −29.8 LUFS, within 0.2 dB. Codex's first version would have been about 17 dB louder.
- **Catalog:** a music track names its `sources` record. `musicUrl()` alone builds `/audio/music/<track>.<sha8>.mp3`. The `music` section is required, and moods must be exactly menu/tension/combat. The old WAV clip, its recipe, the file and the `"music"` processing profile are deleted, together with the test that pinned that profile.
- **Streaming:** checked in real Chrome inside the app (ranged 206 responses, playing) and in WebKit through the same element → Web Audio graph. This Playwright version can't launch its Firefox build.
- **The `street` scene fails on clean `main` too** (checked in a separate worktree on 2026-10-12). That failure is already on `main` and is unrelated to this feature; it is left for its owner, and the closeout's full run will report it.

**2026-10-12, 02b beds (agent):**

- **One bed shape for both sources.** A profile's bed in `VOICE_PROFILES` is `{clip | generated, filter, volume}`: `clip` names a `sounds.json` clip, mixed from its prepared runtime file; `generated` names a lock bed. One resolver in the tool turns either into a file and its sha256, and the processing key hashes that sha. There is no second path for the tank. Its filter is `anull`, so its takes re-rendered to identical bytes.
- **The catalog's recorded sha is the clip's identity.** A file that differs from it refuses to render ("pull Git LFS"), and so does a missing clip id.
- **`check` reports a lock bed no profile names,** so a superseded generated master can't linger.

**2026-10-12, slice 04 pilot (orchestrator ran it):**
- **Scope:** `us_tank_crew` lead voice, all 12 responses × steady/stressed × 2 takes = 48 takes. The set's bark voice (`soldier`) has no designed voice until slice 05, so the pilot measured the lead voice only. That is enough to price the batch: cost scales with characters.
- **Cost and speed:** 1,284 characters and 92 calls (48 speech, 44 transcription; 4 transcripts were already cached), in **20 s** wall time at 8 concurrent calls. The whole library is about 181k characters and 7,100 takes, so it projects to roughly an hour of generation and about 181k characters of text-to-speech credit (plus transcription), well within the Pro plan.
- **Quality:** 3 of 48 takes were flagged (6.25%, under the 10% stop line).
  - "Gunner" misheard as "Gutter" and "sabot" as "Sabo": one retake each fixed both.
  - "Tank here, send it." came out as "Thank you" twice. It was reworded to "Armour here, send it."
  - After that: **0 flagged, 0% mismatch**.
- **Lesson for the batch:** the R/retake loop handles one-off misreads. A line misread twice gets reworded instead. No gate needed; the batch proceeds (user: "don't let me block you here").

**2026-10-12, slice 12 (agent):**
- **Hulls hear near misses (territory).** Flight's near-miss sweep measures every body, hulls included, out to the round's `suppression_radius_m`; only suppression drops hulls. `hulls_hear_near_misses_and_a_sabot_passing_a_tank_is_under_fire` pins it. B5 therefore reads flight's `NearMiss` for every unit, plus each impact and ricochet within the radius of the hull box or the nearest living soldier, scored by suppression's own `near_miss`. No ShoutRules floor was added: a tank under rifle fire does shout, as the slice asks.
- **"Already suppressed"** is the squad's tier after this tick's rounds, so a squad whose tier rises this tick says B6 or B7, never B5.
- **Tie-break order:** B16 > B15 > B10 > B9 > B8 > B7 > B6 > B17 > B5 > B14 > B13 > B12 > B11 > B4. An incoming missile is the most urgent thing a living unit can say; taking fire is the least specific stress bark.
- **B15's footprint** is the hull's extents seen from above (a supported missile steers at the target's last sighting plus aim height, a released one at a ground point), or the squad's footprint radius.
- **B17's witness looks at where the dead unit's crew or soldiers stood** (`sensing::eye`), not the bare ground point: a sight line ending on the terrain is refused, so "sees its position" would never hold. Nearest by ground distance, ties to the lower id.
- **The unseen cue** carries as far as the speaker's own `loudness_m`. A shout spoken on a bucket tick is heard at the next bucket. A destroyed speaker's last shout makes no cue, since hearing hears living sources only.
- **The publication does not change in this slice.** `SoundCategory::Shout` cues ride in `ObservationFrame.audible`, but the encoder leaves them out (`published_audible` in `publication.rs`) until slice 13 adds `Shout` to the layout's sound categories and deletes that filter. None of the five pinned publication streams contains a shout cue, so slice 13's re-bless gets no audible bytes from them.
- **Snapshot:** `shouts::Before` keeps alive, hull hp, each soldier's hp, the tier and each mount's reason, shot count and guided missile, refilled in place each tick. The endurance soak (5 simulated minutes) went from 0.970 to 0.115 M instructions per tick in the phase that held the copy, and 781.2 to 774.3 G step instructions overall; the digest is unchanged.
- **Tests for two table rows read the moment physically rather than literally.** "Behind a ridge": a guided missile needs a clear line from its launcher to the target, so terrain that hides the missile also hides the target; the test hides it beyond the tank's rear sight instead. The rule is the same `sees_point`. "The unit's own missile": a blue squad stands beside the red tank its own launcher fires at, so the missile steers onto the squad's area, and the launcher is checked too.

**2026-10-12, slice 13 (agent; the re-bless commit is recorded when it lands):**
- **Row layout:** `shouts` rows are `own, idLo, idHi, kind, x, y, z, trigger, intensity`. `own` is 1 for the side's own unit and 0 for an identified enemy, as crashes, corpses and projectiles mark it, rather than a new speaker table. The id is the unit id or the identification handle, in limbs. `shoutCount` ends the header and the group follows `crashes`: the newest group goes last.
- **Tag tables:** the layout publishes `shoutTriggers` (the contract's serde names, `b1_contact_infantry` … `b17_friendly_destroyed_nearby`, in B order) and `intensities` (`steady`, `stressed`). `soundCategories` gains `shout`, and `published_audible` is deleted.
- **One name mapping:** `barkTrigger` in `voiceCatalog.ts` turns a simulation tag into its voice name and refuses a tag whose number and name disagree. The generation tool's talk-time keys use it in place of its own regex. The contract test reads the layout from `codec-vectors.json`, whose bark, intensity and sound-category tables the encoder's codec test holds equal to the writer's.
- **The re-bless:** `BLESS_PARITY=1` on `publication::` only. In the five streams every changed value is a `publication_sha256` (1,896 of them); every `digest`, `initial_digest` and `fog_sha256` is unchanged, compared value by value. `codec-vectors.json` changed its layout and bits, and its base frame gained two shouts (own and seen) and a shout cue. The combat stream carries three barks, one of them a seen enemy's; no stream carries a shout cue.
- **Native captures re-encoded, not re-captured:** the Godot fixture captures (`market-town`, `paris-corner`) hold the old layout, which the reader refuses. Each record gained `shoutCount` 0 and the empty shouts delivery the writer emits (length 0, snapshot, payload 0), its expected observations gained `shouts: []`, and paris-corner's `captureHash` was recomputed. Their sample digests are untouched.

**2026-10-12, slice 13 verification (orchestrator):** the re-bless proof parsed every leaf of the six publication records. Only `publication_sha256` values changed (1,896 of them). All 1,901 `digest`/`initial_digest` values and every `fog_sha256` are identical. The re-encoded native Godot captures pass the Rust reader test and the Godot `presentation_capture`, `frame_stepper` and `visual_manifest` tests. `parity_compose_test` fails with or without this slice's native changes (two layer-blocker cases), so that failure was there before this slice.
**2026-10-12, slice 05 (a Claude worker built it; the user approved the picks):**
- **117 voices designed,** 3 candidates each, with 0 failures: 117 design calls and 351 transcriptions in 376 s. The mean speech-to-text match was 0.956. The Chinese sets scored lowest, most likely because of the accent.
- **Descriptions:** the designer replaced 109 placeholder descriptions with full ones in the T1 style (nation, gender, role, age, timbre, delivery, accent). The user saw them on the candidates page and approved the outcome.
- **Picks:** `generate pick --auto` chose, for each voice, the candidate with the best match on its audition text, tie-broken by duration closest to the role's median. The user listened and said "your auto picked ones are good". All 121 voices now have a saved `voice_id`; any can be re-picked later with `generate pick --replace`.
- **Keep everything (user, standing rule):** every generated or extracted asset is kept, unpicked candidates and superseded takes included ("we have more than enough space … we could use them as additional voice samples", as with the unused weapon recordings).
  - The tool no longer deletes superseded raw masters, design candidates or beds.
  - Served files of a superseded take are still removed: they are re-renderable derivations that would otherwise ship with every deploy.
  - The generated helicopter bed that 02b deleted is restored, unused, under `assets/source/audio/voices/beds/`.
  - The pilot's three superseded raw masters were deleted before this rule and were never committed, so they are gone.
**2026-10-12, slice 08 (agent; preference 1 unless noted):**

- **Generated sources join the catalog's `sources`.** A track's source is `Generated` provenance plus `length_s`, `path` (under `assets/source/audio/music/`) and `sha256`; a recording keeps its licence receipt. Validation refuses a clip cropped from a generated source. The `music` section's types and `musicUrl()` moved to `src/music/musicCatalog.ts`, so the Bun-run tool can name a served file without loading the synthesised bank.
- **Track ids:** `combat_1..3`, `tension_sparse_1..2`, `tension_dark_1`; each source has its track's id. Length 150 s for all six.
- **Prompt variations (delegated):** combat_1 keeps the ostinato and strings in front at one flat intensity; combat_2 lets the taiko and drums lead (still far back) with a slow build to a plateau; combat_3 is darker, with a syncopated ostinato and gentle swells every 30 s. The two sparse tracks differ by stillness versus unease; the dark track uses the table's prompt as written.
- **Caching:** a raw master holds while its prompt, model and length are unchanged and its bytes match, as the beds do. `--track` buys that track again and deletes its old raw and served files. A kill between receiving and recording a master would pay for that track again; at six tracks that is accepted.
- **−26 LUFS is the track's integrated loudness after its gain,** on the same scale the menu bed was matched on (−29.8). The gain is computed from the served file, measured by `assets.py music --input/--output`, the one music encoder.
- **The tool writes `sounds.json` through oxfmt,** escaping non-ASCII as `assets.py build` does, so adding a track doesn't reflow the file. The first write also normalised about 40 lines of older layout drift.
- **`assets.py build` decodes only the sources a clip crops.** Music masters stream and are never cut.
- **`presentation.audio.music`** became `{ duck: {depth, attack_s, release_s}, combat_window_s: 20, calm_s: 45, crossfade_s: 4, combat_triggers: [B5–B9] }`. Nothing read the duck numbers yet.
- **Director semantics:** combat starts when the latest qualifying bark is at most 20 s old when the mood is asked; it ends when the latest is over 45 s old. Shuffle is a bag per mood; a new bag never starts with the track just played.
- **Workbench:** voices and music share one generation queue (`apps/sound-workbench/generation.ts`), so the tool's files keep one writer. The Music tab's mood switch drives `MusicDirector` on a clock the tab owns: Combat notes a B5 bark; Tension skips the calm period. Tracks play at their catalog gain, as in battle.
- **Real spend:** 6 `music_v1` calls, 900 s of music, no retries needed.

- **2026-10-12 (orchestrator):** generating a track again keeps the old raw master (the user's keep-everything rule) and drops only the old served file. The workbench shares one generation-tool queue for voices, design, pick and music, so `voices.lock.json` and `sounds.json` each keep a single writer. The user's listen to the six tracks is non-blocking. No track plays in a battle before slice 14, and every track can be regenerated from the Music tab.

**2026-10-12, slice 09 (a Claude worker built it; the orchestrator reviewed):**
- **`Chatter`** (`packages/battle-audio/src/voices/chatter.ts`) is the one place line rules live:
  - at most 4 lines play at once;
  - responses come before own barks, which come before a seen enemy's, nearest the camera first within each;
  - a line is dropped after waiting 1 s;
  - a speaker never overlaps itself;
  - an order to a group gets one answer (requests within 250 ms collapse).
  
  The numbers are in `presentation.audio.voices`. Two choices the spec left open: a request with no position ranks as nearest, since it's the owner's radio; a line whose take isn't loaded is dropped at once.
- **Takes and voices:**
  - The take is drawn with `mulberry32` from the `math` package, keyed by speaker, trigger, intensity and request count.
  - A bark's voice comes from a stable hash of the speaker, so one soldier always has the same voice.
- **Pack format** (`voiceCatalog.ts`): `VoicePack { set, voices[], takes[] }` at `/audio/voices/<set>/pack.json`, built only by `voicePack()`. Slice 06's `generate pack` calls it with accepted takes only.
- **`VoiceBank`** loads packs and MP3s by URL in the background and never blocks battle start. A failed set stays silent.
- **The chatter preview** in the workbench runs a scripted firefight through the real `Chatter` with a fixed seed. Its lane view marks lines heard, collapsed and missing.

**2026-10-12, sound decisions move to Rust (territory: the native-parity session, which works for the same user):**
- The native-rendering-parity work (branch `phase5f`, slice 25 step 7) moves the battle's sound decisions from TypeScript into `crates/presentation/src/sound/`. Both clients read Rust's decisions; native plays them with `kira`.
- Project rule: Rust as smart as possible, clients as dumb as possible. So:
  - Chatter's arbitration moves to Rust as well (new slice 09b);
  - slice 14's bark decisions and slice 15's unseen shout cue are built in Rust;
  - TypeScript keeps playback, the voice assets and the generation tooling.
- The ports wait for `phase5f` to land on `main`. Until then, no new decision logic goes into `battleAudio.ts` or `audioPresentation.ts`.

**2026-10-12, slice 10 (agent; a Claude worker built it, uncommitted for the orchestrator's review):**
- **The seam:** `responseFor(event)` in `voices/responses.ts` maps `selected`, `accepted` (an order and its ack `seq`), `refused` (`OrderError.reason`) and `arrived` to a positionless `tier: "response"` request. `BattleAudio.respond` forwards it to the same `Chatter` and `VoiceBank` the workbench uses (`voices/battleVoices.ts` composes them and plays a line dry on the voices bus). `useUnitControl` gained `onAck` and `onSelect` beside `onIssue`; `apps/battle-lab/src/unitResponses.ts` turns them, and each publication, into events.
- **Who answers:** the clicked unit, else the highest value (ties to the lower id). The unit that answered the selection answers the group's later orders while it is in them. Group keys: `order/<seq>`, `selected` (rapid reselections collapse), `arrived/<journey>`.
- **R12 list:** `out_of_bounds`, `no_valid_destination`, `unknown_target`, `air_contact`, `not_infantry`, `not_a_building`, `capacity_full`, `one_squad_per_building`, `building_occupied`. Purchase, rate, sequence, replay and stale-id refusals get no answer.
- **R11:** the end of a journey begun by an accepted move, attack-move or building order, or by a purchased unit entering under way; reported once per journey by its first unit to arrive, and only after at least 3 s under way (`ARRIVED_AFTER_S`). Stop, attack (it may chase), withdraw and leave-building end a journey silently; a queued order leaves the current one running.
- **Sets loaded:** the assignment of every own unit kind and pending purchase, for the player's faction (in a non-skirmish lab, the row's own faction, else its first roster faction). Each set loads once, after `ready`.
- **Level:** `LINE_GAIN` 0.5 on the takes' loudness calibration. Kept as a constant rather than in `presentation.audio.voices`, because `audioPresentation.ts` was being moved into Rust at the time.
- **`generate pack`:** admits a take judged ok on its served sha whose served file still hashes to it; `--unreviewed` (default off) also admits unjudged, unflagged takes. A set that admits none has no pack. It was not run against the real lock while the batch was writing it.

**2026-10-12, slice 15 (agent; a Claude worker built it, uncommitted for the orchestrator's review):**
- **The shout cue is its own row, `presentation.audio.cues.shout`:** `{ gain: 1, lowpass_hz: 600, wet: 0.6, takes_per_voice: 4 }`, not a `cues.sounds` entry, because a pool is not a recipe. `Sound::note` (`sound/frame.rs`) plays a `shout` cue as one non-positional line on the **voices** bus (so the voices slider covers it), panned by the cue's sector, at `gain` × its band's gain, through the 600 Hz low-pass and a 0.6 reverb send in **both** bands (near is louder, never clearer). The repeat rule (`repeat_s` per category, sector and band) is the other cues'.
- **The take:** Mulberry32 seeded by the cue's key and its time (`shout/<sector>/<band>@<t0>`), indexing the pool. The cue carries no identity, so neither does the choice.
- **The data path:** TypeScript hands Rust the loaded pool and nothing else: `SoundFrame.shouts([{sound, duration}])` (wasm) → `Sound::set_shouts`. A pool take's `sound` is its voice-bank key (`set/voice/trigger/intensity/take`); the sink (`WebAudioSink.addTakes`) plays it by that name, calibrated by `sourceNormalization` as every take is. Rust never learns factions: a sound frame is one side's, so "the listener's enemy's pool" is the only pool it holds. An empty pool (no packs yet, or a lab with no known enemy faction) makes a shout silent. The pool survives a reset, as the prepared sounds do.
- **The pool (delegated size):** `factionSets` (`voiceCatalog.ts`) lists every set the enemy faction's roster fields; `ShoutPool` (`voices/shoutPool.ts`) loads, from each pack that exists, up to `takes_per_voice` (4) stressed barks of each voice that shouts, every trigger's first take before any second. Words are unintelligible, so variety of voices matters more than of lines; four per voice keeps a faction's pool around a hundred small takes. `VoiceBank` gained a take selector and `loaded()` for this; it is the one loader.
- **Outside a skirmish** the enemy faction is unknown and shouts are silent (labs are developer arenas).
- **`cue_voices` is deleted:** its synth (`synth.rs`), its `sounds.json` recipe, the `default` and `infantry` cue rows, its name in `synthSounds.gen.ts`, and its length in the recorded `sound-vectors.json` durations (extra, never played: every command still equals the browser's). `cues.sounds` no longer needs a `default`: a category with no row is silent, so idle unseen infantry is silent and moving infantry keeps `cue_footsteps`.
**2026-10-12, slice 09b (a Claude worker built it, uncommitted for the orchestrator's review):**
- **Vectors first:** before its rules were deleted, the TypeScript `Chatter` wrote `crates/presentation/tests/fixtures/chatter-vectors.json`: 11 cases (the 4-line cap, tier then distance, no position as nearest with the at/arrival tie-breaks, the 1 s late drop, missing takes and a take loading later, self-overlap and a reset, group collapse and its window, seeded takes for seeds 7 and 8 with an `all` voice, and the workbench preview's 33 s script). `tests/voices.rs` replays them; 255 lines and 17 drops match exactly, including the take. The writer was a one-off test, deleted with the rules.
- **The take choice is the browser's exactly:** Mulberry32 over FNV-1a of `seed/speaker/trigger/intensity/count`, as `crate::seed` already defines it, and the `math` package's `choice`. No vector was regenerated.
- **Contract:** `presentation::sound::voices::Chatter` takes a set's voices (`set_voices`), each loaded take with its length (`loaded`), requests (`request(request, now)`), and arbitrates (`update(now, listener)`) into `Lines { commands, started, dropped }`. A started line is a `Command::Start` on the `voices` bus: `sound` is `set/voice/trigger/intensity`, `variant` the take, `gain` `LINE_GAIN` (0.5, moved from TypeScript), dry (low-pass at 20 kHz, no reverb, no position or pan). Its ids are the chatter's own, apart from the effect frame's. `presentation.audio.voices` is now typed and checked in Rust (`ChatterRules`).
- **Listener:** a battle's lines are now heard from Rust's own listener. The wasm `Chatter.hear(soundFrame, now)` reads where the sound frame last heard (`Sound::listener`); the browser passed the world origin before. The preview and the lab still pass their scripted listener.
- **Browser:** `chatter.ts` is the binding only (types and JSON to the wasm `Chatter`). `VoiceBank` tells a `BankListener` (the chatter) each pack's voices and each decoded take, and plays a start command's take by `line(sound, variant)`; its `voices()` and `get()` are gone. `BattleVoices` carries out the start commands at the command's gain times the take's calibration.
- **Left as it was:** the presentation wasm's `serde_json` lacks `float_roundtrip`, so a request's moment can read one ulp off the browser's in the page. It can only move a tie exactly on a window's edge.
**2026-10-12, slice 06 (agent; the orchestrator's brief set the cleanup steps):**
- **Batch:** 7,054 takes in one background run: 14,107 calls, 179,634 characters, 923 flagged (13.0% of the 7,107 locked takes), 1 failed. The branch was fast-forwarded to `main` first, because `generate pack` landed there with slice 10.
- **The failed take** (`se_tank_crew/soldier/opens_fire/steady/0`, "Firing.", under half a second): the shared tank-crew line became "Firing on target." All 8 tank crews' take was bought again; all 8 passed (16 calls, 136 characters).
- **Four voices redesigned** (over 30% flagged, not whispering roles). Their descriptions gained "Clear diction, easy to understand over a radio; moderate accent." Each picked the candidate with the best audition match, tie-broken by duration nearest the role's median. All their takes were bought again on the new voice; every lock row carries the new `voice_id`.

  | Voice | Old id | New id | Flagged before → after redesign → after retake |
  |---|---|---|---|
  | `cn_transport_helicopter_pilot/soldier` | `3Myj6lhgB3MjsL0d3KLD` | `y2LxASTl8wxruxqnBuVT` | 53/68 → 10 → 6 |
  | `ru_attack_helicopter_pilot/lead` | `M3jLQzbRL3dxSEKrz5Nm` | `IvsGJqum4QOI1snNfnfF` | 19/48 → 12 → 9 |
  | `fr_transport_helicopter_pilot/soldier` | `HuCs4o9Kxff7jh9K8Cfj` | `MuBGU3pWWHU0A1yvXJmG` | 23/68 → 25 → 14 |
  | `us_transport_helicopter_pilot/soldier` | `U9G51hy7lBz0hGSSbX1k` | `DX6QGC61tc7j0IOiRgv9` | 21/68 → 8 → 3 |

  4 design calls, 12 transcriptions, 4 saves; regeneration took 504 calls and 6,204 characters.
- **One retake per flagged take:** all 862 then-flagged takes had their `retakes` count bumped once: 1,723 calls, 20,808 characters. 389 passed. One retake failed, again under half a second (`ru_rifle_squad/lead/stop/steady/0`, "Halt."); its old flagged take stays.
- **Flags:** 923 before, 474 after (6.7% of 7,108). Only `fr_recon_vehicle_crew/soldier` is still over 25% (19/68, 28%). Many of the remaining flags are the matcher's, not the voice's: "treeline" heard as "tree line", "Armour" as "Armor", "God damn it" as "goddammit", "stand by" as "standby". 98 of the retakes were flagged only for being overlong.
- **Review gate:** verdict ok, bound to the served sha, for all 6,634 unflagged takes; the 474 flagged takes have no verdict and wait for the user's listen in the Voices tab.
- **Packs:** 56 packs, 6,634 takes, accepted only. `generate check`: 0 problems. `check` doesn't read packs, so a separate script proved that every packed take's file exists, hashes to its lock sha and carries an ok verdict on that sha (0 problems).
- **Spend for the whole slice:** 16,370 calls and 206,782 text-to-speech characters, plus the 4 design auditions.
- **Size:** the served library is 61.8 MB (7,108 MP3s and 56 packs; the packed takes alone are 55.7 MB). The raw library is 306.3 MB (8,595 files, superseded masters and design candidates included under the keep-everything rule). The slice-02 estimate was about 85 MB, and L2's original guess was 20 MB.
