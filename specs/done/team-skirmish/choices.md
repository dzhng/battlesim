# Team skirmish: choices ledger

This is the final record of every decision behind team skirmish: battles from 1v1 to 5v5 where each side is several players, each with its own money, unit cap, army and controller (a human or an AI). Every entry describes what is true in the code now. Earlier amendments are collapsed into their final form, and an entry mentions what it replaced only where that still matters.

Every entry was re-checked against the shipped code (main at `8b7f37b89`, 2026-10-19): slices 00–13c, the slice 14 decisions, and the closing passes that fixed what the whole-spec review and this audit found (web, dispatch, `ai_controlled`, Rust and held-items passes). No entry is left unsound.

## How to read this

Entries are grouped by verdict, and each group maps to an action:
- **Needs you**: decide. Each entry carries the call the code makes today, which stands until you change it.
- **Unsound**: redo. Each entry names the decision the work should be redone from.
- **Sound**: acknowledge. This is the architecture you now own. An entry that was unsound in the earlier draft and has since been fixed says so in its first line ("Corrected in …").

Within each group, entries are ranked least-confident first. *Confidence* is how sure the audit is that you would have made the same call.

Ids are stable:
- `D#` entries are planning decisions, from the 2026-10-16 walk and the spec synthesis.
- `L#` entries are landmines found in the planning sweep.
- `I#` entries are choices the implementation made. I53–I56 are new in this version.

The last section maps every row of the old ledger, and every closing-pass decision, to its entry here.

**Who decided:**
- **user**: you decided it. Your own words are quoted where the record has them.
- **user (earlier session)**: you said it in an earlier conversation.
- **agent on your behalf**: an agent decided it for you. During planning that was under the six preferences below. During implementation it was at a non-blocking review where you didn't reply within the window.
- **agent**: an implementation call the plan left open.
- **code**: the existing code forced it.

**Your six preferences.** These are what the planning delegation decided under:
1. Every player is a full player: its own wallet, cap, army and logistics, never a share of the side.
2. What a side shares follows from where things physically are, not from ownership rules.
3. The AI acts like a competent war-film army: concentrated combined-arms groups, with extra players taking more objectives at once.
4. One general system from 1v1 to 10v10. A 1v1 is not a special case.
5. Reuse existing mechanisms rather than add new machinery.
6. Lobby rules stay simple and predictable.

**Words used throughout:**
- **Side**: Blue, Red, or Neutral (scenery and civilians).
- **Player**: one seat on a side, with its own wallet, 30-unit cap, army, ready flag and controller. The code calls it `Player`.
- **Controller**: who drives a player: Human, active AI, or passive AI (an AI that only returns fire).
- **Entry**: a road on a side's back edge where bought units drive onto the map. A side has 1–3 entries, ordered centre, left, right.
- **Lane**: an entry seen as one AI player's sector of the front.
- **Objective**: a capture zone. Holding most of them wins.
- **Digest**: a hash of the simulation state. The simulation is deterministic, so an unchanged digest proves a change didn't alter any battle. A *re-baseline* is a deliberate, named change of the pinned digests.
- **Doctrine**: one AI player's brain, `BalancedDoctrine`.
- **Team planner**: the per-side coordinator in `crates/sim/src/skirmish_ai/team/`. It gives each AI player its lane, objective and orders to cooperate.
- **Influence**: the planner's reading of "how strong are we here": friendly strength minus visible enemy strength around a point.
- **Strength / value**: what a unit counts for in the planner, since slice 13c (D56). Value is a unit's cost, counted only against the enemy kinds its weapons can hit. Strength is value divided by the roster's mean card cost, so an average unit is about 1.
- **Party**: a few units detached from an AI's main group to one objective: help, hold or claim (D55).
- **Tournament**: the headless AI-versus-AI batch run in `crates/mapgen/examples/skirmish_tournament/`.
- **Preview**: the lobby's background run that generates and admits a map before you deploy.
- **Admission**: the check that a generated map is legal and fair, `sim::map_analysis::admit_skirmish`.
- **Link**: a battle's shareable address, `/battle?…`.

## Review these first

1. **I48, an outgunned lane now asks for help even when its reading has stopped falling.** In a 1v1 there is no ally to answer, so a beaten lane calls repeatedly and nobody comes; in team games about nine calls in ten are answered.
2. **D2, every AI now fields tanks, and you pay for it.** The armour fix landed. Because a tank doesn't fit in the opening budget after air defence, the AI saves for one: on 3v3 seed 4 it takes one objective fewer within 900 s. With D56's other changes, the 1v1 seed 1 match now runs 35 minutes (it still ends with a winner).
3. **I49, nothing shows where an ally's units are going.** Help works, but a human ally can't see it coming.

---

## Needs you

