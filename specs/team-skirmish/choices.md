# Team skirmish choices ledger

These decisions came from the explore-unknowns walk on 2026-10-16. Each row records who closed it:
- **user**: the user decided it.
- **user (earlier)**: the user said it in an earlier session, recorded in the T3 database.
- **territory**: the code settled it.
- **agent**: answered on the user's behalf under the confirmed preferences below.

## Confirmed preferences and delegation scope

The user opted in to agent answers for the rest of the walk, on these preferences:

1. Every player is a full player: its own wallet, cap, army, logistics. It is never just a share of the side.
2. What a side shares follows from where things physically are, not from ownership rules.
3. The AI acts like a competent war-film army: concentrated combined-arms groups, with extra players taking more objectives at once.
4. One general system from 1v1 to 10v10. A 1v1 is not a special case, and nothing has to stay the same as it is today.
5. Reuse existing mechanisms rather than add new machinery.
6. Lobby rules stay simple and predictable.

The delegation covered the planning walk only. It does not authorise implementation choices beyond this ledger.

## Decisions

| # | Decision | Source and reason |
|---|---|---|
| D1 | Each AI player runs its own `BalancedDoctrine` with its own wallet, cap, units and opening. `ai_side` is deleted. The per-player `controller_turn` heuristic (`battle.rs:1521`) stays only for authored-bundle players that are not in a skirmish. | **user** |
| D2 | Each AI fields a complete army of its own: armour, infantry, recon, AA and supply trucks. It never skips a role because an ally covers it. | **user**: each AI manages its own logistics |
| D3 | A per-side team planner gives each AI player a lane and a main objective in that lane. An objective the human's units are visibly heading for (`OwnUnit.goal`) counts as covered. | **user** (chose C, then "do all") |
| D4 | The team planner routes requests for help. A player losing its objective asks, and the nearest ally with spare units responds. | **user** |
| D5 | The team planner sets a shared stance (push, hold, fall back) and times pushes together. | **user** |
| D6 | *(Amended by D40.)* The team plan is rebuilt from the side's observation in player order and is never digested. The command stream replays. | **territory**: invariant from `specs/done/skirmish-ai/README.md` |
| D7 | A supply truck resupplies every friendly unit within reach, allies included (current `supply.rs` rule). Each AI buys and keeps its own trucks with its army. | **user** |
| D8′ | *(Amended by D52.)* A side's entries are shared, with no player assignment. Entry count follows player count: 1 player → 1 entry, 2 → 2, 3 or more → 3 (centre, left, right). Ground units and helicopters arrive at the side's entry with the shortest drive to where they were ordered; ties go to the centre. | **user** (corrected the original group-assignment plan) |
| D9 | Each AI keeps the bulk of its army as one cohesive combined-arms group in its lane. Only scouts, outposts and defensive positions scatter. | **user**: spread units get picked apart |
| D10′ | More players means more objectives taken at once. The team planner hands out lanes; an AI fights its lane by ordering purchases there, and D8′ brings them in at that lane's entry. | **user** |
| D11 | *(Amended by D41.)* D9 applies at every match size. Skirmish digests and fixtures are re-baselined exactly twice, each a named decision: once when the setup changes shape, once when AI grouping changes (D41 names the slices). Non-skirmish digests stay identical throughout. | **user** |
| D12 | Every player earns 200 credits/min. A side with more players earns more; there is no balancing between sides. | **user**: "multiple income of course" |
| D13 | The bounty pool stays per side. Kill rewards, the bonus and refunds go to the wallet of the owning player. Death provenance records the killer's player. | **user (earlier)**, "team bounty", 2026-10-06; **territory** `settlement.rs` |
| D14 | *(Amended by D39.)* Each preview runs a full admitted prepare through `admitBattle` and keeps only the request, map and final sites, then closes its worker. Deploy re-prepares that exact request. A newer preview cancels an older one. | **user** |
| D15 | Jets cross the back edge at the point nearest the side's centre entry; how they leave is unchanged. | **user** |
| D16 | Your own units keep Blue. All allies share one allied colour. Selecting an ally opens a read-only panel showing its owner (e.g. "Blue 2 · AI") with no order controls. Box-select and double-click pick only your own units. Credits and cap shown are yours. The match timeline names each player. | **user**; the per-player timeline is **user (earlier)**, 2026-10-11 |
| D17 | The size picker reads S · M · L · XL, for the internal `small`/`medium`/`large`/`xl`. The "XS" label from `cba1ffdc9` goes. | **user** |
| D18 | Size ratchets. Adding a player raises it to the minimum for the larger side (1 → S, 2 → M, 3–4 → L, 5 or more → XL). Removing a player never lowers it. Sizes below the minimum show disabled with a reason. The URL carries the size actually used. | **user**; Medium for 2v2 from the original spec |
| D19 | *(Amended by D42.)* An old battle link shows the existing refusal naming the unknown parameter. An old menu link opens the lobby with defaults. No migration code. | **user** |
| D20 | Ready is per player. The match starts when every player is ready or the timer runs out; each AI marks itself ready after buying its opening. | **agent**: preference 1 |
| D21 | *(Amended by D35.)* The setup and simulation accept any number of players per side. The lobby's maximum of 5 per side is data. | **agent**: preference 4 ("10v10") and 5 |
| D22 | Lobby layout follows the mock's Direction 3, "Order of battle": player banners with a big index numeral on each side, the map between them under an "N VS M" header, and a bottom strip of presets, map type, size, region, seed status, Reroll and Deploy. | **user** (steal: overall layout; Direction 2 skipped) |
| D23 | The preview map is turned so Blue's edge faces the Blue column. | **user** |
| D24 | The allied colour is pale ice `#b9f0ff`, added to `presentation.hud` for allies only. Allies also get a shape cue (for example a hollow selection mark) so they don't depend on lightness alone. | **user** picked the hue; the shape cue is **agent**, from the game-ui "form before colour" rule |
| D25 | A player row shows its index numeral, its controller (YOU/AI) and a delete ×. | **user** |
| D26 | The lobby model is plain serializable state. Its controller set can later gain **Open**. Moving the human is one "seat a human in player X" operation. In battle, a departing human hands the player to AI through the existing controller transitions. Online play itself is not built. | **user**: online multiplayer will reuse this model |
| D27 | Desktop browser only. [Native gaps](../native-gaps/README.md) tracks what native needs. The shared presentation crate is updated here, never left two-faction. | **user** |
| D28 | The finish line is 5v5 AI versus AI: tournament matches for behaviour, and a browser benchmark battle for frame time with up to 300 units, its limit being a loose tripwire. | **user** (5v5 AI vs AI); the two-run split is **agent**, preference 5 |
| D29 | *(Amended by D33.)* One player type in `contract`, used by skirmish setups and authored bundles alike. An Economy bundle derives its skirmish players from its bundle players. The scenario builder and the lobby share the player model. This overrides the original spec's "do not reuse `PlayerSlot`". | **user** |
| D30 | Replay compatibility needs nothing: any sim change already invalidates saved replays through the engine build id (`crates/sim/build.rs:28`). | **territory** |

