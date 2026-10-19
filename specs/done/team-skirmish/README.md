# Team skirmish

**Status:** shipped 2026-10-19.

Skirmish runs from 1v1 to 5v5 on one general system; a 1v1 is one player per
side, not a special case. A player is one seat on a side with its own wallet,
unit cap, army and logistics, ready flag and controller. Allies share the
side's vision, objectives, score, bounty pool and entry roads. A per-side team
AI gives each AI player a lane, gathers its army and coordinates help and
pushes. The lobby is a desktop order-of-battle screen, Blue's players left and
Red's right around the real generated map that Deploy opens.

[choices.md](choices.md) is the decision ledger: decisions D1–D57, landmines
L1–L13, measured facts, and every call the build made where the plan was
silent. [research.md](research.md) holds the team-AI sources.

## Why it works this way

- **One player type.** Setups and authored bundles share
  `contract::players::Player`; `ScenarioDefinition.players` is the only list.
  Two owners of "player" (bundle seats against the skirmish's faction pair and
  `ai_side`) were the root landmine. The word is *player*; "slot" already
  meant six things.
- **Side or player follows physics.** What is physically shared (sight,
  ground, roads, score) belongs to the side; what one commander spends or
  commands (credits, cap, purchases, ready, controller) to the player. A side
  with more players earns more; nothing balances it.
- **Entries belong to the side.** A unit arrives at the side's entry with the
  shortest drive to where it was ordered. Both sides get the larger side's
  entry count: a side with more entries always has a faster best route, so
  only equal counts stay mirror-fair. A smaller side has a spare road.
- **Entry arms are their own pass with their own random stream,** so a
  one-entry map is byte-identical to before, and centre, left and right are
  told apart by position. Spacing and river mouths are hard admission tests.
- **The lobby shows only an admitted map.** A preview runs the full admitted
  prepare and keeps request, map, sites and picture; Deploy re-prepares that
  request and gets the same battle. The picture is drawn in Rust from the
  compiled map, not the plan, so a native lobby can reuse it.
- **Digests move only by name.** The shape change landed with every digest
  unchanged; skirmish digests were re-baselined at named, committed points,
  and non-skirmish digests never moved. Old replays need no migration: the
  engine build id already invalidates them.
- **The team AI is a commander over squads** (research): sticky lanes in
  player order, a staging point per lane where the main group gathers before
  advancing as one, help raised on a falling trend before the objective
  falls, immediate fall-back and dwell-gated pushes. Help, hold and claim are
  one *party*: units detached from a main group to an objective, one lock, one
  lifecycle. Open objectives are claimed first (easy wins); a taken,
  uncontested main objective is left with a hold and the group moves on.
- **Strength is value, not headcount.** A unit is worth its card cost against
  what its weapons can engage, by target class. Influence, force, surplus,
  party budgets and purchases read the same measure, so a balance change to a
  cost or weapon changes the AI's judgment and buying without code.
- **Desktop browser only.** The shared presentation crate is never left
  two-faction, but native gets no lobby or team UI.

## Invariants

- **One player list.** `PlayerIndex` is a position in it, derived, never
  stored; the string id is the identity for authoring, links and replays.
- **Every skirmish unit has an owner, every command a player.** Buyers own
  what they buy; sequence numbers, rate limits, ready, purchase and cancel are
  per player. A client order for a seat not controlled by a Human is refused
  (`ai_controlled`) before it is recorded.
- **A seat's controller has one record,** installed from the player list at
  tick 0 and changed only by transitions. One function picks the policy; each
  active-AI player runs its own `BalancedDoctrine`. A passive AI's units,
  later arrivals included, only return fire, decided in one place.
- **Every AI order is receipted** in the non-digested `Battle::ai_receipts()`.
  A refused AI order other than a purchase fails a debug assertion; a refused
  purchase stops only that turn's later purchases and is retried.
- **One entry owner.** Every ground or helicopter arrival, refund, fuel return
  and retirement goes through `sim::entries`; jets arrive opposite the centre
  entry. A purchase's entry is chosen once, at reserve time; each entry has
  its own dispatch clock with players taking turns, so a blocked unit holds
  only its own entry.
- **Settlement is order-independent.** Rewards and refunds go to the owning
  player; the bounty pool is the side's, and a capped bonus splits among
  killers in player order.
- **One observation per side.** Its skirmish view carries the side's player
  table, the only economy in the view; the HUD shows your row.
- **The team plan is not authority.** It is rebuilt from the side's
  observation plus non-digested memory (incumbency, dwells, push start, party
  locks) in player order; the command stream replays.
- **Parties:** a unit is in at most one; an engaged party is never re-tasked;
  every party can hold ground (never aircraft alone); a carrier and its
  passengers go together. Claims and holds share a per-player budget; help
  comes from the helper's surplus outside it. A losing lane raises help,
  including one beaten but no longer falling. A score deficit alone never
  causes a retreat.
- **Specialization comes from weapon data,** never a role name: air defence
  holds the backline because its weapons cannot engage the ground. Every AI
  still fields a complete army (D2).
- **One web rule for what you can order:** `controlled(observation, player)`.
  An ally's unit can be selected alone to read, never ordered.
- **Links are exact.** Map choice, battle seed and player list; anything
  else, old shapes included, is refused by name. Single-player has exactly
  one Human, on Blue; zero humans is a valid watched battle.

## Where it lives

- Contract: `crates/contract/src/players.rs` (`Player`, `validate_players`,
  `entries_per_side`, `validate_player_contract`); `skirmish_sizes.rs`, the
  one reader of `fixtures/skirmish-sizes.json`; `skirmish.rs` (`SkirmishView`,
  `PlayerView`, `MatchReport::side_totals`).
