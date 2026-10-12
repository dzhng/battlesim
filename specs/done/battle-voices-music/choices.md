# Battle voices and music: choices

This is the final ledger of the decisions behind the feature, checked against the code as it shipped. Each entry says who closed it:

- **user**: the user decided. These are listed first as givens and are not audited.
- **territory**: the code or a live probe settled it.
- **agent**: an agent decided on the user's behalf. The entry names the preference it relied on.

The agent entries are grouped by verdict: needs-user, then unsound (each since fixed), then sound. Within each group the least confident comes first. "Confidence" means how sure the audit is that the user would have made the same call. The code is the reference for the how, and each entry points to it.

**Review these first:** [N1](#n1-the-six-generated-tracks-play-in-battle-without-a-recorded-listen) (the score plays in battle without a recorded listen), [N2](#n2-the-scores-mood-logic-stays-in-typescript) (the score's mood logic stays in TypeScript while every other sound decision is Rust's), and [S1](#s1-a-misread-under-the-04-word-error-line-ships-unless-a-person-catches-it) (a misread that passes speech-to-text ships without anyone hearing it).

## Words used below

- **Response:** a unit answering its owner when selected or ordered (R1–R12, such as "Roger, moving out."). Only the owner hears it.
- **Bark:** a unit shouting what is happening to it in the battle (B1–B17, such as "Taking fire!"). Anyone near the camera hears it.
- **Shout:** the simulation's record of one bark: who, what kind of unit, where, which bark, steady or stressed.
- **Steady / stressed:** the two deliveries of every line. A unit is stressed when its health is at or below half, or it is suppressed.
- **Voice set:** the voices one country × role speaks with, such as `us_tank_crew`. Each set has a **lead** voice that gives responses and one or two **soldier** voices that bark.
- **Take:** one generated recording of one line: set, voice, trigger, intensity, take number (0 or 1).
- **Raw master:** the file ElevenLabs returned. **Served file:** the processed MP3 the game plays.
- **Lock:** `fixtures/voices.lock.json`, the generation tool's record of every take: its hashes, transcript, match score and verdict.
- **Verdict:** a reviewer's ok or bad on a take.
- **Pack:** `pack.json` beside a set's MP3s, listing the takes the game may play.
- **Chatter:** the arbiter that decides which requested lines actually play, and when (`crates/presentation/src/sound/voices.rs`).
- **Publication:** the packed per-side byte stream the simulation sends the client each tick.

## Givens

### The user's preferences for delegated calls

The user approved agent answers on structural questions: layout, formats, naming and catalog records. Taste and account decisions stayed with the user. The agent was to use four preferences:

1. take the recommendation when it is grounded in the code;
2. atmosphere over leanness;
3. the simulation is the authority;
4. numbers are guesses to tune.

### What the user decided

- **Two kinds of talk.** Responses R1–R12 are heard by the owner only. Barks B1–B17 happen in the world and are heard near the camera by every side, because "you would be hearing people shouting all the time".
- **English only,** with each nation's accent. Generic European infantry speaks with a **German** accent.
- **One voice set per country × role.** The lead voice answers and soldiers bark. A set per unit type was rejected.
- **The simulation emits shouts.** A side hears all its own units' shouts and an enemy's only while that enemy is seen. The client picks the take with its own seeded randomness. The leak is accepted by name: a seen enemy's stressed take or a "pinned!" bark reveals its state, and the comment on `IdentifiedUnit` names this exception.
- **All 29 triggers,** each steady and stressed, × 2 takes. B17 (friendly destroyed nearby) is included even though the agent recommended cutting it.
- **Limits.** In the simulation, an **8 s** cooldown per unit per trigger, and a unit never talks over itself. In the client, at most **4** lines at once. Responses come first, then own barks, then others, nearest the camera first. A line that can't start within **1 s** is dropped. One unit answers for a group.
- **Callsigns** are spoken in R1 (selected) and R11 (arrived) only, one per set.
- **Swearing** is allowed in stressed lines.
- **Voices** are ElevenLabs Voice Design voices, saved permanently. Premade voices and cloning were rejected. The account is on **Pro**.
- **Unseen enemies** are heard as anonymous, muffled shouting from a direction: a random bark from a generic pool, never the speaker's own set, low-passed at about **600 Hz** with reverb so the words can't be made out. The old `cue_voices` murmur, which sounded "like footsteps or heavy breathing", is deleted.
- **Taste.** The four reference voices are those in `assets/saved-voices.json`. Processing follows where the speaker sits: tank crew on a light radio over a faint engine bed, infantry dry, recon on a quiet earpiece, pilots on an intercom over a rotor bed.
- **Beds come from the game's own sound for that machine.** The pilot bed is the `rotor-uh60` clip low-passed at **900 Hz**. The game has no recorded tank engine, so the tank bed stays generated.
- **Music.** One score for every faction: a Pulse-family background underscore (D minor, 125 BPM, synth bass ostinato, no melody or hooks). There are 3 combat and 3 tension tracks at **−26 LUFS**. There are two moods: any own B5–B9 bark within **20 s** means combat, **45 s** without one returns to tension, and the crossfade is **4 s**.
- **Players get music and voices levels** beside master volume, in both menus.
- **Review.** The user is the only reviewer, so the workbench is built for spot checks and "mark bad, regenerate". A set's takes that pass speech-to-text may be bulk-accepted after the flagged takes are played.
- **Spend is delegated.** The pilot reports cost, and the batch proceeds without waiting ("don't let me block you here").
- **Keep everything.** Every generated asset is kept, including unpicked design candidates and superseded raw masters.
- **The auto-picked voices are approved** ("your auto picked ones are good").
- **No backward compatibility.** Cutovers are hard. `ShoutRules` changes the config digest, so old replays are refused; they already were, by build fingerprint.

### What the code or a live probe settled (territory)

- **ElevenLabs is reached only through Duet's proxy:** `POST https://ctl.duet.so/v1/services/elevenlabs/<path>?workspace=<slug>` with the Duet bearer token. The workspace must be a query parameter. Voice, model and user reads are denied, so credit use can only be read in the dashboard. The base URL is one constant, `BASE` in `tools/generate/elevenlabs.ts`.
- **Licence.** Paid plans allow commercial use in games. Provenance records `plan: "Duet ElevenLabs"`.
- **The deployed site serves real Git LFS media,** so the Vercel project's LFS setting is on. It is documented in `web/README.md`.
- **Flight already measures near misses against hulls.** Only suppression ignores hulls, so B5 ("under fire") can read the same evidence for tanks as for squads.
- **Sound decisions live in Rust.** The project rule from the native-parity work is "Rust as smart as possible, clients as dumb as possible". `crates/presentation/src/sound/` decides, and both the browser and the native client play what it decides.

## Needs the user

### N1. The six generated tracks play in battle without a recorded listen

_Agent, slices 08 and 14 · confidence low_

Suppose the player starts a skirmish. `BattleMusic` opens on a tension track and switches to a combat track when an own unit shouts "Taking fire!". Those six tracks were generated and gain-matched, and every one can be regenerated from the workbench's Music tab. But the feature's goal was that the user reviews everything in the workbench **before** it plays in battle. Voice takes enforce that: a take plays only if its pack lists it, and a pack lists only takes with an ok verdict. Music has no verdict at all. A track is in the catalog, so it plays.

- **Gap:** slice 08 made the user's listen non-blocking, and nothing recorded whether it happened.
- **Reach:** every battle's score. A track the user would reject plays until someone notices.
- **Provisional call:** keep the tracks playing; the user listens in the Music tab. To reverse it, remove a track id from `music.moods` in `fixtures/sounds.json`, or regenerate the track from the tab.

### N2. The score's mood logic stays in TypeScript

_Agent, slice 14 · confidence low_

Suppose a native client plays a battle. Rust tells it which barks, cues and lines to play. Nothing tells it which music mood to play. `MusicDirector` and `BattleMusic` (`packages/battle-audio/src/music/`) live in TypeScript. They decide tension or combat from the own barks that Rust's `Chatter` reports, and they duck the score while a line sounds. The agent kept them there because the music tables (`sounds.json`'s `music` section) are owned by TypeScript's catalog, and moving one rule would split the score's owner across two languages.