## Synthesis decisions (write-spec, 2026-10-16)

Three blind drafts with different biases fed the canonical plan:
- fewest slices (Claude);
- risk first (Codex, GPT 6.1 Sol);
- seam quality (Claude).

A research pass ([research.md](research.md)) also fed the team AI slices. The rows below are the calls made in merging them. They are **agent** decisions: either the drafts agreed independently, or one draft found something in the code that the walk had missed. Each names what it amends.

| # | Decision | Source and reason |
|---|---|---|
| D31 | Observations stay one per side. The side's skirmish view carries a `players` list with every own-side player's economy (credits, unit count, cap, pending, ready). The HUD shows only yours (D16). Amends L5's "per-viewing-player view". | **agent**: drafts A and C agreed. Allies already share vision, the team planner reads the side frame (D6), and a second observation per player would duplicate the side's one. |
| D32 | A player's string `id` is its identity for authoring and URLs. `PlayerIndex`, its position in the player list, is always derived and never stored. The digest, RNG streams and the binary publication use the index. | **agent** (draft C): the publication is numeric (`publication.rs:339`). |
| D33 | *(Amended by D54.)* `ScenarioDefinition.players` is the single player list. `PlayerSlot` becomes `Player` in a `contract::players` module with `faction` required. The bundle's separate `slots` list (`bundle.rs:89`) is deleted, and `UnitSetup.owner` names a player id. Refines D29. | **agent** (draft C): one list, so a bundle can't carry it twice. |
| D34 | A setup may have zero humans; the AI-vs-AI benchmark and tournament need that. "Exactly one Human, on Blue" is enforced only by the lobby model and the single-player link. A battle with no human is viewed through Blue's observation. | **agent**: drafts A and C, D28. |
| D35 | Admission refuses more than `max_players_per_side` (fixture data, 10). It's a loose work bound, not a target. The lobby's 5 per side is lobby data. Amends D21. | **agent** (draft B): unbounded player counts mean unbounded planner, publication and dispatch work. |
| D36 | Mapgen lays the left and right entry arms in a dedicated pass with its own named stream (`"entries"`), not by promoting the random corridors. Centre, left and right are told apart by position along the edge. A 1-entry request keeps byte-identical map hashes. Supersedes the "entry roads" fact below. | **agent** (draft C): today's "first stroke at `y == edge`" (`mapgen/src/skirmish.rs:35`) can already match a corridor. A separate stream keeps 1-entry maps identical. |
| D37 | A purchase's entry is chosen once, at reserve time, and stored on the pending purchase. Preview, dispatch and the AI read that stored choice. | **agent**: drafts A and C agreed; it removes three re-derivations. |
| D38 | One setup constructor, `sim::skirmish::setup_for(players, sites)`, replaces the three hand-written ones: `game-wasm/src/lib.rs:991`, `skirmish_tournament/inputs.rs:126` and the builder's `builderEconomy`. Synthetic entry layouts (`inputs.rs:145`, builder `model.ts:80`) also get one owner. | **agent** (draft C): each would otherwise grow its own multi-entry copy. |
| D39 | The lobby's map picture is drawn in Rust from the admitted `MapDefinition` plus sites. `mapgen::inspect` is extended, since today it only reads a `MapPlan` (`inspect.rs:137`). The picture is exposed through wasm, and the preview keeps that SVG with the request, map and sites. Amends D14. | **agent** (draft C found D14 kept too little to draw). One Rust owner is also reusable by a future native lobby (D27). |
| D40 | The team planner keeps non-digested memory: incumbency, dwell timers, push start, locked detachments. This is the same status as `BalancedDoctrine`'s own state. Plans are rebuilt from the observation plus that memory, and the command stream remains the replay authority. Amends D6. | **agent** (draft C, research): sticky assignment and dwell need memory. |
| D41 | Digest policy. Slice 01 must leave every digest unchanged; it proves the shape change alone. Skirmish digests are then re-baselined only at named slices: 02 (ownership), 12 (team AI lanes and cohesion), and 13 (stance), named again if 1v1 changes. Non-skirmish digests never change. Amends D11's "exactly twice". | **agent** (draft C): stance changes 1v1 too, and you chose one general system (preference 4). |
| D42 | Any old-shape battle link is refused with the existing refusal screen, which names the problem: an unknown `faction`/`enemy` parameter, or missing players. An old menu link opens the lobby defaults. Rewords D19. | **agent** (draft C): an old link without `enemy` has no unknown parameter; it is missing players instead. |
| D43 | Settlement keeps a running bonus total and remainder per (victim side, killer player), and splits a capped pool among killers in player order. | **agent**: drafts A and C. Today's exact-sum argument relies on a single opposing wallet (`settlement.rs:25`). |
| D44 | A unit's `owner` (its `PlayerIndex`) is hashed in the unit digest. | **agent** (draft C): `Unit::digest` skips it today (`units.rs:643`), yet the cap and controllers now depend on it. |
| D45 | One function picks the controller policy. In a skirmish, every active-AI player runs `BalancedDoctrine`. `controller_turn` runs only for non-skirmish bundle players. | **agent**: drafts A and C. Recording controllers at tick 0 (L2) would otherwise drive AI players twice. |
| D46 | The tournament runs every player as an in-battle AI. A player can take a scripted opening buy list as data. Each battle exposes a non-digested `ai_receipts()` log of AI order acceptances and refusals. The external doctrine pair (`runner.rs:82–100`) is deleted. Answers the "tournament" fact below. | **agent** (drafts A and C): those two features were the only things the external host provided. |
| D47 | One web selector, `controlled(observation, player)`, owns "which units can I order". Every selection and command path uses it (`useUnitControl.ts:80, 127, 207, 283`, `pointerIntent.ts:46`). | **agent** (draft C): otherwise five sites send ally units and get a silent `WrongSlot`. |
| D48 | Team planner shape, from the research: sticky lane assignment brute-forced in player order with an incumbency bonus; a staging point per lane; a per-side influence grid from the observation; help raised on a trend and answered only by the nearest ally with surplus; detachments locked while engaged; immediate fall-back with dwell-gated pushes. | **agent**, [research.md](research.md). |
| D49 | Slice 00 is a throwaway probe that gates the plan. It measures multi-entry admission rates, prepare timings and opening throughput before any contract work. | **agent** (draft B): if the map fairness assumption is wrong, the plan should die cheaply. |
| D50 | The number of objectives stays driven by map size (`mapgen/src/skirmish.rs:65`). The size floor (D18) is what gives bigger matches more objectives. | **agent** (draft B noticed it); no new rule is needed. |
| D51 | One word: **player**. The code uses `Player`, `players`, `CommandEnvelope.player` and `AuthoredScript.player`; `occupied_slots` becomes `unit_count`; the spec folder is `specs/team-skirmish/`. `UnitSetup.owner` stays, because it names the owning player. | **user**: "pick one and stick to it". "Slot" already had six meanings (L11). |
| D52 | Both sides get the same number of entries: `entry_count` of the larger side's player count. A 2v3 map has 3 entries per side. Amends D8′ ("count follows each side's own players"). | **agent**, from the slice 00 probe ([assets/probe-report.md](assets/probe-report.md)). Symmetric pairings were admitted at the 1v1 rate (3v3 and 5v5 both 2/7, like 1v1). The uneven 2v3 was admitted 0/7, and every refusal was a fairness failure: the side with more entries always has a faster best route. Equal entries keep every map mirror-fair, and a side with fewer players simply has a spare road. Reversible if you prefer per-side counts with a different fairness rule. |
| D53 | The probe's arm pass failed 150 m spacing and river-mouth avoidance on L and XL. Slice 04 treats both as hard admission tests, not tuning. An XL 5v5 takes about 4.2 s to generate and prepare, so the preview (slice 08) must show a loading state from the first frame. | **territory**: probe evidence. The spike code is frozen on branch `spike/team-skirmish-probe` at `f9d14d2e5`. |
| D54 | `Player.faction` is optional in the type. Validation requires a faction for every Blue or Red player and refuses one on a Neutral player. Amends D33's "faction required". | **agent**: the Codex review of slice 01 found the builder's default Neutral player has no meaningful faction. Neutral units are world props and civilians, not a faction's army, so they get no faction rather than an invented one. |

