# Arbitrary-slot skirmish choices ledger

These decisions came from the explore-unknowns walk on 2026-10-16. Each row records who closed it:
- **user**: the user decided it.
- **user (earlier)**: the user said it in an earlier session, recorded in the T3 database.
- **territory**: the code settled it.
- **agent**: answered on the user's behalf under the confirmed preferences below.

## Confirmed preferences and delegation scope

The user opted in to agent answers for the rest of the walk, on these preferences:

1. Every slot is a real player: its own wallet, cap, army, logistics. It is never just a share of the side.
2. What a side shares follows from where things physically are, not from ownership rules.
3. The AI acts like a competent war-film army: concentrated combined-arms groups, with extra players taking more objectives at once.
4. One general system from 1v1 to 10v10. A 1v1 is not a special case, and nothing has to stay the same as it is today.
5. Reuse existing mechanisms rather than add new machinery.
6. Lobby rules stay simple and predictable.

The delegation covered the planning walk only. It does not authorise implementation choices beyond this ledger.

## Decisions

| # | Decision | Source and reason |
|---|---|---|
| D1 | Each AI slot runs its own `BalancedDoctrine` with its own wallet, cap, units and opening. `ai_side` is deleted. The per-slot `controller_turn` heuristic (`battle.rs:1521`) stays only for authored-bundle slots that are not in a skirmish. | **user** |
| D2 | Each AI fields a complete army of its own: armour, infantry, recon, AA and supply trucks. It never skips a role because an ally covers it. | **user**: each AI manages its own logistics |
| D3 | A per-side team planner gives each AI slot a lane and a main objective in that lane. An objective the human's units are visibly heading for (`OwnUnit.goal`) counts as covered. | **user** (chose C, then "do all") |
| D4 | The team planner routes requests for help. A slot losing its objective asks, and the nearest ally with spare units responds. | **user** |
| D5 | The team planner sets a shared stance (push, hold, fall back) and times pushes together. | **user** |
| D6 | *(Amended by D40.)* The team plan is rebuilt from the side's observation in slot order and is never digested. The command stream replays. | **territory**: invariant from `specs/done/skirmish-ai/README.md` |
| D7 | A supply truck resupplies every friendly unit within reach, allies included (current `supply.rs` rule). Each AI buys and keeps its own trucks with its army. | **user** |
| D8′ | A side's entries are shared, with no slot assignment. Entry count follows slot count: 1 slot → 1 entry, 2 → 2, 3 or more → 3 (centre, left, right). Ground units and helicopters arrive at the side's entry with the shortest drive to where they were ordered; ties go to the centre. | **user** (corrected the original group-assignment plan) |
| D9 | Each AI keeps the bulk of its army as one cohesive combined-arms group in its lane. Only scouts, outposts and defensive positions scatter. | **user**: spread units get picked apart |
| D10′ | More players means more objectives taken at once. The team planner hands out lanes; an AI fights its lane by ordering purchases there, and D8′ brings them in at that lane's entry. | **user** |
| D11 | *(Amended by D41.)* D9 applies at every match size. Skirmish digests and fixtures are re-baselined exactly twice, each a named decision: once when the setup changes shape, once when AI grouping changes (D41 names the slices). Non-skirmish digests stay identical throughout. | **user** |
| D12 | Every slot earns 200 credits/min. A side with more players earns more; there is no balancing between sides. | **user**: "multiple income of course" |
| D13 | The bounty pool stays per side. Kill rewards, the bonus and refunds go to the wallet of the owning slot. Death provenance records the killer's slot. | **user (earlier)**, "team bounty", 2026-10-06; **territory** `settlement.rs` |
| D14 | *(Amended by D39.)* Each preview runs a full admitted prepare through `admitBattle` and keeps only the request, map and final sites, then closes its worker. Deploy re-prepares that exact request. A newer preview cancels an older one. | **user** |
| D15 | Jets cross the back edge at the point nearest the side's centre entry; how they leave is unchanged. | **user** |
| D16 | Your own units keep Blue. All allies share one allied colour. Selecting an ally opens a read-only panel showing its owner (e.g. "Blue 2 · AI") with no order controls. Box-select and double-click pick only your own units. Credits and cap shown are yours. The match timeline names each player. | **user**; the per-player timeline is **user (earlier)**, 2026-10-11 |
| D17 | The size picker reads S · M · L · XL, for the internal `small`/`medium`/`large`/`xl`. The "XS" label from `cba1ffdc9` goes. | **user** |
| D18 | Size ratchets. Adding a player raises it to the minimum for the larger side (1 → S, 2 → M, 3–4 → L, 5 or more → XL). Removing a player never lowers it. Sizes below the minimum show disabled with a reason. The URL carries the size actually used. | **user**; Medium for 2v2 from the original spec |
| D19 | *(Amended by D42.)* An old battle link shows the existing refusal naming the unknown parameter. An old menu link opens the lobby with defaults. No migration code. | **user** |
| D20 | Ready is per slot. The match starts when every slot is ready or the timer runs out; each AI marks itself ready after buying its opening. | **agent**: preference 1 |
| D21 | *(Amended by D35.)* The setup and simulation accept any number of slots per side. The lobby's maximum of 5 per side is data. | **agent**: preference 4 ("10v10") and 5 |
| D22 | Lobby layout follows the mock's Direction 3, "Order of battle": slot banners with a big index numeral on each side, the map between them under an "N VS M" header, and a bottom strip of presets, map type, size, region, seed status, Reroll and Deploy. | **user** (steal: overall layout; Direction 2 skipped) |
| D23 | The preview map is turned so Blue's edge faces the Blue column. | **user** |
| D24 | The allied colour is pale ice `#b9f0ff`, added to `presentation.hud` for allies only. Allies also get a shape cue (for example a hollow selection mark) so they don't depend on lightness alone. | **user** picked the hue; the shape cue is **agent**, from the game-ui "form before colour" rule |
| D25 | A slot row shows its index numeral, its controller (YOU/AI) and a delete ×. | **user** |
| D26 | The lobby model is plain serializable state. Its controller set can later gain **Open**. Moving the human is one "seat a human in slot X" operation. In battle, a departing human hands the slot to AI through the existing controller transitions. Online play itself is not built. | **user**: online multiplayer will reuse this model |
| D27 | Desktop browser only. [Native gaps](../native-gaps/README.md) tracks what native needs. The shared presentation crate is updated here, never left two-faction. | **user** |
| D28 | The finish line is 5v5 AI versus AI: tournament matches for behaviour, and a browser benchmark battle for frame time with up to 300 units, its limit being a loose tripwire. | **user** (5v5 AI vs AI); the two-run split is **agent**, preference 5 |
| D29 | *(Amended by D33.)* One player-slot type in `contract`, used by skirmish setups and authored bundles alike. An Economy bundle derives its skirmish slots from its bundle slots. The scenario builder and the lobby share the slot model. This overrides the original spec's "do not reuse `PlayerSlot`". | **user** |
| D30 | Replay compatibility needs nothing: any sim change already invalidates saved replays through the engine build id (`crates/sim/build.rs:28`). | **territory** |