- **Gap:** the "Rust decides" rule listed Chatter (09b), barks (14) and unseen shouts (15). It didn't mention music.
- **Reach:** a native client gets no adaptive score without porting the director. Any new music rule (intensity layers, a victory sting) lands in TypeScript by imitation.
- **Provisional call:** keep it in TypeScript until a native client needs music. Then move the music section's typed reading and `MusicDirector` into `crates/presentation/src/sound/` together, and leave TypeScript only the `MusicPlayer` streaming.

## Unsound, since fixed

### X1. A line's level is data, beside the other line rules

_Agent, slices 10 and 09b · confidence medium · unsound at audit, fixed · now sound_

Suppose a playtest finds every voice line a little too loud against gunfire. At the audit, the tuner opened `fixtures/game.json` → `presentation.audio.voices` and found `lines`, `wait_s` and `group_s`, but no level: that was a `const LINE_GAIN = 0.5` in `crates/presentation/src/sound/voices.rs`, used for answers, for barks, and for the distance cull of heard shouts. It became a constant in slice 10 because the TypeScript file meant to hold it was about to move into Rust. That reason expired when 09b moved Chatter into Rust and gave `presentation.audio.voices` a typed reader (`ChatterRules`).

- **Gap:** slice 10 left the line level unplaced while the move to Rust was pending.
- **Reach:** every voice tuning pass. It was the one voice number a tuner couldn't find in the data.
- **Fix:** the level is `gain` in `presentation.audio.voices` (`"gain": 0.5`), a field of `ChatterRules` in Rust and TypeScript, and both readers refuse a rules object without a positive gain. Chatter's start command and the frame's cull of heard shouts read it from the rules; the constant is gone. Outcomes are unchanged: the recorded chatter vectors carry the same 0.5 and still match line for line.

## Sound

### S1. A misread under the 0.4 word-error line ships unless a person catches it

_Agent, slices 02 and 06b · confidence low_

Suppose a German tank crew's "Bail out!" take comes back as "Fail out!". Speech-to-text (`scribe_v1`) transcribes every raw master, and `transcript.ts` compares the transcript with the script. One wrong word in a three-word line is a word error of 0.33. That is under `MAX_WORD_ERROR = 0.4`, so the take is not flagged. Under the user's review gate, unflagged takes were bulk-accepted, so this take would ship unheard. Three such takes ("Fail out" twice, "Tubes" for "Troops") were found and judged bad by hand. Others may remain among the roughly 6,700 accepted takes. Two things narrow the gap. A keyword list (the callsign, "sabot", "gunner") flags any lost keyword whatever the word error. And the matcher ignores spelling-only differences, so it flags real misreads rather than "Armour" heard as "Armor".

- **Gap:** the plan named the speech-to-text check but no threshold.
- **Reach:** what "reviewed" means for the whole library. Lowering the threshold flags more takes for the user's ears; raising it lets more through.
- **Why sound:** at 0.3 or below, every one-word slip in a short line would be flagged, which buries the user in harmless variants. A person's verdict is the backstop, and the user can mark any take bad in the Voices tab.

