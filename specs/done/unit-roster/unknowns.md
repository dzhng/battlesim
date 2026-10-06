# Skirmish exploration map

Status: exploration complete, 2026-10-06. This is the four-quadrant handoff for
`/write-spec`; no gameplay or asset implementation has started. The [roster spec](README.md),
[starter balance](starter-balance.md), [economy research](economy-research.md) and
[model contract](model-production.md) carry the detailed decisions. This page is
self-contained as the decision/risk map; the shipped code supersedes
its next-action prompt; use README.md for implementation pickup.

## Known knowns — settled ground

Three factions: U.S., Europe, and Eastern Russia/China. Exactly 50 tracked entries
per faction, including variants; one family card groups variants, with at most ten
families per category. Categories are REC/INF/VEH/SUP/HEL/AIR. Shared variants have
one tuning/model owner and multiple memberships. Modern, iconic modern-era units
and plausible prototypes are allowed. No recovery, breaching, dedicated bombers,
fixed-wing transports or gunships. Helicopter transport stays in the future roster.

First implementation covers ground reconnaissance, infantry, vehicles and only
resupply support. All 150 entries remain visible; unsupported ones are disabled.
Every entry has a price, stat profile and weapon pack in starter balance. Individual
named-platform models are a major deliverable, assigned to parallel model agents.

First mode is 1v1 human versus basic AI. The user explicitly defers sophisticated
AI to another spec. Both sides obey the same economy, unit cap, visibility and
objective rules. The basic AI purchases, deploys, contests objectives and uses
existing combat behavior; online/team play is deferred.

Players start with zero units and 1,000 credits, derived from five minutes of
200-credit/minute continuous passive income. No army-size/success income taper.
A preparation phase gives no income, capture or score. Choose a faction variant,
preview a ghost, then left-click a destination; physical entry remains the team's
main-road edge spawn. Maximum 30 living or reserved units per player, squads count
as one. Owned and pending instances appear in the bottom bar.

Objectives earn victory score, never credits. Win at 1,000 points or by capturing
all objectives with none contested. Both sides score proportionally to uncontested
owned flags; the smallest majority held continuously takes 30 minutes to reach
1,000. There is no fixed 30-minute cutoff. Captured empty areas keep scoring.
Enemy ground combat presence pauses capture and scoring. Objective placement is
seeded, with exactly one or three central sites; off-center positions are organic
but matched by access and local makeup. Town squares, useful road intersections,
open fields and farmhouse-framed clearings are valid; forest interiors are not.
Reserve objective areas and appropriate surroundings during map generation.
Existing central X intersections are explicitly acceptable; hills are future sites.

Bounty represents net combat success. For victim original cost C, victim-side pool
B before death, and starting budget S: ordinary reward 25% C; killer pool growth
50% C; bonus min(B, 50% C, B × C / S); deduct bonus then 50% C from victim pool,
floored at zero. Supply trucks pay another 25% C once, independent of stock.
Bonuses do not generate recursive bounty. Refund is 15–75% of original cost, with
fraction 0.15 + 0.60 × ((H+A)/2) × max(0.5, 1−t/1,200). Withdrawal is physical.

Trophy is automatic defensive protection displayed as a weapon row. Four charges,
three-second cooldown. Intercepts anti-tank missiles and RPG rounds, including
future top-attack missiles; not bullets, tank shells or artillery. Supply trucks
restore charges gradually from finite stock without resetting cooldown. Threats
arriving during cooldown can hit normally.

Price changes intentionally change target value priority. Prioritize shootable
return-fire-capable enemies that can damage the shooter, then price. Do not split
purchase cost from value priority or use hidden enemy ammo to classify threats.

Repository facts actually inspected:

- [UnitType and capabilities](../../crates/contract/src/catalog.rs:39) own faction,
  family, cost, bodies/mobility, mounts, inheritance and capability admission.
- [Weapons](../../crates/sim/src/weapons.rs:894) already prioritize cost; the new
  return-threat tier extends that owner rather than replacing it with UI logic.
- [Commands](../../crates/contract/src/command.rs:112) need purchase/retirement
  intents; the current deploy ability is supply setup, not reinforcement entry.
- [Damage outcomes](../../crates/sim/src/damage.rs:247) carry destroyed IDs but
  not economic lethal-source attribution. [Round](../../crates/sim/src/battle.rs:70)
  already carries shooter unit and side.
- [Supply](../../crates/sim/src/supply.rs:1) is finite stock with stationary
  recipients and deployed sources; [fixture service](../../fixtures/game.json:350)
  owns existing stock costs/rates.
