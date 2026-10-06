# Unit-roster choices ledger

Planning baseline: 2026-10-06. The [exploration map](unknowns.md) attributes explicit
user decisions and delegated planning choices; it remains the authority for why
the captured mechanics were selected. This ledger records choices made while
turning that map into the implementation ladder and later committed passes.
Catalog admission and the first skirmish contracts are shipped; this ledger records the rationale that remains durable.

| Choice | Attribution / reason | Owning slice | Evidence / verdict |
|---|---|---|---|
| Hard cutover, no migration/compatibility work | User explicitly selected no backward compatibility. | All | Verified; current replay admission rejects differing identities. |
| Basic AI only | User defers sophisticated AI to another spec. | 07 | Verified; the shipped opponent uses ordinary recorded commands and leaves sophisticated policy to a later spec. |
| Thirteen focused contracts with an early playable checkpoint and concurrent model lane | Agent synthesis of three independent, differently biased drafts under write-spec. | 01–13 | Verified; the implementation landed through focused contracts and a parallel model lane. |
| Planned metadata outside available physical type admission, within one catalog owner | Agent, avoids invalid future aircraft breaking strict ground parser and live-type tests. | 01 | Verified; planned rows serialize in the catalog but cannot acquire a physical type index until supported. |
| Preserve developer generic scenario types without player memberships | Agent, real authored labs are intentional consumers rather than a compatibility shim. | 01/04 | Sound/high: generic scenario units retain their real consumers; no player cards are synthesized for them. |
| Compact skirmish profile, hard identity cutover | Agent, standard 4–10 km travel conflicts with quick edge-entry infantry play. Profile owns 1.8/2.4/3.0/3.6km extents and compact road/river/site budgets; preset revision advanced to layout-presets-19, with parity records regenerated at closeout. | 02 | Small/Open seed1 full geometry+navigation green; bounded Mixed/Metro requests still refuse when route fairness cannot be proven; this is an intentional admission result. |
| Own base geometry replaces camera dependency on an initial unit | Territory finding, required by user zero-unit start. | 02 | Verified for skirmish admission; the generic encounter guard remains documented separately, while skirmish uses explicit road-edge bases. |
| Ground carrier platforms only claim supported ground combat until transport exists | Agent, prevents expanding first phase into unasked transport mechanics. | 01/03 | Admission audit must record exact enabled/disabled role per variant. |
| Trophy is a defensive capability projected into a weapon row | Agent, obeys user panel vocabulary without fake offensive ballistics/commands. | 10 | Verified in the weapon panel and owner-only readiness publication. |
| Service priority: offensive finite ammo, Trophy, HP, soldiers | Agent, retains existing offensive-first logistics pressure with explicit APS service. | 10 | Verified by protection and logistics tests: offensive resources, Trophy charges, cooldown, HP, and soldiers are serviced deterministically. |
| No adversarial review; retain required visual critique | User excludes adversarial review; repository requires unprimed visual evidence. | All visual | Verified; local shape/diff/docs review and the required unprimed visual critique were retained, while adversarial review stayed out of scope. |

For each implementation pass, append a concise decision → reason → proof/verdict
entry for material choices the spec left delegated or silent. Do not turn this
into a file/diff log. Reconcile provisional entries against actual shipped code
at closeout; a future promise is not a verified final decision.

## Slice 01 — catalog contract

**Sound, medium confidence — generic equipment shares tuning across factions.**
Buying a standard rifle squad in any faction names the same equipment/profile
identity; an Eastern SVD marksman and U.S. M110 marksman remain different variants.
The plan required shared named platforms but left generic role sharing unspecified.
This avoids copies of identical tuning while allowing meaningful equipment splits
later. Faction membership changes availability, not a second set of stats.

**Sound, high confidence — family titles are explicit metadata.** A player opens
“M1 Abrams” and chooses a variant inside it. The catalog stores the family title
separately from its stable identifier and concrete variant title, rather than
asking each UI to strip designation text heuristically. The plan specified family
grouping but not the title representation; later pickers consume this one owner.

### Ground resource ownership — sound, high confidence

When: slice 03 ground-data foundation.
Choice: a marksman squad's soldiers each carry their own finite gun/ammunition state.
If one soldier fires two rounds, only that soldier's reserve falls; combining the
weapons into one pooled squad mount would make surviving soldiers share an artificial
magazine. The existing mount owner already supports the physical-gun contract.
Gap: starter loadout authoring did not specify pooled versus individual resources.
Reach: resupply and refund ammunition condition observe these independent owners;
weapon panels should group repeated rows for readability without pooling physics.
Verdict: sound; finite logistics must follow actual carried guns, not a UI grouping.

### BMP-3 gun-launched missile — sound, high confidence

When: slice 03 ground-data foundation.
Choice: the ordinary BMP-3 fires Bastion through its 100 mm gun and uses a separate
30 mm gun. Adding an external Kornet launcher would depict a different loadout.
Gap: planning named the platform, not the admitted weapon mounting.
Reach: model artists and service costs use the same two physical mount owners.
Verdict: sound; platform equipment and supported guidance agree. A third articulation
role is unnecessary because the existing secondary rig can carry the second gun.

### Purchase handles are private to each side — sound, high confidence

When: slice 04 authority checkpoint.
Choice: each player sees reservation numbers from their own counter. If the opponent
buys five unseen units, your next reservation number does not jump by five. Physical
unit IDs remain ordinary authoritative IDs once the vehicle actually enters.
Gap: the plan required stable reservation handles without choosing counter scope.
Reach: cancellation and browser queues use private purchase handles; replay records
still carry the issuing side so the authority knows which queue to address.
Verdict: sound; a shared purchase counter would leak enemy economic activity.