### S2. Eight sets have a female response voice

_Agent, slice 01 · confidence medium_

Suppose the player selects a US Apache. A woman answers on the intercom. Female lead voices are on 8 sets: the attack helicopter pilots of Germany, France, Russia and the US, the US transport helicopter pilots, and one IFV crew per faction (German, Russian, US). The plan said "about 6: pilots, plus one vehicle crew per faction". The user can swap a voice's gender and description in `fixtures/voices.json`, and the Voices tab regenerates its takes.

- **Gap:** the plan gave a rough count and no list.
- **Reach:** which units sound female in every battle.
- **Why sound:** it is close to the plan's figure. The user heard the auto-picked voices and approved them.

### S3. Accents follow the faction for infantry and the country of origin for machines

_Agent, slice 01 · confidence medium_

Suppose a European player fields a Boxer, an Italian Centauro and a rifle squad. The squad speaks with a German accent, the faction default the user chose. The Boxer crew is German and the Centauro crew Italian, because a vehicle or aircraft follows the country that built it. Where several countries built it, there is a mapping: NH90 → German, Tiger HAD → French, Tiger UHT → German, Boxer → German, CV90 → Swedish, Centauro → Italian. National teams keep their own nation: the Akeron team is French, the FN-6 team is Chinese. The result is 56 sets (`fixtures/voices.json` → `sets`). A unit's role comes from its own row data (category, roles, squad bodies), so the Centauro II speaks as a tank crew and the Tiger as attack pilots.

- **Gap:** the user set only the generic European infantry accent.
- **Reach:** every set's accent. Adding a unit means choosing its set by the same rule.
- **Why sound:** it extends the user's own "German for generic European infantry" pattern, and no accent is borrowed.

### S4. The scripts: two different lines per trigger, tags only when stressed

_Agent, slices 01, 04, 06 · confidence medium_

Suppose a US rifle squad is ordered to stop twice. The first answer might be "Halt, hold here." and the second "We hold here.". Take 0 and take 1 are **different words**, not two renders of one line (`fixtures/voices.json` → `roles`). Steady lines carry no audio tag. Stressed lines carry one tag (`[urgent]`, `[shouting]`) and different words, about one in seven swears, and sniper teams whisper like recon. When a voice misread the same line twice, the line was reworded rather than retried a third time: "Tank here, send it." → "Armour here, send it.", "Firing." → "Firing on target.", "Halt." → "Halt, hold here.".

- **Gap:** the plan had role scripts but didn't say what a second take is.
- **Reach:** every line in the game, and the style future roles copy.
- **Why sound:** two texts double the variety for the same cost, and rewording fixes a line the voice can't say.

### S5. Every bark has the same 2.5 s talk time

_Agent, slices 11 and 02 review · confidence medium_

Suppose a tank shouts "Tank, front." at 0.0 s and is hit at 1.0 s. The simulation's `ShoutLedger` keeps the tank "talking" until 2.5 s, so the "We're hit" bark is not emitted. The talk time is `ShoutRules.talk_s` in `fixtures/game.json`: one number per trigger, all 2.5 s today. It is a nominal time. The simulation never reads clip lengths, and the generation tool flags any bark take longer than its trigger's `talk_s`. A response, which the simulation never sees, is held to the tool's `RESPONSE_TALK_S` = 4 s. A bark with no talk time is refused when the rules load (`ShoutRules::check`), so there is no silent default.

- **Gap:** the plan said "talk_s per trigger" with no values.
- **Reach:** how often a unit can shout. Raising one trigger's value thins that unit's later barks.
- **Why sound:** the tested takes run about 2–2.7 s. The original 1.5 s would let a unit's next shout begin under its last. Per-trigger values are data, ready to tune.

### S6. A contact nobody's sensor holds is called by the nearest own unit

_Agent, slice 11 · confidence medium_

Suppose the side identifies an enemy T-90 and two own units are nearby: a tank 300 m away whose sensor holds the T-90's track, and a squad 200 m away that doesn't see it. The tank calls "Tank, front.". Contact goes to the nearest own unit that sees the enemy (`shouts::build`, `crates/sim/src/shouts.rs`). If no own unit's sensor holds the track, the nearest own unit calls it anyway, as a radio relay would. Altitude decides an air contact before body type does, so a helicopter is always "air". A contact speaks before that unit's other barks, and its talk time blocks them.

- **Gap:** the slice named the nearest seeing unit and didn't say what happens when there is none.
- **Reach:** contact calls when identification comes from something other than an own unit's sight.
- **Why sound:** the side learned of the enemy, so someone says so. Silence would hide a real moment.

### S7. Responses are decided in TypeScript from the player's input

_Agent, slice 10 · confidence medium_

Suppose the player box-selects three tanks and orders a move. `responseFor` (`packages/battle-audio/src/voices/responses.ts`) turns each event into a request. A selection asks for R1. An accepted order asks for its R2–R10 line, keyed to the acknowledgement. A refusal reads `OrderError.reason`. Arrival, R11, is read from a unit's goal clearing. Rust's Chatter then arbitrates the request like any line. Only the arbitration is in Rust. Which answer to give is decided in TypeScript, because selection, clicks and acknowledgements exist only in the client.

- **Gap:** the move to Rust named Chatter, barks and unseen shouts, not responses.
- **Reach:** a native client needs its own port of `responses.ts` to answer the player.
- **Why sound:** the inputs are client state. Moving them into Rust would mean publishing UI state into the presentation crate.

### S8. Who answers, and which orders get an answer

_Agent, slice 10 · confidence medium_