- [ArmyDeck](../../web/src/battle/present/armyDeck.tsx:11) shows deployed own units;
  purchase cards, planned catalog entries and pending orders need explicit input.
- [Weapon rows](../../web/src/battle/present/panelRows.ts:303) already share own/enemy
  presentation while hiding enemy readiness. Trophy should extend that surface.
- [Map sizes](../../crates/contract/src/generation.rs:52) are 4/6/8/10 km;
  [generation](../../crates/mapgen/README.md) is seeded and bounded.
- [Scene-assets](../../packages/scene-assets/README.md:16) freezes meters/axes,
  physical fit, mount frames and shared infantry rigs.

## Known unknowns — decision ledger

Attribution: **user** means explicit choice; **agent** means disclosed decision
under the user's delegation; **territory** means code evidence; **OPEN** means a
named research/verification task with its unblocking action. Numeric choices are
first-playtest tuning, not measured balance.

| Question | Closure | Decision and why |
|---|---|---|
| Mode/opponent? | user | 1v1 human/basic AI; sophisticated AI and multiplayer later. |
| Starting wallet/income? | user | Five minutes of 200/minute = 1,000; no taper. |
| Starting force? | user | Zero units, opening purchases one by one; prices fit 5–10 mixed/light units or fewer tank-heavy units. |
| Bounty accounting? | user | Net combat success, selected 25% reward / 50% growth and capped pool bonus; losses reduce pressure. |
| Supply kill bonus? | user | Extra 25% C, once, irrespective of stock. |
| Score model/victory? | user | Both teams score per flag, 1,000 target, majority = 30 minutes; all-flags special win. |
| Preparation? | user/agent | Preparation selected; agent sets 60 s plus both-ready early start, no income/capture/score. |
| Objective site/fairness? | user | Squares/intersections/fields/farmhouses, no forest sites; access/makeup matched, coordinates independent. |
| Counts and spacing? | agent | Small–XL 3/5/7/7, central 1/1/3/3; 150 m site/entry separation, 15% matched travel-time tolerance, bounded admission. |
| Capture membership? | agent | Ground combat REC/INF/VEH; squad living member or hull center in radius; no logistics/air/drone capture, ammo state does not exclude combat troops. |
| Capture transitions? | user/agent | 50 m/20 s starter values; contest preserves progress, departing attackers reset it; one-step takeover, extra units do not accelerate. |
| Empty ownership? | user | Keeps scoring, allows maneuver without permanent garrisons. |
| Simultaneous victory? | agent | First authoritative target crossing; exact tie draws; all-flags requires uncontested ownership. |
| Purchase commitment/slots? | agent | Free preview; atomic charge/reservation on valid confirm; full pre-spawn cancellation; physical spawn converts reserved slot. |
| Spawn route/blocking? | agent | FIFO, one-second clear-footprint dispatch, fastest ordinary road route; blocked entry waits visibly, no overlap/teleport. |
| Refund lifecycle? | user/agent | 15–75% formula selected; pay at road-edge base arrival, condition then, vulnerable slot until arrival, withdrawal cancellable. |
| Refund resource accounting? | agent | Squad original HP denominator; finite ammo rows incl. Trophy and truck stock, lost carriers count missing; no finite rows means A=1. |
| Refund effect on bounty? | agent | None; withdrawing veterans cannot erase team shutdown value. |
| Kill credit source? | agent | Lethal hostile source, once per full unit, shooter may already be dead; propagate through collapse/blast. |
| Friendly/source-free deaths? | agent | No payout/growth and no clearing bounty, avoids friendly-fire shutdown denial. |
| Same-tick bounty/rounding? | agent | Snapshot pools, proportionally cap aggregate payouts, apply losses then growth; deterministic fractional carry. |
| Unseen kill feedback? | agent | Wallet credits only; no new enemy identity/location/death notification. Wallet implies limited economic information. |
| Target price contract? | user/agent | One cost for price/value priority, threat tier first, known weapons/range/position; preserve existing shot admission. |
| Trophy threats/charges/cooldown? | user | Missiles/RPG incl. top attack, four charges, three seconds. |
| Trophy replenishment? | user/agent | Supply selected; agent sets 10 s and 20 stock/charge, finite stock, max four, cooldown unchanged. |
| Trophy timing/effects? | agent | 8 m hull standoff, all-around/top coverage, imminent collision path, swept event timing; ordinary early blast, no duplicate direct hit. |
| All roster prices/stats? | agent | 150 assigned in starter balance; user delegates numerical tuning without per-value questions. |
| Model-worker contract? | user/agent | Parallel family workers; frozen geometry/mount/rig references, exclusive paths, coordinator-only integration; three workers at current harness capacity. |
| Existing generic game mechanics? | territory | Reuse tested movement, sensing, cover, damage, guidance and supply; named capabilities still require admission. |

