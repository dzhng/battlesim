# Battle voices and music

## Next Agent Prompt

**Status (2026-10-12):** slice 01 built (voice data, 56 sets, hand-written scripts). Slices 07 and 11 are in fix passes in their own worktrees. Next lane-A pickup: slice 02. The discovery walk with the user and four independent drafts (fewest-slices, risk-first on Codex, seam-quality, asset-pipeline) produced this plan. Every entry in [`choices.md`](choices.md) is a given. It records who closed each decision and why.

**Pick up here:**
- Start [slice 01](slices/01-voice-data.md).
- Slices [07](slices/07-music-player.md) and [11](slices/11-shout-ledger.md) share no files with 01–06, so they can run in parallel lanes. Give each its own worktree.
- Read `choices.md` first. Then read the slice file. Then read the owners each slice names.

**Blockers and warnings:**
- **Spending doesn't wait for the user.** Slice 04's pilot reports real credit use, and then the batch proceeds (the user: "don't let me block you here"). Stop only on a quota or 401 error, or a quality problem.
- **The proxy is undocumented.** ElevenLabs is reached only through Duet's proxy, `POST https://ctl.duet.so/v1/services/elevenlabs/<path>?workspace=<slug>`, with the Duet bearer token from `~/.duet/config.json`. It is not in Duet's public reference. The `voices_read`, `models_read` and `user_read` permissions are denied, so credit use can only be read in the dashboard.
- **The battle digest must never change.** A digest change is a bug. Publication parity is re-blessed exactly once, in slice 13, by name.

**Global TODO:**
- [x] 01 — Voice data: the `voices.json` schema, the `Generated` provenance kind, the set table, role scripts, assignments for every roster unit, and the `plan` report. [slice](slices/01-voice-data.md)
- [ ] 02 — Generation tool and processing: the proxy client, cache, lock, speech-to-text check, voice profiles in `assets.py`, and mono MP3 under `assets/runtime/audio/voices/`. [slice](slices/02-generator-and-processing.md)
- [ ] 03 — Voices tab: the workbench review grid, the keyboard review queue, verdicts, and regenerating a take. [slice](slices/03-voices-tab.md)
- [ ] 04 — Pilot: `us_tank` with all 29 triggers, a credit and mismatch report, then straight on to the batch. [slice](slices/04-pilot.md)
- [ ] 05 — Voice design: 3 candidates per voice, the user's pick saved permanently. [slice](slices/05-voice-design.md)
- [ ] 06 — Full batch and packs: background generation, the user's review, and per-set packs. [slice](slices/06-full-batch.md)
- [ ] 07 — Music player: the `music` and `voices` buses, streaming, the settings sliders, the `music` catalog section, and moving the menu bed over. [slice](slices/07-music-player.md)
- [ ] 08 — Music tracks: 3 combat and 3 tension tracks, the Music tab, and the mood director. [slice](slices/08-music-tracks.md)
- [ ] 09 — Chatter: line arbitration, the per-set voice bank, and the 30 s chatter preview. [slice](slices/09-chatter.md)
- [ ] 10 — Responses in battle: R1–R12. [slice](slices/10-responses.md)
- [x] 11 — Shout ledger: simulation barks from existing state, not yet published. [slice](slices/11-shout-ledger.md)
- [ ] 12 — Physical barks: B5 (under fire), B15 (incoming missile), B17 (friendly destroyed nearby), plus the anonymous unseen-shout hearing cue. [slice](slices/12-physical-barks.md)
- [ ] 13 — Publish shouts: the publication group and the single named re-bless. [slice](slices/13-publish-shouts.md)
- [ ] 14 — Barks in battle: barks heard near the camera, and the music mood and ducking driven by own barks. [slice](slices/14-barks-in-battle.md)
- [ ] 15 — Unseen shouts (client): the muffled generic pool; `cue_voices` deleted. [slice](slices/15-unseen-shouts.md)
- [ ] 16 — Closeout: the full check and verify runs, docs, and close-spec. [slice](slices/16-closeout.md)

**Before you end a pass,** update this section: status, pickup point, ticked items with commits, and blockers. Append any decision the plan didn't make to `choices.md` → Build log, attributed.