Suppose the player clicks one Abrams in a group of three and orders an attack-move. The clicked tank answers. Without a clicked unit, the highest-value unit answers, with ties going to the lower id. The unit that answered the selection keeps answering the group's later orders while it is in the group. These requests share a group key (`order/<seq>`, `selected`, `arrived/<journey>`), and Chatter collapses requests with the same key within 250 ms, so one voice answers.

- **R12 (refused)** is spoken for refusals a soldier would voice: out of bounds, no valid destination, unknown target, air contact, not infantry, not a building, capacity full, one squad per building, building occupied (`REFUSALS`). Purchase, rate, sequence, replay and stale-id refusals are silent.
- **R11 (arrived)** ends a journey begun by a move, attack-move or building order, or by a purchased unit driving in. It is said once per journey, by the first unit to arrive, and only after at least 3 s under way (`ARRIVED_AFTER_S`). Stop, attack, withdraw and leave-building end a journey silently.

- **Gap:** the plan named R11 and R12's sources and didn't say which refusals are spoken, or who answers.
- **Reach:** how chatty the player's own units are.
- **Why sound:** an answer comes from the unit the player addressed. Bookkeeping refusals aren't moments a crew would voice.

### S9. The score ducks while any line sounds

_Agent, slices 08 and 14 · confidence medium_

Suppose a squad shouts "Man down!" over a combat track. While any Chatter line is sounding, the score dips by `duck.depth` 0.35 over 0.05 s, and it recovers over 0.8 s after the last line ends (`presentation.audio.music.duck`). Unseen-shout cues don't duck it: they are muffled frame sounds, not lines. The duck and the player's music level are separate audio nodes, so neither overwrites the other.

- **Gap:** the user asked for a music bus that "dips under voices" and gave no numbers.
- **Reach:** how present the score feels in a busy fight, where lines overlap and the score may stay ducked for long stretches.
- **Why sound:** words must be heard over the music, and the numbers are data to tune.

### S10. The unseen-shout pool: four stressed barks per bark voice of the enemy faction

_Agent, slice 15 · confidence medium_

Suppose a Russian squad behind a tree line shouts. Blue's speakers hear a muffled shout from the north-east. `ShoutPool` (`packages/battle-audio/src/voices/shoutPool.ts`) holds, from each pack of every set the **enemy faction** fields, up to `takes_per_voice` = 4 stressed barks per bark voice, taking every trigger's first take before any second. The pool comes to about a hundred small takes. Rust picks one with Mulberry32 seeded by the cue's sector, band and time, never by who shouted. The accent family is the faction's, but the voice is not the speaker's own.

- **Gap:** the user chose a "generic pool" and didn't set its size or makeup.
- **Reach:** what unseen fighting sounds like, and the load cost per battle.
- **Why sound:** the words are unintelligible, so the variety of voices matters more than the variety of lines. A small pool loads quickly.

### S11. How flagged voices were repaired

_Agent, slices 05 and 06 · confidence medium_

Suppose a pilot voice comes back with half its takes flagged. Every voice had 3 designed candidates. `generate pick --auto` kept the one whose audition best matched its text, with ties broken by the duration closest to the role's median. After the full batch, a voice with over 30% of its takes flagged (and not a whispering role) was designed again. Its description gained "Clear diction, easy to understand over a radio; moderate accent.", and all its takes were bought again on the new voice. That happened to four helicopter pilot voices. Then every remaining flagged take got exactly one retake. What stayed flagged waits for the user's ears.

- **Gap:** the plan had a pilot stop line and a review gate, but no repair policy for the batch.
- **Reach:** the template for the next batch, and the flagged takes still waiting, unjudged, in the Voices tab.
- **Why sound:** spend is bounded (one redesign, one retake), and nothing flagged ships without a verdict.

### S12. The whole library lives in Git LFS and ships with every deploy

_Agent, slices 02 and 06 · confidence medium_

Suppose a developer clones the repo. The served voice library is about 60 MB under `assets/runtime/audio/voices/`. The raw masters and design candidates are about 300 MB under `assets/source/audio/`. All of it is LFS (`*.mp3` in `.gitattributes`) and fetched only when needed. A deploy serves the runtime folder whole. A battle downloads only its own sets' packs, in the background.

- **Gap:** the plan put audio in LFS but sized it far smaller.
- **Reach:** deploy size, LFS bandwidth, and every new voice batch adding to both.
- **Why sound:** a player never downloads the library, only a battle's sets. Raw size follows from the user's keep-everything rule.

### S13. The six tracks' variations and length

_Agent (delegated), slice 08 · confidence medium_

Suppose the combat mood plays three tracks in turn. They differ on purpose. `combat_1` keeps the ostinato and strings in front at one flat intensity. `combat_2` lets the drums lead, still far back, building to a plateau. `combat_3` is darker, with a syncopated ostinato and swells every 30 s. Tension has `tension_sparse_1` and `_2` (stillness against unease) and `tension_dark_1`. Each is 150 s, built on the user's `STYLE` prompt (`tools/generate/music.ts`, provenance in `sounds.json` → `sources`).

- **Gap:** the user chose the style and moods, not the individual tracks.
- **Reach:** the score heard in every battle (see N1).
- **Why sound:** the variations stay inside the chosen style, and each track can be regenerated alone.

### S14. Voice loudness targets and the encoder

_Agent, slice 02 · confidence medium_

