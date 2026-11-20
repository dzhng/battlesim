# Scripted tutorial: choices ledger (final)

This ledger lists the choices the implementing agent made where the plan said
nothing. It was rewritten at closeout against the shipped code, not the pass
each choice first landed in: choices a later pass overturned are gone,
duplicates are merged, and every "waits for slice N" entry says how it ended.
What the user decided (the interview) and what the plan decided (the synthesis
of three drafts) are in [decisions.md](decisions.md), not here.

Entries are grouped by verdict: **needs-user** (decide; each has a provisional
call already taken), **unsound** (redo, from the corrected decision) and
**sound** (the architecture you now own). Within a group the least confident
entry comes first. Confidence is how sure the audit is that you would have made
the same call.

**Review these first:** N2 (the tutorial's fights are rigged with worn and
disarmed enemies) and N3 (the agent judged all 58 narration takes itself). N1
(anti-radiation missiles now always find their radar) was decided by the user
at closeout: keep.

A few terms used throughout:

- **Director:** the story's script inside the simulation. It is a list of
  *chapters*, each a list of *beats*. A beat has a *trigger* (a fact about the
  battle, such as "blue owns the crossroads") and *actions* (an order to a red
  unit, a grant of credits, a change to the shop).
- **Coach:** the browser's teaching layer: the card at the top left, Lantern's
  voice, camera tours, rings on the ground and brackets round HUD controls. It
  reads only what blue's side is told.
- **Directed seat:** a player seat (`ControllerKind::DirectedAi`) that decides
  nothing itself; only the director moves its units.
- **Solution:** a file of extra beats that plays blue's side, so a test can play
  the tutorial to the end without a person.
- **Walkthrough:** that test (`crates/sim/tests/scenario_walkthrough.rs`).
- **Digest:** the hash of the whole battle state. Same inputs, same digest; it
  is how the repo proves a change didn't alter outcomes.
- **Slice NN, probe Pn, and plan ids (C7, R8):** the build plan's numbered
  steps (slices 13–17 built chapters 3–7), the probes run before building, and
  rows of the plan's contract and risk tables. The plan is gone; the probes
  are recorded in [decisions.md](decisions.md).
- **SEAD:** suppression of enemy air defences: aircraft hunting radar SAMs
  with anti-radiation missiles.
- **Duet and Bento:** two services that reach ElevenLabs (voices and sound)
  for this repo, each with its own login.

---

## Needs-user

### N1. An anti-radiation missile fired at a radar contact homes on the live radar
- **When:** slice 16 (chapter 6).
- **The choice:** a radar contact is what a side's receivers report about an
  enemy radar: an area, reported in 100 m range steps, with a centre that isn't
  the radar itself. Before, a missile fired at that contact flew to the area's
  centre. In chapter 6 the Growler's anti-radiation missiles locked on the Buk
  and then fell 50 m beside it, which no viewer would believe of a missile that
  rides a radar beam. Now a weapon whose catalog row says `anti_radiation`,
  fired at a radar contact, steers to the emitting radar behind it while that
  radar lives. Every other round aimed at a contact still aims at the area, and
  the firing side learns nothing new about where the radar is. The rule is in
  `crates/sim/src/battle/fire.rs`, pinned by
  `radar::an_anti_radiation_seeker_rides_the_beam_to_the_radar_not_the_reports_centre`.
- **The gap:** the plan said "swap the unit in the story, never the rule". There
  was no unit to swap: every SEAD missile behaved this way.
- **The reach:** skirmish balance. A paired probe at closeout (the skirmish
  tournament, us against eastern, blue buying only Growlers and red only Buks,
  five maps, 600 s each) showed the shift. Before: the Growlers killed 0 to 4
  Buks a game and lost up to six jets. After: they killed 10 or 11 Buks every
  game and lost none. Default AI play didn't move: the nine-row skirmish corpus
  and fourteen full self-play games across three faction pairings kept every
  digest, because the default shopping didn't buy SEAD into them. Three
  aircraft carry the missile: the Growler, the Tornado ECR and the Su-30SM2.