## Goal

Units talk like in an RTS. When selected or given an order, a unit answers its owner. In battle, units shout what is happening to them, and anyone near the camera hears it: own units, allies, and seen enemies. Unseen enemies are heard as distant, muffled shouting from a direction. Behind it all plays a background score in the style of Broken Arrow, which moves between tension and combat. Every line and track is generated with ElevenLabs, and the user reviews it in the sound workbench **before anything plays in battle**.

Non-goals:
- languages other than English;
- per-faction music;
- per-unit-type voices (the plan uses one voice set per country × role);
- premade or cloned voices;
- backward compatibility or data migrations (hard cutovers).

## Contracts

| Contract | Shape | Owner |
|---|---|---|
| Shout | `Shout { speaker: ShoutSpeaker::{Own(UnitId), Seen(ObservedTargetId)}, kind: TypeIndex, position: [f64;3], trigger: ShoutTrigger, intensity: Intensity::{Steady, Stressed} }` in `ObservationFrame.shouts`. Carrying `kind` and `position` lets a unit that has just died speak its last line (B16). | `crates/contract/src/observation.rs` |
| Bark names | `ShoutTrigger`: the 17 barks B1–B17, snake_case. The publication layout exports the tags, so TypeScript never hard-codes them. | contract crate |
| Response names | `RESPONSE_TRIGGERS`: R1–R12, client-only. | `packages/battle-audio/src/voices/` |
| Shout rules | `ShoutRules { cooldown_s: 8, talk_s per trigger }` in the simulation's rules (`fixtures/game.json`) | contract + sim |
| Voice data | `fixtures/voices.json`, authored by people and by the workbench's save path: `roles` (scripts), `sets` (country, role, callsign, processing, voices with saved `voice_id`), `assignments` (unit type → set, per faction for shared rows), `retakes`. | battle-audio validator |
| Voice ledger | `fixtures/voices.lock.json`, written only by the generation tool. Per take: input hash, raw and served sha, duration, transcript, match score, provenance, and **verdict** (`ok`/`bad`, bound to the sha it judged). It also holds design candidates. The workbench records verdicts *through the tool*, so the file has one writer. | generation tool |
| Packs | `assets/runtime/audio/voices/<set>/pack.json` lists a set's accepted takes. It has a fixed name, so it is revalidated by ETag; there is no pack index. | generation tool `pack` |
| Music data | a `music` section in `fixtures/sounds.json`, the sound catalog that already owns `menu_music`: `moods { menu, tension, combat } → track ids`; `tracks { id → source, sha256, gain }`. | `catalog.ts` |
| Provenance | `ThirdParty{label, author, license, url, path, sha256} \| Generated{generator, model, text\|prompt, voice_id?, voice_description?, date, plan:"Duet ElevenLabs"}`. | `catalog.ts`, which already owns provenance; it gains the `Generated` kind, and voices import it |
| Asset layout | Generated audio follows the menu-art precedent (`assets/source/menu` → `assets/runtime/menu`). **Raw masters:** `assets/source/audio/{voices,music}/…`. **Served:** `assets/runtime/audio/voices/<set>/<voice>-<trigger>-<intensity>-<take>.<hash8>.mp3` and `assets/runtime/audio/music/<track>.<hash8>.mp3`. Both are under existing LFS rules and inside Vite's `publicDir`. A content hash in the name means a regenerated take never collides with a cached one. No new top-level folder, Vite plugin or cache header. | `assets.py` writes; `voiceUrl()` reads |
| Levels | `SoundSettings { muted, volume, music, voices }` | `packages/battle-audio/src/settings.ts`; one control, `SoundControls.tsx` |

## Single owners: the design as if written today