Suppose an infantry line and a recon line play back to back. The dry, light-radio and intercom profiles render to −16 LUFS integrated loudness, and the earpiece to −19 LUFS, so recon stays the "quiet earpiece" the user chose. Gain is linear, with a −1.5 dBFS limiter for peaks, so a shout stays louder than a mutter within a take. Takes are mono MP3, 40 kbps, 24 kHz, and encoded bit-exact. Hiss is seeded from the take's bytes, and the bed starts at a seeded offset so neighbouring lines don't begin on the same engine note (`VOICE_PROFILES`, `packages/battle-audio/tools/assets.py`).

- **Gap:** the user chose the treatments by ear. The plan gave no loudness targets or encoder settings.
- **Reach:** the balance of every line against the battle, and how much a take weighs.
- **Why sound:** the levels match the approved references, and the radio bands make anything above 24 kHz wasted bits.

### S15. The silence outside a skirmish

_Agent, slices 14 and 15 · confidence high_

Suppose a developer opens a lab scene with no skirmish factions. Own units still answer. Barks and unseen shouts are silent, because which voice set a unit kind speaks with depends on its faction, and a lab has none. `factionVoices` (`voiceCatalog.ts`) builds each side's kind → set table from the roster rows that faction fields. Rust never learns factions: a sound frame belongs to one side, so "the enemy's sets" is the only enemy table it holds.

- **Gap:** the plan assumed a faction was always known.
- **Reach:** labs can't be used to judge battle chatter. The sound lab speaks with a real pack instead.
- **Why sound:** labs are developer arenas, not played.

### S16. The voice data is one file, and assignments live in it

_Agent, plan · confidence high_

Suppose the roster gains a new Russian IFV. Its row in the unit catalog gets no voice field. Instead, `fixtures/voices.json` → `assignments` maps the faction's row to `ru_ifv_crew`. That file is the only voice authority: role scripts, sets (country, role, callsign, processing profile, voices with their saved `voice_id`), assignments and retake counts. `sounds.json` never lists a voice clip. Rows shared between factions (rifle squad, Javelin, Stinger) get one assignment per faction.

- **Gap:** the user's D4 first said "each unit row gets `voice`".
- **Reach:** adding a unit means touching `voices.json`, which the validator checks against the roster.
- **Why sound:** `UnitType` rejects unknown fields, the simulation never reads voices, and shared rows need per-faction answers anyway.

### S17. Voices per set and callsign names

_Agent, slice 01 · confidence high_

Suppose a 9-man rifle squad and a 2-man sniper team are both in a fight. The rifle squad has 3 voices: a lead who answers and two soldiers who bark. The sniper team has 2. Squads of 6 or more get 3 voices. Every other set (small teams and every vehicle or aircraft crew) gets 2: a commander who answers and a crew member who shouts. A barking soldier's voice comes from a stable hash of the unit, so one unit always sounds the same. Callsigns are a national word plus a number ("Dragon 1", "Adler 6"), unique per set.

- **Gap:** the plan said "2–3 voices" for squads and didn't cover vehicles.
- **Reach:** the size of every generation batch: a lead voice has 48 takes (12 responses × 2 intensities × 2 takes), a soldier voice 68 (17 barks × 2 × 2).
- **Why sound:** a crew with one voice would sound flat when answers and barks interleave.

### S18. B5, "under fire": any enemy round close enough to suppress

_Agent under tweak-mechanics, slice 12 · confidence high_

Suppose a rifle round cracks past an M1 Abrams, or lands within its suppression radius. The tank calls "Taking fire!". `shouts::under_fire` counts an enemy round's near miss, impact or ricochet within the weapon's `suppression_radius_m` of the hull box or of the nearest living soldier. It uses suppression's own `near_miss` score but, unlike suppression, includes hulls. Own-side rounds never count; a war film shows "check fire!", not "taking fire!". A squad that is already suppressed or pinned stays silent on B5, because B6 and B7 own that moment. The tier is read after this tick's rounds, so a squad whose tier rises this tick says B6 or B7. The shooter need not be seen.

- **Gap:** the user asked for "under fire" with no physical definition.
- **Reach:** the most common stress bark, and one of the barks that switch the music to combat.
- **Why sound:** it reuses the evidence suppression already scores. No new rule or `Unit` state is added.

### S19. B15 and B17: what a unit sees decides

_Agent under tweak-mechanics, slice 12 · confidence high_

