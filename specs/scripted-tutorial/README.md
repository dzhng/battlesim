# Scripted tutorial: Operation Ferryman

The main menu's **Tutorial** tile opens a real battle: one continuous fight on a
fixed valley map, in seven chapters. A coach teaches each mechanic as the story
needs it, voiced by the HQ officer **Lantern**. Today's web-page tutorial
becomes the **Guide**, a centred modal in the battle's Esc menu, and leaves the
main menu.

The tutorial is the first **authored scenario**: a fixed battle on a fixed map,
under today's rules and roster. A second scenario must need only data (a map
folder and its JSON files), never code. The user wants more explicit scenarios
on explicit maps later.

- [story.md](story.md): the approved story, chapter by chapter (zones, forces,
  steps, gates, narration line ids).
- [decisions.md](decisions.md): the decisions ledger, covering the interview,
  the synthesis of three drafts, and the decisions each slice makes.
- [slices/](slices/): one file per slice.
- [assets/](assets/): the narrator's portrait, the narration script and its
  recorded takes.

## Next Agent Prompt

*Status 2026-11-19: slices 00–15 shipped (through chapter 5, keep it
running, with the chapters' dev loop), and merged to `staging` as each
lands. Next: [slice 16 (chapter 6)](slices/16-chapter-6.md). Start a chapter in
the browser with `/battle?scenario=ferryman-valley/operation-ferryman&chapter=<id>[&beat=<id>]`.*

Read **Contracts** and **One owner per concept** below before touching code;
every slice inherits them. The scenario document lives in
`crates/contract/src/scenario_document.rs` and resolves through
`sim::maps::scenario`; slice 05 adds its browser and WASM loaders. The card
picks for every role are P4's (decisions.md). Climb the check ladder only as far as each slice's gate says. The full run
happens once, in slice 18. Record every decision the plan didn't make in
[decisions.md](decisions.md), then update this section (status, next pickup,
blockers) before you end your pass.

Warnings:

- The deploy key is **T**, not D. Coach text never spells a key: it reads the
  bindings.
- Browser scenes need the WASM build (`bun run --cwd web build:wasm`, after any
  contract or sim change) and the LFS runtime art and effects
  (`assets/runtime/**`, `assets/third-party/effects/**`), all present in this
  checkout. Run them with `WEBGPU_BROWSER=local`.
- Generated maps on `staging` are still flat (the rolling-hills work is in a
  separate session). The Ferryman valley's hills are authored, so nothing waits on
  that work.
- Bento's ElevenLabs proxy has no voice-design route, and voices created through
  it expire after 24 h. Narration uses a shared-library voice, which never
  expires (slice 10).
- Disk runs near full: about 30 GB free. Keep Rust builds serial in one checkout.

TODO:

- [x] [00](slices/00-probes.md) Probes: valley map, resume cost, pre-placed owned units in a skirmish, roster coverage, script sequence bug
- [x] [01](slices/01-guide.md) The tutorial page becomes the Guide, reached from Esc
- [x] [02](slices/02-directed-seats.md) A `DirectedAi` controller: an AI seat the director moves
- [x] [03](slices/03-scenario-map.md) `scenario` map category and the Ferryman valley
- [x] [04](slices/04-scenario-document.md) Scenario document, its resolver and the placement check
- [x] [05](slices/05-scenario-battle.md) Scenario battle playable: address, preparation, offered shop, covering grants
- [x] [06](slices/06-director.md) Director: contract and runtime
- [x] [07](slices/07-published-progress.md) Director progress published to its audience
- [x] [08](slices/08-walkthrough.md) Walkthrough harness and chapters 1–2 directed
- [x] [09](slices/09-coach-core.md) Coach core: document, derivation, UI signals, plain panel
- [ ] [10](slices/10-lantern-comms.md) Lantern's comms panel and narration through the voice pipeline
- [ ] [11](slices/11-coach-direction.md) Camera tour, world marks, HUD highlights; Tutorial tile cutover
- [ ] [12](slices/12-chapter-restart.md) Chapter restart by replay
- [ ] [13](slices/13-chapter-3.md) Chapter 3: The crossroads
- [ ] [14](slices/14-chapter-4.md) Chapter 4: Steel on the ridge
- [ ] [15](slices/15-chapter-5.md) Chapter 5: Keep it running
- [ ] [16](slices/16-chapter-6.md) Chapter 6: Eyes in the sky
- [ ] [17](slices/17-chapter-7.md) Chapter 7: The push, to Victory
- [ ] [18](slices/18-closeout.md) Closeout

## Decided (interview, 2026-11-19)

1. **Guide.** `apps/battle-lab/src/Tutorial.tsx` becomes the Guide. It is reached
   only from the battle's Esc menu, as a centred modal (`PauseMenu.tsx` already
   does this, labelled "Tutorial"). The main menu's Tutorial tile launches the
   tutorial battle.
