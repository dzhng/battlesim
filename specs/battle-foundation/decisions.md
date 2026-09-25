# Planning choices and remaining uncertainty

Last updated 2026-09-25. This file owns spec-authored choices and implementation deviations. Confirmed player-facing rules live in requirements.md. A future pass appends consequential implementation decisions here, updates owning contracts/slices, then refreshes the README handoff.

## Chosen architecture and scope

| Choice | Rationale and rejected alternative | Status |
|---|---|---|
| Village milestone before full match; retain 17–22 as explicit continuation | Tests the requested fun loop early; a single giant first release would hide failure sources. Full-game requirements are retained, not canceled. | Spec scope; matches chosen first checkpoint |
| Reproduce then port low-level renderer foundations | Existing battleScene imports old crowd/assets/environment owners; wholesale copying brings irrelevant gameplay/presentation assumptions. | Spec choice; validate exact manifest in 01 |
| 30 Hz authority, physical swept flight within each tick | Matches sibling scheduling while preventing bullet tunneling; render cadence cannot control physics. | Spec choice; measured in 07/16 |
| Single Rust geometry, XY ground/+Z up, triangulated 4 m height field | Shared surface for collision, sight, routing and renderer; avoids decorative-height mismatch. | Spec choice; query precision independently verified |
| Player observations precede all presentation/AI | Filtering models alone leaks targeting/audio/props and future network truth. | Architectural constraint inherited from information rules |
| No rigid-body engine initially | Needed dynamics are trajectories plus primitive queries and prescribed unit movement. A new full engine is not required for the chosen physics. | Spec choice; if sweeps fail, reslice before adopting a library |
| Perimeter occupant hit regions ahead of building shell | Makes probabilistic garrison cover and ordinary wall obstruction coexist without target-dependent projectile transparency. | Spec choice; focused validation in 11 |
| No AP speculative fallback | Later general-purpose-only decision overrides the truncated original HE-exhaustion suggestion. Unlimited default gun is the fallback. | Precedence resolution, not new user rule |
| Stop while packing reverses toward deployed | User says canceling movement reverses packing; Stop clears movement. Avoids a contradictory frozen deployment state. | Precedence resolution |
| One supported AT missile per launcher initially | Gives an explicit support-capacity rule without inventing multi-channel guidance. | Spec choice; future data may widen with a new fixture |
| Prototype tuning and authored enemy policy | Concrete runnable starting data; values/defense assignments are not asserted final balance or hidden scripting. | Delegated only with paired evidence |
| Current-host measurement; no compatibility/migrations | User explicitly chose both. | User confirmed |

## Interview OPEN-item disposition

| Map item | Current resolution and owner | Remaining work |
|---|---|---|
| O01 balance/scales | fixtures/village.json and encounter.md fix provisional starting values; slices 17–22 own later-unit values | Tune with paired seeds; 1.5 s grace stays selected |
| O02 geometry/collision | contracts.md fixes primitives, triangle owner, sweeps, garrison representation and blast rules | Reproduction 02/07; focused garrison verdict 11 |
| O03 information | contracts.md fixes evidence/contact identity/stability, audio, tracer and dynamic-prop publication | Metamorphic gates 05/06; future aerial extensions retain boundary |
| O04 guidance | contracts.md fixes village own-lock launch/support, one slot and last-ground-point behavior; slice 19 fixes seeker-loss default, 20 sortie | Tune turn/flight values and verify air variants in continuation |
| O05 state intersections | Transition table plus deployment/service contracts; slices 08/12/13 | Translate every row to seam fixtures; add any new event deliberately |
| O06 damage/weapon edges | contracts.md fixes AP conflict, face penetration, turret gate, default fallback and blast sampling | Physical validation 07/09/11; no implicit ricochets/component damage |
| O07 map/remains | Closed bounds, dual approaches, corpse/wreck/ruin distinction, escape policy; geometry/navigation slices | Later full map must review chokepoint isolation |
| O08 match/economy | Slice 21 gives initial capture/scoring/income/entry rules; village local completion explicitly separate | Full-match values are continuation tuning, not needed for 01–16 |
| O09 UI/input | contracts.md fixes commands, queue/policy semantics and readouts | Reversible visual choices delegated under screenshot gates |
| O10 encounter | encounter.md plus fixture fixes map/roster/scripts/opponent restrictions; no air in first encounter | Build and demonstrate 15, not just a sandbox |
| O11 host/browser | Host identified; runtime probe is slice 01 | GPU/browser capability and throughput unmeasured |
| O12 multiplayer | Explicit commands/observation/time boundaries now; network transport/authority deployment/rollback/lockstep remain deferred | Separate future networking spec, no accidental lockstep promise |
| O13 longevity | Workloads/budgets/metrics in validation.md; progressive probes culminating 16/22 | Actual performance evidence, not blanket promises |
| O14 reuse | Revision pinned; research.md source/disposition/test inventory; per-file manifest in 01 | Inspect exact transitive imports/licenses before copying |

The remaining uncertainty is measured behavior and named continuation tuning, not an invitation to invent unlisted rules. Before coding each slice, verify its source inputs and the implementation host. A newly discovered user-impacting tradeoff gets a concrete proposal and owning spec edit; internal reversible choices within the stated decision budget may proceed.

## Recursive fog audit

