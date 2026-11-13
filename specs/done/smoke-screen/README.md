# Smoke screen

**Shipped 2026-11-13** on `t3/verify-smoke-screen-ability`. The user's
decisions (D1–D25) were settled on 2026-11-12 in
[the unknowns map](visualizations/map.html) and are recorded in
[`decisions.md`](decisions.md). The [choices ledger](choices.md) holds what the
build decided where the plan was silent, ranked for review.

Nineteen tanks, four high-end IFVs and three infantry rows can **pop smoke**.
The player presses H or POP SMOKE. The order spends one charge and raises a
stationary circular cloud centred on the unit. The cloud hides the way a very
dense forest does:
- an observer outside sees only a few metres in;
- a unit inside is nearly blind;
- the enemy keeps its existing last-contact marker where the unit vanished,
  and fire aimed there keeps landing.

The right play is to pop smoke, then reverse out. The skirmish AI plays it the
same way, and a player's own units never pop on their own.

## The contract

A cloud is a law over time, owned by `contract::smoke`:

```
age        = (tick − popped) / tick_hz           // gone once age ≥ life_s
bloom      : radius grows in bloom_steps equal steps over bloom_s
hold       : full radius, full strength
fade       : over the last fade_s, strength drops in fade_steps steps
             depth_per_m × strength,  concealment = 1 − (1 − concealment) × strength
smoke depth of a sight line = Σ over clouds of (chord inside the circle, where the
             line runs below ground + height_m) × depth_per_m        // overlaps add
sight      : reach = range × concealment × e^(−(foliage + smoke depth)),
             blocked outright at foliage_full_block                   // forest's formula
concealment of a target standing in a cloud = the strongest of forest, shelter, smoke
```

A roster tank's or IFV's cloud has a 15 m radius, stands 6 m tall and lasts
40 s. An infantry cloud is 12 m, 5 m and 30 s. Each blooms within 1 s, fades over its
last 10 s, and adds 0.2 depth per metre, so 5 m of cloud blocks a line fully.
Every number is data on the carrier's row
(`capabilities.smoke.cloud`; see the [fixtures README](../../../fixtures/README.md)).

## Why it works this way

- **Smoke is forest's rule, made dense.** D1 reuses forest's reach formula and
  its "strongest concealment wins". So smoke needs no new sight mechanic, and
  seeing in and seeing out are the same rule from both ends. A unit 2 m inside
  sees out well through the near side and is blind into the cloud. Forest
  already behaves that way; its density is just too low to notice.
- **Exact circles, not the 8 m foliage grid (D5).** On a grid, the drawn cloud
  and the area that blocks sight would disagree by metres. A finer grid would
  be a second store to keep in step.
- **A cloud is not a prop (D19).** A prop is a box, and sides remember props
  forever, so an expired cloud would linger as a ghost. A prop also has no
  birth tick, forces navigation and air replans, and makes helicopters climb
  over it. Clouds have their own collection, and the old transient-prop
  machinery (`PropBody.lifetime_s` and its expiries) is deleted, not kept
  alongside.
- **Smoke is its own depth term (D20).** `foliage_depth` returns 0 without a
  forest, and the launch-path check refuses fire through any foliage. Only the
  sight paths read `sight_depth`. So artillery and SAMs still launch from
  inside smoke, while wire- and laser-guided missiles lose their line.
- **The law moves in steps.** Every reader asks one law for the same step: the
  simulation's sight, the fog sweep, the GPU fog, the drawn cloud and the pop
  sound. A smooth fade would change every tick and rebuild the GPU fog's eye
  maps every tick. Stepping caps those rebuilds at a handful per cloud.
- **A side sees a cloud at its rim (D12).** A cloud hides its own middle. A
  side sees it when its fog field shows any cell whose centre lies within
  one cell of the cloud's circle, and always sees its own. A side is
  published only the clouds it sees, so drawing them over the fog veil
  reveals nothing.
- **Popping is an instant change beside the unit's orders (D21).** Like
  `UpgradeMove`, it never replaces or cancels what the unit is doing. So the AI sends its pop first and its
  reverse second, and the order rate limit can drop the reverse but never the
  pop.
- **The pop is heard only with sight (D16).** Unseen enemies are heard only
  through the simulation's hearing cues. A new cue category would have
  changed the closed publication layout a second time. So a side hears a pop
  when it sees the cloud while it blooms. The sound frame decides that from
  the clouds already published.
- **No wind, no drift (D8).** The simulation has no wind, so a drawn drift
  would make the screen lie. The billow swirls inside its circle instead.
- **The player decides (D3).** Popping smoke when attacking loses you your
  target, so player units never pop on their own. Only the skirmish AI, a
  playbook rather than a rule of the battle, pops for its units.