### Delegation and corrections

The user explicitly opted into routine decisions following preferences: maneuver,
combat incentives without income taper, short defensive response windows, organic
fair maps, and a focused implementation reusing existing mechanics. They then
explicitly delegated all starter stats/prices without individual confirmation.
Behavior/scope beyond these preferences is not blanket delegated. Implementation was subsequently authorized by the user after the spec is ready:
`/goal /implement-spec`, with no backward compatibility. The walk itself remains
a planning artifact. The user can correct or withdraw delegation.

Rejected/superseded choices: per-unit bounty (fresh-unit loophole), accumulated-kill
pools (balanced exchanges inflate both sides), income taper (feels punitive),
objective credit income (snowball), mirrored grids (unnatural), mandatory garrisons
(restrict maneuver), fixed 30-minute cutoff (replaced by point target), 60/minute
income (user chose 200), and separating cost from target priority (user correction).

### OPEN items — research tasks, not missing numerical approvals

| Item | What unblocks it / owner |
|---|---|
| Compact skirmish layout admission | Owning slice 02: early map/travel probe under mapgen: shipped 1.8/2.4/3.0/3.6 km extents, retain standard maps, test terrain character, flags, edge roads and bounded generation. Numbers delegated; do not silently accept invalid geometry. |
| Exact physical dimensions/mount frames | Owning slices 03/12: platform references plus existing asset-fit validation; coordinator freezes each manifest before dispatch. |
| Authentic top-attack and troop carriage | Owning slice 01: first capability audit determines existing support. Unsupported variants remain visible/disabled; adding missing rules requires an explicit bounded slice or later capability spec. |
| Air/drone/artillery/EW mechanics | Their later specs define mobility, sensing, entry, targeting and resupply; current cards/stats are placeholders only. |
| Drone platform names/model identity | Pick references before their later model tasks; generic placeholders stay clearly labeled. |
| Final balance confidence | Owning slice 13: paired first-phase battles and narrow stat experiments after implementation; no balance claim from these tables. |

Every remaining named question is answered or explicitly OPEN above. Stage 2 is
closed. OPEN items are carried into the spec's first feasibility/capability slice
or later excluded-capability work, not left for an unsupervised builder to invent.

## Unknown knowns — extracted preferences and context

Concrete site examples elicited the user's corrections: dedicated squares and
surrounding buildings, fields/farmhouse clearings, and the existing central X
junction as an acceptable objective. The map's geography should read as a plausible
battlefield rather than a grid. This reshaped placement from post-hoc flag sampling
to reserved objective sites, while preserving useful existing intersections.

The user's corrections established two further tacit contracts: price is a combat
value signal, not only an economy field; and the AI should be serviceable rather
than consume the feature with a sophisticated strategy project. Trophy belongs
in the learned weapon-row vocabulary. Named models and cool designations matter;
extra nationality infantry/loadout filler does not.

Consumer/context probes are answered by requirements and repository evidence:
players use the existing desktop browser/WebGPU game; implementers and model workers
inherit this spec; authored data and simulation are authoritative, never a separate
finished design inventory. Done means a playable faction/economy/deployment/objective
loop with the admitted first-phase ground units, correct named models, basic AI,
and all other entries visible but disabled. The evidence surfaces are the existing
battle lab, map workbench and model workbench, not a new standalone dashboard.

The user reacted to the concrete objective-site choices and supplied these context
corrections. Stage 3 is closed; numeric tuning is delegated rather than another
questionnaire.

## Unknown unknowns — integration sweep and landmines

Coverage: inspected 24 source/config/UI files across catalog, weapon/flight contracts,
commands/observations, battle/damage/supply/targeting/digest/publication, encounter
sites/scenario/generation, town layout/presets, fixture units/weapons and army/weapon
panels, plus their owner READMEs and the Blender source guide. This is a boundary
sweep for a not-yet-implemented feature, not a claim to inspect every future diff.