- **Verdict:** decided by the user at closeout: keep it ("a good find"). The
  rule is right as physics; it makes SEAD a hard counter to radar SAMs where
  it used to be nearly useless. The usual counter, a crew that switches its
  radar off (emission control), isn't modelled yet and would be a balance pass
  of its own.
- **Confidence:** high (the user's call).

### N2. The tutorial's enemies are worn, disarmed or held back so blue wins
- **When:** slices 08, 13, 14, 16 and 17.
- **The choice:** the plan asked for lopsided fights, so a retune wouldn't flip
  a scripted fight. The agent got there by changing each red force's
  condition and orders in the scenario document, never the rules. The outpost
  starts with its RPG rounds spent (it killed the Bradley before chapter 3 could
  teach anything). The counter-attack squad starts with four casualties and no
  rockets, behind a house, holding fire until sent. Chapter 4's T-72 starts at
  10 hull points, so any Javelin hit kills it, approaches holding fire, and halts
  15 s in the AT team's view. The Buk's missiles are spent. Chapter 7's
  defenders are two worn squads that walk onto the flags 30 s in. The scenario sets the
  match's majority hold (`majority_s`) to ten hours for both sides, so nobody
  wins on the score; blue wins by holding every flag at once, and red never
  takes one.
- **The gap:** the plan named the principle (lopsided, gated on actions) but not
  how far to go.
- **The reach:** a player who replays the tutorial looking for a challenge
  won't find one, and the fights don't teach real matchups (a full T-72 shrugs
  off a frontal Javelin hit that kills this one). Retunes mostly can't break the
  walkthrough, which was the aim.
- **Verdict:** needs-user (taste). Provisional call: keep it rigged; the
  tutorial teaches controls and ideas, not balance. To make a fight fairer,
  edit that force's `condition` or orders in
  `fixtures/maps/ferryman-valley/scenarios/operation-ferryman.json` and rerun
  the walkthrough.
- **Confidence:** medium.

### N3. The agent judged all 58 narration takes good, by speech-to-text
- **When:** slice 10.
- **The choice:** every narration line has to be judged before it ships, as
  unit voices are. Lantern's 58 lines were rendered once, each checked word for
  word by speech-to-text (the only differences were armour/armor and
  treeline/tree line), imported as raw masters (the purchased audio before
  processing) and marked `ok` in the voice lock (`fixtures/voices.lock.json`)
  by the agent. No person listened to all of them.
- **The gap:** the pipeline needs a verdict per take and nothing said who gives
  it.
- **The reach:** a take with a bad delivery (wrong emphasis, odd breath) ships
  until someone hears it.
- **Verdict:** needs-user (taste). Provisional call: ship the agent's verdicts.
  To reverse one, mark the take bad in the sound workbench (or
  `generate verdict`), bump `narration/lantern/<line>` in `voices.json`'s
  `retakes`, and run `generate narration`, which buys only that line again in
  the same voice.
- **Confidence:** medium.

### N4. Unit voices still go through Duet; only narration uses Bento
- **When:** slice 10.
- **The choice:** the plan said the voice generator's transport moves to the
  `bento` CLI. Narration does go through Bento
  (`packages/battle-audio/tools/generate/bento.ts`). Unit voices still need
  ElevenLabs' voice design, which Bento's proxy refuses, so they stay on the
  old Duet client. Retiring Duet waits until voice design has another route.
- **The gap:** the plan assumed Bento could do everything Duet did.
- **The reach:** the generator keeps two transports. Anyone generating unit
  voices, effects or music still needs the Duet login.
- **Verdict:** needs-user (external service). Provisional call: two transports
  until Bento admits voice design. To reverse, move every ElevenLabs call
  (unit voices, designs and picks, beds, effects and music) onto Bento, which
  for unit voices means library voices instead of designed ones; then delete
  `duetClient`.
- **Confidence:** medium.

### N5. Two bugs found and left for their owners
- **When:** slices 13 and 17.
- **The choice:** two engine bugs turned up while building the solution, and
  neither was fixed here.
  - A carrier given a move order, then told to unload where it stands, keeps
    its squad aboard forever. The same unload without the move works, and so
    does an unload after an attack-move (chapter 7's solution does that at the
    bridge). Chapter 3's solution unloads 3 s after the carrier reaches the
    farm wall.
  - A Javelin gunner also carries a rifle and swaps to it whenever the
    Javelin's shot stops assessing as good, which resets his aim. Against a
    moving tank behind flickering cover he never fires. The tutorial's tank now
    halts 15 s in view.
- **The gap:** both are outside the tutorial's owners (carriage and movement;
  the weapon owner).