## Invariants

- **One owner per concept:**

  | Concept | Owner |
  |---|---|
  | A cloud's law over time; smoke depth along a line; concealment at a point | `contract::smoke` (`state`, `piece_depth`, `segment_depth`, `concealment_at`) |
  | The capability's schema and validation | `contract::catalog::SmokeCapability` (`catalog/charges.rs`), beside flares and active protection |
  | Live clouds, and the sight depth of a line | `sim::world::SmokeClouds`, with `WorldGeometry::sight_depth` and `smoke_concealment` (`world/smoke.rs`) |
  | Popping, and which clouds a side sees | `sim::smoke::{pop, seen_by, seen}`, called from `Battle::apply` |
  | Ground concealment | `sensing::ground`: the strongest of forest, shelter and smoke |
  | The fog sweep's smoke and its cache key | `sim::visibility::sweep`, `SmokeClouds::key_near` |
  | Smoke in the observation and publication | `ObservationFrame.smoke_clouds`, `OwnUnit.smoke`; `crates/sim/src/publication/smoke.rs` and `reader/smoke.rs` (`smokeClouds`, `smokeCharges`, `smokeLooks`) |
  | GPU fog smoke | `FogSight::set_smoke` and `crates/presentation/src/fog/smoke.rs`, fed in `presentation/side_fog.rs`; WGSL `fogSmokeStep` in `fog.wgsl` |
  | The drawn cloud | `crates/presentation/src/effects/frame/smoke_screen.rs`, its look a `smoke_screen_looks.<name>` row |
  | The pop sound | `crates/presentation/src/sound/smoke_pops.rs`, the look row's `sound`, the mix's `smoke_pop` level |
  | Supply | `sim::supply`'s `Need::Smoke`, served after replacement soldiers |
  | The AI's smoke | `crates/sim/src/skirmish_ai/smoke.rs` (`HpWatch`, `smoke_orders`, `backing_out`) |
  | The button, key and panel row | `commandBar.tsx`, `CommandBindings.pop_smoke`, `panelRows.ts` `smoke` |

- **One law, one deliberate copy.** The WGSL `fogSmokeStep` is the only
  copy of `contract::smoke::piece_depth`, because a shader can't call Rust. A
  probe parity test holds the two together; change them together.
- **Smoke touches sight only.** It blocks no movement, stops no rounds and
  gives no cover. A pop bumps no obstacle or navigation revision. A planning
  snapshot holds no clouds. `launch_clear`, map analysis and map generation
  read tree crowns only. Helicopter clearance and hearing cues are unchanged.
- **A battle without smoke hashes as before.** Clouds fold into the old
  expiry slot, and a battle that never pops folds only a zero count. A unit's
  charges and resupply clock fold only when it carries smoke. Giving an
  existing test or capture unit smoke would move its pinned digests; add a new
  row that extends it instead.
- **Who smokes is data.** No rule reads a role, family or name. Each carrier
  authors `capabilities.smoke` on its own row, never on a profile, because
  profiles mix in units that shouldn't carry it. A child row overrides only
  what differs.
- **The observation never names who popped.** A cloud row carries where, when
  and what, never the unit or its kind. Its id counts every pop in the
  battle, the same way soldier and prop ids count (choices S1).
- **The drawn cloud never shows smoke where sight has none.** Puffs stay
  inside the stepped circle and below the top. The drawn cloud stands at the
  law's first step from its pop tick, as concealment does.
- **Every smoke look has a sound and every fielded cloud has a look.**
  Fielding refuses a cloud whose look has no row. Sound admission refuses a
  row whose sound is unknown or loops.
- **The AI makes no random draws for smoke**, reads only its own units'
  losses (friendly fire included, D22), and never overwrites a unit's reverse
  with another order.

## Where it's tested

- The law: `crates/contract/tests/smoke.rs`. WGSL parity:
  `crates/renderer-core/tests/probes.rs`
  (`fog_smoke_step_adds_what_the_contract_adds_for_every_piece_of_a_sight_line`).
- The simulation: `crates/sim/tests/smoke.rs` (popping, life, revisions,
  resupply, replay), `smoke/sight.rs` (vanishing, reversing out, the rim,
  HIDDEN, launches, guided missiles, firing from smoke, cloud visibility),
  `smoke/publication.rs` (round trip and privacy) and `smoke/players.rs`
  (player units never pop). The fog sweep's cache tests are in
  `crates/sim/src/visibility.rs`.
