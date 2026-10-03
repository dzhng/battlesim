# C50: durability balance and fitted terminal art

**Depends on:** C40–C43, C14 and C58's fixed-seed encounter. **Kind:** slice.

## Question
Are the fixture-owned hp and ruin coefficients playable while terminal geometry and immutable template art stay fitted?

## Contract it unlocks
Tune only the accepted fixture-owned coefficients (hp per m²·band and ruin ratio). Existing lifecycle/seating semantics remain unchanged. Hp-only tuning updates rules/identity. If ruin ratio changes physical bounds, rerun C14's terminal bake/fit/visual gate and publish a new appearance hash. Intact physical catalogue/maps do not change from an art rebake; update rule/config identity and reference replay expectations where the rule change requires it.

Replay is same-build playback, as in today's engine. Stored compiled scenario/rules and matching engine-build/digest identity are required; mismatch refuses playback. Cross-build simulation compatibility, pixel-exact historical art and archival asset retention are outside this spec. Current validated art may cover the same physical catalogue; the run records its appearance hash separately.

## API seam
Fixture-owned rules → existing lifecycle geometry; C14/C13 source export and C32 library fit → final release map/scenario identities. C54 then validates the final integrated candidate.

## What the human can run or see
Paired balance reports and, only if physical ruin height changes, matched terminal fit overlays with the new library identity.

## Verification
- Apply tweak-mechanics/write-tests before rule tuning; use the narrow/quick report while iterating.
- Run the full village_report once for the final balance candidate, plus the generated encounter's relevant scripted battle; C51 consumes this evidence unless later work moves balance.
- Record intentional digests, rule/map/library identities and final physical/art fit.
- On ratio change, repeat C14's compare-screenshots and unprimed screenshot-critique gates last, with preview-shots non-blocking. Hp-only changes do not invent a visual gate.

## Delegated to the implementer
Coefficient values from measured candidate comparisons. New lifecycle rules, unfitted terminal art and mutable replay history are not delegated.

## Must stay green
Seating/lifecycle semantics and physical appearance fit.

## Feedback that would change this slice
Rejected play or terminal fit revises the measured candidate and dependent release identities.

## Current candidate: retain the coefficients (2026-10-02)

The current hp coefficient and ruin ratio remain candidates without tuning. Local native battles on saved layout-12 Market Town (map `fb1e8584…`, seed 1) use the unchanged game rules, one tank and one garrisoned rifle squad. The tank stands at a navigation-admitted exposed position and receives an ordinary ground attack at the building owner's physical part, as the existing bombardment script does.

| Building | Initial integrity | Exposure | After one minute |
| --- | ---: | ---: | --- |
| Three-floor corner shop | 432 | 75 m | Collapsed at 18.27 s after three HE shots plus HMG fire; 3.1625 m ruin, five survivors outside by 60 s |
| Four-floor apartment slab | 1,155 | 250 m | 455 integrity left; nine HE shots, HMG trajectory blocked |
| Six-floor courtyard apartment | 3,960 | 200 m | 3,099.25 integrity left; nine HE and 171 HMG shots, seven occupants alive |

The shop has no exposed candidate in the probe's 100–250 m search. A courtyard frame origin can lie in its empty court; aiming there held fire correctly and is retained as a rejected probe, separately from the owner-part result. These checks establish local behavior, not the generated encounter's balance. Exact inputs, raw reports, probe source and hashes are retained in ignored `throwaway/c50-durability-pickup/` in the main checkout.

These local cases do not justify a coefficient or terminal-art change. The final village comparison and generated combat proofs are recorded below; consume their distinct scopes rather than repeat an unchanged full balance run. C50’s final integrated disposition remains open with the parent release gates.

### Current generated combat evidence

The final layout-13 Mixed Small seed 1 uses map `9b8ed835…`, configuration `397018f3…` and the integrated infantry weapon rules. The existing battle sweep runs 2,700 simulated seconds and launches 2,838 rounds, ending at digest `774aa7ac980edade`. Its initial group command places eight of nine attacking units; the second tank's destination is refused with no whole-command error. All eight placed attackers physically depart. The three rifle squads walk approximately 3.05, 3.24 and 3.16 km before being killed, ending 181, 119 and 271 m from their assigned goals. This establishes executed kilometre-scale infantry movement and real combat, not a capture or successful arrival by every attacker. The report retains its `command_refused` status and exit failure; the refusal is not hidden or replaced with another seed. No coefficient is changed to make this battle win.

