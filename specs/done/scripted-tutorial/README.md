# Scripted tutorial: Operation Ferryman

**Status:** shipped 2026-11-19. Rationale record: [decisions](decisions.md)
holds what the user decided and how the plan was chosen, [choices](choices.md)
holds the calls made where they were silent, and [story](story.md) is the
approved chapter-by-chapter story.

## What shipped and why

The main menu's **Tutorial** tile opens a real battle: one continuous fight on
the fixed valley map `ferryman-valley`, in seven chapters (briefing, recon,
the crossroads, an ambushed tank, supply and artillery, air and SEAD, the push
to Victory). A coach teaches each mechanic as the story needs it, voiced by
the HQ officer **Lantern**. The old web-page tutorial is the **Guide**, a modal
in the battle's pause menu, and left the main menu.

The tutorial is the first **authored scenario**: a fixed battle on a fixed map
under today's rules and roster. A second scenario that the director's current
vocabulary can tell needs only data (a map folder and its JSON files) to play
at its address; listing it on Home is one `MENU_TILES` row, and a trigger or
order the vocabulary lacks is a simulation change. The user wants more explicit
scenarios on explicit maps, which is why the map category is `scenario`, not
`tutorial`.

The shape follows three constraints the user set:

- **One continuous battle, restartable by chapter.** Units carry forward.
  Restart replays the recorded battle to the chapter's start; there are no
  snapshots.
- **A headless walkthrough guards it.** A test plays the scenario to Victory
  by its solution and fails by the beat that never fired. Fights are lopsided
  by design and steps advance on actions, so routine retunes don't break it.
  The walkthrough and the sim tests pin no stat, loadout, price or text; the
  browser scene finds controls by their accessible names, never by class.
- **Lines carry intent, never keys or numbers.** A spoken key goes stale with a
  rebinding, a spoken number with a retune; the card fills keys in from their
  owners (`{key:…}`, `{camera:…}`) and states no amount.

## Principles and invariants

- **A scenario is a skirmish laid out in advance, and it overrides no unit,
  weapon or catalog rule** (it may patch the match rules, as Frontline does).
  `contract::scenario_document::ScenarioDocument` has no `overrides` field, by
  type, so a shipped scenario always plays the current rules and roster. Its
  one resolver is `ScenarioDocument::on`, which refuses by field; every reader
  (native, WASM, tests) makes its battle there, and all of them end in the one
  runtime shape, `ScenarioDefinition`. Encounters (film and test staging that
  may pin stats) never stand on a `scenario` map.
- **Placed forces are ordinary units.** Each names an owner, enters at its
  current catalog price charged to nobody, and counts for income taper, the
  unit cap and bounties like a bought one. Opening credits and grants are
  computed from current prices (`CreditGrant::Covering`, `ToCover`), so a price
  change never leaves the player short.
- **The shop's offer is enforced by the sim.** A purchase outside a player's
  offer is refused `NotOffered`; the offer is published, and the picker shows
  only what is offered.
- **A directed seat decides nothing.** `ControllerKind::DirectedAi` gives no
  orders of its own, fights at its units' authored engagement, and is moved
  only by the director. A directed seat no person may take counts as ready; a
  person's seat the solution plays readies itself.
- **The director is part of the battle's rules.** `contract::director` holds
  its types and admission, `sim::director` and `sim::battle::direct` run it. It
  is digested, runs the same live and in replay, and runs before orders each
  tick and once more on the tick the match is decided. Its orders go through
  `Battle::apply_authored_order`, the one authored-order path, unrecorded since
  a replay reruns the director; a replay is pinned to its director. Triggers
  read authoritative state (where units are, what a side knows, a unit's
  current order, objectives, the result), never the command log, so a beat
  holds however its state came about. A beat acts through the battle's own
  owners and never places or removes a unit; red's helicopter arrives by an
  ordinary purchase paid by a grant.