The minimal five-rung draft combined too many mechanisms to diagnose failures. The canonical ladder splits renderer reproduction, world geometry, authority, navigation, sensing, observation projection, projectile flight, weapon transitions, consequences, guidance, garrisons, deployment, service, readouts and encounter composition. Each has one dominant seam and a focused verdict. Full-frame readability waits until component variables have evidence. High-risk garrison geometry and projectile sweeps are explicit rejection points, not “make it realistic” assignments.

The materialized plan has one owner for projection, world geometry, knowledge, weapon state, unit policy, deployment, service stock, protocol layout, RNG/time and fixture data. No old-game adapters or compatibility branches are planned. The lab emitter and renderer reproduction remain small diagnostic consumers of production seams, not temporary alternate engines.

## Implementation decision log

Add entries with: owning slice; observation/evidence; chosen option and why; rejected alternative; any affected requirement; verification result. If the choice changes a selected user rule, leave that rule unmodified until the user decides.

- **01 — TypeGPU resource ownership.** Observation: repeated scene rebuilds leaked 11 GPU buffers each; `root.destroy()` in TypeGPU 0.12.5 does not destroy buffers created through the root. Choice: every scene registers its allocations and destroys them explicitly; the lab's allocation tracker (wrapping `device.createBuffer/createTexture`) proves return-to-baseline. Rejected: relying on root teardown. Verification: foundation scene resize/rebuild check.
- **01 — Evidence location.** The spec originally stored captures under `assets/evidence/<slice>/`; the implementation workflow keeps raw captures out of git. Choice: scenes regenerate captures into gitignored `throwaway/evidence/<fixture-id>/`; verdicts record metrics and critique dispositions. validation.md and every slice updated.
- **03 — Village fixture map schema.** The fixture's `map` section was rewritten into `contract::map::MapDefinition`, the same schema the geometry lab uses, with identical numbers. The ridge became a `ridge` relief, building footprints became `building` props 8 m tall, and the tree rule became `forests` with a 24 m spacing and 2 m clearance. The shared 35° cutoff moved from `movement` to `map.slope_cutoff_deg`, and building rules moved to a `buildings` section (capacity, HP, ruin height, cover strength). Descriptive keys the schema now encodes (`rect_format`, `ridge_function`, `tree_placement`, `foliage_has_projectile_collider`, `approaches`, `mandatory_water_crossing`, `village_center`) were dropped; the encounter keeps its success zone. No second copy exists.
- **04 — Pan keys.** contracts.md gives both "WASD pans" and S = Stop, A = attack-move. The command keys win; the camera pans with the arrow keys and the screen edge. Recorded as needs-user in choices.md.
- **15 — Village tuning log.** All runs: seeds 1,2,3,5,8,13,21,34,55,89, 900 s cap, `cargo run -p sim --release --example village_report`.
  - *AT placement.* Observation: with the ordinary AT team at [780,880] the red tank, fed by the garrisons' shared 600 m spotting, destroys road-bound tanks from ~700 m before either AT team sees one. Tried [706,826] and [740,845] closer to the road; neither changed which unit kills the tanks, so the authored [780,880] was restored. The AT trigger range is recorded in the fixture (`defender_policy.at_attack_range_m` = 900, the value encounter.md already names). No other numeric data changed.
  - *Ground attacks on buildings.* Observation: HE aimed at a building's centre reported `BlockedTrajectory` (the building shields its own centre). Choice: a ground point inside a building is aimed at the facing wall (`weapons::facade`). Rejected: exempting the target building from the line check, which would let rounds pass through walls.
  - *Script fixes, not physics.* The ambush script's retreat point lay east of where its tanks stalled (toward the enemy) and it ignored hits its own tanks felt; it now drives by Move, cues on a felt hit or a fresh firing area within 400 m, and falls back west. The supported script sometimes left a hidden squad sitting behind a ruin inside the zone, contesting it forever; once in the zone and still contested, it now shells any building still standing and then sweeps the zone. Before → after: supported 6/10 → 8/10 captured (after review, the script also stopped stalling while enemy armour stayed identified); prompt/delayed ambush tank losses 14/14 → 0/3.
  - *Result against encounter.md targets.* Supported 8/10 captured, 1040 cost points vs unsupported 4762 — met. Prompt retreat 0 tank losses vs delayed 3, a survivor in 10/10 — met. Prepared crossfire: 0 tank losses — **not met**: the scripted tanks turn back on the red tank's first long-range hit, ~400 m short of either AT team, so the second angle never engages. Per slice 15's feedback rule the crossfire mechanic is split out rather than forced: slice 10's ambush lab shows a crossfire beating a prompt escape through the same physics. No spawn was moved to manufacture a loss.


- **After 16 — What props block (user decision).** The user ruled that wrecks block vehicles but not infantry, and that blocking is a per-prop property by mover class, so future props can stop infantry but not vehicles (fences) or light but not heavy vehicles. Built as `contract::map::MoverClass` and `PropKind::blocks(class)`, a per-class navigation grid, and `PropKind::occludes()` kept separate for sight. Rejected: a special case for wrecks in movement. Replaces the slice 09 choice that a wreck blocks all ground movement.

## Handoff audit corrections

Two independent read-only audits confirmed all 80 rules were retained and identified interactions that needed tighter contracts. The finalized plan now makes visibility grace take precedence over automatic contact fallback; logs both sides and disables bots/input during replay; names loaded ammo kinds and conserved AP/HE swapping; specifies continuous environmental strengths and spread/blast formulas; defines target-facing garrison slots; fixes map geometry/collider inputs, ordinary/crossfire variants and bot opening triggers; and upgrades double-click moves even across applied ticks. The full-match target is consistently 45–60 minutes.