## Landmines from the sweep

The full cards are on the [unknowns map](visualizations/unknowns-map.html). All the fixes are agent decisions on your behalf, except L8, which became D29.

| # | Landmine | Decision | Slice |
|---|---|---|---|
| L1 | Six sites use "the side's first entry" (`battle.rs:692, 744, 946, 2430, 3529, 3963`); the tournament indexes `entries[side.index()]`. They still compile once entries are a list, but are silently wrong, and retirement can strand a unit. | One entry owner (`sim::entries`) for all of them. | 05 |
| L2 | `set_controllers` drops a controller for a player with no units (`battle.rs:1480`); skirmish units spawn unowned (`:874`). | Spawn owned by the buyer; install controllers from the player list at tick 0. | 02 |
| L3 | AI orders send `slot: None` (today's field name) and discard errors (`battle.rs:2443`). | Orders carry their player. A refusal fails a debug assertion and is logged in `ai_receipts()`. | 02 |
| L4 | Ready, purchase and cancel skip player checks (`battle.rs:1628`). Purchase IDs are per side (`skirmish.rs:126`). RNG streams ignore the player (`battle.rs:752, 1145`). | Player-check them, scope purchase IDs per player, and mix `PlayerIndex` into both streams. | 02 |
| L5 | The economy view and `OwnUnit` carry no owner (`publication.rs:201–214, 339`). | D31; `owner` on own units; re-bless the codec. | 03 |
| L6 | The match report hard-codes `["blue","red"]` (`MatchResultOverlay.tsx:97, 131`). | Per-player totals grouped by side; VICTORY judged against the viewer's side. | 11 |
| L7 | Replays carry no setup. | D30. | n/a |
| L8 | Two owners of the player concept (bundle `PlayerSlot` vs skirmish `factions`/`ai_side`). | D29, D33. | 01 |
| L9 | One dispatch clock per side (`battle.rs:820–857`). | A clock per entry, with players taking turns. A player's purchase for one entry never waits behind its purchase for another. | 05 |
| L10 | Preparation assumes "player, then enemy" (`preparation.rs:20`, `game-wasm/src/lib.rs:993`, `prepare.ts:182`). | D38; the request carries players; the camera starts on the human's lane. | 06 |
| L11 | "Slot" already means six things in the code (unit count, squad member, garrison seat, script, bundle seat, a local). | Rename `occupied_slots` to `unit_count`. | 01 |
| L12 | The earlier arbitrary-sides spike died on 172 fixed two-side owners. | Keep Blue/Red/Neutral; players live inside sides. | n/a |
| L13 | Fixed limits degrade at 300 units: 24 muzzle lights, 1024 pending sounds. | Judge by eye and ear in the 5v5 browser run. | 14 |

## Facts slice 00 measures

These are facts, not decisions. Slice 00 records each one here.

- **Fairness admission.** Rate for 1v1, 2v3, 3v3 and 5v5 on the corpus (`fixtures/skirmish-corpus.json`), with each side's best travel time over its entries held within 15%.
- **Entry arms.** Whether the dedicated arm pass (D36) yields legal, spaced, river-safe left and right entries.
- **Opening throughput.** Dispatch only runs once the match is Active (`battle.rs:735`). Drafts estimated about 15 s per entry for a 5v5 opening, against about 9 s today. Measure it.
- **Prepare timings.** Skirmish-profile prepare time per size (`report.timings`). The preview's responsiveness depends on them.

### Slice 00 results (2026-10-17)

The probe ran over all 7 corpus rows, seeds 1–7 ([full report](assets/probe-report.md)):

| Pairing | Admitted | Generate and prepare | Opening field time (estimate) |
|---|---|---|---|
| 1v1 | 2/7 | S: 627 ms | 9 s |
| 2v3 | 0/7 | L: 710 ms | 9 s |
| 3v3 | 2/7 | L: 890 ms | 9 s |
| 5v5 | 2/7 | XL: 4.2 s | 15 s |

These answer the four facts above:
- **Fairness admission:** fine with equal entry counts; fails with unequal ones, which led to D52.
- **Entry arms:** the rough pass failed spacing and river checks on larger maps, so slice 04 makes both hard tests (D53).
- **Opening throughput:** the 15 s estimate is well inside the preparation timer.
- **Timings:** acceptable, as long as the preview shows a loading state (D53).

The sample is small: 7 rows, 4 of them saved maps with synthetic objectives. Slice 04 re-measures admission on its real arm pass.

### Slice 04 results (2026-10-17)

The real arm pass uses the shared entry count on both sides. On the seven-row corpus, the measured admissions were:

| Pairing | Entries per side | Admitted | Slice 00 |
|---|---:|---:|---:|
| 1v1 | 1 | 3/7 | 2/7 |
| 2v3 | 3 | 1/7 | 0/7 (unequal counts) |
| 3v3 | 3 | 1/7 | 2/7 |
| 5v5 | 3 | 1/7 | 2/7 |

The two refused three-entry generations were named generation refusals before admission; the remaining maps were measured with every entry standing and routing to every objective. These rates are evidence for the current arm and objective candidates, not a new fairness threshold. The request's one-entry form keeps the existing map bytes and hashes; only the generator identity version changes.

## Implementation choices

Decisions the implementation made where the spec was silent, with a verdict on each.

| Slice | Choice | Verdict |
|---|---|---|
| 01 | The publication wire field is `unitCount` and the JSON contract field is `unit_count`, following each layer's existing casing. | Sound: it matches the codec's existing camelCase. |
| 01 | A `human` player must also be `human_capable`; validation refuses one that isn't. | Sound: it keeps the bundle's earlier rule and lets online seats (D26) mark a player human-capable before anyone takes the seat. |
| 01 | Players from today's links are named `blue-1` (Human) and `red-1` (active AI). Parity fixtures use `red-1` as a passive AI, which matches their earlier behaviour of having no AI opponent. | Sound. Slice 06 replaces the link grammar. |
| 01 | Wasm preparation checks the scenario's player contract before constructing the battle, and returns a normal refusal instead of panicking. | Sound. It is the one admission point, and it fixed a panic found in review. |
| 01 | The bundle format version is now `scenario-bundle-2`. | Sound: hard cutover, no migration. |
| 07 | The lobby state is `{ players, map, battleSeed }`, and players stay sorted by side, then by id number. | Sound: one serializable shape, in deterministic order. |
| 07 | Until slice 06, `toUrl` and `fromUrl` carry the players as JSON, and `toRequest` fills placeholder generator fields. Each is marked for slice 06. | Provisional. Slice 06 replaces all three with the link grammar and `setup_for`. |
| 07 | The lobby's limit of 5 players per side is one named constant, `MAX_PLAYERS_PER_SIDE`, in the lobby model. | Sound: one owner. It is lobby data, separate from admission's `max_players_per_side` (D35). |
| 02 | Player state is a `Vec<PlayerState>` in scenario player order, and `PlayerLens` is a plain struct over the side frame. | Sound. |
| 02 | Scripted openings live on the match setup, as `SkirmishSetup.openings` keyed by player id, not on the shared `Player` seat type. | Sound: corrected in review. Seats stay identity only. |
| 02 | Report totals are kept per player; side totals come from one helper, `side_totals()`. | Sound: corrected in review, so there is one owner. |
| 02 | The kill bonus split uses floor shares, then hands out the leftover credits by largest fractional remainder, ties to the lower player index. The cap is applied before the remainder is carried. With one player per side it pays exactly what the side-only settlement paid. | Sound: corrected in review. A reference test pins it against the old algorithm. |
| 02 | An unowned killer in a skirmish fails a debug assertion and earns nothing, rather than being credited to the side's first player. | Sound: every skirmish unit has an owner, so a fallback would hide a bug. |
| 02 | Until slice 03, the side's skirmish view shows the human player's economy if the side has one, otherwise the first player's. | Provisional. Slice 03's player table replaces it. |
| 02 | The browser sends orders as the viewing side's Human, or the side's first player when it has none (`apps/battle-lab/src/commandingPlayer.ts`, used by the battle session for every route). | Sound: found in my own verification. After slice 02, every generated-battle order was refused with `MissingPlayer`, because no route passed a player. Slice 10's `controlled(observation, player)` builds on the same player. |
| 04 | Entry sites are ordered centre, then left-to-right flanks per side; until slice 05, existing simulation consumers use the centre site through `centre_entry`. | Sound: it preserves slice-02 behavior while making the list order explicit for the next owner/dispatch slice. |
| 04 | Left and right arms sit at mirrored edge fractions on both sides, drawn once from the `"entries"` stream. An arm pair is accepted only if both sides pass spacing, river-mouth and join checks. | Sound. It removes unfairness the arms would otherwise create; added after the first measurement came back low. |
| 04 | The size floor and the skirmish extents have one owner, `fixtures/skirmish-sizes.json`, read by `contract` and imported by the lobby model. There is one admission entry point, `admit_skirmish(q, rules, players)`. | Sound: corrected in review, from three copies of the size table and two entry points. |
| 04 | The generator version is bumped to `layout-19`. The map-layout parity records change their version string, and the two compiled-map records change their output hash because the output embeds that version. 1-entry geometry is unchanged, which tests pin on an explicit request and on the corpus's generated rows. | Sound: the spec required the bump. |
| 04 | Until slice 05, simulation consumers read the centre entry through `SkirmishSites::centre_entry`. | Provisional. Slice 05's entry owner replaces it. |
| 08 | The picture is `mapgen::inspect::skirmish_picture(map, sites)`, a sibling module (`inspect/lobby.rs`) of the plan inspector. It reads only the compiled `MapDefinition` and admitted `SkirmishSites`; wasm exposes it as `skirmish_picture(map_json, sites_json) -> svg`. | Sound: one owner, and it draws what the battle is fought on, not the plan. |
| 08 | Terrain colours are the picture's own constants, taken from Direction 3. Side accents and mark text are the HUD's `--hud-accent`, `--hud-enemy` and `--hud-text`, read as CSS custom properties, so the hosting page supplies them. | Sound: the HUD theme stays the one owner of player colours. A page without the theme draws those marks unfilled. |
| 08 | Simplification by size: weights scale with the map's longer side, so marks keep their screen size; points under 1/800 of the side are dropped; tree lines, copses and lone trees are left out; buildings are one footprint path with a thin edge; streets are dim, country roads cased and above buildings; there is no relief. A Large 3v4 picture is about 100 KB and takes about 1 ms natively. | Delegated (slice 08). |
| 08 | Objectives are numbered rings, in `sites.objectives` order (the score strip's numbering), at least a minimum mark size, over a dark disc. They are uniform, with no dashed or primary ring. Entries are side-accent arrowheads set straight in from their nearest frame edge, without N/C/S labels, because no entry belongs to a player (D52). | Delegated. Labels can come back when slice 12's lanes give entries names. |
| 08 | The preview asks the worker for its picture with `PrepareMessage.preview`. The worker draws the SVG next to the admitted map, and `PreparedBattle.preview` carries `{ map, sites, svg }` as JSON text. Battles never set the flag. | Sound: the drawing stays in the worker (D39), and Play pays nothing for it. |
| 08 | `mapPreviews` (`web/src/battle/prepare/preview.ts`) has the states idle, loading, retrying (with the attempt number), ready and failed. It counts retries from `admitBattle`'s `onRequest`, and on ready closes the winning worker with `Admission.cancel()`. | Sound. |
| 08 | The lab route `/labs/map-picture` runs real previews with seeds counted from 1, two at a time, at a 576 px frame. A pairing below its size floor shows the real refusal. The lab sets its own deadline (90 s) and keeps the game's attempt count. | Provisional. In this browser an XL preparation took about 16 s, against the game's 8 s `generated_deadline_ms`. Every XL preview, and probably XL Play, would time out here. Slice 09 needs a decision on the deadline. |
| 08 | The TypeScript `GenerationRequest` mirror gains an optional `entries_per_side`, and the lobby model exports `entriesPerSide(players)`. | Provisional until slice 06's request seam. |
| 03 | The publication carries a `players` group (index, id, controller, ready, credits, unit count, cap), and pending purchases and own units carry their owner. The old header fields `readyBlue`, `readyRed`, `credits`, `unitCount` and `maxUnits` are gone, along with every fallback that read them. `SkirmishView.players` is required in TypeScript. | Sound. |
| 03 | Adding the players group first shifted the `shouts` group's end boundary, so the decoder read payloads with the wrong counts. The writer was fixed. A presentation-crate test now decodes a 2v2 frame without a browser, and the capture fixtures were re-recorded from a second browser run taken after the fix. | Sound: found in my own verification of the full web suite. |
| 03 | The market-town capture window is rebuilt by a kept `BLESS_PARITY`-gated test, `publication_reader::market_town_window_matches_a_fresh_stream_after_its_preamble`, which needs `PARITY_RUN=<browser run dir>`. It replays the deterministic menu battle to tick 1825 and publishes ticks 1825–1831 from a fresh publisher. | Sound: the next layout change re-runs this instead of building the window by hand. |
| 05 | `sim::entries` owns every entry choice: `select`, `centre`, `site` and `retire_at`. `EntryIndex` is a transparent `usize`. Route selection receives an entry-site callback. | Sound. |
| 05 | Arrival entry is chosen once at reserve time and stored on the pending purchase. A unit turning back (refund, fuel return) goes to the entry nearest it at that moment, and retires on reaching any of its side's entries. Helicopters use the chosen entry; jets keep the centre entry and back edge. | Sound: corrected in review. Codex first kept each unit's arrival entry as its way home. |
| 05 | Each entry has its own dispatch clock, and players take turns at an entry; one blocked unit doesn't stall another entry or another player. | Sound. |
| 08 | Review checkpoint: shots of 3v4 Large, 5v5 XL, 1v1 Small, the Direction 3 reference, loading, and the 3v4-on-Small refusal were opened for the user (non-blocking). With no reply, I accepted them on the evidence: the map reads at lobby size, and entries and objectives sit above the terrain, after five unprimed critique rounds. | Sound. The user can still change colours, the numbered objectives or the loading state. |
| 08 | **Open: the XL preparation deadline.** In the browser an XL preparation took about 16 s, but `generated_deadline_ms` is 8 s for all attempts together. That was measured while two test suites loaded the machine, and natively it was about 4.2 s. The lab uses its own 90 s deadline. | OPEN until slice 09: measure XL prepare on a quiet machine, on this branch and on main, then set the deadline as a loose tripwire, per size if needed. If XL Play already times out on main, fix that in its own commit. |

### Slice 04 admission sweep (2026-10-17; side-band fix rerun)

`cargo run --release -p mapgen --example entry_admission -- --seeds 30`. Generated maps only, at each pairing's floor size, with equal entries (D52). Admitted out of 30:

| Type | 1v1 (S) | 2v2 (M) | 3v3 (L) | 5v5 (XL) |
|---|---|---|---|---|
| open | 29 | 28 | 18 | 19 |
| mixed | 27 | 25 | 19 | 17 |
| metro | 23 | 23 | 19 | 19 |

Rerun note: three-entry arms now choose opposite edge bands around the known centre exit; mirrored fractions, joins, spacing and river-mouth checks remain hard rules.

Before the side-band fix, 3v3 and 5v5 were admitted 20–27 of 30; requiring one flank arm on each side of the centre costs about 3–10 of 30 there. The lowest cell is now 57%, so a preview's 5 hidden attempts fail about 1.5% of the time: accepted, because lanes across the whole front matter more (slice 12). Raising it is a later tuning target. Fairness is still most of the refusals.
