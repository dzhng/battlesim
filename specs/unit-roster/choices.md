# Unit-roster choices ledger

Planning baseline: 2026-10-06. The [exploration map](unknowns.md) attributes explicit
user decisions and delegated planning choices; it remains the authority for why
the captured mechanics were selected. This ledger records choices made while
turning that map into the implementation ladder and later committed passes.
No gameplay behavior has been implemented or verified yet.

| Choice | Attribution / reason | Owning slice | Evidence / verdict |
|---|---|---|---|
| Hard cutover, no migration/compatibility work | User explicitly selected no backward compatibility. | All | Planned; current replay admission already rejects differing identities. |
| Basic AI only | User defers sophisticated AI to another spec. | 07 | Planned; existing defensive controller is not a purchasing policy. |
| Thirteen focused contracts with an early playable checkpoint and concurrent model lane | Agent synthesis of three independent, differently biased drafts under write-spec. | 01–13 | Planned; avoid both a giant all-systems slice and serializing independent families. |
| Planned metadata outside available physical type admission, within one catalog owner | Agent, avoids invalid future aircraft breaking strict ground parser and live-type tests. | 01 | Planned; contract::catalog is the existing owner. |
| Preserve developer generic scenario types without player memberships | Agent, real authored labs are intentional consumers rather than a compatibility shim. | 01/04 | Planned; do not confuse hard schema cutover with deleting developer fixtures. |
| Compact skirmish profile research, standard map sizes intact | Agent, existing 4–10 km travel conflicts with quick edge-entry infantry play. | 02 | OPEN measured admission; candidate extents are delegated tuning. |
| Own base geometry replaces camera dependency on an initial unit | Territory finding, required by user zero-unit start. | 02 | Planned; prepare.ts:245–248 currently throws without a blue unit. |
| Ground carrier platforms only claim supported ground combat until transport exists | Agent, prevents expanding first phase into unasked transport mechanics. | 01/03 | Admission audit must record exact enabled/disabled role per variant. |
| Trophy is a defensive capability projected into a weapon row | Agent, obeys user panel vocabulary without fake offensive ballistics/commands. | 10 | Planned; reuse shared weapon UI and side-only readiness. |
| Service priority: offensive finite ammo, Trophy, HP, soldiers | Agent, retains existing offensive-first logistics pressure with explicit APS service. | 10 | Planned; verify finite stock and cooldown independently. |
| No adversarial review; retain required visual critique | User excludes adversarial review; repository requires unprimed visual evidence. | All visual | Planned; local review and visual critique are distinct from adversarial review. |

For each implementation pass, append a concise decision → reason → proof/verdict
entry for material choices the spec left delegated or silent. Do not turn this
into a file/diff log. Reconcile provisional entries against actual shipped code
at closeout; a future promise is not a verified final decision.
