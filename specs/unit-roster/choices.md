# Unit-roster choices ledger

Planning baseline: 2026-10-06. The [exploration map](unknowns.md) attributes explicit
user decisions and delegated planning choices; it remains the authority for why
the captured mechanics were selected. This ledger records choices made while
turning that map into the implementation ladder and later committed passes.
Catalog metadata admission is implemented; skirmish gameplay is still in progress.

| Choice | Attribution / reason | Owning slice | Evidence / verdict |
|---|---|---|---|
| Hard cutover, no migration/compatibility work | User explicitly selected no backward compatibility. | All | Planned; current replay admission already rejects differing identities. |
| Basic AI only | User defers sophisticated AI to another spec. | 07 | Planned; existing defensive controller is not a purchasing policy. |
| Thirteen focused contracts with an early playable checkpoint and concurrent model lane | Agent synthesis of three independent, differently biased drafts under write-spec. | 01–13 | Planned; avoid both a giant all-systems slice and serializing independent families. |
| Planned metadata outside available physical type admission, within one catalog owner | Agent, avoids invalid future aircraft breaking strict ground parser and live-type tests. | 01 | Sound/high: planned rows serialize with the same catalog but cannot acquire a physical type index. |
| Preserve developer generic scenario types without player memberships | Agent, real authored labs are intentional consumers rather than a compatibility shim. | 01/04 | Sound/high: generic scenario units retain their real consumers; no player cards are synthesized for them. |
| Compact skirmish profile, hard identity cutover | Agent, standard 4–10 km travel conflicts with quick edge-entry infantry play. Profile owns 1.8/2.4/3.0/3.6km extents and compact road/river/site budgets; preset revision advanced to layout-presets-19, with parity records regenerated at closeout. | 02 | Small/Open seed1 full geometry+navigation green; bounded Mixed/Metro probes retain named generation/route refusals pending wider profile tuning. |
| Own base geometry replaces camera dependency on an initial unit | Territory finding, required by user zero-unit start. | 02 | Planned; prepare.ts:245–248 currently throws without a blue unit. |
| Ground carrier platforms only claim supported ground combat until transport exists | Agent, prevents expanding first phase into unasked transport mechanics. | 01/03 | Admission audit must record exact enabled/disabled role per variant. |
| Trophy is a defensive capability projected into a weapon row | Agent, obeys user panel vocabulary without fake offensive ballistics/commands. | 10 | Planned; reuse shared weapon UI and side-only readiness. |
| Service priority: offensive finite ammo, Trophy, HP, soldiers | Agent, retains existing offensive-first logistics pressure with explicit APS service. | 10 | Planned; verify finite stock and cooldown independently. |
| No adversarial review; retain required visual critique | User excludes adversarial review; repository requires unprimed visual evidence. | All visual | Planned; local review and visual critique are distinct from adversarial review. |

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
