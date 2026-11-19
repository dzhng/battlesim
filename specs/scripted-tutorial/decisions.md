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
  were spellings like armour/armor and treeline/tree line. Slice 10 imported them
  into the voice pipeline as raw masters, and its generator replaced the
  scratch tools that rendered them.
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
  the zones file (since folded into the scenario document, slice 08).
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

## Slice 05

- **Offers travel by unit type.** The skirmish holds each narrowed shop as
  resolved `OfferedPurchase { kind, carrier }` pairs, resolved from the
  document's names when the battle is built, because the publication writer
  has no catalog. The publication adds an `offers` group (owner, kind,
  carrier) and a `shopNarrowed` flag on the player row, so a shop narrowed to
  nothing still reads as narrowed. The parity fixtures were re-blessed once
  for the new layout, a named decision (agent).
- **`NotOffered` counts as malformed** (agent): the sender sees its own offer,
  as it sees its faction for `WrongFaction`. The page has no words for it,
  because the picker never shows a card the shop doesn't sell.
- **Empty tabs hide only in a narrowed shop** (agent, after an unprimed
  critique found a lone card under nine empty tabs "semi-broken"). A skirmish
  faction sells in every group, and the picker's tests build tiny card sets
  that click tabs, so the general rule would only have broken them.
- **A squad sold only riding opens its menu on its ride**, and the menu drops
  "On foot" when the shop doesn't sell the squad on foot (agent).
- **An authored scenario's battle doesn't open from a replay file** (agent).
  `parseReplayFile` admits only a generated battle's file; chapter restart
  (12) replays inside the session. A scenario replay from a file is a later
  addition if it's ever wanted.
- **`Skirmish::{offer, grant}` move to slice 06** (agent), where the director
  first calls them, so no function lands unused.
- **A capture refresh re-records its battle digests** (agent, its own commit).
  The paris-corner capture's recorded digests had gone stale on staging before
  this branch (proved on the base commit), so no layout change could re-bless
  it. That follows the user's rule that recordings follow the game.
- **The Ferryman document offers chapter 1's two purchases** (scouts; a rifle
  squad in a Bradley), and the opening credits cover exactly them: the
  battle opens with 390 CR at today's prices.

## Slice 06

- **One `Director` type, carried resolved by name** (agent). The setup carries
  `DirectorSetup { chapters, zones, forces: name → UnitId }`, and the battle
  looks names up at run time, so there is no parallel "resolved director" type.
  `Director::admit` refuses at `ScenarioDocument::on` any name no zone, force,
  player or earlier beat answers to.
- **The vocabulary is chapters 1–2's** (agent): triggers `chapter_opens`,
  `fired {beat, after_s}`, `phase`, `bought {player, purchase}`,
  `units {who, are: in | engagement}`, `identifies {side, who}`,
  `contact_in {side, zone}`, `all`, `any`; selectors `force` and
  `bought {player, variant}`; actions `order` (`move_to`, `attack_ground`,
  `engage`), `grant`, `offer`. The spec's `Knows` became the two concrete
  triggers `identifies` and `contact_in`, because a contact is an area, not a
  unit. Its `Hidden` fact is left out; no gate needs it, since the story's
  hidden step waits on return fire.
- **The director runs before orders apply** (agent), just ahead of fixed
  scripts, so a cue's order applies on the tick its beat fires.
- **`apply_authored_order` is the one authored-order path** (agent). The fixed
  scripts' survivor filter and admission moved into it; fixed scripts call it
  with no player, the director with the owning player (a skirmish needs one).
- **An order for a seat that isn't directed is skipped silently, not refused**
  (agent): when a person plays blue, the walkthrough's solution beats are
  inert by design.
- **Identification reads the side's track** (known within its grace), not
  "seen this very tick" (agent), because the director runs early in the tick,
  before this tick's sensing.

## Slice 07