- **The reach:** a player who presses unload while driving meets the first; a
  player ambushing a moving tank meets the second.
- **Verdict:** needs-user (scope). Provisional call: separate fixes, each with
  its own test, outside this spec.
- **Confidence:** medium.

### N6. A chapter restart rebuilds the battle's world
- **When:** slice 12.
- **The choice:** "Restart chapter" swaps in a standby battle that is already
  at the chapter's opening (S7 below), so no replay runs when you press it. The
  page still rebuilds its world: the battle view drops its art while a new
  client has no observation, as the plain Restart always has. The browser
  scene measured 1.8 s from the confirmation to the chapter's first card when
  it was built, and 4.1 s in the closeout's full run on a busy machine.
- **The gap:** the plan asked for restart by replay and said nothing about the
  page's art.
- **The reach:** restart shows a short loading moment.
- **Verdict:** needs-user (feel). Provisional call: accept a few seconds. Keeping the
  art across a restart means changing how the battle view loads and drops its
  art with each new client, outside this spec.
- **Confidence:** medium.

---

## Unsound

None survived the closeout audit.

---

## Sound

### S1. Chapter 7's defenders move on a timer, not on blue's approach
- **When:** slice 17.
- **The choice:** the story says the last defenders step onto the bridge and
  hill as blue comes. No selector names a whole side, and the beat was written
  on a timer (30 s after the chapter opens) rather than on red's own
  `contact_in` near the flags.
- **The gap:** the vocabulary grew only as chapters needed it.
- **The reach:** a slow player meets defenders already on the flags. The story
  still reads.
- **Verdict:** sound for one scenario; a second scenario that wants "when the
  enemy comes near" can use `contact_in` for the defending side, or add a
  side-wide selector, rather than another timer.
- **Confidence:** medium.

### S2. A directed seat is a controller kind, not a playbook beside one
- **When:** slice 02.
- **The choice:** the plan's draft called it `Playbook::Directed`. But controller
  kinds already say what an AI seat does (`ActiveAi` plays, `PassiveAi` holds
  fire), so a separate playbook field could say "passive controller, directed
  playbook", two owners of one question. `DirectedAi` joined
  `ControllerKind`. It is set like any controller: at setup, or by a recorded
  controller change. The walkthrough sets it at setup (`with_solution` makes
  every human-capable seat `DirectedAi`); the chapters' dev loop uses a
  recorded change to hand the seat back to a person. Battle addresses spell it `d`.
- **The gap:** the plan named the behaviour, not where it lives.
- **The reach:** every future AI behaviour is another controller kind.
- **Verdict:** sound.
- **Confidence:** high.

### S3. A directed seat no person may take counts as ready; a person's seat says so itself
- **When:** slice 02; narrowed at closeout.
- **The choice:** preparation ends when every seat is ready. Red's directed
  seat never presses Ready, so `PlayerState::is_ready` counts it as ready by
  rule, and the stored `ready` flag (in the digest) still says only what the
  seat did. At first the rule covered every directed seat. That had a cost the
  closeout review found: the walkthrough hands blue's seat to the director, so
  blue counted as ready too, preparation ended on the first tick, and the test
  bought chapter 1's units mid-battle, which no player does. Now the rule
  covers only a directed seat no person may take (`human_capable` false). A
  person's seat played by the solution says Ready itself, as the person would,
  so the walkthrough plays preparation the way a player does.
- **The gap:** the plan said "counts as ready", not how.
- **The reach:** a generated battle with a `d` seat starts when its humans are
  ready; a scenario's solution readies its own seat.
- **Verdict:** sound.
- **Confidence:** high.