- **The simulation alone decides who shouts what.** The client decides only the take, the priority and whether a line plays. No AI reads `frame.shouts`; a test pins this.
- **Shout state lives in a per-side `ShoutLedger`** beside `observations`. It is never on `Unit`, never in the digest, and never drawn from simulation RNG.
- **`voices.json` is the only voice authority.** `sounds.json` never lists a voice clip, and unit rows carry no voice field.
- **One writer per file.** People and the workbench save path write `voices.json`. The generation tool writes `voices.lock.json`, the packs and every audio file. A verdict given in the workbench is recorded by the tool. Credits are spent only through the generation tool. The browser and the workbench UI never call ElevenLabs; the workbench server runs the tool.
- **No new places.** Two fixture files (`voices.json`, `voices.lock.json`). Audio goes under the existing `assets/source/audio/` and `assets/runtime/audio/`. Music lives in the existing sound catalog. Anything a slice wants to add beyond these needs a recorded reason in `choices.md`.
- **`assets.py` is the only owner of ffmpeg and encoding.** The generator calls it.
- **`Chatter` is the only place line limits are enforced.** Battle playback and the workbench preview use the same `Chatter`.
- **`MusicPlayer` is the only music path,** menu included. The decode-whole-file path for music is deleted in slice 07.
- **Hearing stays anonymous.** An unseen shout carries only listener, sector and band. No set, trigger or position reaches the client.
- **One provenance owner and one levels owner.**
- **Short-lived seam, named:** between slices 13 and 15, a published `Shout` hearing cue has no client mapping, so it falls back to the `default` cue sound (the murmur). Slice 15 removes the fallback, the murmur and its rows. Nothing else is transitional.

## Firewalls found in the code

These were all read; the file and line references are from 2026-10-11.

**Data and digests**
- `Unit::digest` folds the unit's complete state (`crates/sim/src/units.rs:565`), while observations are not digested (`battle.rs:280`). That is why shout state belongs in the ledger.
- `combat_rng` is digested and drives the menu reel (`battle.rs:3766`, `crates/sim/tests/menu_reel.rs`). Shouts never draw from it.
- `fired` is digested (`battle.rs:3708`). Shouts read it and never write it.
- `config_digest` hashes the whole of `Rules` (`battle.rs:517-519`). `ShoutRules` will change it, so old replays are refused. They are already refused by build fingerprint (`battle.rs:859-866`), so this costs nothing new; it is a named decision.
- `UnitType` rejects unknown fields (`crates/contract/src/catalog.rs:113`). That is one reason assignments live in `voices.json`.

**Simulation inputs**
- The packed publication is positional and its reader refuses any mismatch (`publication.rs:415-470`; `publication/reader.rs:286`). Use the `audible` group as the template (`:234`, `:451`). Integers travel as 16-bit limbs (`:14-17`).
- Rounds that ended this tick are gone after `rounds.retain` (`battle.rs:1977`). B5 must capture its evidence before that.
- Suppression filters squads only (`hull.is_none()`, `damage.rs:388`). B5 must not inherit that filter. Confirm with a test that hulls actually receive `NearMiss`; standoff is checked at `flight/mod.rs:1014` and `:1289`.
- Hearing runs only on sound-bucket ticks (`battle.rs:1380-1392`). Unseen shouts between buckets accumulate until the next bucket (slice 12).

**Loading and assets**
- `BattleAudio.connect` waits for every assigned sound before it is ready (`battleAudio.ts:67-72`). Voice packs therefore load after `ready`.
- The bank loads only `sounds.json` clip ids (`soundBank.ts:58`). Voice clips need a load path keyed by URL.
- The art bake deletes every **top-level** 64-hex directory under `assets/runtime/` that it didn't produce (`web/asset.mjs:272-279,293-295`). That namespace is the bake's. Audio stays under `assets/runtime/audio/`, which the bake already leaves alone, and never uses a bare sha directory.
- `assets.py` normalises to a fixed peak (`:162-165`), filters a single input (`:41-46`, `:145`) and checks by re-rendering bytes (`:200`). Voice profiles need a loudness target and a second input for beds, and the voice check verifies hashes.

**Workbench**
- `SoundWorkbenchStore` hashes every clip on each snapshot (`apps/sound-workbench/server.ts:86-101`). The voice store hashes only its JSON files.
- `FixturePublication` has an editor allowlist (`apps/fixture-publication/publication.ts:15`, `:107-111`). Add `voices` (music rides on the existing `sounds` editor).
- Hot-update suppression covers only `sounds.json` (`server.ts:303`).
- `Library.tsx:65-100` renders every row. Voices never go through it.