- Map: arms in `crates/mapgen/src/skirmish.rs`; the picture
  `mapgen::inspect::skirmish_picture`; fairness
  `sim::map_analysis::admit_skirmish`.
- Simulation: `crates/sim/src/skirmish.rs` (`PlayerState`, `setup_for`),
  `entries.rs`, `battle/dispatch.rs`, `battle/skirmish_turn.rs`, the owner
  index in `battle/state_digest.rs`; `policy_for_player`, `apply_controller`
  and `ai_receipts` in `battle.rs`.
- Team AI: `crates/sim/src/skirmish_ai/team/` (`party.rs`, `help.rs`,
  `stance.rs`, `backline.rs`, `value.rs`'s `unit_value`), purchases in
  `skirmish_ai/purchase.rs`, tuning in `fixtures/skirmish-ai.json`.
- Browser: `apps/battle-lab/src/lobby/` (pure model and `Lobby.tsx`);
  `skirmishSetup.ts`, the one browser setup owner; `battleLinks.ts`;
  `web/src/battle/prepare/preview.ts`; `controlled.ts`; `playerName.ts`;
  `MatchResultOverlay.tsx`; the allied colour `presentation.hud.ally` in
  `fixtures/game.json`; wasm exports in `crates/game-wasm/src/skirmish.rs`.
- Proof: `crates/contract/tests/players.rs`; `crates/sim/tests/skirmish_*.rs`,
  `entries.rs`, `settlement.rs`; `skirmish_ai/team/tests/`; `web/tests/`
  (`lobbyModel`, `lobbyScreen`, `battleLinks`, `mapPreview`, `unitControl`,
  `allyPanel`, `matchResultOverlay`); scenes `lobby`, `map-picture`, `allies`,
  `ally-help`. Behaviour at scale: the tournament,
  `crates/mapgen/examples/skirmish_tournament/` (usage in its opening
  comment).

## Closing evidence

The closing 5v5 run (three uncapped AI-versus-AI matches, mixed XL maps, seeds 1, 2 and 4) finished every match with a winner in 29–39 minutes, with zero refused AI orders. Every push advanced every lane together (36 of 36), help was answered 121 times of 138, and a side held up to six objectives at once. The tournament's `cohesion` field does not measure "fights as one group": it samples units at the staging point after the group first gathers, and stays empty in most matches. Push synchrony is the measure to read.

## Dead ends and divergences

- **Per-side entry counts.** The probe
  ([assets/probe-report.md](assets/probe-report.md)) admitted uneven pairings
  0 of 7, every refusal a fairness failure; equal counts replaced them (D52),
  not a looser threshold.
- **Arms from random corridors** failed spacing and river mouths on L and XL;
  arms became a mirrored dedicated pass, with a flank each side of the centre
  so lanes span the front, at a few admissions per 30.
- **Setups built in the browser.** Lobby, link and builder each built one and
  renumbered players, so Deploy did not reproduce the preview. One
  constructor (`setup_for`) and one browser owner replaced them.
- **A stalling planner.** An AI parked on its first objective; replans on
  contested flickers left a 1v1 holding nothing for 30 minutes. Replans react
  to ownership only, and a ready group keeps advancing.
- **Score-driven fallback** made the side behind retreat to its entry and
  lose; a mean-influence fallback pulled every lane back for one bad one.
- **Headcount strength** let three squads that cannot hurt a tank outweigh it.
  Plain value per credit then bought rifles against helicopters, so the air
  pivot buys weapons made for aircraft.
- **One budget for help, claims and holds** filled with claims as a team took
  ground, and help went unanswered.
- **Greedy ride matching** re-matched carriers every decision until none
  arrived; a carrier coming for a squad stays its ride.
- **The 30-minute tournament cap** equalled the majority hold time, so
  contested matches could not finish; the cap derives from the match rules.
- **Zero-tolerance shove,** found on the way: a vehicle sat forever against a
  fence panel because generated fences overlap by millimetres; bodies within
  the shove clearance now touch without pressing (`ad809fdab`, on main).
- **No armour.** The AI never bought an MBT; the recipe fields one, at a
  measured capture cost (D2 over win rate).

## Out of scope and open

- **Native** has no lobby, ally cues, ally panel, per-player report or
  skirmish replay: [native gaps](../../native-gaps/README.md) item 10.
- **Performance** (user, 2026-10-19): the 300-unit browser benchmark, the
  `benchmark` scene's real-time check and the fixed limits at 300 units (L13)
  belong to a future performance spec.
- **Online play, mixed factions per side, mobile.** The lobby model is plain
  state ready for an Open seat (D26).
- **Open cues:** a stronger ally form cue than pale ice and dashes; a cue for
  an ally's intent, which the `ally-help` shot cannot show; a per-player
  filter for the 5v5 report timeline.
- **Tuning:** the worst XL cell admits about 57% per try (hidden retries fail
  about 1.5%); faction balance is outside this spec.

## Visual provenance

- [`visualizations/lobby-directions.html`](visualizations/lobby-directions.html),
  the lobby mock from the planning walk: three directions over a mock 3v4.
  The user chose Direction 3, "Order of battle" (D22), with Blue's edge facing
  the Blue column (D23), pale ice `#b9f0ff` for allies (D24) and rows of
  numeral, controller and delete (D25). The map picture and lobby screen were
  compared against it; Deploy kept the HUD's accent blue, not the mock's amber.
- [`visualizations/unknowns-map.html`](visualizations/unknowns-map.html): the
  planning walk and its landmine cards.
- Baselines in `web/scenes/baselines/`: `ui/menu-skirmish.png` (the lobby),
  `ui/report-*.png` (the report, from `reportSpecimens.ts`; the old report had
  none, so "before" came from main's component) and `panels/panels-ally.png`
  (the read-only ally panel).
