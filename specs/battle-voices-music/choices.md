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
  - **The leak is accepted by name:** a seen enemy's *stressed* take or a pinned bark reveals its state. The comment on `IdentifiedUnit` at `observation.rs:14-15` ("never health…") is amended in slice 13 to name this audible exception.
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

  | Speaker | Treatment |
  |---|---|
  | Tank crew | **light** radio (high-pass ~250 Hz, low-pass ~3.8 kHz, gentle compression, faint hiss) over a faint engine bed. "Processed, but less" than the first radio. |
  | Infantry in the open | **dry** (slapback was rejected) |
  | Recon | quiet earpiece |
  | Pilots | intercom over a rotor bed |

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