Suppose an ATGM is steering at the second of two tanks. Only that tank shouts "Incoming missile!", and only once the missile clears the terrain into its view. B15 needs the missile's commanded point to lie on the unit's footprint (its hull box seen from above, or the squad's radius) and the unit to see the missile itself (`shouts::incoming`). Now suppose that tank is destroyed. Only the **nearest** friendly that can see where its crew sat shouts "We lost one!", with ties going to the lower id. One witness, never a chorus, and a loss nobody saw gets no shout. The witness looks at the dead unit's eye point, because a sight line ending on bare ground is refused.

- **Gap:** the plan defined these in words. The code needed the geometry.
- **Reach:** both barks follow the fog rules every other sense follows.
- **Why sound:** they use the existing `sensing::sees_point`. No new sight rule was added.

### S20. Which bark wins on a busy tick

_Agent, slices 11 and 12 · confidence high_

Suppose on one tick a squad loses a man, drops below a third of its strength and hears a round pass. It says one thing, B10 ("Heavy losses."). `PRIORITY` in `crates/sim/src/shouts.rs` orders the barks, most specific first: destroyed > incoming missile > badly damaged > man down > hit > pinned > suppressed > friendly destroyed > under fire > missile away > out of ammo > reloading > kill > opens fire. A bark fires on a change, never a steady state. B4 fires on a new target, B6 and B7 when the tier rises, B13 once when a weapon runs dry. B12 is for infantry only, because a crew's loader works unheard. A dead or withdrawn unit never speaks again, except B16, its own last line. A squad's "health" is living members over starting members, because a squad has no hull hp. That figure decides both "stressed" and B10.

- **Gap:** the plan listed triggers, not ties or edges.
- **Reach:** what the player hears first in every firefight.
- **Why sound:** it is data, and specific beats generic.

### S21. Shout state lives in a per-side ledger, outside the digest

_Agent, slices 11 and 12 · confidence high_

Suppose two runs of the same battle are compared. Their digests match whether or not anyone shouted. `ShoutLedger` (cooldowns, who is talking, last tick's contacts and targets) sits per side beside the observations. It is never on `Unit`, never digested, and never draws simulation randomness. Barks are found by comparing each unit with a small `Before` note made each tick (alive, hp, each soldier's hp, tier, and each mount's reason, shot count and missile), not with a full copy of every unit. `shouts::build` makes a side's own shouts. `shouts::heard` copies an enemy's shouts into the other side's frame as `Seen(handle)` while that side identifies the speaker. `shouts::unheard` turns the rest into anonymous hearing cues. One owner builds each unit's shouts; hearing only filters them.

- **Gap:** the plan fixed the ledger's home and left its contents and split open.
- **Reach:** any future presentation-only evidence should follow this pattern.
- **Why sound:** the digest is the proof of determinism, and shouts change no outcome.

### S22. An unseen shout's cue: range, timing and sound

_Agent, slices 12 and 15 · confidence high_

Suppose a hidden enemy squad shouts on a tick between hearing updates. Its cue reaches Blue at the next hearing update. It carries as far as the speaker's own `loudness_m`, and it holds only listener, 8-way sector and near/far band. A destroyed speaker's last shout makes no cue, because hearing hears living sources only. The client plays it on the **voices** bus, so the voices slider covers it. It is panned by sector, low-passed at 600 Hz and sent 0.6 to reverb in both bands: near is louder, never clearer (`presentation.audio.cues.shout`, `Sound::note` in `sound/frame.rs`). It has its own row because a pool is not a recipe. With no pool loaded, a shout is silent. Idle unseen infantry is silent, and moving infantry keeps its footsteps cue.

- **Gap:** the user set the effect and left the plumbing open.
- **Reach:** the anonymous-hearing contract. No set, trigger or position reaches the client.
- **Why sound:** it reuses hearing's existing path and repeat rule.

### S23. The shout's publication layout

_Agent, slice 13 · confidence high_

Suppose the browser decodes a tick's publication. After `crashes` comes a `shouts` group of rows: `own, idLo, idHi, kind, x, y, z, trigger, intensity`. `own` is 1 for the side's unit and 0 for an identified enemy, as crashes and corpses mark it. The id travels as 16-bit limbs. The layout publishes `shoutTriggers` (the contract's names, `b1_contact_infantry` … `b17_friendly_destroyed_nearby`, from `ShoutTrigger::ALL`) and `intensities`, so TypeScript hard-codes no tag. `barkTrigger` (`voiceCatalog.ts`) is the one TypeScript mapping from a tag to a voice line name. The publication was re-blessed once, by name. The native Godot fixture captures were re-encoded to the new layout, not captured again.

- **Gap:** the plan named the contract, not the byte layout.
- **Reach:** every reader of the publication: browser, native and fixtures.
- **Why sound:** it follows the existing positional-group pattern exactly.

### S24. Chatter's open edges

_Agent, slices 09 and 09b · confidence high_

Suppose the player orders a group and three answers arrive 100 ms apart. One plays, because requests sharing a group key within `group_s` (250 ms) collapse to the first. Other edges the plan left open:

- A request without a position counts as nearest: an owner's radio is in the player's ear.
- A line whose take hasn't loaded is dropped at once, not queued.
- The take is chosen by Mulberry32 over a hash of seed, speaker, trigger, intensity and request count, so a battle replays the same takes.
- Answers come from the set's first responding voice. A barking unit's voice comes from a hash of its name, and an own unit's name is the same for its answers and its barks, so it never shouts over its own answer.

Chatter is pure (`crates/presentation/src/sound/voices.rs`). The workbench's chatter preview runs the same code. Vectors that the old TypeScript Chatter wrote before it was deleted still hold the Rust port to its behaviour.

- **Gap:** the user set the limits and left the edges open.
- **Reach:** every line played.
- **Why sound:** each edge follows the user's rules, and the vectors pin them.

### S25. Barks are placed in the world; answers are dry

_Agent (delegated curve), slice 14 · confidence high_

Suppose an own tank 200 m to the left shouts "Taking fire!". It plays from the left, quieter and duller with distance, using the same distance gain, air low-pass and reverb as every unit sound (`Sound::place`). A bark too far off to pass the distance cull asks for nothing. Its own answer to an order plays dry and centred, like a radio in the player's ear. A bark asks to play when the presentation clock reaches its tick, as gunshots wait for theirs.

- **Gap:** the plan said barks are "heard near the camera" and didn't define the curve.
- **Reach:** the feel of every bark.
- **Why sound:** one placement rule covers every positional sound.

### S26. Loading: packs after `ready`, only this battle's sets

_Agent, plan and slice 14 · confidence high_

Suppose a skirmish starts. The battle becomes playable without waiting for any voice. After `ready`, `VoiceBank` fetches, in the background, the packs of the sets the battle's units speak with (which sets, and how takes are held, is [S38](#s38-a-battle-loads-only-the-sets-on-its-battlefield-and-decodes-a-take-when-its-line-starts)). Until a take is fetched, its line is dropped. A set that fails to load stays silent. Music streams through an `<audio>` element into Web Audio (`MusicPlayer`), outside `WebAudioSink`, because offline rendering can't host a media element.

- **Gap:** the plan's loading question.
- **Reach:** startup cost stays flat however big the library grows.
- **Why sound:** voice is never worth a slower start.

### S27. Music is not a mixer bus, and the menu bed streams too

_Agent, slice 07 · confidence high_

Suppose the player drags the music slider in the menu. The menu bed, now an MP3 at gain 0.16 (matching the old WAV's level), changes through `MusicPlayer`'s chain: element → track gain → level → duck → master. The mixer's bus list is units, effects, ambience and voices, with no `music` bus, because nothing plays on one. The old decode-whole-file WAV path is deleted. The voices bus keeps its distance-reverb send, because barks are positional.

- **Gap:** the plan named a `music` bus.
- **Reach:** any new music feature goes through `MusicPlayer`.
- **Why sound:** one music path, and no dead structure.

### S28. How the director moves between moods

_Agent, slices 08 and 14 · confidence high_

Suppose the player's squad is pinned at 3 s. The score crossfades to combat. If nothing qualifying happens until 48 s, it crossfades back to tension. Only **own** barks in `combat_triggers` (under fire, suppressed, pinned, hit, man down) count, because enemies' barks would leak their state. Combat starts when the latest qualifying bark is at most 20 s old and ends when it is over 45 s old. Within a mood, tracks are drawn from a shuffled bag, and a new bag never starts with the track just played. The order uses unseeded randomness, because no replay depends on it. A battle opens on tension, fading in under the menu's fade-out.

- **Gap:** the user set the numbers and left their reading open.
- **Reach:** the score's whole behaviour (see N2).
- **Why sound:** it reads the user's rule literally.

### S29. Music in the sound catalog

_Agent, slices 07 and 08 · confidence high_

Suppose someone generates a fourth combat track. It joins `fixtures/sounds.json`. Its raw master is a `sources` record with `Generated` provenance (model, prompt, date, plan), plus a path under `assets/source/audio/music/` and a hash. Its entry in `music.tracks` names that source, the served file's hash and its gain. The gain is computed so the served file plays at −26 LUFS after gain, on the same scale the menu bed was matched on. The moods must be exactly menu, tension and combat. `musicUrl()` (`src/music/musicCatalog.ts`) alone builds `/audio/music/<track>.<sha8>.mp3`. The tool writes `sounds.json` through the formatter, so adding a track doesn't reflow the file.

- **Gap:** the plan named the section, not its shape.
- **Reach:** one catalog owns every recording's provenance, including generated ones.
- **Why sound:** it extends the existing owner rather than adding a `music.json`.

### S30. The generation tool's shape

_Agent, plan and slice 02 · confidence high_

Suppose the user marks a take bad and presses R. The workbench runs `bun run generate voices --set … --voice … --only bad` (`packages/battle-audio/tools/generate/`). Credits are spent only through this TypeScript tool. It caches at two levels:

- a **generation key** (the text after callsign expansion, `voice_id`, model, settings and retake count) names the raw master, so a killed run never pays twice;
- a **processing key** (the raw master's hash, the profile's whole definition, the bed's hash and the encoder) names the served file, so retuning a profile re-renders locally for free.

At most 8 text-to-speech, 4 transcription, 3 design and 2 music calls run at once (`LIMITS`). Calls retry with jittered backoff, and a 401, 402 or quota error stops the run. Speech-to-text reads the raw master, so retuning never re-transcribes. `assets.py` stays the only owner of ffmpeg. The tool uses `eleven_v3` at stability 0.5, one of the three values v3 accepts.

- **Gap:** the plan's choice of tooling.
- **Reach:** every future batch, of voices or music.
- **Why sound:** it is resumable, testable offline (fetch is injected) and cheap to retune.

### S31. One bed shape, and the catalog's hash is a clip's identity

_Agent, slice 02b · confidence high_

Suppose a pilot take is rendered. Its profile's bed is `{clip: "rotor-uh60", filter, volume}`. The tank's is `{generated: "tank_engine", …}`. One resolver turns either into a file and its hash, and that hash enters the processing key. A catalog clip whose file differs from the hash in `sounds.json` refuses to render ("pull Git LFS"). `generate check` reports a generated bed that no profile names.

- **Gap:** the user's beds rule needed a mechanism.
- **Reach:** a new machine bed is one profile line.
- **Why sound:** there is one path for both kinds of bed, and a stale file can't render silently.

### S32. Verdicts are bound to the bytes heard, and packs carry only ok takes

_Agent, slices 03, 10, 06b · confidence high_

Suppose the user marks a take ok, then a profile tweak re-renders it. The verdict no longer holds, because it carries the hash of the bytes the user heard (`verdictOf`). `generate pack` lists only takes judged ok on their current served hash whose file still hashes to it. `--unreviewed` (off by default) also admits unjudged, unflagged takes, and `generate check` reports such a pack as stale. `check` proves each pack against the lock and the files. Packs sit at fixed URLs (`/audio/voices/<set>/pack.json`) and are revalidated by ETag. There is no pack index. Served takes are named `<voice>-<trigger>-<intensity>-<take>.<hash8>.mp3`, so a regenerated take never collides with a cached one.

- **Gap:** the plan required review before play without saying how review binds to audio.
- **Reach:** "reviewed" stays true through every re-render.
- **Why sound:** a verdict can't outlive the sound it judged.

### S33. One writer per file

_Agent, plan and slices 03, 08 · confidence high_

Suppose the user regenerates a music track while a voice batch is running. The workbench server queues both through one generation-tool queue (`apps/sound-workbench/generation.ts`), so `voices.lock.json` and `sounds.json` each have one writer. A verdict given in the Voices tab is recorded by the tool, not the server. People and the workbench's save path write `voices.json`. The browser never calls ElevenLabs.

- **Gap:** the plan named the writers and didn't say how the workbench reaches them.
- **Reach:** any new generated asset goes through the tool's queue.
- **Why sound:** concurrent writers to one JSON file would lose data.

### S34. The Voices tab's review flow

_Agent, slice 03 · confidence high_

Suppose the user opens `de_rifle_squad`. Nothing plays until a key or click. The queue (flagged takes first) is fixed when the set opens. A (accept), X (bad) and R (regenerate) each advance to the next take and play it. "Accept unflagged in set" is one tool run. Line edits and R save without a review diff, so the keyboard flow stays fast. The tab doesn't reuse the Library grid, and a snapshot hashes only `voices.json`, so a verdict never invalidates a pending edit. Voice descriptions are shown but not edited here.

- **Gap:** the user wanted fast spot checks and left the controls open.
- **Reach:** the user's main review surface.
- **Why sound:** it follows the "mark bad → regenerate" loop the user asked for.

### S35. Keeping everything, except stale served files

_Agent, applying the user's rule, slices 05 and 08 · confidence high_

Suppose a take is regenerated. Its old raw master stays under `assets/source/audio/voices/`, along with every unpicked design candidate and superseded bed. Its old **served** file is deleted, because it is a re-renderable derivative that would otherwise ship with every deploy.

- **Gap:** the user's keep-everything rule didn't distinguish source from derivative.
- **Reach:** the source folder only grows; the served folder matches the lock.
- **Why sound:** it keeps all paid-for media and ships none that is stale.

### S36. Where generated audio lives

_Agent, plan · confidence high_

Suppose a new generated asset is needed. Its raw master goes under `assets/source/audio/<kind>/` and its served file under `assets/runtime/audio/<kind>/`, following the `assets/source/menu` → `assets/runtime/menu` precedent. A top-level `assets/generated/` served by its own Vite plugin was rejected after the user asked to "make sure the folder structure makes sense as a whole". So were separate `music.json`, review and pack-index files. The art bake owns the top-level hash directories in `assets/runtime/`, so audio never uses one.

- **Gap:** where generated media goes.
- **Reach:** every future generated asset.
- **Why sound:** it adds no new place or serving machinery.

### S37. Workbench, provenance and levels owners

_Agent, plan · confidence high_

Suppose someone adds a new generated sound. Its provenance uses the `Generated` kind in `catalog.ts`, the existing provenance owner, which voices import. Player levels are `SoundSettings { muted, volume, music, voices }` in `settings.ts`, shown by one `SoundControls` in both menus. The workbench has a Voices tab, a Music tab with a manual mood switch, and a 30 s chatter preview through the real Chatter.

- **Gap:** the plan's structural questions.
- **Reach:** one owner each for provenance, levels and review.
- **Why sound:** it extends the owners that already existed.

### S38. A battle loads only the sets on its battlefield, and decodes a take when its line starts

_Agent, closeout memory fix · confidence high_

Suppose a US player deploys into a small skirmish against the eastern faction. At closeout the battle loaded every set both factions' rosters speak with and kept every take decoded: a probe of that deploy fetched 3,909 takes (33 MB of MP3) and held 3,909 decoded buffers, 1,264 MB of PCM. Now:

- **Which sets load.** Own units' and pending purchases' sets, as for answers (`UnitResponses`), and an enemy kind's set from the first time the side identifies one (`BattleAudio.identified`, fed each publication's `identified` kinds). Each loads in the background as it appears, so a newly seen kind's first barks, before its set arrives, are dropped as any unloaded take is. The unseen-shout pool still draws from every enemy set, bounded by `takes_per_voice` (S10): 64 (US), 84 (eastern) or 100 (European) takes in all, 23 to 37 MB decoded.
- **How a take is held.** `VoiceBank` keeps the fetched MP3 bytes and decodes when Rust's start command arrives; the last `DECODED_TAKES` = 32 played stay decoded. The shout pool decodes each of its takes once and keeps them itself, outside that limit. Rust learns each take's length when its bytes arrive, from the pack's `duration_s`, so nothing is decoded to measure it. That length is the rendered speech; ffmpeg decodes the MP3 0.05 to 0.06 s longer (encoder padding, 20 takes sampled), so a line now counts as sounding for that much less than its samples run.
- **A line decoded late.** It starts when decoded, unless that is more than the chatter's `wait_s` (1 s) after its start command's time; then it is dropped. Rust has already counted it as started, so the cap and the score's duck treat it as sounding either way.

The same deploy now fetches 84 takes (0.85 MB) and holds 84 decoded buffers, 32.5 MB: the enemy's shout pool and nothing else, because the player has fielded nothing yet. Fielding one Abrams adds its set's 116 takes (1.0 MB of MP3), and its first line decodes one more (32.8 MB in all).

- **Gap:** the plan said a battle loads its sets, and didn't weigh a whole roster decoded.
- **Reach:** every battle's memory, and when a line's first play pays for a decode (a few milliseconds for a 2 s take).
- **Why sound:** Rust arbitrates the same requests by the same rules and seeded takes; only each take's length loses its padding. Memory falls by a factor of about forty, and the pool, the one set of takes kept decoded, is bounded by data. Decoding at a 24 kHz context would halve the pool again; it wasn't needed.

Trivial discretion, not listed: command and flag names (`generate voices`, `process`, `pack`, `check`, `design`, `pick`, and the `--set`/`--voice`/`--trigger`/`--only` filters), track ids, and internal type names.