2. **Scope: all seven chapters** of [the story](story.md), built in order, so
   the early chapters are playable first.
3. **One continuous battle.** Units carry forward. "Restart chapter" replays
   the recorded battle to that chapter's start, reusing the deterministic
   replay.
4. **A fixed saved map in a new, general `scenario` category**, for player-facing
   authored scenarios. The name is deliberately not "tutorial". It is generated
   once, tuned in the map workbench and saved. Positions are coordinates, and a
   check proves every placement is still legal.
5. **Walkthrough gate.** A headless run follows the solution to Victory and
   fires every beat, and it fails when a change breaks the tutorial. Fights are
   lopsided by design and steps advance on actions, so routine retunes don't
   break it. No test pins a stat, loadout, price, text or class name.
6. **Narrator.** Lantern, the HQ operations officer, calls the player
   "Ferryman". Voice: ElevenLabs shared-library "Eryn" (`wa4sQVgbDDzUDEzJwch3`,
   `eleven_v3`). Portrait: [assets/portraits/lantern.png](assets/portraits/lantern.png)
   (candidate F1 of [four](assets/portraits/candidates.jpg)). It animates as
   a still portrait with comms effects (an audio-driven signal bar and a little
   scan flicker), not lip-synced video.
7. **No backward compatibility and no data migrations.** Hard cutovers.

## Contracts

Each contract is listed with what it is today and what it becomes. The slice
that lands it owns the exact field names (delegated). The owner and the
direction of data flow are decided here.