- **A published beat is its place among the director's beats** (agent). The
  publication writer has no director, so `ScenarioView.beats` holds
  `(place, tick)`, and the layout's `beats` and `chapters` tables name them.
  The reader gives the page names: `scenario: { chapter, beats: [{beat, tick}] }`,
  with `chapter` null once every chapter has closed.
- **Every side knows the open chapter; only the audience hears a beat**
  (agent, from C7). Header words `scenarioChapter` (−1 without a director) and
  `beatCount`, and a `beats` group. A battle without a director publishes no
  scenario. The parity fixtures were re-blessed once more for the layout.

## Slice 08

- **The solution buys and readies through two more actions, `buy` and
  `ready`** (agent). Like `order`, they act only for a directed seat, so a
  person's battle ignores them. `Solution { max_minutes, chapters }` is a
  contract type; `ScenarioDocument::with_solution` merges its chapters by id
  and hands every human seat to the director.
- **Chapters 1–2 play through in 115 game seconds** (about 1.2 s of test time),
  the baseline for R3. The walkthrough can write its replay
  (`WALKTHROUGH_REPLAY=<dir>`) to watch with `/battle?replay`.
- **Three content fixes the walkthrough found** (agent, all lopsiding the story
  as the spec asks):
  - The outpost starts with its RPG rounds spent: it killed the Bradley with
    its launcher before chapter 3 could teach anything.
  - The solution places the squad and its Bradley at the ridge, not the farm,
    which is inside the outpost's reach. The farm is chapter 3's.
  - The river guns stand on clear ground at the woods' edge and fire at a
    `river_bank` zone they can reach: from inside the woods, and at the first
    aim point, their trajectory was blocked (`BlockedTrajectory`, never
    fired). A throwaway probe found which aim points they fire at. The
    `river_woods` zone was centred on the woods' edge so the heard contact
    meets it.
- **The zones live only in the scenario document** (agent): the spec's
  ferryman-zones.json asset is deleted, since it had drifted; the valley
  picture keeps the original layout for review.

## Slice 09

- **The page never loads the scenario document** (agent): it holds red's
  forces. The coach reads only the published progress. The coach-against-
  director check runs as a test over the shipped files (`web/tests/coach.test.ts`),
  not at run time, and the page-side outline (C9's `ScenarioPublic`) waits
  for slice 11's marks, its first runtime reader.
- **UI signals come from where each act happens** (agent): camera pan, turn,
  tilt and zoom from the viewport's one camera entry (`steer`, by what the
  step changed); selection and show-orders from the unit control's state;
  a move with facing from its one order sender (`issue`); the shop opening
  from the picker's state; contact focus from the viewport's jump.
- **Contact focus's key joins the bindings** (`ContactFocusBinding`, N), so the
  viewport, the Guide and the coach read it from one place (agent; the viewport
  had it hard-coded).