**Browser audio and deployment**
- `MediaElementAudioSourceNode` doesn't exist on `OfflineAudioContext`, which `WebAudioSink` and offline review use. `MusicPlayer` stays outside `WebAudioSink`.
- **Deploy:** the live site serves real LFS media (`armata-game.vercel.app/audio/clips/battlefield2-menu.wav` returned a 28.2 MB RIFF on 2026-10-11). The Vercel project's Git LFS setting is therefore on. It lives only in the Vercel dashboard; document it in `web/README.md` (slice 07).

## Slice graph

```
Lane A (library):  01 → 02 → 03 → 04 → 05 → 06
Lane B (music):    07 → 08            (08 needs 02's client)
Lane C (sim):      11 → 12 → 13
Battle:            09 (needs 03 + pilot clips) → 10 → 14 (needs 08, 10, 13) → 15 → 16
```

## Review map

| Slice | The user sees or hears | Checkpoint |
|---|---|---|
| 01 | `visualizations/voice-sets.html`: every roster unit → set, country, accent, callsign, female sets; the role scripts | non-blocking veto |
| 02 | the four reference voices through every processing profile | non-blocking |
| 03 | the Voices tab | non-blocking |
| 04 | the pilot set, all 29 triggers; the credit report | non-blocking |
| 05 | 3 design candidates per voice | the user picks; other lanes continue |
| 06 | every set in the Voices tab | review |
| 07 | the menu bed streaming; sliders in both menus | non-blocking |
| 08 | the 6 tracks, mood switch | non-blocking |
| 09 | the 30 s chatter preview | non-blocking |
| 10, 14, 15 | a played skirmish | non-blocking listen |

Non-blocking means: open the evidence for the user with [`preview-shots`](../../.agents/skills/preview-shots/SKILL.md) for pictures, or an opened audio page for sound. Wait about 5 minutes. If the user stays silent, decide on the evidence, record the decision in `choices.md`, close what you opened, and proceed.

## Verification gates

- **Narrowest check per slice,** as AGENTS.md says. The full `bun run check` and `bun run verify` run once, in slice 16.
- **Digests unchanged.** Every slice from 11 to 15 runs `crates/sim/tests/menu_reel.rs` and the pinned parity digests. Only slice 13 re-blesses publication bytes.
- **Visual surfaces:**
  - Workbench tabs (03, 08, 09) and the menu sliders (07): **the last check is an unprimed [`screenshot-critique`](../../.agents/skills/screenshot-critique/SKILL.md)**.
  - Slice 07 changes the existing menus, so also use [`compare-screenshots`](../../.agents/skills/compare-screenshots/SKILL.md) before and after.
  - Load [`game-ui`](../../.agents/skills/game-ui/SKILL.md) before touching the player's menus.
- **Rule changes:** load [`tweak-mechanics`](../../.agents/skills/tweak-mechanics/SKILL.md) before slices 11 and 12. Write a test for each ruled-out moment.
- **Audio:** a listen is not proved by a passing test. Open the audio for the user.

## References

- Discovery map (superseded where `choices.md` says so): [`visualizations/discovery-map.html`](visualizations/discovery-map.html).
- Reference voices and the chosen music direction: [`assets/`](assets/). The saved voice ids are in `assets/saved-voices.json`.
- Broken Arrow's score is by Evgeny Shchukin and Alexander Tochilkin: analog synth plus orchestral, on an FMOD adaptive-intensity engine. Sources: [Steam OST](https://store.steampowered.com/app/3693780/Broken_Arrow_Soundtrack/), [Apple Music](https://music.apple.com/us/album/broken-arrow-vol-1-original-videogame-soundtrack/1818789596).
- ElevenLabs: [voice slots per plan](https://help.elevenlabs.io/hc/en-us/articles/24351056337937-How-many-voice-slots-do-I-get-per-tier-and-how-can-I-increase-it), [pricing](https://elevenlabs.io/pricing), [summary of output rights](https://terms.law/ai-output-rights/elevenlabs/).
