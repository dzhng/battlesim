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

C50 remains open. Run the full village report once after the final mechanics candidate and the relevant scripted generated encounter on the final physical map. These local cases do not justify a coefficient or terminal-art change, and do not replace those gates.