- **Placing a purchase sets no facing** (found while writing chapter 1's
  steps): the drag only turns the ghost, and a purchase carries no facing. The
  `c1.place` line promised one; it was re-worded ("they drive there from our
  road") and re-recorded, and the place step now waits on the purchase itself.
- **`pause` steps wait for slice 11** (agent), with the camera tour they serve.
- **The coach panel's unprimed critique waits for slice 10's comms look**
  (agent): this slice's panel is the plain card that slice restyles.

## Slice 10

- **Narration is a section of `voices.json`, not a unit set** (agent):
  `narration.speakers` (library `voice_id`, model, seed, processing profile)
  and `narration.lines` by coach line id. Its takes share the units' lock
  (`narration/<speaker>/<line>`), cache keys, verdicts and pack rule, so one
  pipeline judges every voice.
- **Bento carries narration only; Duet stays for unit voices** (agent). The
  spec said the Duet path goes, but unit voices still need Voice Design, which
  Bento's proxy refuses. Retiring Duet waits until unit design has another
  route.
- **Lantern goes through `voice-earpiece`, not a radio profile** (agent, by
  ear): `voice-radio-light` mixes a tank-engine bed under the voice, which
  reads as a crew member, not HQ.
- **The 58 review takes were imported and judged ok by the agent** after each
  passed speech-to-text word for word. The user can listen in the sound
  workbench and mark any take bad. A bad verdict drops it from the pack, and
  `retakes` buys it again.
- **No caption of the spoken line** (agent): the card already says what to do,
  with the keys. A second block of near-identical text made the panel read like
  a chat log.
- **The signal bar runs for the take's length** (agent), not from its live
  level: there's no analyser node for one animation. It's hidden when nothing
  plays.
- **A newer line replaces the one speaking**, as on a radio net (agent).
- **Ducking unit chatter under a line moves to slice 13** (agent). Narration
  and chatter share the voices bus, and holding chatter means a hold in the
  Rust chatter arbitration. Chapters 1–2 have no combat chatter under a line,
  and chapter 3 is the first that does.
- **The panel puts the card beside the portrait** (agent, after an unprimed
  critique): keys as tabs (the guide's key style, shared), each pair in
  keyboard order (W/S, A/D), the chapter title dimmed at the right of the
  name line, the portrait monochrome in the HUD accent behind visible scan
  lines, and the panel flush with the HUD's top edge.
- **Panel shots cover the briefing over open fields at the default zoom**
  (agent). The fog, smoke and far-zoom variants come with slice 11's camera
  tour, which frames those views on purpose.

## Slice 11

- **`BattleView` owns the coach** (agent): its `coach` prop is the document,
  and one hook (`useCoach`) gives the step that drives the card, the tour, the
  marks and the rings. The render-prop slot from slice 09 is gone, and
  `CoachPanel` only draws the step it's handed.
- **A tour is one pilot for the battle's life** (`useCoachTour`, agent): the
  viewport reads its pilot once. The pilot flies a step's reel from the first
  frame after the step appears and returns nothing once it ends, so the
  player's camera carries on from the tour's last framing. It doesn't snap
  back to where the player was. Tours never dip to black and follow no unit,
  and the validator refuses either.
- **The cursor's order action follows a flying pilot, not a mounted one**
  (agent): with a coach pilot always mounted, `!pilot` would have disabled
  orders for the whole battle.
- **A coach mark is a dashed ring in the HUD accent** (agent, after an
  unprimed critique). In the zone colour it read as a fourth objective, since
  a neutral objective is dashed in that colour. No objective state is dashed
  in the accent: a held objective is solid. Its geometry comes from the
  resolved scenario's `director.zones`, which the page already parses for its
  overlay, so no extra outline was added to the report. The `entry` zone moved
  28 m inside the map edge, so the border no longer cuts its ring.
- **HUD anchors are a typed data attribute** (agent): a control spreads
  `hudAnchor("shop")` where it's drawn, and the coach finds
  `[data-hud-anchor]`. There's no registry and no class names. A highlight
  is the HUD's corner brackets in the accent, standing off the control. It
  follows the control's rectangle each frame, pulses once as it appears, then
  breathes slowly (still under reduced motion). The critique found the first
  look, a plain outline, blended into the command grid.
- **A command button names its binding once** (agent, found while anchoring):
  `command: "toggle_fire_policy"` gives it the key in its label and its
  `command:<name>` anchor. The hand-built `(F)` suffixes are gone.
- **Home's tiles are their own list** (agent): `MENU_TILES` holds
  `{ id, title, path }`, where a path is a menu page's or any address. The
  Tutorial tile is the tutorial battle's address, and pages no longer carry a
  `tile` flag.
- **The `ui` scene checks the Guide where it now lives** (agent): the pause
  menu's guide opens with every picture decoded, and the old `menu-tutorial`
  baseline was deleted with its page.
- **The flags step points at the objective flags, not the score** (critique):
  preparation shows no score bar, so the card no longer describes one, and
  the score gets an anchor of its own for later chapters. The spoken line
  ("First side to the top wins") still fits and wasn't re-rendered. The tour
  comes in closer over each flag.
- **The road in keeps its lane number** (critique, not changed): the "1" on
  the entry chevrons is the skirmish HUD's lane label, outside this spec.
- **`pause` steps aren't built yet** (agent): no step in chapters 1–2 needs
  one. Chapter 1 runs in preparation, where nothing moves. The first chapter
  slice with a tour over a live battle adds them (see 13's shape).
- **One tutorial scene** (agent): `scenario-battle` walks the briefing from
  the coach document itself, step by step (rings, tours, marks), instead of a
  second `tutorial.mjs`.

## Slice 12

- **Resume is a cut replay, stepped and then let go** (agent): the log,
  controller transitions, script trace and seat events are cut at the tick
  and replayed. Admission already re-records each command and advances the
  sequence counters, so the live battle's record continues from the prefix
  without copying anything. A transition recorded at the tick applies at
  once, as a live change made then would.
- **What refuses `AiMemory`** (agent): an active AI seat before the tick (from
  the setup or a transition), or an encounter opponent, whose memory is
  outside the digest. A replay's end-of-battle `controllers` list counts only
  for a record without transitions, because it says how the battle ended,
  not how it began.
- **"Past the end" means the battle had finished by the tick** (agent): a
  replay doesn't record how long its battle ran, only when it finished.
- **The standby is a whole second client** (agent): when the page first sees
  a chapter, it asks the playing authority for its replay and starts a
  worker that resumes at that publication's tick, publishing nothing.
  "Restart chapter" hands that client to the session, the way a restart
  hands it a fresh one, so the view and its built world stay mounted. If
  publications batch ticks, the restart point is the first published tick
  of the chapter, not the director's exact opening.
- **A restart still rebuilds the page's world** (measured, not changed): the
  battle view drops its art while a new client has no observation yet, as
  the plain Restart always has. So a chapter restart remounts the viewport.
  It takes 1.8 s from the confirmation to the chapter's first step in the
  scene. The standby removes the replay's cost, not that rebuild. Keeping
  the art across a restart is a change to battle art admission, outside
  this spec.
- **The coach forgets on a new run** (agent): `useCoach` keys what it heard
  and dismissed by the session's client, so a restart re-derives the coach
  from the publication alone.
- **The browser check restarts the briefing** (agent): reaching chapter 2 in
  a browser means buying through the shop. The walkthrough already proves a
  later chapter restarts (it resumes the latest chapter from its own replay,
  and every beat fires again).
- **The dev loop moves to slice 13** (agent, found here): the walkthrough's
  replay belongs to the solved scenario, not the shipped one, so it can't
  resume into the player's battle. See 13's shape.

## Slice 13

- **Vocabulary added** (agent): facts `inside` (`carrier` or `building`),
  `suppression` (at least a tier), `attack_moving`; trigger `objective`
  (`owned_by` a side, held and not contested; `capturing` by a side;
  `contested`); directed orders `unload` (in place), `occupy` (the
  garrisonable building nearest the zone's centre, within the zone),
  `attack_move_to`, and `withdraw` (return to base and leave the battle). The
  spec listed only the facts, but the solution needed the orders, and `withdraw`
  was the physical fix below. Each is tested on test units
  (`crates/sim/tests/director_facts.rs`).
- **Chapter 2's river guns withdraw once heard** (agent, found by the
  walkthrough). Left on the map, the BMP traded fire with the Bradley (which
  fires at will) and killed it with its missiles. The story says they are
  heard, never seen, so they now leave the battle six seconds after blue
  hears them.
- **The counter squad is placed and worn for the lesson** (agent, by the
  walkthrough): just behind the house north of the ring, holding fire until
  sent, four casualties, rockets spent. Placed farther out, it died before
  contesting. Fresh, it killed blue's squad.
- **The Bradley overwatches from the outpost, not the ring** (solution): parked
  in the ring, it stalemated beside the counter squad.
- **The drive gets its own zone** (`farm_wall`): the farm zone's centre is
  inside the farmhouse, so a carrier sent there stopped at its edge.
- **The chapters' dev loop** (agent, from 12's finding):
  `/battle?scenario=<map>/<name>&chapter=<id>[&beat=<id>]` prepares the solved
  scenario (`scenario_chapter`, over `Battle::chapter_start`) and resumes it
  where the chapter opens or the beat fires, with every human-capable seat
  handed back to a person. A human seat takes no director orders, so the
  solution goes quiet. A scenario battle's player is its human-capable seat,
  so the page takes orders from the seat it is handed. Solutions now ship to
  the page, since a tutorial's solution is no secret.
- **The scene plays every UI step the walkthrough can't** (agent): it starts
  each later chapter at its opening and after each beat that comes before a
  UI step, then does what the coach shows (select, facing move, contact
  jump, show orders).
- **Chatter holds while the narrator speaks** (agent, from 10): new unit lines
  wait out a narration line. The lines already playing are cut, as before,
  when the narrator keys in.
- **`pause` steps are still unbuilt** (agent): chapter 3 has no tour over the
  live battle. The first chapter that does adds them.
- **Found, not fixed: an in-place unload after a move order does nothing**
  (agent, while making the solution). A carrier told to move, then to unload
  where it stands, keeps its squad aboard indefinitely. The same unload
  without the move works. The solution unloads once the carrier has stopped,
  so the walkthrough doesn't depend on it, but a player who presses unload
  while driving meets it. It needs its own fix, in carriage or movement.
- **After an unprimed critique of chapters 2–3's shots** (agent): the
  show-orders key's label is "Space", and the card and Guide say "Hold". The
  facing and attack-move steps name flag 1 and mark the ridge and the
  crossroads. Two findings are left as they are and recorded here:
  - unit readouts overlap when units stand close (the readout layout's own
    problem, beyond the tutorial);
  - the contact jump can show little when the newest contact is already in
    view. That depends on where the contact falls, and a scripted camera move
    before it would teach less than seeing the real jump.

## Slice 14

- **Vocabulary added** (agent): facts `popped_smoke` (fewer charges than the
  unit carries), `reversing` (the drive the sim already publishes),
  `ordered: {attack: <selector>}` (the unit's current order); trigger
  `destroyed` (every unit the selector names is dead, and it named one).
  This is a trigger, not the spec's unit fact, because a fact reads the
  living. Also directed orders `pop_smoke`, `reverse_to`, and `attack` (by
  the handle the ordering side identifies the target by; refused when it
  doesn't), and a beat's `recovery` flag.
- **A recovery beat isn't waited for** (agent): a chapter closes when its
  other beats fired, and the walkthrough doesn't demand them. Solution beats
  that answer a recovery are flagged too. A recovery beat fires once.
  `director_facts` proves the not-waiting and the grant on a loss, on test
  units, which stands in for R8's "AT team dies before the tank".
- **The tank comes to the ambush holding fire** (agent, by the walkthrough,
  revised in slice 15). At fire at will it shelled everything in view from the
  north road while the AT team walked six minutes in. Then it spotted the
  team at the copse first and shot it. Now it waits on the north road and
  advances holding fire. It swings through `west_lane`, north of the copse,
  in the team's view, then turns for the crossroads.
- **The ambush stands just outside the copse, facing north** (`ambush_woods`
  at (741, 806)). From inside the trees the Javelin's flight path was
  blocked, and its launcher needs its own sight line. A probe of the map's
  sight lines showed the copse's edge sees north and north-west, and almost
  none of the north road.
- **The tank is worn to 10 hull points** (lopsided): a Javelin that hits head
  on does partial damage, so any hit kills it.
- **The AT team is bought, not placed** (story): offered and granted when the
  chapter opens, and a coach step (without a line) asks for it before
  Lantern's "We have an anti-tank team in the woods" plays.
- **Groups need one signal, not two** (agent): a UI step waits on a signal,
  not a count of them. The card asks for two groups.
- **Group keys come from their owner** (`FirstGroupKeys` in
  `controlGroups.ts`), read by the coach's `{key:group_assign}` and
  `{key:group_recall}`. The card names group 1's keys rather than a range,
  which read as one odd key (critique).
- **The army ring blends into the tray** (critique, not changed): its corner
  brackets fall on the tray panel's own. Ringing single cards needs per-card
  anchors, which no step needs yet.
- **Found and fixed: setting a control group never told the coach**
  (`group_set` was declared in slice 09 but never emitted). The scene's
  chapter 4 step caught it.
- **The solution's tripwire went from 10 to 20 minutes**: the AT team's walk
  alone takes six.

## Slice 15

- **Vocabulary added** (agent): facts `deployed` and `serviced` (being
  replenished, repaired or reinforced now, the supply owner's published
  status); `ordered: {attack_ground: <zone>}`; the grant `to_cover` (only
  the wallet's shortfall; as opening credits it is `covering`, since an empty
  wallet's shortfall is the whole price); the directed order `deploy`.
- **The treeline team is an RPG team** (agent): hidden in `far_treeline` on
  return fire only. Once suppressed it withdraws, which is the "done"
  beat's other way out.
- **A unit serviced is any of the fleet the player has** (Bradley, squad,
  scouts, Javelins), so whichever unit the player brings in counts.
- **Adding a force reshuffles the battle** (found): forces take unit ids by
  name, so the treeline team moved every bought unit's id and the seeded
  outcomes with them. Chapter 4's ambush, which had passed by luck, failed
  until it was rebuilt on sight lines (above).

## Slice 16

- **Vocabulary added** (agent): facts `airborne` and `arrived` (on the
  field); triggers `pending` (a purchase reserved and on its way) and
  `radar_contact` (a side holds a contact its receivers found); the directed
  order `attack_contact_in` (attack the ordering side's contact meeting a
  zone). Red's helicopter is bought by the existing `buy` with a red grant,
  as the spec asked, so it arrives by an ordinary purchase. It can't be worn
  and short of ammunition, since a purchase takes no condition, so it is a
  full Mi-8.
- **Mechanic changed: an anti-radiation seeker rides the beam** (agent, by
  `tweak-mechanics`). A seeker fired into a radar contact steered at the
  contact's centre, which radar reports in 100 m range steps. The Growler's
  missiles fell 50 m off a Buk they had locked on to: a HARM that misses a
  radar it was riding is a moment a viewer calls wrong. Now an anti-radiation
  seeker (the weapon's `anti_radiation`) aimed at a radar contact homes on
  the emitter behind it while it lives and emits. Any other round aimed at
  the contact still aims at the reported area, and the side learns nothing.
  Proven by `radar::an_anti_radiation_seeker_rides_the_beam_to_the_radar_not_the_reports_centre`
  (red before the change). Parity, capture and publication digests were
  unchanged.
- **The SAM's missiles are spent** (lopsided): the Buk reaches most of the
  map, and the chapter teaches finding it, not trading with it.
- **Chapter 4's tank is P4's T-72** (agent): the earlier T-14 came from a
  search that missed the T-72's card.
- **Reaction lines are spoken when their gate is met** (agent, found while
  writing chapter 6). A step's line was spoken as it appeared, so "Tank
  down", "Splash one" and "They're pinned" played before the moment. A step
  that `reacts` speaks its line when its gate is met, before the next step's
  line, one after the other (`linesBetween`, unit-tested). A step a closing
  chapter dropped says nothing. Chapters 2–6 mark their reactions, and
  chapter 3's capture became four steps (capturing, taken, contested,
  cleared), so each line meets its moment.
- **Directed orders are proven where the solution leans on them** (agent):
  the walkthrough fails by beat if one stops working. Only the facts and
  triggers each have a test on test units.