### I49 · Nothing shows where an ally's units are going
- **When:** slice 13 review checkpoint. **Who:** agent on your behalf (you didn't reply to the shots).
- **The choice:** You play a 2v2 with one AI ally. Your squad is fighting for an objective, and the AI ally sends a detachment to help you. On screen you see the ally's pale-ice units moving, but nothing tells you they are coming to *you*: there is no route line, no destination marker, and nothing on hover. Two unprimed critiques of the `ally-help` shot said the enemy holding your objective reads clearly, but the ally's intent does not. Today nothing like that is built, because it would be new UI the plan didn't include.
- **The gap:** The plan made help work, but never said how a human learns that help is coming.
- **The reach:** The 5v5 experience with a human depends on reading allies' intent. Any intent cue is new HUD work that would use `game-ui`.
- **Verdict:** needs you. **Provisional call:** leave it unbuilt. **To reverse:** add an ally route or destination cue on hover, or while a help detachment is on its way. It would read the ally's published orders, which the client already receives but doesn't draw (ally orders are deliberately hidden today, D24).
- **Confidence:** low.

### I38 · The match report's story reads like an activity log
- **When:** slice 11. **Who:** agent on your behalf (critiques; you didn't reply).
- **The choice:** After a 5v5 ends, the report's timeline lists the battle's story: each player's purchases, refunds and every kill. To keep it short, a player's purchases in the same match minute fold into one line ("BLUE 3 · AI bought 4 units, 1,250 credits"). Kills each keep their own line. Two unprimed critiques still called the result "an activity log": in a 5v5 the opening minutes are mostly purchases. There is no way to filter by player, and no fighting-only view.
- **The gap:** Slice 11 delegated how to collapse the timeline at 5v5. It didn't say what the story is for.
- **The reach:** This shapes how the report scales to 10 players and any future post-match analysis.
- **Verdict:** needs you. **Provisional call:** keep the per-player-per-minute collapse, with no filter. **To reverse:** add a per-player filter to the story, or show fighting only by default with purchases behind a toggle. Both are local to `MatchResultOverlay.tsx`.
- **Confidence:** low.

### I32 · How strongly allies stand out from your own units
- **When:** slice 10 review checkpoint. **Who:** agent on your behalf (you didn't reply).
- **The choice:** In a 2v2, your units and your ally's are both on Blue. Yours keep the HUD's Blue. Every ally is pale ice (`#b9f0ff`, D24). Ally panels hang on a dashed leader line and underline, and a selected ally's ground ring is dashed with no facing arrowhead. The unprimed critique said "allies are too subtle at a glance". With no reply, the slice landed as built.
- **The gap:** D24 fixed the hue. The shape cue was delegated, and how strong it should be was left to judgement.
- **The reach:** Any future ally marking (names, minimap, native HUD) should copy whatever you settle here.
- **Verdict:** needs you. **Provisional call:** as built. **To reverse:** add an ally marker at the start of the unit's name line. That puts the cue on text, so it doesn't depend on a ring you only see when the unit is selected.
- **Confidence:** medium.

### I16 · Which links may hold humans, and what a link may leave out
- **When:** slice 06; tightened in the web lane. **Who:** agent.
- **The choice:** A battle link names each side's players by controller, for example `blue=h,a&red=a,a` (Human, AI; AI, AI). The parser, `askedBattle` in `apps/battle-lab/src/battleLinks.ts`, is the one place a link is checked. Its human rule is now the same at every size:
  - a link with no Human is an AI-only battle you watch (D34), at any size, 1v1 included (`blue=a&red=a` used to be refused, and now opens);
  - a link with any Human has exactly one, on Blue. So two Humans, or a Human on Red, are refused with "a link with a Human has exactly one Human on Blue".

  The earlier draft's worry is gone: a link can no longer open a battle with a Human seat nobody commands (the browser commands only Blue's Human, I08). What a link may leave out did not change. It must name both sides' players and factions. The map type, size, map seed and battle seed may be left out, and then default to `mixed`, `small`, seed 1 and the default battle seed. The lobby always writes all of them, so only a hand-typed link uses the defaults. The README and the link file's own comment now say this, rather than "anything else is refused" (D42).
- **The gap:** D34 made "exactly one Human, on Blue" a lobby and single-player rule. Nothing said what a link with humans means before online play exists.
- **The reach:** Online play (D26) will want several humans per link, and will have to relax this rule on purpose. A defaulted size can still fall below the players' size floor; admission then refuses the battle on the refusal screen.
- **Verdict:** needs you. **Provisional call:** as built: one Human on Blue or none, at any size; omitted map fields default. **To reverse:** either refuse a link that omits map type, size or seed, or (when online play exists) allow several humans per link. Both are changes to `askedBattle` alone.
- **Confidence:** medium.

---

## Unsound

None. Every entry an audit found unsound was corrected in the closing passes; each now appears under Sound with its final decision and keeps its id.

---

## Sound

### I48 · An outgunned lane asks for help, not only a falling one
- **When:** slice 13; corrected in the closing held-items pass. **Who:** agent.
- **The choice:** Each lane has an *influence* reading: friendly strength minus visible enemy strength around its objective (D56). A lane below `stance_fallback_influence` is *outgunned*: it falls back to its staging point. Help is raised while a lane is outgunned **or** its influence is falling by at least `help_trend_threshold` between readings, or when a help detachment is lost; it clears only on relief. Answering is unchanged: the nearest ally with surplus sends a detachment that can fight the threat. Before the fix, a lane beaten in one sharp exchange that then sat steady never asked, because only a falling reading raised help. Test `a_lane_beaten_but_steady_still_asks_for_help`.
- **Consequence you own:** with no ally (1v1), a beaten lane keeps calling and nobody answers; that is harmless, since answering is gated.
- **Confidence:** medium.

### I04 · Each AI reads its own economy row
- **When:** slice 02 (a seam left from before the per-player table); corrected in the held-items pass (review finding 5). **Who:** agent.
- **The choice:** The side's skirmish view (`SkirmishView`) carries one economy table, `players`, a row per player. Each AI turn narrows the side's observation to that player's units, and the AI reads its own `PlayerView` row through one lens, `own_player`. The five hidden side-level fields (`ready`, `credits`, `unit_count`, `max_units`, `pending`), filled from the side's first player and overwritten per AI turn, are deleted. They were never published or hashed, so no digest moved.
- **Confidence:** high.

### I53 · A player's controller has one record
- **When:** slices 02 and 12; corrected in the held-items pass (review finding 7). **Who:** agent.
- **The choice:** A skirmish player's controller lives on its player record (`PlayerState`), set from the player list at tick 0 and changed only by controller transitions; the battle's own controller map keeps only encounter players. One lookup, `Battle::controller`, serves both the client-order refusal (`ai_controlled`, I08) and the team planner. The planner takes the battle's player records directly (`&[PlayerState]`); the `TeamPlayer` trait that let it also read `Player` is gone, and tests build records with `PlayerState::new`. The battle's controller code lives in `battle/controllers.rs`, and its wording says "player", not "slot" (D51).
- **Confidence:** high.

### I54 · Help reads lanes from the plan, and a request names its party
- **When:** slices 13 and 13b; corrected in the held-items pass (review finding 11). **Who:** agent.
- **The choice:** Help asks only for lanes that have an owner: an AI player's planned lane, or the objective a human teammate covers. An AI player the plan left unassigned (more AI players than lanes, or a passive AI) gets no lane and asks no help, instead of a lane made up from its position on its side. A `HelpRequest` is the call itself (requester, lane, objective, who answered, when it was raised); the detachment's units are read from the answering party through `TeamPlan::help_party`, not copied. Test `an_ai_player_the_plan_left_unassigned_asks_no_help`.
- **Consequence:** the lane rule moved AI-driven 5v5 outcomes (more AI players than objectives there); captures rose slightly and no order was refused.
- **Confidence:** medium.

### I55 · Who only returns fire is decided in one place
- **When:** dispatch lane; collapsed in the held-items pass; corrected in review. **Who:** agent. Corrected: a controller change reset every unit to fire at will, so a unit authored to return fire only fired at will once the browser installed its Human or active-AI controller at admission.
- **The choice:** A unit keeps its own engagement: authored, else fire at will (neutral's units only return fire), and changed by orders. `controllers::put_under` (`battle/controllers.rs`) is the one place a controller acts on it: a passive AI holds the unit at return fire and keeps its own engagement to give back; any other controller leaves it with its own. A controller change and an arriving purchase both call it, so a passive AI's reinforcements behave like its starting units, and installing a controller never overwrites an authored engagement. A *passive* AI is used in labs, for example so a test's anti-air doesn't shoot a timed jet. Tests `a_purchase_arriving_for_a_passive_ai_player_only_returns_fire` and `a_controller_change_keeps_each_units_own_engagement`.
- **Confidence:** high.


### D57 · Air defence stays in the backline
- **When:** slice 13b. **Who:** you: "AA, especially long range AA, should stay in the backline, with the artillery".
- **The choice:** In a 13b probe, a Patriot battery drove alone onto an open objective. It couldn't capture it, and it couldn't fight anything on the ground. Now a unit whose weapons can't engage ground targets (decided from its weapon data, not its name) holds the player's *backline post*. That is the position of the player's nearest artillery piece if it has one; otherwise it is three quarters of the way from the lane's entry to its staging point, behind the group. It never advances toward the objective, never claims one, and a carrier with air defence aboard joins no party. Recon units are exempt, by their `recon` role tag. A review flagged that tag as a named role. It was kept because `development.md` lists role tags as the doctrine's deliberate playbook.
- **The gap:** Nothing in the plan said where a unit that can't fight on the ground should be.
- **The reach:** This is the template for any unit that can't take ground. D56's value measure agrees with it: against a ground-only enemy, an air-defence unit's value is zero, because both read the same weapon check (`capability::unit_engages`).
- **Verdict:** sound. It is your rule, read from weapon data.
- **Confidence:** medium. The recon role-tag exemption is the one part you might want expressed as data.

### D56 · The AI weighs units by value: cost, counted against what its weapons can hit
- **When:** slice 13c. **Who:** you (2026-10-18): "you can just by cost and specialization (if it's AT, AA.. etc)". The implementation choices below are agent.
- **The choice:** Before 13c, the planner counted its own units by head and enemies by cost (I45). Now one function, `unit_value` in `team/value.rs`, says what any unit is worth, for both sides:
  1. Take the *threat*: the enemy kinds in play, each weighted by its card cost.
  2. Find the share of that threat (by cost) that this unit's weapons can engage, judged per altitude layer and target class by `capability::unit_engages`. That is the one engage check; the older "can it hit this target" and "can it hit the ground" checks now call it.
  3. Value = the unit's card cost × that share. Against a single enemy kind, value is simply the cost, or zero.

  *Strength* is value divided by the mean card cost of the roster. That scale comes from the data, so no number is hard-coded, and both sides use it. Say Red has only tanks in sight. Blue's 600-credit anti-tank team counts its full cost; Blue's Patriot counts zero. If Red adds helicopters, the Patriot starts counting for the helicopters' share.

  Implementation choices:
  - Each side is measured against the *other side's kinds across the whole map*, not per grid cell: our units against the enemies we have identified, theirs against our own kinds. With no enemy known, the threat is "the ground".
  - Readers of strength: the influence grid (I45), each player's force, the party budget (`party_budget_share` 0.35 of force), help surplus and help need (D4, D55), and `recruit`, which picks a party's units.
  - `recruit` takes an ask in both units and strength, and stops once the strength gathered meets it. The last unit taken may overshoot, so one expensive tank can answer a lane's whole need. Help needs surplus above zero.
  - Claim and hold party sizes stay unit counts (2 and 2), so 13b's small parties are unchanged.
- **The gap:** Your rule named the inputs. The grid scope, the scale, the overshoot and which counts stay headcounts were left open.
- **The reach:** A change to a unit's cost or weapons now changes the AI's judgement and buying with no code change. One consequence you own: with 13c's changes, the 1v1 seed 1 match runs 35 minutes where it used to end sooner; it still ends with a winner.
- **Verdict:** sound. Your rule, read from weapon data, with one owner.
- **Confidence:** medium. The whole-map threat (rather than per cell) is the call most worth a look.

### D2 · Every AI fields a complete army, tanks included
- **When:** the recipe predates the spec; corrected in slice 13c. **Who:** you decided the rule ("each AI manages its own logistics"); the fix is agent. Corrected in 13c: the earlier draft found that no AI ever bought a tank.
- **The choice:** You decided that each AI fields its own armour, infantry, recon, anti-air and supply trucks, and never skips a role because an ally covers it. There is no ally check anywhere in purchasing.

  Until 13c, tanks (the `mbt` role) appeared only in the `TankSwarm` opening, which no AI is in during preparation, and not in the reinforcement rotation. Now:
  - the default opening's four infantry picks became three, and the fourth slot is an `mbt`;
  - the reinforcement rotation is infantry, recon, anti-tank, light vehicle, **mbt**, artillery, helicopter, jet.

  No per-card cap was needed: within a role, every capable card scores the same against a single threat, so the recipe's ordinals keep the variety (I56). A test pins that both an opening and the reinforcements field armour. With 1000 starting credits, after the opening's air defence a tank usually doesn't fit, so the AI saves for one and armour mostly arrives as a reinforcement.
- **The gap:** Slice 12 planned a test for a complete opening; none shipped, so the gap went unseen.
- **The reach:** One consequence you own: saving for armour costs tempo. On 3v3 seed 4, the AI takes one objective fewer within 900 s than the same build without armour.
- **Verdict:** sound. Your rule, now true in play.
- **Confidence:** medium, because of that trade-off.

### I56 · Within a role, the AI buys the card that best counters what it has seen
- **When:** slice 13c. **Who:** agent, under D56.
- **The choice:** The purchase recipe asks for a role, say "anti-tank". Several cards fill it. `skirmish_ai/purchase.rs` ranks them:
  - with enemies identified: by value per credit against them (D56), with cost as the tie-break;
  - with nothing seen: by cost alone, as before. There is no matchup to judge yet, so openings match 13b's.

  The *air pivot* (buying air defence when enemy aircraft are seen) needs one more rule. Shipped rifles list low-air targets, so plain value per credit bought an 80-credit scout patrol to answer helicopters. Now the pivot ranks only cards carrying a weapon *made for aircraft*: a weapon row that engages that air layer and reaches no other layer (`unit_specializes_in`), or every capable card if a deck has none. That reproduces 13b's picks (Stinger and FN-6 teams against helicopters; Patriot, NASAMS, S-300 against jets) without a second scoring rule. The old scorer, `candidate_score` (reach × damage, an affordability bias and `purchase_noise`), is deleted. Two AIs with the same doctrine still differ: each starts its reinforcement rotation at a seeded point (L4).
- **The gap:** D56 said "prefer counters"; it didn't say how to rank, or what to do before any sighting.
- **The reach:** Any new card is ranked by its data alone. Removing `purchase_noise` makes purchases a pure function of what the AI sees.
- **Verdict:** sound.
- **Confidence:** medium.

### I45 · Both sides are counted by the same measure
- **When:** slice 12; corrected in slice 13c. **Who:** agent. Corrected in 13c: the planner used to count its own units by head and enemies by cost.
- **The choice:** The planner keeps a coarse *influence grid* for its side: at each cell, friendly strength minus enemy strength, blended with the previous reading. Before 13c, each own unit added a flat 1.0 and each identified enemy subtracted its cost / 100, so a side with expensive units read itself as weaker than it was. Now both terms are D56 strength: each unit's value against the other side's kinds, over the roster's mean card cost. A 600-credit tank weighs the same on either side of the grid.

  The thresholds were tuned against the old lopsided scale, so they were rescaled (agent, on quick-check evidence): `influence_weight` 49, `help_trend_threshold` 0.2, `stance_fallback_influence` −0.4, `stance_push_influence` −0.1, and `party_budget_share` 0.35 (up from 0.25).
- **The gap:** Slice 12 delegated the grid's resolution and weights, but not what a unit is worth.
- **The reach:** Every help, stance and party decision reads this grid.
- **Verdict:** sound.
- **Confidence:** medium. The retune rests on a quick tournament check, not a sweep.

### I09 · Every refused AI order is recorded; a malformed one also fails a debug assertion
- **When:** slice 02 (the fix for landmine L3); corrected in the dispatch lane, and again in review. **Who:** agent, with your decision on the split. Corrected twice: the invariant said "fails a debug assertion" but the code only logged; then the assertion fired on every non-purchase refusal, including ones the world causes (a 30-order burst against up to 30 units, an attack-move to a point wreckage has filled).
- **The choice:** Each AI order is sent as a normal command carrying its player. Landmine L3 was "AI orders bypass the per-player command checks". Its fix, as now stated in the spec README and in code:
  - every refused AI order is appended to `ai_receipts()`, a log of AI order acceptances and refusals outside the digest;
  - a refused order *other than a purchase* also fails a `debug_assert!` when the refusal is *malformed* (`OrderError::is_malformed`): the order broke the command contract in a way the AI could have seen from its own view (not its unit, out of sequence, off the map, an unknown target). A unit test that drives an AI into such an order fails at once;
  - a refusal the world's state decides (`RateLimited`, `NoValidDestination`, building capacity or occupancy, room to unload, credits, the unit limit, the match having ended) is only receipted: the AI cannot see its order bucket or test every placement, and a correct AI still meets them. The split is an exhaustive `match`, so a new refusal must be put on one side;
  - a refused *purchase* does not assert, because a scripted purchase plan (I05) may name a card the battle won't sell; the plan retries it (I10).
- **The gap:** The assertion was planned but never written.
- **The reach:** Debug test builds catch any malformed AI move, Ready or cancel. A purchase refusal, or one the world decides, shows only in the receipts, which the tournament reports.
- **Verdict:** sound.
- **Confidence:** medium. Exempting purchases is the narrowing you might not have chosen.

### I08 · The browser commands as its side's Human; the battle refuses client orders to an AI's player
- **When:** slice 02, tightened on main (`90ba2588d`); extended to skirmish in the `ai_controlled` lane. **Who:** agent.
- **The choice:** After slice 02, every order the browser sent was refused, because no route said which player it came from. Now the battle session sends orders as the viewing side's Human seat (`apps/battle-lab/src/commandingPlayer.ts`). A side with no Human takes no input; an earlier fallback to "the side's first player" was removed, so a watched AI battle can't be steered.

  The battle checks the same thing itself. Say a client sends an order as `red-1`, which an AI plays. `Battle::issue` refuses it with `AiControlled` *before* recording it, because the replay log doesn't say who issued a command and a replay couldn't tell it from the AI's own. The battle's own AI orders are unaffected, and a seat handed to a human mid-battle takes orders from then on. This used to exempt skirmish; it no longer does. Tests that ordered an AI seat directly now declare it Human through one shared seat list (`tests/common::human_duel`), and tests replaying fixtures hand `red-1` to a human at tick 0; no assertion changed.

  **Known limit:** a human who takes over an AI seat inherits the AI's command sequence counter, so the human's first order is refused `OutOfSequence`. No product path hands an AI seat to a human today.
- **The gap:** No route carried a player, and no rule said who may order an AI's units.
- **The reach:** `controlled()` (D47) and the report's viewer (I34) build on this player. Online play's seat takeover will have to resolve the known limit.
- **Verdict:** sound.
- **Confidence:** medium, because of the known limit.

### D48 · How the planner assigns lanes and objectives
- **When:** slices 12–13. **Who:** agent on your behalf, from the [research](research.md) (Killzone 3's commander, Kohan II, Age of Empires II, UAlbertaBot).
- **The choice:** Every five seconds, or at once when an assigned objective changes hands, the planner matches AI players to lanes and main objectives:
  - For up to three AI players it tries every permutation.
  - For larger teams it uses a bounded subset search.
  - Each match is scored on distance and influence, with a bonus for keeping the current assignment, so a marginally better option doesn't make players swap lanes.
  - Ties keep the first permutation found, which is player order.
  - Every lane is considered. The first version only looked at the first few, so a lone AI took lane 0 whatever its objective.
  - A map has at most three entries, so teams of four or five share lane slots by wrapping round. Each AI still gets a distinct objective where the map has enough.

  The influence grid is blended with its previous value (65 % old), so one bad reading doesn't flip plans.
- **The gap:** Grid resolution, weights, bonus and search method were delegated.
- **The reach:** Every team behaviour starts from this assignment. The large-team search grows exponentially in the number of objectives, which is fine at today's map sizes. Help still has a second lane rule of its own (I54).
- **Verdict:** sound.
- **Confidence:** medium.

### D3 · Each AI gets a lane and a main objective, and skips the ones you're taking
- **When:** slices 12 and 13b. **Who:** you (chose option C, then "do all").
- **The choice:** The team planner gives each AI player a lane and a main objective in it. An objective you're visibly going for counts as covered: one of your units stands inside it, or has a move goal within twice its radius. Covered objectives aren't given to your AI allies, and allies don't send claim parties there (D55). Two limits in the final code:
  - each human player covers at most one objective, the first one found;
  - covered objectives are taken out of the AI's choices only when there are more objectives than AI players, so an AI is never left with nothing to do.

  The main objective moves on once it is taken (D55).
- **The gap:** How "heading for it" is measured, and what to do when objectives run short.
- **The reach:** A human with units at two objectives only shields one of them from allies.
- **Verdict:** sound.
- **Confidence:** medium.

### D9 · Each AI's army fights as one group
- **When:** slice 12, sharpened by three review rounds. **Who:** you: "spread units get picked apart".
- **The choice:** Each AI keeps the bulk of its army as one combined-arms *main group* in its lane:
  - it gathers at the lane's staging point, then advances on the main objective together;
  - recon, logistics and passengers aren't counted as group units, so they move on their own;
  - one function, `team::group_positions`, decides who is in the group, and the gather order moves exactly those units;
  - a group that has gathered keeps advancing;
  - readiness counts only units at staging, or actually heading there, so a unit busy elsewhere doesn't hold the group back;
  - plans change only when an objective changes owner, not when a capture flickers to "contested".

  A review found 1v1 games with no objective held in 30 minutes, which is what those last three rules fixed.
- **The gap:** Slice 12 delegated the group radius and the staging choice.
- **The reach:** Side objectives (D55) and the backline (D57) both detach units *from* this group. Its force is measured in D56 strength.
- **Verdict:** sound.
- **Confidence:** medium.

### I40 · The team plan owns where a group goes; transport owns how it gets there
- **When:** slice 12, when it merged with main's AI transport. **Who:** agent.
- **The choice:** Main gained AI transport while slice 12 was in flight: squads ride carriers when the walk is long. The merge splits responsibility in two:
  - the team plan decides *where* a player's group goes (staging, then the objective);
  - transport decides *how* (ride or walk).

  Squads ride only their own player's carriers. A loaded carrier drops at the staging point until the group has gathered, then at the objective. One loader reads `fixtures/skirmish-ai.json`, with the team planner's settings flattened into the doctrine's. An armed carrier that is empty counts toward the group's readiness. A carrier that is loaded or on its way to collect a squad is left out of readiness, and so are its passengers. An unarmed carrier never counts.
- **The gap:** Two features each had an opinion about where squads go.
- **The reach:** Every party (D55) reuses this split.
- **Verdict:** sound.
- **Confidence:** medium.

### D36 · Mapgen lays flank entry roads in their own pass
- **When:** slice 04. **Who:** agent on your behalf.
- **The choice:** A 3v3 needs three entry roads per side. Mapgen keeps the centre entry it always made. It then lays left and right *entry arms* (short roads from the back edge to the road network) in a separate pass with its own random stream, `"entries"`, so a one-entry map is byte-for-byte unchanged. The rules for the arms:
  - Arms sit at the same edge fraction on both sides, drawn once, so neither side gets a better road.
  - A pair is kept only if, on *both* sides, the start is at least 150 m from other entries, clear of river mouths, and joinable to the network.
  - With three entries, one flank goes on each side of the centre road. This cost a few percent of admissions (the generator retries with another seed), but gives lanes across the whole front.

  Entries are then ordered centre first, then the flanks by map position.
- **The gap:** Slice 04 delegated band widths and join strategy.
- **The reach:** Lanes (D48) are these entries. More entries per side would need this pass extended.
- **Verdict:** sound.
- **Confidence:** medium.

### I20 · What the lobby's map picture shows
- **When:** slice 08. **Who:** agent (delegated); accepted on the evidence when you didn't reply to the shots.
- **The choice:** The lobby draws the admitted map top-down as an SVG, in Rust:
  - terrain colours are the picture's own constants, from the chosen Direction 3 mock;
  - side colours and text come from the page's HUD theme variables (`--hud-accent`, `--hud-enemy`, `--hud-text`), so the HUD stays the one owner of player colours;
  - it is simplified for lobby size: tiny points are dropped, there are no single trees or tree lines (only woods), buildings are one footprint shape, and there is no relief;
  - objectives are numbered rings, in the score strip's order, over a dark disc;
  - entries are unlabelled side-coloured arrowheads, since no entry belongs to a player (D8).

  Five unprimed critique rounds judged that the map reads at lobby size, with entries and objectives above the terrain.
- **The gap:** The marks, the simplification level and the colours were delegated.
- **The reach:** A future native lobby can reuse the same Rust picture (D39).
- **Verdict:** sound.
- **Confidence:** medium. You may still want different colours, labels or a loading state.

### I21 · One 30-second preparation deadline for every map size
- **When:** opened in slice 08, resolved on main before slice 09. **Who:** agent (main session).
- **The choice:** Preparing a battle means generating a map and admitting it, retrying with new seeds if it fails. In slice 08 an XL map took about 16 s in a loaded browser, against an 8 s deadline for all attempts together, so XL previews would have timed out. Main raised the deadline to 30 s (`generated_deadline_ms` in `fixtures/generated-battle.json`), one value for every size. The `/labs/map-picture` lab keeps its own 90 s deadline, because it runs many previews at once.
- **The gap:** No size-specific deadline was ever needed.
- **The reach:** It is a loose tripwire, not a target. If generation slows, the lobby shows "failed" instead of hanging.
- **Verdict:** sound.
- **Confidence:** medium.

### I36 · The report is one scoreboard per side, with no team summary
- **When:** slice 11. **Who:** agent (row density was delegated).
- **The choice:** The report shows a scoreboard per side, with one row per player: units, credits spent, kills and refunds. Your row is lit. Names are in the battle's colours: yours, your ally's pale ice, the enemy's. In a battle you watch without a seat, names are coloured by side. Sides come from the battle's players, not a fixed Blue/Red list. There is no team-total row above the players.
- **The gap:** Slice 11 delegated density, and named a team summary as the feedback that would change it.
- **The reach:** A summary row can be added on top without touching the per-player rows.
- **Verdict:** sound.
- **Confidence:** medium.

### I37 · Collapsing the report's timeline at 5v5
- **When:** slice 11. **Who:** agent (delegated).
- **The choice:** Each player's purchases, or refunds, in the same whole match minute fold into one line with their unit glyphs, a count and credits. Each kill keeps its own "KILLED" line, with the victim's glyph drawn broken. The grouping is by player and minute, not by adjacency, because at 5v5 the AIs buy in interleaved waves and neighbours are rarely the same player. Two other fixes came from the unprimed critiques:
  - the list sizes to its content up to the window, then scrolls with a fade at its foot (before this, every report drew an empty timeline);
  - credits read in capitals, and the score is labelled SCORE.

  Minutes are fixed buckets, so two buys seconds apart across a minute boundary show as two lines.
- **The gap:** Delegated.
- **The reach:** See I38 for the open question about what the story is for.
- **Verdict:** sound.
- **Confidence:** medium.

### I10 · A refused AI purchase holds back only that turn's later purchases
- **When:** slice 12; corrected in the dispatch lane. **Who:** agent. Corrected: one refused purchase used to drop the AI's whole turn, Ready and movement included.
- **The choice:** Each AI decision returns a list of orders: purchases, Ready, and movement. Say a scripted plan's second card is refused (the battle won't sell it). The battle skips the turn's *later purchases*, so a plan never buys out of order, and still issues the Ready and every move order. The plan does not advance past the refused card, and tries it again next turn (I05). The rule lives in the AI turn loop, `battle/skirmish_turn.rs`.
- **The gap:** Slice 12 decided that a refused card isn't skipped. It didn't decide what happens to unrelated orders.
- **The reach:** An AI whose purchase is refused still moves and readies.
- **Verdict:** sound.
- **Confidence:** medium to high.

### I13 · The size table has one reader per language
- **When:** slice 04; lobby side slice 09; corrected in the web and Rust lanes. **Who:** agent. Corrected: the rule used to be restated in mapgen, parsed twice in the contract crate, looked up twice in TypeScript, and the link's 5-per-side limit was a literal.
- **The choice:** The *size floor* is the smallest map size a match can be played on: one player per side needs at least Small, two need Medium, three or four need Large, and five or more need XL (D18). The entry count and the map extent come from the same table, `fixtures/skirmish-sizes.json`. Now:
  - in Rust, `contract::skirmish_sizes` is its one reader (tier, extent and floor). Mapgen's floor check, the contract's entry count, admission's extent check and the tournament's size all call it;
  - in TypeScript, `sizeTier` (`apps/battle-lab/src/skirmishSetup.ts`) is its one reader, used by the lobby's floor and by the entry count;
  - the lobby and links share `MAX_PLAYERS_PER_SIDE` (5), also in `skirmishSetup.ts` (D35).

  Retune the fixture and mapgen, admission, the lobby and links all follow.
- **The gap:** The slice-04 review fixed the data duplication but not the rule restatements.
- **The reach:** Any retune of the size floor or the per-side limit.
- **Verdict:** sound.
- **Confidence:** medium to high. One loose end: the `map-picture` browser scene (`web/scenes/map-picture.mjs`), a test, re-reads the fixture with its own floor lookup. The unused `skirmish_size_rule` wasm export was deleted.

### I31 · A soldier whose target is in his own grid cell walks straight to it
- **When:** slice 10, found by the 2v2 scene, in its own commit. **Who:** agent.
- **The choice:** Soldiers plan routes on a fine grid. When a soldier's target lay inside the cell he already stood in, route planning returned an empty route, and following it crashed. Now he walks straight at the target, as he already did when no route was found. Only runs that had crashed change, so no digest moved. The exact trigger in the shipped game, probably a rejoin point he had already reached, was inferred, not reconstructed.
- **The gap:** A bug fix. The choice is "walk straight", not "refuse the order".
- **The reach:** This is the movement fallback every soldier uses.
- **Verdict:** sound.
- **Confidence:** medium to high.

### D35 · Up to 10 players per side in the simulation, 5 in the lobby and links
- **When:** slices 01 and 07; the TS constant's home settled in the web lane. **Who:** agent on your behalf (preferences 4 and 5). It replaces D21's "any number".
- **The choice:** The simulation accepts any player count up to `max_players_per_side`, which is 10 in `fixtures/skirmish.json`. The limit exists because unbounded counts mean unbounded planner, publication and dispatch work. It is a loose bound, not a target. Admission refuses a side with no players, duplicate ids, mixed factions, more than 10 players, or the wrong entry count.

  The lobby and battle links cap a side at 5. That is one TypeScript constant, `MAX_PLAYERS_PER_SIDE` in `skirmishSetup.ts`, read by the lobby model, the lobby screen and the link parser. It stays a TS constant rather than fixture data, separate from the simulation's bound.
- **The gap:** The walk said "any number", which an unbounded loop can't honour.
- **The reach:** A 10v10 is admitted by the simulation today. Only the lobby stops at 5v5.
- **Verdict:** sound.
- **Confidence:** medium to high.

### D52 · Both sides always get the same number of entries (with D8)
See **D8**, which holds the merged entry rules.

### D8 · Entries are shared by the side, and a unit uses the quickest one
- **When:** slices 04–05. **Who:** you. You corrected the original plan, which assigned groups of players to entries. The equal count (D52) is agent on your behalf, from the slice 00 probe. The single entry owner fixes landmine L1.
- **The choice:**
  - No player owns an entry. A side's entries come from the *larger* side's player count: 1 player → 1 entry, 2 → 2, 3 or more → 3. Both sides get the same count, so a 2v3 map has three entries per side.
  - The probe showed why the count must be equal: uneven pairings were never admitted, because the side with more entries always had a faster best route to the objectives. A side with fewer players simply has a spare road.
  - A bought ground unit or helicopter arrives at the side's entry with the shortest route to where it was ordered, and equal routes go to the earlier-listed entry (the centre, when it is among the tied).
  - A unit turning back (a refund, or a helicopter low on fuel) heads for the entry nearest it *at that moment*, not the one it came in by, and it leaves the map on reaching any of its side's entries. A first version sent units home by their arrival entry, and review changed it.
  - One module, `sim::entries`, owns every one of these choices. Six scattered "first entry of the side" lookups were silently wrong once entries became a list.
- **The gap:** How to share roads among players, and whether uneven sides can be fair.
- **The reach:** Lanes (D48) are entries. Fewer or more than three entries per side would need mapgen work (D36).
- **Verdict:** sound. Reversible: per-side counts would need a different fairness rule.
- **Confidence:** medium to high.

### D13 · The side shares one bounty pool; kill money goes to the killer's player
- **When:** slice 02. **Who:** you in an earlier session ("team bounty", 2026-10-06); the split method (D43) is agent on your behalf.
- **The choice:** Killing enemy units earns money from the victim side's capped *bounty pool*. The pool belongs to the side. Rewards, the kill bonus and refunds go to the wallet of the player who owns the unit. The split works like this:
  1. In one step, Blue 2's tank and Blue 3's squad both kill Red units.
  2. The bonus owed to Blue is summed, capped at what the pool holds, and a sub-credit remainder is carried per victim side to the next step.
  3. Each killer gets the whole-credit floor of its share.
  4. Leftover credits go one at a time by largest fractional remainder, ties to the lower player index.

  With one player per side this pays exactly what the old side-only settlement paid, and a reference test pins that. The killer's player is looked up from the killing unit's owner when the step settles.
- **The gap:** The old settlement's "exact sum" argument assumed a single opposing wallet.
- **The reach:** Any future economy rule (shared income, gifts) builds on per-player wallets with a side pool.
- **Verdict:** sound.
- **Confidence:** medium to high.

### D55 · Side objectives: claim open ones, hold taken ones, and keep the main group together
- **When:** slice 13b; budget and help measured in strength since 13c. **Who:** you (2026-10-18): "there should be side objectives that the AI can have multiple of, where it can still send a few units to go take and hold … if the AI took one objective, and it's not contested, the main objective should actually move as well (and the prev one can be downgraded to a side)"; and "open unclaimed objectives should be highest priority for the AI (since they are easy wins)".
- **The choice:** An AI player has one main objective, for its main group, and may hold several *side objectives* with a few units each. A *party* is units detached from the main group to one objective, and there are three kinds:
  - **help** answers an ally's request (D4);
  - **hold** stays on an objective the player took;
  - **claim** goes to take an *open* objective: one that is neutral, with no enemy visible within its radius, and that no teammate (AI or human) is going to.

  All three share one lifecycle (`team/party.rs`). Every party includes a unit that can capture, so aircraft never go alone. It is never re-tasked while fighting. A lost party is replaced while it is still needed. A unit is in at most one party. Each round:
  1. When the main group takes its objective and nobody contests it, the main objective moves on and a hold party stays behind.
  2. Claims are filled before holds, from one budget: 35 % of the player's force, in D56 strength. Units already on any party duty count against it. Claim and hold parties are two units each.
  3. Help is *not* bounded by that budget. It comes from the helper's surplus (D4), so a team holding many objectives still answers a lane under pressure.
  4. A hold party under pressure raises help like a lane does. A player whose lane and hold share an objective asks only once.

  This adds side objectives to D3's one-main-objective rule. The main group still concentrates (D9).
- **The gap:** The plan had one objective per AI. The budget share and party sizes were delegated.
- **The reach:** All AI detachment logic is one concept. Help requests still copy some party data (I54).
- **Verdict:** sound.
- **Confidence:** medium to high.

### D4 · Allies answer requests for help
- **When:** slices 13 and 13b; measured in strength since 13c. **Who:** you.
- **The choice:** Your lane is losing an objective. Its influence is falling (I48 covers a lane that is beaten but steady). The planner raises a *request for help* for that lane; this works for human lanes too. It is answered by the nearest AI ally, measured from the centre of its units, that has *surplus*: its strength beyond its own lane's need times a margin (1.25), minus units already on party duty, and above zero. The ally sends a *detachment*, not its army:
  - only units whose weapons can engage the threat there, judged by target class from weapon data (with no threat seen, ground-capable units);
  - gathered until their strength meets the lane's need, so one strong unit may be enough (D56);
  - always including one unit that can hold the objective;
  - riding its own carriers when the trip is long.

  Help stays until the lane is relieved, meaning influence at its objective is no longer negative for the help *dwell* (a minimum time). It is never recalled mid-fight. If the detachment is destroyed, the lane asks again. Only AI allies answer, and help may take the helper's last unit able to hold its own objective. The browser scene first showed a Patriot, then a lone Little Bird, answering a ground fight, which is why the target-class rule exists.
- **The gap:** Thresholds, margins and dwell were delegated.
- **The reach:** D55's parties generalise this detachment.
- **Verdict:** sound.
- **Confidence:** medium to high.

### D5 · The team pushes together and falls back lane by lane
- **When:** slice 13; thresholds rescaled in 13c. **Who:** you.
- **The choice:** Each side has a shared *stance*: push, hold or fall back.
  - An *outgunned* lane (influence below −0.4) falls back to its staging point on its own.
  - The whole team falls back only when outgunned lanes are a strict majority.
  - A push needs the mean of all lanes' influence at or above −0.1 (with no majority outgunned), *or* a falling score trend. It also needs the stance dwell to have passed, and every lane gathered at staging, or a 30 s timeout. Then every lane gets its advance in the same decision interval.
  - A push ends when the mean drops below halfway between the two thresholds.
  - A score deficit never causes a retreat.

  Two earlier versions failed. A score-driven fallback sent the trailing side back to its entry, where it lost. A mean-influence fallback pulled every lane back for one bad lane. One function, `TeamPlan::assignment_for`, turns stance into each player's orders, and the battle no longer steers AI orders itself.
- **The gap:** The plan named the three stances. Thresholds and dwell were delegated.
- **The reach:** Every coordinated move a team makes.
- **Verdict:** sound. I48 records how help reaches an outgunned lane.
- **Confidence:** medium to high.

### I44 · Which objectives an AI may be given
- **When:** slice 12 review, extended in 13b. **Who:** agent.
- **The choice:** A review found an AI parked for good on the first objective it took: the planner kept treating held objectives as targets. Now:
  - an objective your side holds *uncontested* is not a main-objective choice while any objective is neutral or the enemy's;
  - one your side holds that the enemy *contests* stays a choice, so the group defends it instead of walking away;
  - the planner replans the moment an assigned objective changes hands, instead of waiting for its five-second interval.

  A held objective is defended by its hold party (D55).
- **The gap:** The plan didn't say what an AI does after winning its objective.
- **The reach:** This pairs with D55's "the main objective moves on".
- **Verdict:** sound.
- **Confidence:** medium to high.

### I46 · What the tournament measures, and how long it lets a match run
- **When:** slices 12–13b. **Who:** agent.
- **The choice:** The tournament is the behaviour proof (D28). It reports:
  - accepted and refused AI orders, from `ai_receipts()`;
  - each side's peak number of objectives held at once (`peak_objectives_held`), which replaced a per-lane metric that read near 1 once AIs moved on from taken objectives;
  - parties by kind, and how long hold parties were on duty;
  - how many pushes were synchronised. This counts only lanes that have a group to advance, so a lane holding nothing but its backline anti-air isn't counted as late.

  The match cap comes from the rules: preparation plus three majority holds (about 91 minutes). The old 30-minute cap equalled one majority hold, so a contested match couldn't finish inside it. Faction balance is outside this spec: Red's 1v1 wins followed the US-against-Europe pairing, not the side.
- **The gap:** The plan named the tournament and left its measures open.
- **The reach:** Slice 14's 5v5 run reads these measures.
- **Verdict:** sound.
- **Confidence:** medium to high.

### I41 · AI purchases are placed a short way up their lane
- **When:** slice 12. **Who:** agent.
- **The choice:** An AI buys a unit for its lane. Before this change, the purchase's destination sat on the entry itself: the new unit arrived and parked there, and because an arrival waits while any hull overlaps the entry, the next arrival waited until the AI's next decision, about 8–10 s, or forever if the parked unit started firing. Now an AI purchase is ordered 24 m up the lane toward the staging point (`purchase_advance_m`), capped at half the way. Arrivals drive off at once, and the next waits only for the hull to clear, about 5 s. The brief had asked for arrivals "one second apart". Meeting that would have meant letting arrivals overlap, so the footprint rule was left unchanged.
- **The gap:** The brief's number was wrong for physical arrivals.
- **The reach:** Any future change to the entry footprint or the dispatch interval.
- **Verdict:** sound.
- **Confidence:** medium to high.

### I51 · How rides are matched, and where a loaded carrier drives
- **When:** slice 13b. **Who:** agent.
- **The choice:** Before this change, carriers were re-matched to squads at every decision, so a carrier flipped between squads for minutes and none arrived. Now:
  - a carrier the AI's *last decision* sent to collect a squad stays that squad's ride. This is AI memory, outside the digest, and a move that merely ends beside a squad reserves nothing;
  - a loaded carrier with a planned goal turns toward a fight only if the enemy is nearer that goal than the carrier is. Before this, an enemy claim off to the side pulled main-group carriers away from their own objective;
  - a loaded carrier whose route is refused lets its squad out where it stands, instead of keeping it aboard for minutes;
  - a party's squad of any kind rides when the trip is worth it, in any stance.
- **The gap:** Transport and parties met for the first time in 13b.
- **The reach:** Any future transport behaviour.
- **Verdict:** sound.
- **Confidence:** medium to high.

### I25 · A preset re-seats the lobby but keeps the map choices
- **When:** slice 09. **Who:** agent.
- **The choice:** Pressing the 3v3 preset replaces the players with three per side. It keeps the map type, region, seed and both factions, and drops the size to the 3v3 floor (Large). A preset is a new match shape, not a new map choice.
- **The gap:** The plan named presets but not what they reset.
- **The reach:** Every lobby preset.
- **Verdict:** sound.
- **Confidence:** medium to high.

### I50 · A party takes a carrier and its passengers together, or neither
- **When:** slice 13b. **Who:** agent.
- **The choice:** On one 1v1 map, a claim party took two loaded carriers whose squads belonged to the main group. The squads got out at the claim's objective, 650 m from their own, and waited minutes for another ride. In another replay, an ally's claim squads rode main-group carriers to the *main* objective and walked 500 m back. Now:
  - a party takes a loaded carrier and the squads aboard it as one unit, or neither, and the budget counts every unit;
  - a carrier with a party's squad aboard drives to that party's objective, whichever group the carrier belongs to;
  - a carrier with air defence aboard joins no party (D57).
- **The gap:** Parties and transport weren't designed together.
- **The reach:** Parties of any kind (D55).
- **Verdict:** sound.
- **Confidence:** high.

### I14 · A blocked arrival holds up only its own entry
- **When:** slice 05 (the fix for landmine L9); corrected in the dispatch lane. **Who:** agent. Corrected: a blocked purchase used to hold up the same player's purchases at every other entry.
- **The choice:** Each entry has its own dispatch clock: one unit drives on per dispatch interval. The players of a side take turns at an entry in player order, and the turn moves on after each dispatch or block.

  Say Blue 2 buys a tank for the left entry and then a helicopter for the right entry, and a wreck sits on the left entry. Each tick, the dispatcher (`battle/dispatch.rs`) walks Blue 2's purchase queue. It finds the tank, marks it blocked, and marks the *left entry* as held for this tick. It then keeps searching, skipping every purchase for a held entry, an entry whose clock isn't due, or an entry where it isn't Blue 2's turn. It reaches the helicopter and sends it in at the right entry. Aircraft that wait off the map are handled separately and never queue at an entry.
- **The gap:** Slice 05 delegated the clock's data structure. Its first test covered another player and the centre entry, not the same player's other entry.
- **The reach:** In big teams every AI buys in waves across several lanes; one blocked entry no longer stalls a player's whole reinforcement.
- **Verdict:** sound. It is the plan's rule: "a player's purchase for one entry never waits behind its purchase for another entry".
- **Confidence:** high.

### I18 · Synthetic maps list their entries centre first
- **When:** slice 06; corrected in the Rust lane. **Who:** agent. Corrected: three synthetic entries used to be listed left, centre, right.
- **The choice:** Some battles use a map with no generated entry roads, so they get *synthetic* entries spread along the back edge: the tournament's saved-map rows and the scenario builder. The contract says a side's entries are ordered centre, left, right, and the entry owner (`sim::entries`) relies on it: `centre` returns the side's first entry, and equal-length routes go to the earlier entry. `sim::skirmish::synthetic_sites` now places three entries at 0.5, 0.2 and 0.8 of the edge width, in that order (two entries at 0.35 and 0.65; one at 0.5). So on those maps, as on generated ones:
  - jets arrive opposite the centre entry (D15);
  - ties go to the centre entry;
  - a battle with no human points the camera at the centre entry (I17).

  The change was part of the named digest re-baseline (D41).
- **The gap:** Slice 06 gave synthetic layouts one owner but didn't test their order.
- **The reach:** The tournament's saved-map rows and the builder's team battles.
- **Verdict:** sound.
- **Confidence:** high.

### D38 · One setup constructor, used by every skirmish including the scenario builder
- **When:** slice 06; completed in the web and Rust lanes. **Who:** agent on your behalf. Corrected: the scenario builder used to build its own rules and objectives in TypeScript, and an uncalled wasm export, `skirmish_fields`, was still around.
- **The choice:** A skirmish *setup* is the match rules plus the map's sites (entries and objectives). `sim::skirmish::setup_for(players, sites)` is the one constructor. Generated battles (through wasm), the tournament and now the scenario builder use it. Say you author a team battle in the builder's economy mode. The builder calls the wasm function `synthetic_skirmish_setup(size, players)`, which runs `setup_for` over the synthetic sites for that map size. The builder then applies only its two authoring overrides, 2 starting minutes and 20 s of preparation, so a draft can be tried quickly. It no longer hard-codes the `center`, `west` and `east` objectives. `skirmish_fields` is deleted.
- **The gap:** The slice listed the builder as a replaced constructor; its test checked only the entries.
- **The reach:** A change to synthetic objectives or starter rules is made once, and the builder's battles match the tournament's.
- **Verdict:** sound.
- **Confidence:** high.

### I29 · The renderer learns "ally" from the viewing player's index
- **When:** slice 10. **Who:** agent.
- **The choice:** The presentation crate draws x-ray outlines and colours. It gets one new input, `Ui.player`: the viewing player's index, an optional argument to the wasm `frame()` call. An own-side unit whose published `owner` differs is drawn as an ally (the `overlay.xray.ally` colour), even when it is selected. TypeScript sends only the index, so the unit's `owner` stays the one source of truth.
- **The gap:** The plan said the ally relation comes "from owner". This is how it gets there.
- **The reach:** A native renderer gets allies by passing the same index.
- **Verdict:** sound.
- **Confidence:** high.

### D24 · Allies are pale ice, with a dashed shape cue
- **When:** slice 10. **Who:** you picked the hue. The shape cue is agent on your behalf, from the `game-ui` rule "form before colour".
- **The choice:** Allies get one shared colour, pale ice `#b9f0ff`. It is stored as float RGB in `fixtures/game.json`, under `presentation.hud.ally` and `overlay.xray.ally`, and order styles derive their ally colour from it rather than holding a third copy. So that allies don't rely on lightness alone:
  - an ally's info panel hangs on a dashed leader and underline (6 px dash, 4 px gap), where yours are solid. The dash was coarsened after a critique said finer dashes read as solid at far zoom;
  - a selected or airborne ally's ground marker is a dashed ring with no facing arrowhead;
  - an ally's orders are never drawn.
- **The gap:** The cue's geometry was delegated.
- **The reach:** I32 holds the open "make it stronger" question.
- **Verdict:** sound.
- **Confidence:** high.

### D16 · You see allies but command only your own units
- **When:** slices 10–11; the HUD economy rule settled in the web lane. **Who:** you. The per-player timeline is you in an earlier session (2026-10-11). The selection and HUD details are agent.
- **The choice:**
  - Your units keep Blue, and allies are pale ice (D24).
  - Clicking an ally selects it alone, to read. Its panel shows an owner line such as `BLUE 2 · AI` and no order controls.
  - Shift-click never mixes an ally in. Box-select and double-click take only your units.
  - With an ally selected, the command bar is disabled, a right-click sends nothing, and no unit voice answers.
  - Every command reads the selection of units you control, including Stop, which used to read raw ids.
  - The credits and cap shown are the commanding player's (`commandedEconomy`). In a battle you watch with no human seat, the HUD shows no economy at all, rather than falling back to the side's first player.
  - The match timeline names each player.
- **The gap:** How a mixed selection behaves, and whose money a spectator sees.
- **The reach:** Online co-op will use the same rule per seat.
- **Verdict:** sound.
- **Confidence:** high.

### D47 · One rule decides which units you can order
- **When:** slice 10. **Who:** agent on your behalf.
- **The choice:** `controlled(observation, player)` (`web/src/battle/input/controlled.ts`) takes the player id the client commands as. It finds that player's index in the side's published player list, and lets you order exactly the units whose `owner` is that index. Its complement, `allyOwners`, labels every other own-side unit with its owner for the panel. Unit selection, the pointer intent, the Space reveal and the army row all read it. Without it, five places would have sent ally units in orders and had them silently refused. In a battle with no player-owned units (a lab), every unit is orderable, and so is a unit with no owner.
- **The gap:** There were five ad-hoc filters.
- **The reach:** Any new selection or command path must use it.
- **Verdict:** sound.
- **Confidence:** high.

### I05 · Scripted purchases are match data, not part of the player seat
- **When:** slices 02 and 12; the tournament flag widened in the Rust lane. **Who:** agent (corrected in review).
- **The choice:** The tournament needs a player to follow a fixed buy list. That list is stored on the match setup, `SkirmishSetup.openings`, keyed by player id. It isn't on `Player`, which stays identity only (id, side, faction, controller, human-capable). In battle, the list becomes that player's AI purchase plan (`Battle::script_purchases`):
  - the AI buys its affordable run, then readies;
  - it advances past a card only once the purchase is accepted (I10);
  - a list without a `|` marker repeats from the start; cards before a `|` run once, and cards after it repeat.

  The tournament's `--buy-blue` / `--buy-red` flags give the list to *every* player on that side, not just the first.
- **The gap:** The plan said "a player may take a scripted opening as data", but not where.
- **The reach:** Tournament inputs and any authored opening.
- **Verdict:** sound.
- **Confidence:** high.

### I34 · The report reads the battle's seats from the scenario
- **When:** slice 11. **Who:** agent. This fixes landmine L6.
- **The choice:** The report has to name enemy players, but the published player table holds only your own side's players (D31). So the report takes `seats` (every player, in index order, from the scenario) and `viewer` (the human the client plays, or none) from the battle session. Report row `i` belongs to seat `i`, as the Rust per-player totals are ordered. No publication field changed.
- **The gap:** Where enemy names come from.
- **The reach:** Any end-of-match or spectator view.
- **Verdict:** sound.
- **Confidence:** high.

### I35 · Results, scores and objectives read from the viewer's side
- **When:** slice 11; the top bar joined in the web lane. **Who:** agent.
- **The choice:** `outcome(result, viewerSide)` in `battleStatus.tsx` returns VICTORY, DEFEAT or DRAW from the viewer's side. When the client plays no human seat, it returns `BLUE WINS` or `RED WINS`. The top status strip had its own literal `"blue"` and now uses it too. The top bar's scores and objective marks are likewise read from the viewer's side; a client watching without a seat reads as Blue (D34). A client is a spectator when the player it commands as isn't a human. Scores in the publication stay ordered by side, by contract.
- **The gap:** The plan named the report. The status strip and top bar held the same assumption.
- **The reach:** Every result and score display.
- **Verdict:** sound.
- **Confidence:** high.

### I30 · One function names players
- **When:** slices 10–11. **Who:** agent.
- **The choice:** `playerName` (`web/src/battle/present/playerName.ts`) produces `BLUE 1 (YOU)`, `RED 3 · AI` and `BLUE 2 · HUMAN`, in the HUD's capitals, from the canonical id `blue-1`. The ally panel's owner line and the report both use it. The panel's owner line shows only while the panel is open.
- **The gap:** The wording was delegated.
- **The reach:** Any future player label.
- **Verdict:** sound.
- **Confidence:** high.

### I39 · The team plan is cached and refreshed every five seconds
- **When:** slice 12. **Who:** agent.
- **The choice:** `TeamMemory` holds the side's last plan and refreshes it every five seconds (`planner_decision_s` in `fixtures/skirmish-ai.json`), or at once when an assigned objective changes hands (I44). It also holds the influence grid, stance timers, push wait, party locks and gathered-group flags. Carrier reservations live in each doctrine (I51). None of it is in the digest.
- **The gap:** Delegated.
- **The reach:** Planner cost stays bounded at 5v5, and the command stream stays the replay authority (D40).
- **Verdict:** sound.
- **Confidence:** high.

### I43 · How AIs measure distance, and when they replan a blocked route
- **When:** slice 12 (transport review). **Who:** agent.
- **The choice:**
  - Picking the nearest target uses true 3D body distance, so a helicopter overhead isn't "at" the tank beneath it.
  - Artillery uses an explicit ground distance.
  - A unit whose route is reported blocked gets a fresh plan (`live_goal`) instead of waiting forever.
  - A human seat taken over by AI mid-battle starts its AI at once, and a seat keeps one AI across controller changes, so its plan and memory survive.
- **The gap:** Review findings with no plan coverage.
- **The reach:** All AI targeting and takeovers.
- **Verdict:** sound.
- **Confidence:** high.

### I42 · A route estimate to an off-map point is "no route"
- **When:** slice 12. **Who:** agent.
- **The choice:** The AI asks "how long to drive from this entry to there?" (`Battle::route_time`, which delegates to `entry_route_time`). For a point off the map, the estimate used to snap to the grid edge and answer, for example, 130 s for a point 2 m outside. The AI then picked goals that movement refused. Now the answer is "no route". The battle does not filter AI orders to hide this.
- **The gap:** A bug fix. The choice is to answer "no route", not to clamp.
- **The reach:** Every entry and goal choice.
- **Verdict:** sound.
- **Confidence:** high.

### I47 · The plan owns stance; the battle passes AI orders through untouched
- **When:** slice 13. **Who:** agent.
- **The choice:** `TeamPlan::assignment_for` is the one place a stance becomes each player's assignment. The battle used to steer and filter AI orders as well. Now it only delivers them, and any refusal is recorded (I09). The planner reads the tick rate from the rules, so no lint exemptions were needed.
- **The gap:** There were two owners of "what an AI should do".
- **The reach:** Future team behaviours add to the plan, not the battle.
- **Verdict:** sound.
- **Confidence:** high.

### I52 · A vehicle may shove a fence panel it only touches
- **When:** found in a 13b probe; landed on main in its own commit (`ad809fdab`). **Who:** agent.
- **The choice:** A Stryker sat against a fence for a whole match. The shove rule refused any body that would slide even a fraction of a millimetre deeper into the panel it touched, and generated fence lines overlap by millimetres. Now bodies within the 1 cm shove clearance count as touching, not pressing. No pinned digest moved.
- **The gap:** A physical rule's tolerance.
- **The reach:** Every vehicle against fences.
- **Verdict:** sound.
- **Confidence:** high.

### I23 · Deploy plays exactly the map, and the request, the lobby previewed
- **When:** slice 09; one request builder since the web lane (review finding 2). **Who:** agent.
- **The choice:** Each preview tries the lobby's own map seed first. If that seed's map is refused (unfair or illegal), hidden retries try fresh seeds. When a retry succeeds, the lobby adopts that seed without starting another preview, so the seed Deploy sends is the map on screen. Changing a map input starts the new preview within the same click, so an older map is never shown as Deploy's. One function, `skirmishRequest` (`skirmishSetup.ts`), builds the preparation request for both the lobby's preview and the battle route, so Deploy prepares exactly the request that was previewed, not a second copy that could differ.
- **The gap:** How a hidden retry and the lobby's seed agree, and who builds the request.
- **The reach:** D14's guarantee that Deploy reproduces the preview.
- **Verdict:** sound.
- **Confidence:** high.

### I24 · Opening the lobby offers a fresh map
- **When:** slice 09. **Who:** agent.
- **The choice:** Each visit to the lobby draws a new map seed, unless the menu address names one, as the old unpinned Play did.
- **The gap:** Unspecified.
- **The reach:** Shared lobby links pin a map by naming its seed.
- **Verdict:** sound.
- **Confidence:** high.

### I22 · The lobby screen is one component with one picker
- **When:** slice 09. **Who:** agent.
- **The choice:** `apps/battle-lab/src/lobby/Lobby.tsx` replaces the menu's old New Battle page. One small open-on-click list, `Pick`, serves the side factions, a Blue AI seat's controller and the region. The strip's other settings are option rows. Region became a list so the strip fits at 1280 px.
- **The gap:** Delegated.
- **The reach:** Any new lobby setting.
- **Verdict:** sound.
- **Confidence:** high.

### I27 · A request is pinned to this build in one place
- **When:** slice 09; merged into `skirmishRequest` in the web lane. **Who:** agent.
- **The choice:** The lobby model is pure TypeScript and can't know the generator version, preset revision, template catalogue or limits. `skirmishRequest` fills them in through `generationRequest`, along with the entry count for the players. The old helper file `lobby/candidate.ts` is gone; the lobby's `lobbyCandidate` only supplies the seed each preview attempt tries. The battle route and the map-picture lab use the same builder.
- **The gap:** What slice 06's request code didn't already own.
- **The reach:** A generator version bump flows to the lobby automatically.
- **Verdict:** sound.
- **Confidence:** high.

### I28 · Unpinned Play is gone
- **When:** slice 09. **Who:** agent.
- **The choice:** The lobby links only to maps it admitted, so the old unpinned `/battle?play=1` became unreachable. It was deleted, along with the battle route's own admission and its helpers (`playHref`, `preparedBattleHref`, `usePublishBattleAddress`). A `play` parameter is now refused like any unknown one. The generated-battle scene's checks for exhausted generation and stalled workers moved to the lobby preview.
- **The gap:** The plan left the old path's fate open.
- **The reach:** The lobby's preview is the only admission path.
- **Verdict:** sound.
- **Confidence:** high.

### I19 · The lobby's state is one plain object, numbered as a link names it
- **When:** slice 07; numbering settled in the web lane. **Who:** agent.
- **The choice:** The lobby model (`apps/battle-lab/src/lobby/model.ts`) is `{ players, map, battleSeed }` plus the current size floor. It is serializable (D26), and its operations are pure functions: preset, add, remove, set controller, seat human, set faction, set size, reroll, to URL. Every change re-derives the players through `skirmishPlayers`, which numbers each side's players by their place, exactly as a link names them. So removing Blue 2 from a three-player side renumbers Blue 3 as `blue-2`; the lobby and the link never disagree about who `blue-2` is.
- **The gap:** The representation and the ids' stability were delegated.
- **The reach:** Online play (D26) can send this object as-is. A player's id is its place, not a permanent name; online play will need a separate seat identity if a player must keep its id when another leaves.
- **Verdict:** sound.
- **Confidence:** high.

### I17 · The camera starts on your lane
- **When:** slice 06. **Who:** agent. This fixes landmine L10's camera part.
- **The choice:** You are Blue 2 in a 3v3. The camera opens on Blue's entry at index 2 (your position among Blue players, modulo the entry count). This is framing only: you can buy at any entry (D8). With no human, the camera opens on Blue's first entry, which is the centre on every map now that synthetic entries are ordered too (I18).
- **The gap:** Where a team player's camera starts.
- **The reach:** Framing only.
- **Verdict:** sound.
- **Confidence:** high.

### I15 · The battle link grammar
- **When:** slice 06; defaults documented in the web lane. **Who:** agent (spelling delegated "within readable and canonical").
- **The choice:** A link reads, for example, `type=mixed&size=large&seed=7&blue=h,a,a&blue_faction=…&red=a,a,a&red_faction=…`, plus the battle seed:
  - each side lists its players in order by controller code: `h` Human, `a` active AI, `p` passive AI;
  - player ids are derived as `blue-1`, `blue-2` and so on, the same way the lobby numbers them (I19);
  - both sides' players and factions are required; map type, size and both seeds default when left out (I16);
  - a wrong value, or a parameter the battle doesn't read (the old `faction=`/`enemy=` shape), is refused by name (D42).

  Preparation builds the scenario as parsed JSON in Rust, with no string splicing. The stress-benchmark path still splices.
- **The gap:** Delegated.
- **The reach:** Every shareable battle. I16 covers the human rules.
- **Verdict:** sound.
- **Confidence:** high.

### I12 · The generator version moves to `layout-19`
- **When:** slice 04. **Who:** agent (the spec required a bump).
- **The choice:** The entry count is part of a map's identity, so the generator version became `layout-19`. The generation request gained `entries_per_side`. In Rust it defaults to 1 and is left out when it is 1, so one-entry requests keep their exact bytes and map hashes. In TypeScript it is optional. Generation refuses values outside 1–3. Two contract tests still use older version strings (`layout-18`, `layout-5`), but only as sample input for "an old version is refused".
- **The gap:** Required.
- **The reach:** Saved map links from before `layout-19` are refused by version.
- **Verdict:** sound.
- **Confidence:** high.

### I07 · An unowned killer earns nothing and trips a debug check
- **When:** slice 02. **Who:** agent.
- **The choice:** Every skirmish unit has an owner. If a killer somehow had none, a debug assertion fires and the kill pays nobody, rather than crediting the side's first player and hiding the bug.
- **The gap:** Unspecified.
- **The reach:** This guards the per-player economy.
- **Verdict:** sound.
- **Confidence:** high.

### I06 · Totals are kept per player; side totals come from one helper
- **When:** slice 02. **Who:** agent (corrected in review).
- **The choice:** Units bought, credits spent, kills and refunds are counted per player. Side totals are derived by one helper, `side_totals()`, never kept separately.
- **The gap:** Unspecified.
- **The reach:** The report (I34) and the tournament.
- **Verdict:** sound.
- **Confidence:** high.

### I02 · A skirmish's contract is checked before a battle is built
- **When:** slice 01; widened in the Rust lane. **Who:** agent.
- **The choice:** A link with a bad player list (say, duplicate ids) used to panic the wasm module. Now preparation checks the contract first and returns a normal refusal, which the refusal screen shows. The contract, `validate_player_contract` in `contract::players`, says who may play (D29) and that a skirmish carries no authored units, events, scripts, opponent or encounter: its players buy everything it fields. That rule lives only there; the battle's own asserts and a dead "side's first player" fallback were removed. A skirmish bundle that breaks it gets one `invalid_skirmish` diagnostic at `$.scenario`.
- **The gap:** A review finding, then the Rust-lane review.
- **The reach:** This is the one admission point for players and skirmish content.
- **Verdict:** sound.
- **Confidence:** high.

### I01 · A Human seat must be marked human-capable, and a generated skirmish marks exactly its humans
- **When:** slice 01; generated-skirmish rule in the web lane. **Who:** agent.
- **The choice:** A player has a `human_capable` flag. Validation refuses a `human` controller on a seat that isn't marked human-capable, as the old bundle format already did. In a generated skirmish (the lobby and links), a player is human-capable exactly when a human plays it (`skirmishPlayers`). So a human who hands the seat to the AI in the lobby makes it a plain AI seat, no longer human-capable. Authored scenarios can still mark future online seats human-capable before anyone sits in them (D26).
- **The gap:** Kept from the old bundle rule; generated seats had no rule.
- **The reach:** Online seats.
- **Verdict:** sound.
- **Confidence:** high.

### D1 · Every AI player runs its own brain
- **When:** slice 02. **Who:** you. The single policy owner (D45) and tick-0 controllers (the fix for landmine L2) are agent on your behalf.
- **The choice:** Each AI player runs its own `BalancedDoctrine`, with its own wallet, cap, units and opening. The old single "AI side" is gone. One function picks each player's policy. In a skirmish, every active-AI player runs its doctrine each turn, and the older per-player heuristic `controller_turn` runs only for authored non-skirmish scenarios. Without this, an AI could have been driven by both. Controllers are installed from the player list at tick 0, so a player with no units yet still has its AI. Every seat, including a Human one, has a doctrine ready, so a mid-battle takeover by AI starts at once (`Battle::set_controllers`).
- **The gap:** The old code had one AI per side and dropped controllers for players without units.
- **The reach:** Every per-player behaviour.
- **Verdict:** sound.
- **Confidence:** high.

### L4 · Every order is checked against its player
- **When:** slice 02. **Who:** agent on your behalf.
- **The choice:**
  - A human and two AI players on Blue issue orders in the same tick. Each has its own sequence counter and rate limit, so none is refused as out of sequence or rate-limited.
  - Ready, purchase and cancel orders are checked against the player that sent them, so an ally can't spend your money or ready for you.
  - Purchase ids are numbered per player.
  - The aircraft arrival delay and the AI's seeded choices (such as where its reinforcement rotation starts) mix in the player index, so two AIs with the same doctrine differ.
  - A bought unit spawns owned by the buyer.
- **The gap:** All of these were per side.
- **The reach:** This is the command contract online play will use.
- **Verdict:** sound.
- **Confidence:** high.

### D12 · Every player earns its own income
- **When:** slice 02. **Who:** you: "multiple income of course".
- **The choice:** Every player earns 200 credits a minute. A side with more players earns more in total, with no balancing between sides.
- **The gap:** None.
- **The reach:** Uneven matches (2v3) favour the larger side by design.
- **Verdict:** sound.
- **Confidence:** high.

### D20 · Ready is per player
- **When:** slice 02. **Who:** agent on your behalf (preference 1).
- **The choice:** Each player readies on its own. The match starts when every player is ready or the preparation timer runs out. Each AI readies after buying its opening.
- **The gap:** The walk didn't say whose Ready counts.
- **The reach:** Online lobbies.
- **Verdict:** sound.
- **Confidence:** high.

### D37 · A purchase's entry is chosen once, when you buy it
- **When:** slice 05. **Who:** agent on your behalf.
- **The choice:** When a purchase is reserved, the battle picks its entry (D8) and stores it on the pending purchase. The purchase preview and dispatch read that stored entry, and nothing re-derives it. The AI picks its lane's entry through the team plan before buying.
- **The gap:** There were three re-derivations.
- **The reach:** A pending purchase's arrival point is stable even if the front moves.
- **Verdict:** sound.
- **Confidence:** high.

### D15 · Jets arrive behind the centre entry
- **When:** slice 05. **Who:** you.
- **The choice:** A bought jet crosses the side's back edge at the point nearest the centre entry. How jets leave is unchanged. Helicopters use the chosen entry like ground units.
- **The gap:** None.
- **The reach:** "Centre" is the side's first entry, which is the middle road on every map, synthetic ones included (I18).
- **Verdict:** sound.
- **Confidence:** high.

### D7 · Supply trucks resupply allies too
- **When:** walk; no code change. **Who:** you.
- **The choice:** A supply truck resupplies every friendly unit within reach, allies included. That was already the rule in `supply.rs`, which filters by side. Each AI buys and keeps its own trucks.
- **The gap:** None.
- **The reach:** What a side shares follows physical reach (preference 2).
- **Verdict:** sound.
- **Confidence:** high.

### D10 · More players means more objectives taken at once
- **When:** slices 04 and 12. **Who:** you. That map size sets the objective count (D50) is agent on your behalf.
- **The choice:** Each AI fights its lane by buying into it, and D8 brings those units in at that lane's entry. A bigger match gets more objectives because the size floor (D18) forces a bigger map, and map size sets the objective count: Large and XL have three central objectives, smaller maps one, plus mirrored pairs. No new objective rule was added.
- **The gap:** None.
- **The reach:** A retune of objective counts is a mapgen change.
- **Verdict:** sound.
- **Confidence:** high.

### D18 · Map size ratchets up with the players
- **When:** slices 07 and 09. **Who:** you. "Medium for 2v2" is from the original spec.
- **The choice:**
  - Adding a player raises the size to the floor for the larger side: 1 → S, 2 → M, 3–4 → L, 5 or more → XL.
  - Removing a player never lowers the size you chose, but it re-opens smaller sizes down to the new floor. A first version kept them closed.
  - Sizes below the floor show disabled, with the reason.
  - The link carries the size actually used.
- **The gap:** None.
- **The reach:** I13 covers where the table lives.
- **Verdict:** sound.
- **Confidence:** high.

### D17 · Sizes read S · M · L · XL
- **When:** slice 07. **Who:** you.
- **The choice:** The size picker reads S · M · L · XL, for the internal `small`/`medium`/`large`/`xl`. The earlier "XS" label is gone.
- **Verdict:** sound. **Confidence:** high.

### D22 · The lobby is an order-of-battle screen
- **When:** slice 09. **Who:** you (Direction 3 from the mock; Direction 2 skipped). The banner contents (D25) are yours. The banner rules are agent.
- **The choice:** Blue's player banners sit on the left, Red's on the right, and the map between them under an "N VS M" header. A bottom strip holds presets, map type, size, region, seed status, Reroll and Deploy. Each banner shows:
  - a big numeral (its position on its side);
  - its controller, You or AI;
  - a delete ×, hidden for You and for a side's last player.

  "Add player" hides at five. A player added to a side takes that side's faction (a bug fix in the web lane: it used to take a default). Red's controller is a fixed "AI" label, since only Blue can seat the Human. Picking one side's faction never changes the other's.
- **The gap:** The banner rules were delegated.
- **The reach:** A native lobby should copy this layout.
- **Verdict:** sound.
- **Confidence:** high.

### D23 · The preview map is turned so Blue's edge faces Blue's column
- **When:** slice 08. **Who:** you.
- **The choice:** The picture rotates the map so Blue's back edge sits on the left, beside Blue's banners.
- **Verdict:** sound. **Confidence:** high.

### D39 · The preview keeps a Rust-drawn picture of the exact map Deploy will play
- **When:** slice 08. **Who:** you (D14: preview by full admission, Deploy re-prepares). Drawing in Rust (D39) is agent on your behalf.
- **The choice:** Each preview runs a full admitted preparation in a worker. It keeps only the request, the compiled map and the final sites, plus an SVG of the map drawn in Rust by `mapgen::inspect::skirmish_picture` from that compiled map, then closes the worker. Deploy re-prepares that exact request (I23) and gets the same battle. A newer preview cancels an older one. Its states are idle, loading, retrying (with the attempt number), ready and failed. Only previews ask for the picture; battles never pay for it. Drawing from the compiled map, not the generator's plan, means the picture shows what you fight on. It can also be reused by a native lobby.
- **The gap:** D14 as first written kept too little to draw a picture.
- **The reach:** Any future map preview (native, a map browser).
- **Verdict:** sound.
- **Confidence:** high.

### D53 · Entry spacing and river mouths are hard checks, and the preview shows loading at once
- **When:** slices 04 and 08. **Who:** code (the probe's evidence).
- **The choice:** The probe's rough arm pass broke 150 m spacing and river-mouth clearance on Large and XL maps. So these are hard checks: a map that breaks either is refused and regenerated, never tuned through. Admission (`admit_skirmish`, the one entry point) also needs every entry to stand and to route to every objective for each representative unit. Fairness compares each side's best travel time over its entries, within 15 %. An XL 5v5 takes seconds to generate, so the preview shows a loading state from the first frame.
- **The gap:** None.
- **The reach:** Any loosening of these is a named decision.
- **Verdict:** sound.
- **Confidence:** high.

### D42 · Old battle links are refused, and old menu links open the lobby
- **When:** slice 06; docs aligned with the code in the web lane. **Who:** you (D19). The wording (D42) is agent on your behalf.
- **The choice:** A pre-team battle link is refused on the existing refusal screen, which names the problem: an unknown `faction`/`enemy` parameter, or missing players for an old link without `enemy` ("missing players: blue and red are required"). An old menu link opens the lobby with its defaults. There is no migration code. The docs used to say a link must name everything; they now say what the code does: players and factions are required, and the map type, size and seeds may default (I16).
- **The gap:** An old link without `enemy` had no unknown parameter to name.
- **The reach:** None. There is no backward compatibility by design.
- **Verdict:** sound.
- **Confidence:** high.

### D26 · The lobby model is ready for online play, which isn't built
- **When:** slice 07. **Who:** you: online multiplayer will reuse this model.
- **The choice:** The lobby state is plain serializable data (I19). Moving the human is one "seat a human in player X" operation, which turns the old Human seat into AI. In battle, a departing human hands the seat to AI through the existing controller change (`Battle::set_controllers`); the battle screen's button reads "Hand your player to the AI" (it used to read "Take human slot over by AI"). The controller set today is Human, active AI and passive AI. An **Open** seat can be added later. The lobby UI offers only You and AI.
- **The gap:** None.
- **The reach:** Online play. The reverse takeover (a human taking an AI seat) has a known limit (I08).
- **Verdict:** sound.
- **Confidence:** high.

### D27 · Desktop browser only; native is tracked separately
- **When:** throughout; the native-gaps entry corrected at slice 14. **Who:** you.
- **The choice:** Team skirmish is built for the desktop browser. Item 10 of [native gaps](../../native-gaps/README.md) ("Native has no team skirmish") says what native can do today: it replays the browser's parity captures of *encounters*, fielding each unit kind with one look. It can't replay a skirmish, because that needs the battle's players, and each side's faction from them, passed to the shared fielding as the browser does (a side-keyed Blue/Red pair through `presentation-wasm`). Allies' models need nothing more. Beyond replay, native lacks the lobby, the ally colour and shape cue, the read-only ally panel with own-only selection, and the per-player report.
- **The gap:** The item first overstated what native replays.
- **The reach:** A native team skirmish starts from replay, then UI.
- **Verdict:** sound. **Confidence:** high.

### D28 · The finish line is a 5v5 AI-versus-AI tournament; performance is a later spec
- **When:** planning, revised at slice 14 (2026-10-19). **Who:** you (5v5 AI vs AI); you again on 2026-10-19: "don't bother with the browser benchmark, we can push the perf stuff to a future spec. let's just try to close this spec as soon as we can".
- **The choice:** The spec closes on 5v5 tournament matches that prove behaviour. The planned 300-unit browser frame-time benchmark, the `benchmark` scene's real-time check, and judging the fixed effect limits by eye and ear at 300 units (24 muzzle lights, 1,024 pending sounds; landmine L13) move to a future performance spec. The `street` scene still runs in the full suite.
- **The gap:** None.
- **The reach:** Nothing in this spec proves 5v5 frame time.
- **Verdict:** sound.
- **Confidence:** high.

### D29 · One player type, one player list, one validator
- **When:** slice 01; one validator since the Rust lane. **Who:** you (D29, overriding the original "don't reuse `PlayerSlot`"). The single list (D33) and the optional faction (D54) are agent on your behalf. This fixes landmine L8.
- **The choice:** There was one player concept for authored scenarios and another for skirmish. Now one `Player { id, side, faction, controller, human_capable }` lives in `contract::players`. `ScenarioDefinition.players` is the only list. The bundle's separate list is gone, and a unit's `owner` names a player id. An Economy bundle derives its skirmish from its players, and the scenario builder and the lobby share the type.

  One function, `validate_players(players, seating)`, checks every list. Its `Seating` argument says which rules apply:
  - **Authored** scenarios admit a Neutral player (scenery and civilians), which has no faction and can never be human; Blue and Red players need a faction.
  - **Skirmish** admits Blue and Red only, at least one player per side, and one faction per side.

  Before the Rust lane, the authored and skirmish rules were written twice.
- **The gap:** Whether Neutral needed an invented faction.
- **The reach:** Every scenario and every link.
- **Verdict:** sound.
- **Confidence:** high.

### D51 · One word: player
- **When:** slice 01. **Who:** you: "pick one and stick to it".
- **The choice:** The code says `Player`, `players`, `CommandEnvelope.player` and `AuthoredScript.player`. `occupied_slots` became `unit_count`. "Slot" now keeps only its unrelated meanings (squad member, garrison seat); it used to mean six things (landmine L11). The JSON field is `unit_count`, and the TypeScript observation view calls it `unitCount`, following each layer's casing. A few comments still say "slot" for a player (one is part of I53).
- **Verdict:** sound. **Confidence:** high.

### D31 · One observation per side, with a table of that side's players
- **When:** slice 03. **Who:** agent on your behalf. This fixes landmine L5.
- **The choice:** Allies share vision, so each side still gets one observation, not one per player. Its skirmish part carries a `players` table: every own-side player's index, id, controller, ready flag, credits, unit count, cap and pending purchases. Each own unit carries its `owner`. The HUD shows only your row. The old side-level header fields are gone from the published frame. (Their in-memory leftovers are I04.)
- **The gap:** Whether to publish per player or per side.
- **The reach:** Online play still sends one frame per side.
- **Verdict:** sound.
- **Confidence:** high.

### D32 · A player's id is its identity; its index is derived
- **When:** slice 01. **Who:** agent on your behalf.
- **The choice:** A player's string `id` (`blue-2`) is its identity in scenarios, links and replays. `PlayerIndex`, its position in the player list, is always derived and never stored in a scenario. The digest, random streams and the binary publication use the index, because the publication is numeric; the scenario's identity hash keeps the ids.
- **Verdict:** sound. **Confidence:** high.

### D34 · A battle may have no human, at any size
- **When:** slice 01; links aligned in the web lane. **Who:** agent on your behalf.
- **The choice:** A setup with zero humans is valid, for AI-versus-AI tournaments, benchmarks and watched battles, 1v1 included. Such a battle is viewed through Blue's observation, and the top bar and result read as Blue (I35). "Exactly one Human, on Blue" is a lobby and link rule for battles that have a human, not a simulation rule (I16).
- **Verdict:** sound. **Confidence:** high.

### D40 · Planner memory sits outside the digest
- **When:** slices 12–13b. **Who:** code (the existing skirmish-AI invariant, D6), with D40 agent on your behalf.
- **The choice:** The team plan is rebuilt from the side's observation plus planner memory, evaluated in player order. The memory holds the cached plan, influence grid, stance and push timers, party locks and gathered-group flags (I39). Like the doctrine's own state, it isn't digested. The command stream is the replay authority: replaying the same commands reproduces the battle without recomputing the plan.
- **Verdict:** sound. **Confidence:** high.

### D41 · Digest policy
- **When:** throughout. **Who:** you (D11: re-baselines are named decisions, and non-skirmish digests stay identical). The slice list is agent on your behalf.
- **The choice:**
  - Slice 01 changed no digest, proving it was a change of shape only.
  - Re-baseline 1: slice 02 (ownership) re-baselined the skirmish digests, with before and after values in its commit.
  - Re-baseline 2: slice 12, values in its commit. Slice 13 was also allowed one but moved no pinned digest.
  - Re-baseline 3, in the Rust lane (commit `24c40a9a1`), named for three changes together: a unit's digest hashes its owner's `PlayerIndex` (D44); the skirmish digest hashes every entry's dispatch clock and turn unconditionally, where it used to skip them in a 1v1 (a branch no decision named, against preference 4); and synthetic entries list the centre first (I18). Only skirmish digests moved; every objective, score and result row is unchanged. Before and after values are in the commit.
  - Non-skirmish digests never changed.
- **Verdict:** sound. **Confidence:** high.

### D44 · A unit's owner is in its digest, by index
- **When:** slice 02; completed in the Rust lane. **Who:** agent on your behalf.
- **The choice:** The unit digest used to skip `owner`, yet the cap, controllers and money depend on it. A unit's digest now hashes its owner's `PlayerIndex`, derived by one function (`owner_index`) that the publication's owner field also uses, rather than a hash of the id string (D32). Part of re-baseline 3 (D41).
- **Verdict:** sound. **Confidence:** high.

### D46 · The tournament runs every player as an in-battle AI
- **When:** slices 02 and 12. **Who:** agent on your behalf.
- **The choice:** The tournament used to drive two doctrines from outside the battle. Now every player runs as an in-battle AI, a player can take a scripted buy list (I05), and each battle exposes its `ai_receipts()` log. The external driver is deleted.
- **Verdict:** sound. **Confidence:** high.

### D30 · Old replays need nothing
- **When:** planning. **Who:** code. This also answers landmine L7, "replays carry no setup".
- **The choice:** Any simulation change already invalidates saved replays through the engine build id (`crates/sim/build.rs`), so no replay migration exists.
- **Verdict:** sound. **Confidence:** high.

### D49 · A throwaway probe went first
- **When:** slice 00. **Who:** agent on your behalf.
- **The choice:** Before any contract work, a throwaway branch measured multi-entry admission, preparation time and opening throughput. If fair multi-entry maps had been impossible, the plan would have died cheaply. The probe is what produced D52 (equal entries) and D53 (hard checks).
- **Verdict:** sound. **Confidence:** high.

### L12 · Players live inside the existing Blue and Red sides
- **When:** planning; written into `development.md` in the Rust lane. **Who:** agent on your behalf.
- **The choice:** An earlier attempt at arbitrary sides died on 172 places that assume two sides. So sides stay Blue, Red and Neutral, and players live inside sides. `development.md`'s game rules now state the split: a side's players are one team and share the side's faction, its roads in, the objectives and their score, and everything any of its units sees; each player owns its units, its credits (income and kill rewards), its own unit cap and its controller, and readies on its own. The bounty pool also belongs to the side (D13).
- **Verdict:** sound. **Confidence:** high.

---

## Trivial discretion

Four entries, listed only so their ids resolve:
- **I03:** the scenario bundle format string is `scenario-bundle-2`, a hard cutover with no migration.
- **I11:** the market-town publication capture window is rebuilt by a kept test that is gated behind `BLESS_PARITY` and `PARITY_RUN`.
- **I26:** Deploy is the lobby's one solid accent plate, not the mock's amber, so no new colour role is added. The seed shows under "Ready", and there is one Reroll, in the strip.
- **I33:** test scenes and gallery fixtures. `allies` and `ally-help` are 2v2 browser scenes; `ally-help` names its battle seed (the link default) so its route is its own, and the scene registry allows one scene per route. The `air-ingress` lab's Red is a passive AI, so its anti-air doesn't shoot the timed jet. The `allies` smoke search is bounded to 60 s. The `menu-skirmish` shot hides the map and seed, which change every visit. The report gallery draws `report-*` shots from `reportSpecimens.ts`.

---

## Where the old rows went

| Old row | Now |
|---|---|
| D1, D45, L2 | D1 |
| D2 | D2 (sound since 13c) |
| D3 | D3 (see D55 for side objectives) |
| D4 | D4 |
| D5 | D5; the help gap is I48 |
| D6, D40 | D40 |
| D7 | D7 |
| D8′, D52, L1 | D8 |
| D9 | D9 |
| D10′, D50 | D10 |
| D11, D41 | D41 |
| D12 | D12 |
| D13, D43 | D13 |
| D14, D39 | D39 |
| D15 | D15 |
| D16 | D16 |
| D17 | D17 |
| D18 | D18 |
| D19, D42 | D42 |
| D20 | D20 |
| D21, D35 | D35 |
| D22, D25 | D22 |
| D23 | D23 |
| D24 | D24 |
| D26 | D26 |
| D27 | D27 |
| D28, L13, slice 14 performance row | D28 |
| D29, D33, D54, L8 | D29 |
| D30, L7 | D30 |
| D31, L5 | D31 |
| D32 | D32 |
| D34 | D34 |
| D36 | D36 |
| D37 | D37 |
| D38, L10 (setup part) | D38 (sound since the web and Rust lanes) |
| D44 | D44 |
| D46 | D46 |
| D47 | D47 |
| D48 | D48 |
| D49 | D49 |
| D51, L11 | D51 |
| D53 | D53 |
| D55 | D55 |
| D56 | D56 (sound since 13c); its purchase half is I56 |
| D57 | D57 |
| L3 | I09 (sound, narrowed: only malformed non-purchase refusals assert) |
| L4 | L4 |
| L6 | I34 |
| L9 | I14 (sound since the dispatch lane) |
| L10 (camera part) | I17 |
| L12 | L12 |
| Slice 00 and 04 measurement sections | Evidence, dropped; their conclusions are D8, D36 and D53 |
| 01 rows | I01, I02, I03; casing in D51; temporary link names replaced by I15 |
| 02 rows | I04, I05, I06, I07, I08; kill split in D13 |
| 03 rows | D31, I11; the decoder-boundary fix dropped as fix narration |
| 04 rows | D36, I12, I13; `centre_entry` dropped (replaced by `sim::entries`) |
| 05 rows | D8, I14 |
| 06 rows | I15, I16, I17, I18, D38 |
| 07 rows | I19, D35; the stub row dropped (replaced by slice 06) |
| 08 rows | I20, I21, D39, I12; the XL "OPEN" row is resolved in I21 |
| 09 rows | I22–I28, D18, D22; timing evidence dropped |
| 10 rows | D47, D16, I29, D24, I30, I31, I32, I33; the fixture re-bless and two stale-picture approvals dropped as evidence |
| 11 rows | I34, I35, I30, I36, I37, I38, I33 |
| 12 rows | I39, I40, I41, I42, I43, I44, I46, D48, D9, I10, I45; the final-tournament "OPEN" row is resolved by slice 13 |
| 13 rows | D4, D5, I46, I47, I48, I49, I33; the tournament evidence dropped |
| 13b rows | D55, I50, I51, I52, I46, I44, D57, D3; the tournament evidence and memory-hygiene item dropped |
| 13c rows: value owner and engage check | D56 |
| 13c rows: strength scale, whole-side threat, retune | D56, I45 |
| 13c rows: party sizes, `recruit`, help surplus | D56, D55, D4 |
| 13c rows: purchase ranking, air pivot, `purchase_noise` removed | I56 |
| 13c row: armour in the recipe | D2 |
| 13c quick check and full runs | Evidence, dropped; the trade-offs you own are one-line consequences in D2 (3v3 seed 4) and D56 (1v1 seed 1 at 35 min) |
| 14 row: benchmark dropped | D28 |
| Native gaps item 10 correction | D27 |
| Web lane: lobby ids numbered by place | I19 |
| Web lane: `human_capable` exactly when a human plays | I01 |
| Web lane: one human on Blue or none, at any size | I16, D34 |
| Web lane: `MAX_PLAYERS_PER_SIDE` in `skirmishSetup.ts` | D35, I13 |
| Web lane: `sizeTier` is the TS size owner | I13 |
| Web lane: one request builder, `skirmishRequest` (review finding 2) | I23, I27 |
| Web lane: HUD shows the commanding player's economy, none when watching | D16 |
| Web lane: an added player takes its side's faction | D22 |
| Web lane: links require players and factions; map fields default; old shapes refused | I15, I16, D42 |
| Web lane: builder economy from the simulation's setup | D38 |
| Web lane: top bar reads the viewer's side; a watcher reads as Blue | I35, D34 |
| Web lane: `ally-help` names its battle seed; one scene per route | I33 |
| Web lane: "Hand your player to the AI" | D26 |
| Web lane: dismissed report findings (score order; old version strings) | Dropped; noted in I35 and I12 |
| Dispatch lane: blocked purchase holds only its entry | I14 |
| Dispatch lane: refused purchase holds only later purchases | I10 |
| Dispatch lane: refused orders recorded; malformed non-purchases assert | I09 |
| Dispatch lane: passive-AI purchases return fire only (held) | I55 |
| `ai_controlled` lane: client orders to an AI's player refused, in skirmish too | I08 |
| `ai_controlled` lane: tests declare ordered seats Human (`human_duel`) | I08 |
| `ai_controlled` lane: inherited command sequence on takeover | I08 (known limit) |
| Rust lane: one player validator with `Seating` | D29 |
| Rust lane: a skirmish carries no authored content; `invalid_skirmish` | I02 |
| Rust lane: one Rust reader of `skirmish-sizes.json` | I13 |
| Rust lane: owner `PlayerIndex` in the unit digest | D44, D41 |
| Rust lane: unconditional entry clock and turn in the digest; re-baseline 3 | D41 |
| Rust lane: synthetic sites centre first | I18 |
| Rust lane: builder setup from `synthetic_skirmish_setup` | D38 |
| Rust lane: tournament `--buy-*` scripts every player on a side | I05 |
| Rust lane: `development.md` states the side/player split | L12 |
| Review finding 5 | I04 (held) |
| Review finding 7 | I53 (held) |
| Review finding 11 | I54 (held) |
| Held items: passive-engagement owner | I55 (held) |
| Held items: `ai_controlled` doc says "slot" | I53 (held) |