- The AI: `crates/sim/src/skirmish_ai/tests/smoke.rs`.
- Presentation: `crates/presentation/tests/fog_smoke.rs`, `fog_ground.rs`
  (a published cloud rebuilds only the eyes it reaches), `smoke_screen.rs`
  (the drawn cloud's bounds, budget and refusals) and `sound_smoke_pop.rs`.
- Input: `web/tests/popSmoke.test.tsx`. The browser scenes are
  `web/scenes/smoke-screen.mjs`, run against the smoke screen lab
  (`/labs/smoke-screen`), and the panels scene's smoke cases.

## Where the build left the plan

- **Seen clouds draw over the fog veil.** The plan said smoke puffs must not
  lift the fog mask. On screen, a unit's own cloud then broke into a hatched
  blob round a flat patch. Smoke is now an ordinary effect (choices S5).
- **The pop's trigger is in the sound frame,** reading published clouds, not
  a new event in the effects publication. No layout or client type changed
  for sound (choices S34).
- **One icon for the button and the row.** The planned separate HUD icon read
  as "upload" or "cloud storage" (choices S24).
- **The observation reuses `contract::smoke::SmokeCloud`**, not a planned
  `ObservedSmoke` that matched it field for field (choices S21).
- **The AI watches a window of samples, not one baseline**, and doesn't clamp
  its reverse to the map. With no route back, a unit pops and stays in its
  cloud (choices S4, S31).
- **Two firewalls moved, both named.** New test rows shifted catalog order and
  moved one pinned test-unit battle (choices S9). The frozen generation rules
  lost the deleted field by hand edit (choices S8).
- **The captures were re-published natively,** not re-recorded in a browser
  (choices S3).

## Dead ends

- **Grid smoke, drifting smoke, a cloud as a transient prop:** rejected in
  planning (D5, D8, D19).
- **Linear puff opacity** (`opacity × density`): the half-faded cloud looked
  full under dozens of overlapping puffs.
- **Fewer, smaller, fainter puffs:** they broke the billow into separate
  cotton balls.
- **Fog-sight vectors with a cloud:** they would have been a second pinned
  recording. A behaviour test through a real publication holds the rebuild
  rule instead.
- **Test rows renamed to sort last** (`test_zsmoke_*`) to dodge the pin move:
  it worked, but names chosen to dodge a pin were worse than one re-record.
- **Splitting an oversized file in a commit before the change:** the size
  ratchet measures a commit against its parent, so the split frees no room.
  Splits land in the commit that needs the room.
- **A shoulder-SAM duel against a helicopter as the fire-and-forget proof:**
  unreliable, because the authored Stinger's first missile fell short even
  unhindered. A test-defined seeker missile proves it instead.

## Open to the user

- **The sound takes** were chosen from waveforms. Nobody has listened yet
  (choices N1).
- **Firing from inside smoke (D9).** A unit in smoke may fire at what its
  side has spotted, and each shot keeps drawing return fire. If the user
  overturns this, express it as data: a mount whose eye has smoke depth at or
  past the full block toward its target may not engage.
- **The Centauro II** (a wheeled tank destroyer with the `light_vehicle` role)
  is not on D2's list, so it carries no smoke unless the user says so.
- **The AI's eagerness** is three numbers in `fixtures/skirmish-ai.json`. The
  balance corpus can't yet show smoke's effect on who wins, because no row
  reaches a result within its horizon (choices S7).

## Visual provenance

- [`visualizations/mock.html`](visualizations/mock.html) is the moment the user
  reacted to in planning: a tank pops smoke, the enemy's view closes behind it,
  and its last-contact marker stays. It drove the lab's layout and the look
  (D11). Its key (X) and sound are superseded by H and D16, as its banner says.
- [`visualizations/map.html`](visualizations/map.html) is the four-quadrant
  walk of 2026-11-12 that produced every decision.
- [`assets/06/`](assets/06/) holds the fog closing behind a cloud from red's
  and blue's sides, before, during and after, judged against the mock
  (`vs-mock-*.png`). The triage is in its `critique.md`.
- [`assets/07/`](assets/07/) holds the drawn cloud over grass, road and
  forest, at bloom, full and mid-fade, overlapping, at far zoom and beside a
  burning wreck. The billow was judged against the mock's
  (`vs-mock-billow.png`) and against wreck smoke, which it must not resemble.
- [`assets/07-followup/`](assets/07-followup/) holds the before/after pairs and
  fog-mask views for the change that draws a seen cloud over the veil.
- [`assets/08/`](assets/08/) holds the command bar and panel crops: SMOKE n,
  spent, and HIDDEN in the cloud.

No external reference image was used. The standard was the war-film test, a
dense white-grey billow distinct from the grey-black of wrecks, judged by
unprimed critiques at each visual slice while the user was silent
(choices S6).
