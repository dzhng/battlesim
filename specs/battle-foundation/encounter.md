# Village encounter and tuning

The only initial numeric owner is [assets/village.json](assets/village.json). It is a **proposed starting fixture**, not balanced historical data. On implementation, move the authoritative fixture to `fixtures/village.json` and replace this planning copy with a provenance pointer; do not maintain two editable tuning tables. Update this document to point at the runtime owner in the same pass.

## Playable question

Can reconnaissance, suppression, flanking, and rotating survivors through supplies outperform an unsupported tank push, while an ordinary ambush still permits a narrow escape through prompt response?

The authored 1.6 km square village is a focused test region, not a reduction of the agreed full 4 km map. Blue starts west with a recon squad, three rifle squads, two tanks, an AT squad and one supply truck. Red has three rifle squads assigned to the three buildings, one active AT team and one tank in the ordinary variant. The prepared-crossfire variant adds a second active AT team at [1120,650]. The catalog fixture lists both AT placements but the ordinary variant explicitly disables the second. Both use the same unit definitions. Red is a small observation-constrained policy, not a full strategic AI: defend authored areas, start AT teams in Return fire only; an AT team issues one explicit attack on the highest-cost tank its own optical sensor identifies within 900 m, switching its entire squad to Fire at will under the agreed rule. Other defenders begin Fire at will and hold assigned positions. A tank below 35% own HP may issue ordinary move to [1250,850]; an infantry squad below 50% original living strength may withdraw to [1200,950]. Trigger once per unit, and use only own/observed state; no reinforcement or hidden retreat tracking. It receives no hidden Blue state and uses no scripted unavoidable hits.

Use two routes into the village: direct road and longer flank. Place a ridge and forest edge to create meaningful sensor differences and an escape path. The first fixture has no mandatory bridge; water/bridge behavior remains independently verified in the geometry/navigation lab. One ordinary ambush is separable from a stronger crossfire variant using the same map and weapon data.

The encounter reports success after an eligible Blue ground combat unit holds the village zone uncontested for 30 seconds. This is a local fixture completion condition, explicitly not an alternate implementation of full-match majority scoring. Report failure if Blue loses all combat units; after 15 minutes report inconclusive and allow continued play/reset. A reset reconstructs from seed; replay reconstructs from seed and accepted commands from both sides, with input and command-generating AI disabled, rather than loading a mid-state snapshot.

## Tactical comparison scripts

Store scripts as real tick-stamped commands through the same API as input; labels are proposed fixture IDs, not existing tests.

1. `unsupported-road-push`: tanks advance first with no scout and no supporting suppression.
2. `scout-suppress-flank`: recon observes from cover; infantry pins the visible defense; tanks take the flank; damaged/depleted survivors withdraw to the deployed supply and rejoin.
3. `ordinary-ambush-retreat`: issue retreat a fixed 0.75 seconds after the first legal incoming-fire cue, never using hidden launch time. Compare the same seed with a 3-second delayed response.
4. `prepared-crossfire`: add the second AT angle in the authored variant; prompt reaction may still lose a tank. Do not guarantee a loss by bypassing physics.

Across seeds 1,2,3,5,8,13,21,34,55,89, the supported approach should capture at least 7/10 trials and have fewer aggregate Blue casualty-cost points than the unsupported push. Prompt retreat should reduce aggregate tank losses versus delayed retreat, and leave at least one survivor in at least 7/10 ordinary trials. Prepared crossfire must produce at least one genuine tank loss across the set; not every seed must kill a tank. These are **spec acceptance targets to tune**, not measured results. If they fail, inspect visibility/flight/order evidence before editing numbers. Never inject scripted misses/hits or widen tolerances merely to green the report.

Independently demonstrate finite supply depletion, visible repair, ammunition refill, surviving-squad replacement, and later re-entry into combat. The village player UI includes reset, pause, seed display and replay export/import for the current scenario/config hash. Replay rejects mismatched fixture digests with a clear error; there are no migrations.

## Tuning freedoms

Keep selected 1.5-second grace fixed unless the user changes it. Other provisional numeric data may change with a recorded comparison report; preserve user role relationships and 45–60-minute full-match intent. Do not let the village's short action loop redefine full-match pacing. HE/AP share a single cannon aim/reload mount; changing ammunition on that mount does not create a second gun or parallel reload. Default gun ammunition remains unlimited. A squad's specialist mount is not multiplied by every soldier.

Record each meaningful adjustment in `decisions.md`: observation, before/after values or runtime config hash, paired seeds, consequence, and why the chosen option better matches N03. Art, formation spacing, and camera changes must not silently change hitboxes or range.