- **Chapters close on the story's beats.** A recovery beat and a solution's
  beats are not waited for. Admission refuses a director whose last chapter
  doesn't end on a match result, and any name a beat uses that the scenario
  lacks (zones, forces, players, objectives, cards its player can't buy).
- **Each side hears only its own story.** Every side is told the open chapter;
  a beat is published only to the sides in its `audience`; a frame carries
  each beat's place, never its id string, and the publication's layout names
  chapters and beats. The coach reads only that publication and its own UI acts, never the
  scenario document (which holds red's forces).
- **The coach is presentation, and a pure function.** `coachView` derives the
  step from the coach document, the side's published progress, the UI acts and
  what was dismissed. Coach text never enters the sim, so editing a prompt
  moves no digest. HUD controls are found by `data-hud-anchor`, camera tours
  are capture reels, and keys come from their owners (`commandBindings.ts`,
  the camera controller's `CAMERA_KEYS`, `controlGroups.ts`).
- **Restart is resume.** `Battle::resume` cuts a replay at a tick, replays it
  unpublished and goes live; it refuses when an active AI played before the
  tick (its memory isn't in the digest) or the battle had ended. The page keeps
  a standby client resumed at the open chapter, so Restart chapter is a client
  swap.

## Where it lives

| Concept | Code | Tests |
| --- | --- | --- |
| Scenario document and resolver | `crates/contract/src/scenario_document.rs` | `crates/sim/tests/scenarios.rs` (refusals, prices, placement of every shipped force) |
| Director vocabulary and admission | `crates/contract/src/director.rs` | `crates/sim/tests/director.rs` |
| Director at run time | `crates/sim/src/director.rs`, `crates/sim/src/battle/direct.rs` | `crates/sim/tests/director_facts.rs` (every unit fact and most triggers on test units; `identifies`, `contact_in` and `any` are proven by the walkthrough) |
| Directed seat | `ControllerKind::DirectedAi`, `PlayerState::is_ready` | `crates/sim/tests/directed_ai.rs` |
| Resume and the chapters' dev loop | `crates/sim/src/battle/resume.rs` (`Battle::resume`, `chapter_start`), `crates/game-wasm/src/scenarios.rs` | `crates/sim/tests/resume.rs`; `chapter_start` in `crates/sim/tests/scenario_walkthrough.rs` (`starts_at`) |
| The walkthrough | `fixtures/maps/ferryman-valley/scenarios/operation-ferryman.solution.json` | `crates/sim/tests/scenario_walkthrough.rs` |
| Coach | `web/src/battle/coach/`, `fixtures/maps/ferryman-valley/scenarios/operation-ferryman.coach.json` | `web/tests/coach.test.ts` (every shipped coach against its director) |
| UI signals and HUD anchors | `web/src/battle/input/uiSignals.ts`, `web/src/battle/present/hudAnchors.ts` | `web/scenes/scenario-battle.mjs` (every UI step played in the browser) |
| Chapter restart | `apps/battle-lab/src/useSimSession.ts`, `PauseMenu.tsx` | `web/scenes/scenario-battle.mjs` |
| Narration | `fixtures/voices.json` `narration`; `packages/battle-audio/tools/generate/narration.ts` (generation, through `bento.ts`); `packages/battle-audio/src/voices/narration.ts` (playback) | `tools/generate/transcript.ts` and the lock's verdicts (`tools/generate/lock.ts`) |
| Guide | `apps/battle-lab/src/Guide.tsx` | `web/scenes/ui.mjs` |
| Anti-radiation homing | `crates/sim/src/battle/fire.rs` (`Target::Contact`) | `radar::an_anti_radiation_seeker_rides_the_beam_to_the_radar_not_the_reports_centre` |

Start one chapter in the browser with
`/battle?scenario=ferryman-valley/operation-ferryman&chapter=<id>[&beat=<id>]`.
How to add a chapter is in [fixtures/README.md](../../../fixtures/README.md#saved-maps).

## How the build diverged from the plan

- **A controller kind, not a playbook.** The plan's `Playbook::Directed` became
  `ControllerKind::DirectedAi`, so one field says what an AI seat does.
- **Restart needed a standby.** Probe P2 measured plain replay at about
  3 minutes for 45k ticks, so the page keeps a second worker resumed at the
  open chapter. A restart still rebuilds the page's world (1.8–4.1 s in
  the scene).
- **The valley's hills are authored.** Generated maps were flat when the map
  was made; it began as a generated `open small` map (seed 3), regenerated in
  the `paris` region, and is saved as an authored map with two ridges and a
  farm wall added by hand. Hills on generated
  maps landed on `staging` later; the valley keeps its own relief.
- **One mechanic changed.** An anti-radiation seeker fired into a radar
  contact homes on the live radar behind it ([choices N1](choices.md)). Before,
  it flew to the report's rounded centre and missed a radar it had locked on.
  A paired skirmish probe (Growlers against Buks, five maps) moved from 0–4
  kills a game to 10–11; default AI play kept every digest.
- **Unit voices still use Duet.** Narration goes through the `bento` CLI;
  unit voice design still needs Duet, which Bento's proxy refuses.
- **`pause` steps were never built.** No chapter tours over a moving battle;
  chapter 1's tours run in preparation.
- **The director's vocabulary grew by chapter.** Each unit fact and most
  triggers were tested on test units as they landed; the rest are proven by
  the walkthrough ([choices S9](choices.md)).

## Dead ends

- **The authored-script interpreter as the director.** Stateless from tick to
  tick, addresses units by list position, can't buy, unload or garrison, and
  its events are never published. A typed director is checked at admission.
- **A recorded `Advance` command from the coach.** It would make the battle's
  progression wait on presentation. The director never waits on the coach.
- **Growing `EncounterDefinition` into the scenario format.** It would need a
  category-dependent refusal of `overrides`, and mix film staging with player
  content.
- **Shipping the tutorial as a `ScenarioBundle`.** A bundle freezes rules and
  catalog, so the tutorial would stop following the roster.
- **A mid-battle spawn action.** Pre-placed forces and ordinary purchases paid
  by grants cover every arrival.
- **Snapshots or cloning for restart.** Replay is the proof; the standby is
  its cache.
- **Voice design through Bento.** Its proxy refuses `/v1/text-to-voice/design`,
  and voices created through it expire in 24 h; narration uses a
  shared-library voice (Eryn), which never expires.
- **Lip-synced portrait video.** Every line edit would be a paid re-render; the
  portrait is a still with comms effects.
- **Reseeding a fight that passed by luck.** Adding a force renumbers units and
  reshuffles the seeded streams; each time a fight broke, the fix was physical
  (sight lines, a halt, a worn unit), never a seed.

## Duplicates left for later

- *Fixed `ScriptedOrder`s against the director.* A fixed order is a beat with
  a tick trigger; dozens of fixture files carry `scripts`, a mechanical
  rewrite of its own.
- *The encounter `Defender`, the naive `controller_turn` and authored scripts
  against controller kinds.* Each becomes a controller kind's arm once a seat
  runs exactly one behaviour.
- *`ScriptEvent` against beats.* Fold them when bundles adopt the director.
- *Two ElevenLabs transports.* Duet goes once unit voices no longer need voice
  design.

## Visual provenance

- [assets/ferryman-valley.png](assets/ferryman-valley.png): the valley and its
  story places as first laid out (probe P1). An unprimed review of it renamed
  two zones to match the map; the zones now live only in the scenario
  document.
- Lantern's portrait: the leftmost (F1) of
  [four candidates](assets/portraits/candidates.jpg), generated through Bento and picked to
  match the voice; its master and prompt are
  [`assets/source/portraits/lantern.png`](../../../assets/source/portraits/lantern.png)
  and [`assets/runtime/portraits/provenance.json`](../../../assets/runtime/portraits/provenance.json).
- The look was held to the game's holo-tactical HUD (the `game-ui` skill):
  each coach surface (comms card, ground rings, HUD brackets, the closing card
  over Victory) passed an unprimed screenshot critique, whose findings are in
  [choices S18](choices.md).