| Status | Evidence | Why it bites / required consequence |
|---|---|---|
| OPEN feasibility | [MapSize extents](../../crates/contract/src/generation.rs:52) | Edge-to-center infantry travel is ~11–27 minutes; a 30-minute majority target does not make a short match. Probe compact skirmish geometry before committing layout sizes. |
| decided | [Ground-only mobility/capabilities](../../crates/contract/src/catalog.rs:107), [strict unit decoding](../../crates/contract/src/catalog.rs:38) | Disabled future planes cannot just be invalid playable unit rows. Extend the one catalog owner with explicit planned/available state; planned metadata is visible but never admitted as a live unit. |
| decided | [Automatic value selection](../../crates/sim/src/weapons.rs:894) | Price tuning changes targeting intentionally. Keep one cost, add return-threat ranking with side knowledge, verify higher-price changes and incapable targets. |
| decided | [Outcome destroyed IDs](../../crates/sim/src/damage.rs:247), [building-collapse consequences](../../crates/sim/src/battle.rs:1618) | Destruction alone loses economic source and can pay twice. Carry stable source/side/original-cost through full-unit death and collapse; ledger once. |
| sharp edge | [Unseen-death suppression](../../crates/sim/src/battle.rs:1608) | Kill credits can reveal an unseen loss indirectly. Accepted wallet-only reward must not add enemy identity/position or corrupt contact memory. |
| decided | [Finite supply validation](../../crates/sim/src/supply.rs:77) | Every finite weapon needs a stock price. Trophy needs an explicit service item/timer; avoid instant refill or restarting cooldown. |
| decided | [Town parks become wooded](../../crates/mapgen/src/layout/towns/mod.rs:823) | An empty park today may become trees later. Reserve square/junction/field objective sites through final layout and forest generation, then validate physical accessibility. |
| sharp edge | [Site centers are street junctions](../../crates/contract/src/encounter.rs:73) | Settlement centers need not lie exactly on the halfway line. Preserve selected center-line objectives by composing the site, never silently snapping off-line. |
| decided | [Enemy weapon rows hide readiness](../../web/src/battle/present/panelRows.ts:303) | Showing Trophy ammo/cooldown for enemies leaks state. Own panels show live values; enemy panels show only admitted equipment evidence. |
| decided | [Preparation needs a blue unit](../../web/src/battle/prepare/prepare.ts:245) | A zero-unit encounter throws and cannot frame the camera. Slice 02 uses admitted base/start geometry instead. |
| decided | [ArmyDeck depends on own units](../../web/src/battle/present/armyDeck.tsx:11) | Zero-unit starts currently remove the army footer. Purchase access must stay visible at zero units; pending cards need explicit observed economy data. |
| decided | [AI command/replay path](../../crates/sim/src/battle.rs:1391) | Current defensive AI has no purchase/objective planner. Extend a basic observation-bound controller; record orders and do not rerun policy in replay. |
| decided | [Digest enumerates authoritative state](../../crates/sim/src/battle.rs:2824) | Credits, pools, reservations, objectives, retirement and APS state must join digest/replay/native–WASM parity; UI timers cannot become authority. |
| decided | [Flight admission](../../crates/contract/src/ballistics.rs:6) | Range/speed/gravity/lifetime/scatter are coupled. Starter envelopes require real admission; use swept Trophy interception rather than a per-frame distance check. |
| sharp edge | [Asset fit and atomic loading](../../packages/scene-assets/README.md:7) | 150 cards do not require loading unsupported meshes at startup. Use explicit placeholder bindings for planned entries; no disabled entry should make the entire playable catalog fail asset admission. |
| sharp edge | Selected 200/minute income and 30-unit cap | Thirty minutes injects 6,000 passive credits, not 1,800. Check field density and replacement churn; do not silently add upkeep/taper to solve crowding. |
| sharp edge | Selected score-target victory | If every flag stays contested, a match can run indefinitely. This follows removal of the hard cutoff; no unstated overtime or forced score mechanic is added. |

All findings are decided, OPEN with an unblocking task, or noted sharp edges.
Stage 4 is closed. No gameplay tests/battles ran because only planning files changed.

## Handoff

The user requested `/write-spec` after exploration. That plan is now materialized
in README.md and slices/; this map's planning kickoff is superseded by the
the closed README and code pointers are authoritative.
Reuse the starter tables and model contract; keep remaining research in an early
feasibility/capability checkpoint, not buried inside implementation passes.
Do not run adversarial review: the user explicitly excluded it. The user subsequently authorized `/goal /implement-spec` once the spec is ready,
with no backward compatibility. Begin only from the materialized plan, not an
exploration note.

Historical planning prompt (implementation is complete):

> Implement specs/unit-roster/README.md from its first unchecked slice. Preserve
> the selected economy/objective/targeting contracts, use starter tuning without
> per-stat approval, and use parallel family model workers under the frozen model
> contract. Keep unsupported units visible and disabled. Report narrow proof per
> slice and run full verification once at implementation closeout.