### S4. Forces are named, and the battle numbers them in name order
- **When:** slice 04; consequence found in 15 and 17.
- **The choice:** a scenario's forces are a map from a story name to a unit
  (`"outpost": {...}`), so the director and coach can say "the outpost". The
  battle gives them unit ids in name order. Consequence: adding a force
  renumbers every bought unit after it and reshuffles the seeded random streams,
  so a fight that passed by luck can fail. That happened twice (the treeline
  team, then chapter 7's forces), and each time the fix was physical (the
  ambush rebuilt on sight lines; the tank halting), never a reseed.
- **The gap:** the plan said "tagged forces".
- **The reach:** every new force in a scenario can move its outcomes; the
  walkthrough catches it by beat name.
- **Verdict:** sound; a scenario that only passes on one seed isn't robust.
- **Confidence:** high.

### S5. A placed force enters at today's price, charged to nobody
- **When:** probe P3 and slice 04.
- **The choice:** a skirmish refused pre-placed units, because every unit
  carries an entry price that income taper, the unit cap, refunds and bounties
  read. Now a skirmish admits authored forces when each names one of its
  players as owner. Each gets an entry at its current catalog price at tick 0,
  charged to no wallet and shown nowhere as spending. Killing red's placed tank
  pays blue the normal bounty. Events, scripts, an opponent and encounters stay
  refused in a skirmish, and so do authored scripts outright.
- **The gap:** P3 found the refusal; the fallback's price wasn't decided.
- **The reach:** a scenario-builder economy bundle may carry owned forces too,
  under the same rule.
- **Verdict:** sound.
- **Confidence:** high.

### S6. The shop's offer is held as unit types and published to its owner
- **When:** slice 05.
- **The choice:** a scenario narrows a player's shop to named cards. The
  skirmish resolves those names to `(kind, carrier)` pairs when the battle is
  built, because the publication writer has no catalog. The publication gains
  an `offers` group and a `shopNarrowed` flag, so a shop narrowed to nothing
  still reads as narrowed. A purchase outside the offer is refused
  `NotOffered`, counted as malformed (the page never shows such a card). In a
  narrowed shop, empty tabs hide; a full faction shop never has empty tabs. A
  squad offered only riding opens its menu on its ride.
- **The gap:** the plan named the offer, not its shape on the wire.
- **The reach:** parity fixtures were re-blessed for the layout; a future
  campaign shop reuses the same group.
- **Verdict:** sound.
- **Confidence:** high.