Exact inputs, per-unit movement, acknowledgements and tick-cost data are in the main checkout's ignored `throwaway/city-maps-final-integrated/generated-assault-current-mechanics.jsonl` and matching log.

The original current-mechanics village report completes all fifty trials in 413.6 s wall time on two threads, retiring 17,200 G instructions. It exits 101 because flank seed 5 has one refused scripted order despite capturing at 552 s; other trials have zero refusals. That failed original is retained separately from the corrected comparison below. No coefficient tuning or unchanged full rerun is justified. Raw rows/log are `throwaway/city-maps-final-balance/village-current-mechanics.jsonl` and `.log` in the main checkout.


### Comparison-controller correction

The final village report retained one controller refusal: a replenished rifle's fixed return offset failed physical move admission, although the objective was admitted at the same state. The controller now asks the existing movement authority before choosing either intent, counts only an accepted return, and retains resting units after failure. Failed checks reuse the existing ten-second wait rather than rehearse unchanged destinations every tick. No durability coefficient, movement rule or generation number changes.

Focused real-service tests cover an inaccessible offset with a reachable objective, and two blocked destinations whose access later opens through actual destruction. The ten-flank comparison preserves nine previous digests; seed 5 has no refusal and retains its capture time and losses. This closes the controller fault; the assembled balance scope is recorded below, separately from generated-encounter admission.

### Assembled final comparison

The final fifty rows combine the original full report with the retested ten-flank cohort after the controller correction. Forty rows are retained; nine retested flank digests match. Seed 5 alone changes to `9951d92485980ff2`, removes the refused order and retains capture at 552.033333 s, cost 292.5, zero losses and three rejoined units. The assembled report has ten of each trial class and zero refused orders. Push captures five of ten and flank seven of ten, with two tanks lost across the flank cohort; ambush and crossfire capture none. No durability coefficient or physical-map rule changes. This is a full report plus an affected-cohort replacement, not a second full run. The receipt pins both input reports and their exact replacement scope in `throwaway/city-maps-final-balance/village-final-assembled.receipt.json`; final rows are beside it. The generated assault retains the separate partial tank destination refusal described above.

### Partial tank order disposition

A narrow current-Wasm probe regenerates the same Mixed Small seed 1 map/configuration and reproduces the original group acknowledgement exactly. Tank 3’s individual original formation slot also remains unplaced, while its individual objective is admitted. After the unchanged group order, a second ordinary command to an existing deployment-road waypoint is admitted: the tank departs at 0.1667 s, drives 297.578 m and physically settles idle within 0.022 m at 38.9333 s (tick 1,168), digest `a26e7f1758243807`. Its spawn is usable.

The existing movement contract permits partial group orders and requires demonstrated travel within bounded work. A failed certificate for one proposed slot does not prove every route to that point impossible. No trapped-spawn or generation defect is established, and no placement, fairness or work-budget number is tuned. Preserve the long battle’s `command_refused` status as a specific partial order, without making universal arrival or victory a release condition. Inputs, exact acknowledgements, one-second physical positions and hashes are in the main checkout’s ignored `throwaway/city-tank-refusal-audit/`.

### Integrated player combat scope

The [normal player encounter frame proof](C05-measuring-tools.md#normal-player-encounter-frame-proof) confirms actual rifle fire, opposing tank fire and projectiles through the released browser route, with ordinary rules and the partial order preserved. It complements the native long-travel report rather than replacing its failed status or the assembled village comparison. Keep the current reasonable coefficients and geometry; later personal parameter tuning belongs to the deferred live map workbench. Functional browser/source evidence is owned by [C54](C54-generation-gate.md#current-functional-integration), while synthetic stress and the final release-scope disposition remain [C05’s](C05-measuring-tools.md#corrected-stress-workload-v4).