| # | Boundary | Today | After |
|---|---|---|---|
| C1 | `contract::players::ControllerKind` | `Human`, `ActiveAi` (the skirmish doctrine), `PassiveAi` (does nothing, holds return fire); two hand-written lists of them | Adds `DirectedAi`: an AI seat that issues nothing on its own, keeps its units' authored engagement, counts as ready in preparation, and is moved only by the director. `ControllerKind::ALL` is the one list the publication layout and player rows read. **Shipped** ([02](slices/02-directed-seats.md)). |
| C2 | `contract::maps::MapCategory`, `web/src/maps/catalogue.ts` | `test`, `capture`; "no saved map is the game's" | Adds `scenario`: a reviewed battlefield that player-facing authored scenarios stand on. Labs and tests refuse it, as they refuse `capture`. `meta.json` lists `scenarios`. The content owners in `development.md` and `fixtures/README.md` go from three to four. ([03](slices/03-scenario-map.md)) |
| C3 | Scenario document → `ScenarioDefinition` | Saved maps carry `encounters/<name>.json` (`EncounterDefinition`, with rule `overrides` for films and labs) | New `ScenarioDocument` at `fixtures/maps/<id>/scenarios/<name>.json`. It holds the battle seed, players (red as `DirectedAi`), a `match_rules` patch over `fixtures/skirmish.json`, sites, tagged forces, opening credits, offers, zones and the director. **It has no `overrides` field, by type**, so a shipped scenario always plays today's rules and roster. `ScenarioDocument::on(map, rules)` is its one resolver. `public()` returns what the page may know (chapters, beats with audiences, zones, objective ids, never forces). `MapCatalog::scenario` is renamed `encounter_scenario`. ([04](slices/04-scenario-document.md)) |
| C4 | Credits and the shop | Every player gets the same opening wallet, and the faction decides what is sold | `CreditGrant::{Covering(purchases), Exactly(n)}`, resolved from **current** catalog prices by one function in contract. `SkirmishSetup` gains per-player `opening_credits` and `offered`. A purchase outside an offer is refused `NotOffered` at `Skirmish::reserve_at`. The offer is published, and the purchase picker shows only offered cards. ([05](slices/05-scenario-battle.md)) |
| C5 | Preparation and the battle address | `prepare` (skirmish request), `prepare-replay`, `prepare-bundle` | Adds a `prepare-scenario { map, name }` message and `contract::preparation::PrepareScenarioRequest`. The address is `/battle?scenario=<map>/<name>`, so the menu tile links `/battle?scenario=ferryman-valley/operation-ferryman`. `PreparationReport` carries `ScenarioPublic`. ([05](slices/05-scenario-battle.md)) |
| C6 | The director (new) | none | `Director { chapters: [Chapter { id, beats: [Beat { id, when: Trigger, audience: [Side], then: [Action] }] }] }`. Triggers read **authoritative facts** (zones, engagement, holds, suppression tier, objectives, what a side knows, the match result), as the referee does. Actions are orders to `DirectedAi` seats, credit grants and offer changes. Its state is digested, it runs the same live and in replay, and its orders go through one `Battle::apply_authored_order` extracted from the fixed-script path (not recorded, because replay reruns the director). ([06](slices/06-director.md)) |
| C7 | `ObservationFrame` and the publication | Script events are digested and never published; the page sees only `referee_view()` | Adds a header word for the open chapter and a `beats` group of `[beat, fired tick]` rows, each sent only to the sides in that beat's audience. The layout carries `chapters` and `beats` id tables. The page reads it as `ScenarioView`. ([07](slices/07-published-progress.md)) |
| C8 | Walkthrough solution (test data) | none | `scenarios/<name>.solution.json`: beats in the director's own vocabulary that order the human player's units. At test time, `ScenarioDocument::with_solution` merges it into the director and puts the human seat under `DirectedAi`. There is no test-only runtime path. ([08](slices/08-walkthrough.md)) |
| C9 | Coach (new, browser presentation) | none | `scenarios/<name>.coach.json`: chapters of steps. Each step has `card` (text with `{key:<command>}` placeholders resolved from `commandBindings.ts`), an optional `line` (a narration id), `until` (`beat`, `ui` or `continue`), optional marks, a camera reel and `pause`. `coachView(doc, ScenarioView, uiSignals)` is a pure function. UI signals come from one typed bus that the input owners emit into. Coach text never enters the sim, so editing a prompt moves no digest. ([09](slices/09-coach-core.md), [11](slices/11-coach-direction.md)) |
| C10 | Narration (battle audio) | `fixtures/voices.json` holds unit voice sets; the generator designs voices through the old Duet proxy | A narration set (speaker Lantern, a library `voice_id`, lines by id) joins the same catalog, lock and judged packs. The generator's transport moves to the `bento` CLI, as scene-assets skins already use. The coach plays a step's `line` through battle audio. ([10](slices/10-lantern-comms.md)) |
| C11 | `Battle::resume(setup, replay, tick)` (new) | `from_replay` refuses input for its whole life | Replays a prefix and then goes live, refusing `AiMemory` if an `ActiveAi` seat played before `tick`, because that memory is outside the digest. Exposed in WASM and as a worker `prepare-resume` with a fresh worker. The pause menu gets "Restart chapter" in scenario battles. ([12](slices/12-chapter-restart.md)) |
| C12 | Menu | Tutorial tile → `/tutorial` page; pause item "Tutorial" | The Tutorial tile → the Ferryman battle address. The `/tutorial` page is deleted. The pause item and modal are "Guide" (`Guide.tsx`). ([01](slices/01-guide.md), [11](slices/11-coach-direction.md)) |

## One owner per concept