## Synthesis decisions (write-spec, 2026-10-16)

Three blind drafts with different biases fed the canonical plan:
- fewest slices (Claude);
- risk first (Codex, GPT 6.1 Sol);
- seam quality (Claude).

A research pass ([research.md](research.md)) also fed the team AI slices. The rows below are the calls made in merging them. They are **agent** decisions: either the drafts agreed independently, or one draft found something in the code that the walk had missed. Each names what it amends.

| # | Decision | Source and reason |
|---|---|---|
| D31 | Observations stay one per side. The side's skirmish view carries a `players` list with every own-side slot's economy (credits, unit count, cap, pending, ready). The HUD shows only yours (D16). Amends L5's "per-viewing-slot view". | **agent**: drafts A and C agreed. Allies already share vision, the team planner reads the side frame (D6), and a second observation per slot would duplicate the side's one. |
| D32 | A slot's string `id` is its identity for authoring and URLs. `PlayerIndex`, its position in the player list, is always derived and never stored. The digest, RNG streams and the binary publication use the index. | **agent** (draft C): the publication is numeric (`publication.rs:339`). |
| D33 | `ScenarioDefinition.players` is the single player list. `PlayerSlot` moves to a `contract::players` module with `faction` required. The bundle's separate `slots` list (`bundle.rs:89`) is deleted, and `UnitSetup.owner` names a player id. Refines D29. | **agent** (draft C): one list, so a bundle can't carry it twice. |
| D34 | A setup may have zero humans; the AI-vs-AI benchmark and tournament need that. "Exactly one Human, on Blue" is enforced only by the lobby model and the single-player link. A battle with no human is viewed through Blue's observation. | **agent**: drafts A and C, D28. |
| D35 | Admission refuses more than `max_players_per_side` (fixture data, 10). It's a loose work bound, not a target. The lobby's 5 per side is lobby data. Amends D21. | **agent** (draft B): unbounded slot counts mean unbounded planner, publication and dispatch work. |
| D36 | Mapgen lays the left and right entry arms in a dedicated pass with its own named stream (`"entries"`), not by promoting the random corridors. Centre, left and right are told apart by position along the edge. A 1-entry request keeps byte-identical map hashes. Supersedes the "entry roads" fact below. | **agent** (draft C): today's "first stroke at `y == edge`" (`mapgen/src/skirmish.rs:35`) can already match a corridor. A separate stream keeps 1-entry maps identical. |
| D37 | A purchase's entry is chosen once, at reserve time, and stored on the pending purchase. Preview, dispatch and the AI read that stored choice. | **agent**: drafts A and C agreed; it removes three re-derivations. |
| D38 | One setup constructor, `sim::skirmish::setup_for(players, sites)`, replaces the three hand-written ones: `game-wasm/src/lib.rs:991`, `skirmish_tournament/inputs.rs:126` and the builder's `builderEconomy`. Synthetic entry layouts (`inputs.rs:145`, builder `model.ts:80`) also get one owner. | **agent** (draft C): each would otherwise grow its own multi-entry copy. |
| D39 | The lobby's map picture is drawn in Rust from the admitted `MapDefinition` plus sites. `mapgen::inspect` is extended, since today it only reads a `MapPlan` (`inspect.rs:137`). The picture is exposed through wasm, and the preview keeps that SVG with the request, map and sites. Amends D14. | **agent** (draft C found D14 kept too little to draw). One Rust owner is also reusable by a future native lobby (D27). |
| D40 | The team planner keeps non-digested memory: incumbency, dwell timers, push start, locked detachments. This is the same status as `BalancedDoctrine`'s own state. Plans are rebuilt from the observation plus that memory, and the command stream remains the replay authority. Amends D6. | **agent** (draft C, research): sticky assignment and dwell need memory. |
| D41 | Digest policy. Slice 01 must leave every digest unchanged; it proves the shape change alone. Skirmish digests are then re-baselined only at named slices: 02 (ownership), 12 (team AI lanes and cohesion), and 13 (stance), named again if 1v1 changes. Non-skirmish digests never change. Amends D11's "exactly twice". | **agent** (draft C): stance changes 1v1 too, and you chose one general system (preference 4). |
| D42 | Any old-shape battle link is refused with the existing refusal screen, which names the problem: an unknown `faction`/`enemy` parameter, or missing slots. An old menu link opens the lobby defaults. Rewords D19. | **agent** (draft C): an old link without `enemy` has no unknown parameter; it is missing slots instead. |
| D43 | Settlement keeps a running bonus total and remainder per (victim side, killer slot), and splits a capped pool among killers in slot order. | **agent**: drafts A and C. Today's exact-sum argument relies on a single opposing wallet (`settlement.rs:25`). |
| D44 | A unit's `owner` (its `PlayerIndex`) is hashed in the unit digest. | **agent** (draft C): `Unit::digest` skips it today (`units.rs:643`), yet the cap and controllers now depend on it. |
| D45 | One function picks the controller policy. In a skirmish, every active-AI slot runs `BalancedDoctrine`. `controller_turn` runs only for non-skirmish bundle slots. | **agent**: drafts A and C. Recording controllers at tick 0 (L2) would otherwise drive AI slots twice. |
| D46 | The tournament runs every slot as an in-battle AI. A slot can take a scripted opening buy list as data. Each battle exposes a non-digested `ai_receipts()` log of AI order acceptances and refusals. The external doctrine pair (`runner.rs:82–100`) is deleted. Answers the "tournament" fact below. | **agent** (drafts A and C): those two features were the only things the external host provided. |
| D47 | One web selector, `controlled(observation, slot)`, owns "which units can I order". Every selection and command path uses it (`useUnitControl.ts:80, 127, 207, 283`, `pointerIntent.ts:46`). | **agent** (draft C): otherwise five sites send ally units and get a silent `WrongSlot`. |
| D48 | Team planner shape, from the research: sticky lane assignment brute-forced in slot order with an incumbency bonus; a staging point per lane; a per-side influence grid from the observation; help raised on a trend and answered only by the nearest ally with surplus; detachments locked while engaged; immediate fall-back with dwell-gated pushes. | **agent**, [research.md](research.md). |
| D49 | Slice 00 is a throwaway probe that gates the plan. It measures multi-entry admission rates, prepare timings and opening throughput before any contract work. | **agent** (draft B): if the map fairness assumption is wrong, the plan should die cheaply. |
| D50 | The number of objectives stays driven by map size (`mapgen/src/skirmish.rs:65`). The size floor (D18) is what gives bigger matches more objectives. | **agent** (draft B noticed it); no new rule is needed. |