### S7. Restart is a cut replay with a standby battle waiting
- **When:** slice 12.
- **The choice:** `Battle::resume` cuts a replay at a tick (commands,
  controller changes, script trace, seat events), replays it with nothing
  published, and lets the battle go live; its record continues from the
  prefix. It refuses when an AI with memory outside the digest (an active AI seat, or an
  encounter's opponent) played before the tick, or when the battle had
  finished by then. Plain replay
  was too slow late in the tutorial (probe P2: about 3 minutes for 45k ticks),
  so the page builds a standby: when it first sees a chapter, it asks the
  playing battle for its replay and starts a second worker that resumes at that
  publication's tick. "Restart chapter" hands the session that client. If
  publications batch ticks, the restart point is the chapter's first published
  tick, not its exact opening.
- **The gap:** P2's fallback was "hold a chapter-start replay in the worker";
  the shape was open.
- **The reach:** a second worker and battle per scenario battle (memory for
  speed); restart for any other battle type would reuse it.
- **Verdict:** sound.
- **Confidence:** high.

### S8. The chapters' dev loop starts a solved battle and hands blue back
- **When:** slices 13 and 17.
- **The choice:** to iterate chapter 5 you shouldn't have to play chapters
  1–4. `/battle?scenario=<map>/<name>&chapter=<id>[&beat=<id>]` prepares the
  scenario with its solution merged in, plays it to where the chapter opens
  (or the beat fires) through `Battle::chapter_start`, then hands every
  human-capable seat back to a person. A human seat takes no director orders,
  so the solution goes quiet. A solution's beats are never waited for (as a
  recovery beat isn't): a chapter closes on its story's beats, so a person who
  plays a step differently from the solution still moves the story on. Before
  the closeout review, a solution beat such as "the mortar sets up at the
  supply point" could hold a chapter open forever in the dev loop. The solution file ships to the page for this; a
  tutorial's solution is no secret. A start on the tick that decided the match
  resumes one tick earlier, which decides it again, so the Victory frame can be
  shot. A scenario battle's player is its human-capable seat.
- **The gap:** slice 12 found the walkthrough's replay belongs to the solved
  battle, which can't resume into the shipped one.
- **The reach:** every future scenario gets the dev loop for free.
- **Verdict:** sound.
- **Confidence:** high.

### S9. The director's vocabulary grew chapter by chapter, each word tested on test units
- **When:** slices 06 and 13–17.
- **The choice:** triggers, unit facts and directed orders were added only when
  a chapter needed them. Admission checks every name in one walk, including
  every unit type a selector, trigger or action names, against what that
  player may buy, so a renamed card refuses the scenario instead of stalling a
  beat. Facts: inside, suppression, attack-moving, popped
  smoke, reversing, ordered (attack / attack ground), deployed, serviced,
  airborne, arrived. Triggers: objective, destroyed, pending, radar contact,
  match result. Orders: unload, occupy, attack-move, withdraw, pop smoke,
  reverse, attack, deploy, attack a contact in a zone. Actions: buy, ready,
  open shop, and a grant `to_cover` (only the wallet's shortfall). `destroyed`
  is a trigger, not a fact, because a fact reads the living.
  `attack_contact_in` picks a ground contact, a radar's report first. Each fact and
  trigger has a test on test units (`crates/sim/tests/director_facts.rs`);
  the directed orders are proven where the solution leans on them, so the
  walkthrough fails by beat name if one breaks.
- **The gap:** the plan delegated field names to each slice.
- **The reach:** a second scenario writes in this vocabulary and adds words
  the same way.
- **Verdict:** sound.
- **Confidence:** high.

### S10. Recovery beats aren't waited for; the last chapter must end on the result
- **When:** slices 14 and 17.
- **The choice:** a recovery beat answers trouble (the AT team dies before the
  tank: grant credits for another). A chapter closes when its other beats
  fired, and the walkthrough doesn't demand recovery beats. A recovery beat
  fires at most once. Separately, admission refuses a director whose last
  chapter doesn't end on a match result, and a battle decided this tick runs
  its director once more so a beat can answer the result. Nothing ticks after.
- **The gap:** The plan's risk list named recovery beats, not their effect on closing a chapter.
- **The reach:** every test director needs a closing chapter (the `closing()`
  helper).
- **Verdict:** sound.
- **Confidence:** high.

### S11a. A replay is pinned to its story, and a withdrawn unit isn't destroyed
- **When:** closeout review.
- **The choice:** two fixes the whole-spec review found. First, a replay
  records commands but not what the director did, so a replay of the tutorial
  played under an edited story would have run a different battle without
  complaint. The replay's identity now includes the director, only when there
  is one, so every other battle's identity is unchanged. Second, the
  `destroyed` trigger counted a unit that left the battle: a player who sent
  the Javelin team home got its refund and then the recovery grant for losing
  it. Now `destroyed` means killed, not withdrawn.
- **The gap:** neither case was in the plan.
- **The reach:** an old tutorial replay stops loading after a story edit,
  which is the point.
- **Verdict:** sound.
- **Confidence:** high.

### S11. Orders for a seat a person plays are skipped silently
- **When:** slice 06.
- **The choice:** directed orders (and `buy`, `ready`) act only on a directed
  seat. An order for a human seat is dropped, not refused. That is what makes
  one file serve both the walkthrough (blue directed, solution plays) and a
  person's battle (blue human, solution inert).
- **The gap:** the plan didn't say what happens to an order with no directed
  seat to take it.
- **The reach:** a beat that names the wrong seat (a real seat a person plays)
  goes quiet rather than failing; a name no seat answers to is refused at
  admission.
- **Verdict:** sound.
- **Confidence:** high.

### S12. The director runs before orders, and identification reads the side's track
- **When:** slice 06.
- **The choice:** the director runs each tick just before orders apply, so a
  beat's order takes effect on the tick it fires. Because it runs before this
  tick's sensing, "blue identifies the outpost" reads blue's track (known
  within its grace period), not "seen this very tick".
- **The gap:** the plan said "runs the same live and in replay", not when.
- **The reach:** any trigger on knowledge is one sensing tick behind.
- **Verdict:** sound.
- **Confidence:** high.

### S13. A published beat is its place in the director; every side sees the open chapter
- **When:** slice 07.
- **The choice:** the publication header carries the open chapter (−1 without
  a director) and a beat count, and a `beats` group of `(place, tick)` rows,
  each sent only to the sides in that beat's audience. The layout's `chapters`
  and `beats` tables give the page names. A battle without a director
  publishes no scenario.
- **The gap:** The plan named the published progress, not its encoding.
- **The reach:** the page can't learn red-only beats; parity fixtures were
  re-blessed once for it.
- **Verdict:** sound.
- **Confidence:** high.

### S14. The coach is a pure function of published progress, UI acts and dismissals
- **When:** slices 09, 11, 12 and 17.
- **The choice:** `coachView` takes the coach document, blue's published
  scenario progress, the UI acts the player made and what they dismissed, and
  returns the step to show. It never loads the scenario document (which holds
  red's forces). Its state is keyed by the session's client, so a restart
  re-derives it from the publication alone. Rules it keeps:
  - a chapter's unfinished steps drop when the director opens the next one;
  - a Continue card before a step whose beat was already told is done (a
    restarted run finds it so), but a step that asks for an act still waits
    for the act, because beats fire in any order;
  - once every chapter has closed, the last chapter's steps stay; a battle
    decided in its last chapter keeps that chapter open, since nothing ticks
    after the result. Either way the closing line plays over the result;
  - a UI step is done once each signal it lists has been heard since its
    chapter opened, never a count of them (the card asks for two groups; one
    `group_set` completes it).
  The coach-against-director check is a test over the shipped files
  (`web/tests/coach.test.ts`), not a runtime check.
- **The gap:** the plan named the coach function, not these rules.
- **The reach:** every future scenario's coach behaves the same way.
- **Verdict:** sound.
- **Confidence:** high.

### S15. UI acts come from where each act happens, on one bus
- **When:** slices 09, 11 and 14.
- **The choice:** camera moves come from the viewport's one camera entry,
  selection from unit control, a facing move from the one order sender, the
  shop from the picker, contact focus from the viewport's jump, a control group
  from the group owner (`group_set`, which was declared but never emitted until
  chapter 4's browser check caught it). Keys come from their owners too: the
  contact-focus key joined the bindings (N), group keys come from
  `firstGroupKeys` (the control-group owner), and a command button names its binding once, which gives it
  both its key label and its `command:<name>` anchor.
- **The gap:** the plan said "one typed bus", not its sources.
- **The reach:** a new coach step on a new act needs its owner to emit it.
- **Verdict:** sound.
- **Confidence:** high.

### S16. Reaction lines are spoken when their moment comes
- **When:** slice 16.
- **The choice:** a step's line used to be spoken as the step appeared, so
  "Tank down" played before the tank died. A step marked `reacts` speaks its
  line when its gate is met, before the next step's line, one after the other
  (`linesBetween`). A step a closing chapter dropped says nothing. Lines queue
  and play one after the other: a step's own line is cut short when the next
  step has something to say, step lines still waiting drop when the step
  changes, and a reaction is always heard to its end. Unit chatter holds new
  lines while the narrator speaks.
- **The gap:** found while writing chapter 6.
- **The reach:** coach authors mark reactions in data.
- **Verdict:** sound.
- **Confidence:** high.

### S17. Narration is a section of the voice catalog, through an earpiece profile, without captions
- **When:** slice 10.
- **The choice:** `voices.json` gains `narration` (speakers with a library voice
  id, model, seed and processing profile; lines by coach line id). Takes share
  the unit voices' lock (`narration/<speaker>/<line>`), cache keys, verdicts and
  pack rule. Lantern goes through `voice-earpiece`: the radio profile mixes a
  tank-engine bed under the voice, which reads as a crew member, not HQ. The
  panel shows no caption, because the card already says what to do. The signal
  bar runs for the take's length rather than its live level.
- **The gap:** the plan named the narration pipeline, not its shape.
- **The reach:** a second narrator is a new speaker row.
- **Verdict:** sound.
- **Confidence:** high.

### S18. The coach's look: comms card, accent rings and HUD brackets
- **When:** slices 10, 11 and 17, each after an unprimed critique.
- **The choice:** the card sits beside a monochrome portrait behind scan lines,
  flush with the HUD's top edge, keys drawn as the Guide's key tabs. A ground
  mark is a dashed ring in the HUD accent: in the zone colour it read as a
  fourth objective. A HUD highlight is the HUD's corner brackets in the accent,
  following the control's rectangle, found by a `data-hud-anchor` attribute (no
  registry, no class names). Once the match ends, the closing card docks
  bottom left (at top left it hid VICTORY) and loses its Continue.
- **The gap:** the plan named the parts, not their look.
- **The reach:** the critique's leftovers were recorded, not fixed: unit
  readouts overlap when units stand close, the army ring blends into the tray,
  and the match report draws the winner's score no bigger than the loser's.
  Each belongs to its own owner.
- **Verdict:** sound.
- **Confidence:** medium.

### S19. A camera tour is one pilot for the battle's life, ending where it stops
- **When:** slice 11.
- **The choice:** the viewport reads its camera pilot once, so
  `useScenarioCoach` mounts one pilot for the battle's life (its `tour`) that
  flies a step's reel and returns nothing when it ends. The player's camera carries on from the tour's last framing instead
  of snapping back. Tours never fade to black or follow a unit; the validator
  refuses either. The cursor's order action is disabled only while a pilot is
  flying, not while one is mounted. `pause` steps were never built: no chapter
  tours over a moving battle.
- **The gap:** the plan named tours and listed `pause` steps.
- **The reach:** a future tour over live action needs `pause` added then.
- **Verdict:** sound.
- **Confidence:** medium.

### S20. The valley is a flat generated map with authored hills, in the paris region
- **When:** probe P1.
- **The choice:** generated maps were flat when the map was made, so the agent
  took the best of 60 requests (`open small`, seed 3), regenerated it in the
  `paris` region so the village reads European, and authored two ridges (14 m
  under the wood by blue's road, 26 m for the radar on red's bank) and a farm
  wall. It is an `authored` map pinning its own content hash and templates.
  Bridge and hill are each other's counterparts, which only map analysis reads.
  Hills on generated maps have since landed on `staging`; the valley keeps its
  authored relief.
- **The gap:** the plan expected the generator to make the valley.
- **The reach:** template changes can drift the map; map tests refuse a drift
  and the placement test checks every force.
- **Verdict:** sound.
- **Confidence:** high.

### S21. Home's tiles are their own list
- **When:** slice 11.
- **The choice:** `MENU_TILES` holds `{ id, title, path }`, where a path is a
  menu page or any address, so the Tutorial tile is the battle's address. Menu
  pages no longer carry a `tile` flag.
- **The gap:** the cutover needed a tile that isn't a page.
- **The reach:** a future campaign tile is one row.
- **Verdict:** sound.
- **Confidence:** high.

### S22. Smaller calls
- `MatchRules::patched` is the one match-rule patch; Frontline goes through it
  too, refusing a key the rules lack (slice 04).
- An authored scenario doesn't open from a saved replay file; restart replays
  inside the session (slice 05).
- A capture refresh re-records its battle digests, in its own commit, after the
  paris-corner capture's digests went stale on `staging` (slice 05).
- No `scenario_report` example: the placement test is the legality oracle
  (slice 04).
- "Serviced" counts any of the units blue fields before chapter 5 (Bradley,
  rifle squad, scouts, Javelin team), so whichever of them the player brings to
  the supply truck counts (slice 15).
- The `ui` scene checks the Guide inside the pause menu, and the old tutorial
  page's baseline was deleted with it (slice 11).
- **Verdict:** sound. **Confidence:** high.