| Concept | Single owner | Must never |
|---|---|---|
| Resolved battle setup | `contract::scenario::ScenarioDefinition` | gain a second runtime shape for scenarios |
| Authored player scenario (source) | `ScenarioDocument` + `::on` | carry `overrides`; be read by anything but its resolver |
| Portable package | `ScenarioBundle` (scenario builder) | be the tutorial's format (it freezes rules) |
| Test and film staging on a map | `EncounterDefinition` | appear on a `scenario` map |
| Scenario progression (chapters, beats, cues, grants, offers) | the director, in the sim | read UI state; be rerun from a recorded trace |
| What an AI seat does | its `ControllerKind` (`ActiveAi`, `PassiveAi`, `DirectedAi`) | grow a second, overlapping behaviour field |
| Authored orders reaching the battle | `Battle::apply_authored_order` | have a second admission path |
| Who may buy what | the skirmish offer | be filtered in the picker by its own rule |
| Game facts the coach waits on | director milestone beats | be judged again in TypeScript |
| Instruction overlay, step order, highlights, camera tour | the coach | draw HUD parts itself; read forces |
| Permanent battle readouts | the HUD (army deck, top bar, objective markers) | carry tutorial text or step logic |
| Names of published ids (beats, chapters) | the observation layout | be hard-coded on the page |
| Authored camera moves | the capture tool's `Reel`/`Shot` | get a second "tour" format |
| Command and camera keys | `commandBindings.ts` and the camera bindings | be written into coach text or narration |
| Spoken lines, voices, takes, verdicts | battle audio's voice catalog, lock and packs | ship a take nobody judged; live as loose files in the app |
| How the game is played (reference) | the Guide | be on the main menu |
| Restarting a chapter | `Battle::resume` | snapshot or clone battle state beside the replay |

**Existing duplicates this feature consolidates:** controller kinds gain `DirectedAi` and one shared list, `ControllerKind::ALL` (02); the fixed-script order block becomes `apply_authored_order`
(06); "scenario" stops meaning two things (`encounter_scenario`, 04); "tutorial"
stops meaning two things (the page becomes the Guide, 01); the voice
generator's transport becomes the `bento` CLI (10).

**Duplicates named for later specs.** No slice carries any of these. Each has a
removal condition:

- *Fixed `ScriptedOrder`s against the director.* A fixed order is a beat with an
  `At(tick)` trigger. Fold them once the director ships; 46 fixture files carry
  `scripts`, so it is a mechanical rewrite of its own.
- *The encounter `Defender`, the naive `controller_turn` and authored scripts
  against controller kinds.* Each becomes a controller kind's arm. Remove them when the
  bundle's "economy bundles carry no authored scripts" refusal can go, because a
  seat then runs exactly one behaviour.
- *`ScriptEvent` against beats.* Two named-event concepts. Fold them when bundles
  adopt the director.

**End state ("reads as designed today").** A battle is prepared from a skirmish
request, a replay, a bundle or a scenario document, and all four resolve to one
`ScenarioDefinition`. Every seat has a controller (who commands) and, if it is
AI, its kind says what it does. A scenario's director is part of the battle's
rules: digested, replayed, and publishing chapter and beats the way objectives
publish ownership. The page coaches from published progress and its own UI
signals, keeps its text in data, and never sees the enemy except through its
side's observation. The Guide is one page in the Esc menu. Nothing in the
tutorial is a special case.

## Slice graph

```text
00 probes ──────────────► gates 03, 04, 12's design
01 Guide ─────────────────────────────────────────────► 11 (tile cutover)
02 directed ──────────────┐
03 map ─► 04 document ────┴─► 05 scenario battle ─┐
                └─► 06 director ─► 07 progress ───┴─► 08 walkthrough (ch 1–2)
08 ─► 09 coach core ─► 10 Lantern comms ─► 11 coach direction + tile
08 ─► 12 chapter restart
(11, 12) ─► 13 ch3 ─► 14 ch4 ─► 15 ch5 ─► 16 ch6 ─► 17 ch7 ─► 18 closeout
```

Chapters 1–2 are playable and coached at 11. Restart lands before chapter 3, so
every later chapter is iterated by jumping straight to it.

## Standing rules for every slice

- **Write the failing test first.** A test runs on test units and test maps,
  unless it is a scenario's own check (placement legality, the walkthrough),
  which finds scenarios by directory and names none.
- **Must stay green:** the tests of the crate or package the slice touches,
  never the workspace suite before 18. For a sim change, every existing digest
  it could move stays put, or the move is a named decision in the ledger.
- **Visual gates.** Before adding or changing anything the player sees, load
  [game-ui](../../.agents/skills/game-ui/SKILL.md). Every visual slice ends with
  an unprimed [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md)
  of its shots as the **last** check. It uses
  [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md)
  wherever it has a before shot or a reference. It shows the user its pictures
  inline in the chat as a **non-blocking** checkpoint: allow about five minutes,
  then on silence decide on the evidence, record the decision here and carry on.