## Landmines from the sweep

The full cards are on the [unknowns map](visualizations/unknowns-map.html). All the fixes are agent decisions on your behalf, except L8, which became D29.

| # | Landmine | Decision | Slice |
|---|---|---|---|
| L1 | Six sites use "the side's first entry" (`battle.rs:692, 744, 946, 2430, 3529, 3963`); the tournament indexes `entries[side.index()]`. They still compile once entries are a list, but are silently wrong, and retirement can strand a unit. | One entry owner (`sim::entries`) for all of them. | 05 |
| L2 | `set_controllers` drops a controller for a slot with no units (`battle.rs:1480`); skirmish units spawn unowned (`:874`). | Spawn owned by the buyer; install controllers from the player list at tick 0. | 02 |
| L3 | AI orders send `slot: None` and discard errors (`battle.rs:2443`). | Orders carry their slot. A refusal fails a debug assertion and is logged in `ai_receipts()`. | 02 |
| L4 | Ready, purchase and cancel skip slot checks (`battle.rs:1628`). Purchase IDs are per side (`skirmish.rs:126`). RNG streams ignore the slot (`battle.rs:752, 1145`). | Slot-check them, scope purchase IDs per slot, and mix `PlayerIndex` into both streams. | 02 |
| L5 | The economy view and `OwnUnit` carry no owner (`publication.rs:201–214, 339`). | D31; `owner` on own units; re-bless the codec. | 03 |
| L6 | The match report hard-codes `["blue","red"]` (`MatchResultOverlay.tsx:97, 131`). | Per-player totals grouped by side; VICTORY judged against the viewer's side. | 11 |
| L7 | Replays carry no setup. | D30. | n/a |
| L8 | Two owners of "slot". | D29, D33. | 01 |
| L9 | One dispatch clock per side (`battle.rs:820–857`). | A clock per entry, with slots taking turns. A slot's purchase for one entry never waits behind its purchase for another. | 05 |
| L10 | Preparation assumes "player, then enemy" (`preparation.rs:20`, `game-wasm/src/lib.rs:993`, `prepare.ts:182`). | D38; the request carries players; the camera starts on the human's lane. | 06 |
| L11 | "Slot" already means six things. | Rename `occupied_slots` to `unit_count`. | 01 |
| L12 | The earlier arbitrary-sides spike died on 172 fixed two-side owners. | Keep Blue/Red/Neutral; slots live inside sides. | n/a |
| L13 | Fixed limits degrade at 300 units: 24 muzzle lights, 1024 pending sounds. | Judge by eye and ear in the 5v5 browser run. | 14 |

## Facts slice 00 measures

These are facts, not decisions. Slice 00 records each one here.

- **Fairness admission.** Rate for 1v1, 2v3, 3v3 and 5v5 on the corpus (`fixtures/skirmish-corpus.json`), with each side's best travel time over its entries held within 15%.
- **Entry arms.** Whether the dedicated arm pass (D36) yields legal, spaced, river-safe left and right entries.
- **Opening throughput.** Dispatch only runs once the match is Active (`battle.rs:735`). Drafts estimated about 15 s per entry for a 5v5 opening, against about 9 s today. Measure it.
- **Prepare timings.** Skirmish-profile prepare time per size (`report.timings`). The preview's responsiveness depends on them.
