# Decisions ledger

What was decided, by whom and why. Append entries; never rewrite one. Each slice
records the decisions it had to make that the plan didn't, with the reason.

## Interview (user, 2026-11-19)

- **Guide.** The old page is the Guide, reached only from Esc as a centred
  modal. The main menu's Tutorial tile starts the battle.
- **Story.** Operation Ferryman ([story.md](story.md)) was approved as drafted.
- **Scope.** All seven chapters (recommended option), including supply,
  artillery, helicopters, jets and SEAD.
- **Structure.** One continuous battle. Restart chapter replays the recorded
  battle to the chapter's start (recommended option).
- **Map.** A fixed saved map. Asked about a `tutorial` category, the user said:
  "name it to be something more generalized, because I do want to create
  explicit scenarios with explicit maps in the future". So the category is
  `scenario`, and the format is general.
- **Test gate.** A headless walkthrough to Victory, with a lopsided design
  (recommended option).
- **Narration and avatars.** The user asked for Bento to create narrations and
  any assets needed. Voice: Eryn, picked from five library voices (Eryn, Pam, Brody, Rolando, Philip), all reading the same opening line. Portrait:
  "match the voice's gender", so the agent picked F1, which faces the camera.
  Animation: a still portrait with comms effects (recommended option).
- **Compatibility.** Neither backward compatibility nor migrations (the
  write-spec default; the user didn't opt in).

## Narration assets (agent, 2026-11-19)

- **Library voices only.** Bento's ElevenLabs proxy doesn't admit
  `/v1/text-to-voice/design`, and voices created through it expire after 24 h.
  Shared-library voice ids work directly in text-to-speech and don't expire, so
  any line can be re-rendered later in the same voice.
- **58 lines** were rendered in the `armata` workspace with `eleven_v3`, seed 7.
  Every take passed a speech-to-text check word for word; the only differences
  were spellings like armour/armor and treeline/tree line. They are review takes
  in [assets/narration/takes](assets/narration/takes); slice 10 brings them into
  the voice pipeline. The scratch tools that rendered them (render.sh,
  check.sh) are in `throwaway/tutorial-assets/narration/`, and slice 10
  replaces them with the pipeline's own generator.
- **Lines carry intent, never keys or numbers** (see [story.md](story.md)). A
  spoken key name would go stale with a rebinding; a spoken number with a
  retune.
- **No shoulder-launched air defence in the `us` roster file** (its air
  defence is the M-SHORAD, Patriot and NASAMS; draft C reports a Stinger team
  in the shared roster). The chapter 6 line stays generic ("get air defence
  up"); P4 picks the card.

## Synthesis

Three blind drafts from the same brief: A (fewest slices, Claude), B (risk
first, Codex gpt-6.2-sol), C (seam quality, Claude). The drafts are in
`throwaway/tutorial-drafts/` until this spec closes.

**All three agreed** on: probes first (map, mixed skirmish and scripts, replay
cost); one runtime contract (`ScenarioDefinition`); a shipped scenario follows
current rules (never a frozen bundle); grants computed from current prices; an
offer list enforced by the sim and mirrored by the shop; beats published only to
their side; the coach as client presentation reading only its own side; restart
as a replay prefix plus going live; a native walkthrough; the deploy key is T.

**Where they split:**

| Question | A | B | C | Call | Why |
|---|---|---|---|---|---|
| Who runs the story and red's cues | Authored scripts, extended with event memory, tags and places | Authored scripts plus declared public beats | A typed director (triggers and actions) in the sim, digested, rerun on replay | **C** | The interpreter is stateless, position-addressed, can't buy, unload or garrison, and isn't published. A typed director is checked at admission, the coach validator can see its beat list, and orders reuse the fixed-script path. The interpreter stays the bundle's custom-code surface (named follow-up in README). |
| Coach ↔ sim link | Recorded `Advance { stage }` from the coach; stages gate red | Coach reducer waits on public beats and UI; no command | The director never waits on the coach; the coach pauses the offline battle for cards | **C** | Progression never depends on presentation. A UI step that must not let time pass pauses the offline battle, which changes no determinism. Chapter openings come from the director's digested state, so restart needs no extra marker. |
| Scenario source format | Extend `EncounterDefinition` | Extend `EncounterDefinition` | New `ScenarioDocument`, no `overrides` by type | **C** | A minority call, made deliberately. Encounters are film and test staging that may pin stats; a player scenario must not. Keeping each type's invariants in its fields beats a category-dependent refusal. The loaders share the map-loading code, and only the small document loader is new. |
| Red seat behaviour | A red director script on a red player | A red authored script | `Playbook::Directed`: fights by ordinary target choice at authored engagement, moves only on director cues | **C** | `PassiveAi` forces return fire, so neither the ambush tank nor a suppressible outpost works under it. Playbooks also consolidate four ad-hoc opponent drivers. |
| Walkthrough | Rust solution code against the scenario | Browser/test driver sending normal commands | Solution beats in the director's vocabulary, the human seat `Directed` at test time | **C** | No test-only runtime path; the walkthrough also exercises the director's order path; the solution is data beside the scenario. |
| Bundle | Untouched | Add `ScriptedEconomy` mode | Untouched; export is resolve then wrap | **A/C** | Nothing in this feature needs a new bundle mode. |
| Slice count | 4 | 9 + 4 probes | 25 | **19** | C's seams, merged where one gate covers two seams (05, 06, 11), and A's restart-before-chapter-3 ordering. |

**Picked up from a single draft:**

- A: P5, the suspected seq bug: player-owned authored script orders are stamped
  with the side's sequence number while admission checks the player's own.
- A: the restart cost must be reported as a loose tripwire, not a target.
- C: `MapCatalog::scenario` → `encounter_scenario`.
- C: per-chapter recovery beats against soft locks.
- C: camera tours reuse the capture tool's reel format.
- B: compare the browser publication overhead of a fast-forward, not only native
  tick cost (P2).

## Map generation hills (user, 2026-11-19)

The user pointed out that map-generation hills are being worked on in a separate
session ("Map Gen Hills Status and Spec", branch `t3/map-gen-rolling-hills`),
and said to coordinate with it if needed. On `staging`, generated maps have no
hills. The ground-sight check refuses relief, and the branch caps hills at 8 m
until a crest counts as cover. P1 now checks for that work first and falls back
to authored relief on the saved map, so the tutorial never blocks on it. The
hills session was told what the valley needs.

## Probes P3 and P5 (agent, 2026-11-19)

- **P3: pre-placed owned units inside a skirmish are refused by contract.**
  `ScenarioDefinition::validate_player_contract` refuses a skirmish with
  authored units, events, scripts, an opponent or an encounter ("its players
  buy what it fields"), because every fielded unit carries an `EnteredUnit`
  price that the army-size taper, refunds, bounties and settlement read.
  **Slice 04 takes the fallback:** the contract admits authored forces in a
  skirmish when each names one of its players as owner. Events, scripts, an
  opponent and an encounter stay refused. At setup each authored unit gets an
  `EnteredUnit` at its **current catalog price**, charged to nobody, at tick 0.
  Not zero: killing red's pre-placed tank should pay blue its normal bounty,
  and red's cap and taper should count it like a bought unit.
- **P5: not a bug.** A player's own order sequence exists only inside a
  skirmish, and a skirmish refuses authored scripts, so a script's orders are
  always sequenced by side. No fix, no test.

## Slice 02

- **`ControllerKind::DirectedAi` instead of a `Playbook` field** (agent).
  Controller kinds already encode what an AI seat does (`ActiveAi` plays,
  `PassiveAi` holds fire); a playbook beside them would be a second owner of one
  question, and a seat could say "passive controller, directed playbook". So a
  directed seat is a controller kind, and it is set the way any controller is
  (setup, or a recorded controller transition, which is how the walkthrough
  hands the human seat to the director). The walkthrough (C8) and restart (C11)
  wording follow: restart refuses after an `ActiveAi` seat played.
- **A directed seat counts as ready rather than being marked ready** (agent): a
  derived rule (`PlayerState::is_ready`), so a seat that becomes directed
  mid-preparation counts too, and the stored `ready` flag (in the digest) stays
  what the seat said, so existing digests are unchanged.
- **Battle addresses carry `d` for a directed seat** (agent): the address's
  controller codes are typed over every controller kind. A directed seat in a
  generated skirmish just sits and fights back; harmless, and no special case.
- **The bundle's controller normalisation keeps any non-human seat as is**
  (agent, a cleanup while adding the arm): one guard (an untaken human seat
  sits passive) instead of an arm per kind.

## Probes P1, P2, P4 (agent, 2026-11-19)

- **P4: the rosters cover every role.** The picks, with the reason each one
  is the right card, are what the chapter slices field:

  | Role | Card | Why this card |
  |---|---|---|
  | Blue recon | `us_army_scouts_light_patrol` | Sees 750 m; concealment 0.8 |
  | Blue squad and carrier | `rifle_squad` + `us_m2_bradley_ifv_m2a4` | The only `us` 8-seat carrier with smoke, which chapter 4 needs; its autocannon suppresses |
  | Blue AT team | `fgm_148_javelin_team` | Top attack beats a T-72's roof; minimum range 100 m |
  | Blue supply | `us_m977_hemtt_general_resupply` | Deploys |
  | Blue mortar | `us_m1129_stryker_mortar_120_mm_mortar_carrier` | Indirect, 100–900 m |
  | Blue air defence | `fim_92_stinger_team` | Hits low air |
  | Blue helicopter | `us_ah_1z_viper_attack_helicopter` | |
  | Blue jet | A-10C | Attack runs and arrival countdown |
  | Blue SEAD | `us_ea_18g_growler_electronic_warfare_aircraft` | Radar receiver 5 km, anti-radiation missile |
  | Red squads | `eastern_rifle_squad` | |
  | Red tank | `eastern_t_72_t_72b3_2016` | |
  | Red helicopter | `eastern_mi_8_hip_mi_8amtsh` | The Mi-35M's armour is beyond the Stinger |
  | Red radar SAM | `eastern_buk_buk_m3_medium_range_air_defense` | Emits; the Pantsir would shoot down the SEAD missile |
  | Red treeline team | `eastern_rpg_team_rpg_7` | |

  The Buk reaches 1500 m, most of the map, so chapter 6's red SAM must not
  engage before the radar step (its authored engagement, then a director cue).
- **P1: the flat valley generates; the hills are authored.** The rolling-hills
  branch hasn't landed. The best of 60 requests is `open small` seed 3 (1800 m).
  It was regenerated in the `paris` region so the village reads European (the
  seed drew `china`; `region` is a generation request field, and the layout
  didn't change). Saved as `fixtures/maps/ferryman-valley` with two authored
  ridges (a 14 m ridge under the wood beside Blue's road, and a 26 m hill on
  red's bank of the river for the radar) and a low stone wall on the farm's
  village side, east of the road. It is an `authored` map: its identity pins its
  content hash and its own template selection (8 paris templates), as the
  fixtures README asks of an authored map. The generated `sites.json` stays out
  of the folder (that file belongs to generated maps); the scenario document
  carries its sites. The map and its zones:
  [assets/ferryman-valley.png](assets/ferryman-valley.png),
  [assets/ferryman-zones.json](assets/ferryman-zones.json).
- **Zone names follow the map, and so do the two spoken lines that named
  them.** An unprimed review of the picture found every story place, and
  flagged that "east woods" lie across the river to the west, and that the tank
  comes down the north road, not a ridge road. The zones are now `river_woods`
  and `north_road`; `c2.gunfire` ("across the river") and `c4.tank` ("on the
  north road") were re-recorded and pass speech-to-text.
- **The ridge treeline is about 530 m from the village's south edge**, inside
  the scouts' 750 m sight. So chapter 2's outpost stands at the village's south
  edge. The walkthrough (08) proves the sighting.
- **P2: restart by plain replay is too slow at full scale.** Native release, at
  about 30 units a side: 65.6 M instructions a tick, 261 ticks/s, so 45k ticks
  takes about 3 minutes. A normal small battle with 23 units runs at about
  800 ticks/s (56 s). In the browser, `SimClient.advance` also publishes every
  tick and waits for the page's buffer. **Slice 12 takes the fallback** (see its
  file).

## Slice 04

- **Forces are a map from a name to a unit** (`forces: {name: UnitSetup}`), not
  a list with an optional tag (agent). Every force has a name its story uses, so
  the director (06) and the coach can say "the outpost". The battle numbers
  forces in name order. A tagged list would have needed `#[serde(flatten)]`
  over `UnitSetup`, which can't keep `deny_unknown_fields`.
- **The skirmish contract now admits owned authored forces, and refuses authored
  scripts outright** (agent, from P3). `validate_player_contract` refuses events,
  scripts, authored scripts, an opponent or an encounter in a skirmish, and any
  authored unit that isn't one of its players'. One consequence: a
  scenario-builder Economy bundle may now carry owned forces as well, the same
  rule for both sources. A bundle test now checks the refusal by its field
  (`units[0]`) instead of its wording.
- **A placed force enters at its catalog price, charged to nobody, with no spawn
  receipt** (agent): it counts for income taper, cap and bounty like a bought
  unit, but never shows as spending in the match report.
- **`MatchRules::patched` is the one match-rule patch** (agent). Frontline's
  resolution now goes through it too, which refuses a key the rules lack.
- **Bridge and hill are each other's counterparts** (agent). Skirmish site
  admission wants one central objective and a counterpart pair. Counterparts
  only matter to map generation and analysis, so pairing them changes no play.
- **The refusal and price tests live in the sim crate, on test units** (agent).
  Resolution needs a real catalog, which the contract's tests can't load. They
  give test units roster membership by catalog patch, as the transport tests
  do, so no roster unit is pinned.
- **No `scenario_report` example** (agent). The placement test is the legality
  oracle and the spec's valley picture is the review surface; a report example
  would have been a second owner of the same checks.
- **The browser and WASM scenario loaders move to slice 05** (agent), where
  they are first used, so this slice adds no unused code.
- **Red's `river_guns` stands at the edge of `river_woods`, not inside it**
  (agent): the placement test refused it among the trees, since trees are bodies.