- **Mechanic gates.** Any rule change loads
  [tweak-mechanics](../../.agents/skills/tweak-mechanics/SKILL.md) first.
- **Reslice when a slice's ledger keeps filling.** Split it in the spec first,
  then continue.
- **Review before pushing** ([review](../../.agents/skills/review/SKILL.md)).
  Pushes target `staging`.

## Risks

| # | Risk | Cheapest probe or mitigation | When |
|---|---|---|---|
| R1 | The generator can't make the valley (ridge, farm, village, bridge, hill in one small map) | P1: an hour in the map workbench. Places are data, so the story binds to the map, not the reverse | 00 |
| R2 | Restart by replay is too slow late in the tutorial (~45k ticks at 30 Hz) | P2: time a long small battle's replay. If too slow, keep replay as the proof and hold a chapter-start replay in the worker, or reslice 12 | 00 |
| R3 | The walkthrough takes minutes | Time 08's two-chapter run. If it extrapolates past a few minutes, the full walkthrough becomes a closeout-tier check plus per-chapter resume tests | 08 |
| R4 | Pre-placed owned units break settlement, bounties or the unit cap inside a skirmish | P3 | 00 |
| R5 | The roster lacks a story role (radar emitter, SEAD, MANPADS, deployable supply) | P4. Swap the unit in the story, never the rule | 00 |
| R6 | Red's score ends the battle before chapter 7 | `majority_s` in the document's match patch; the walkthrough asserts the human side wins | 04 |
| R7 | A retune flips a scripted fight | Lopsided `UnitCondition`; steps gated on actions; the walkthrough fails by beat name, never by a stat | every chapter |
| R8 | The player soft-locks (loses the only AT team) | Recovery beats (data), plus a director test that a recovery beat re-grants | 14 |
| R9 | The preparation countdown cuts the briefing short | The match patch; a `DirectedAi` red seat is ready at once | 04, 05 |
| R10 | The coach leaks red's state | By type: coach input has no forces. The validator refuses a waited-on beat whose audience lacks the human side | 09 |
| R11 | Narration goes stale when a mechanic or key changes | Lines carry intent only, and keys and numbers live on the card. Re-render a changed line in the same library voice | 10 |
| R12 | Coach steps and the director drift apart (the director moves on while a UI step is still showing) | The coach drops a chapter's unfinished steps when the director opens the next chapter; a coach test covers it | 09 |
| R13 | The scenario map drifts when building templates change | `SOURCES.json` pins the catalogue; map tests refuse a drifted map; regenerate and rerun the placement check | ongoing |

## Alternatives the synthesis rejected

Three drafts were written blind with different emphases: fewest slices, risk
first (Codex), and seam quality. The
[decisions ledger](decisions.md#synthesis) records where they disagreed and why
each call went the way it did. The main ones:

- **The authored-script interpreter as the director** (drafts A and B). It is
  stateless from tick to tick, addresses units by list position, can't buy,
  unload or garrison, and its events are never published. Giving it memory and
  names would be a rewrite, and the coach validator still couldn't know the
  beat list ahead of time. A typed director can be checked at admission.
- **A recorded `Advance { stage }` command from the coach** (draft A). It would
  make the battle's progression wait on presentation. Instead, the director
  never waits on the coach, and the coach pauses the offline battle when a card
  must be read.
- **Growing `EncounterDefinition` into the scenario format** (drafts A and B).
  It would need a category-dependent refusal of `overrides` and lab events, and
  it would mix film staging with player content in one type.
- **A `ScriptedEconomy` bundle mode now** (draft B). Exporting a scenario as a
  bundle is "resolve, then wrap" and needs no new mode. That belongs to a later
  bundle spec.
- **Shipping the tutorial as a `ScenarioBundle`.** A bundle freezes rules and
  catalog, so the tutorial would stop following the roster.
- **A mid-battle spawn action.** Pre-placed idle forces and normal purchases
  paid for by granted credits cover every arrival.
- **Snapshots or cloning for restart.** The interview chose replay, so a cache is
  only P2's fallback.
- **Lip-synced portrait video.** Every line edit would mean a paid re-render;
  the user chose a still portrait with comms effects.