### Credit arithmetic keeps fractional carry — sound, high confidence

When: slice 04 authority checkpoint.
Choice: the authority stores millionths of a credit and carries the remainder from
each income division. After one active minute each player receives exactly the
configured minute's income, even when the tick rate does not divide it evenly.
The browser displays a numeric snapshot and never computes or spends its own wallet.
Gap: the plan required deterministic fractional carry but not the numeric format.
Reach: kill rewards and refunds can reuse one credit owner instead of parallel
floating-point balances. Exact state is proved through native/WASM digests.
Verdict: sound; integer carry avoids accumulated floating-point income drift.

### Expanded weapon vocabulary changes indexed combat identity — sound, high confidence

When: ground data integrated with slice 04 publication records.
Choice: new weapon rows enter the existing sorted arsenal, so a rifle's numeric
weapon index moves. Once a bullet is in flight its indexed identity changes the
battle digest, even when the shot geometry is the same. Records are regenerated
against the canonical vocabulary; no old-index compatibility map is retained.
Gap: the hard cut permits content identity changes but did not name this existing
index consequence. Reach: paired records and replays use current engine/catalog
admission; weapon names remain the authored stable reference.
Verdict: sound; one current arsenal implements the requested hard cut. All five
publication streams retain fog; noncombat streams retain their complete digests.

### Objective final-tick ordering — sound, medium confidence

When: slice 06 authority checkpoint.
Choice: existing ownership earns a tick's points before that tick completes a capture.
If two scores reach the limit during one tick, compare their exact fractions of the
tick; the earlier crossing wins and only equal crossing times draw. A capture that
completes at the end of that tick then evaluates all-flags victory if score has not
already ended the match. The last displayed scores include only the winning fraction.
Gap: the plan required deterministic finish ordering but did not choose the ordering
between a score crossing and an end-of-tick capture.
Reach: replays and browser fast-forward share one terminal moment; changing this
ordering later is an intentional gameplay change.
Verdict: sound; scoring from previously held territory precedes a newly completed hold.

### Terminal advancement returns the actual final tick — sound, high confidence

When: slice 06 authority checkpoint.
Choice: asking a finished match to advance another hundred ticks returns its existing
final tick. A request that encounters victory partway through also returns that final
tick and publishes the terminal frame once. The browser stops its tick scheduling;
no timeout or repeated identical frames stand in for match completion.
Gap: the old browser advancement contract assumed every match could keep advancing.
Reach: replay viewers, probes and player transport can finish without a no-progress loop.
Verdict: sound; explicit authoritative completion avoids treating a stalled engine as victory.

### Admitted geography chooses from reserved alternatives — sound, medium confidence

When: slice 02 final route admission.
Choice: generated maps reserve a bounded set of potential central and paired sites
before placing forests, parcels and props. Final admission measures infantry, light
vehicle and tank travel on the furnished map and selects a fair set. The browser and
native preparation use those selected coordinates, rather than measuring an
alternative but playing on the original. Reciprocal pair references follow selection.
Gap: the plan required organic route fairness but left the bounded search strategy open.
Reach: furnishing must protect every candidate until admission; map identity includes
the selected sites. Generation can still refuse when no fair set is provable.
Verdict: sound; measured final terrain, matching access/makeup and the existing 15%
travel limit remain admission requirements rather than best-effort placement.

### Automatic priorities apply when acquiring a target — sound, medium confidence

When: slice 11 authority checkpoint.
Choice: when a weapon chooses a new target, an observed enemy that can damage the
shooter at its current observed range comes before a harmless enemy. Purchase cost
then breaks that priority, followed by distance and the stable observed handle.
A weapon already tracking a usable target keeps that lock; an arriving expensive
or more threatening target does not interrupt an aim/reload cycle. Explicit attacks
still choose the player's target. The enemy's hidden ammunition, orders and actual
unobserved position never decide whether it poses a threat.
Gap: the plan required threat-first value priority but did not explicitly replace
the game's existing lock persistence with continuous target switching.
Reach: price tuning affects acquisition and reacquisition. Continuous reprioritizing
would be a separate intentional targeting change, rather than a hidden side effect.
Verdict: sound; stable fire control avoids aim thrashing while implementing the
selected value priorities whenever the weapon needs a target.

### Purchase ghost uses the named resting model — sound, medium confidence

When: production purchase interface.
Choice: the free placement cue draws the selected variant's actual mesh with its
authored resting pose, tinted through the existing selection color. For a squad,
one actual soldier represents the unit; the authority still admits the complete
squad footprint. The preview casts no shadow and writes no depth.
Gap: the request specified a unit ghost, without choosing a full formation preview.
Reach: no invented client formation or second placement geometry; a full squad
formation preview can later consume authority-owned destinations.
Verdict: keeps model identity recognizable without duplicating squad placement.

### AI commands run before tick advancement — sound, high confidence

When: basic skirmish policy integration.
Choice: the basic opponent issues ordinary commands against the last completed
observation before the next tick advances. Purchases reserve credits immediately
through the same admission path as player purchases. Replays execute recorded
commands and do not rerun the policy.
Gap: policy scheduling was unspecified beyond the five-second decision interval.
Reach: wallets and reservations change at the same boundary live and in replay.
Verdict: policy timing preserves exact replay outcomes without an AI-only economy.
