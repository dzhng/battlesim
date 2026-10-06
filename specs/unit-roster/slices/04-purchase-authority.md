# 04 — Zero-unit preparation, credits, reservations and physical entry

Status: not started. Dependencies: 01,02,03. Follow the README handoff and repository
skills before editing. This slice is a committed checkpoint, not a stopping point.

## Contract and scope

Compose a focused simulation skirmish owner rather than grow battle.rs with every policy. Shared setup carries chosen factions, admitted bases/sites and rule values. Phases are preparation/active/finished; wallets, reservations and readiness are authoritative. Preparation: zero live units, 1,000 credits, 60 s or both-ready early start, no income/capture/score. Active passive rate 200/minute with deterministic fractional carry.

Commands add Ready, ConfirmPurchase and CancelPending. Distinguish reinforcement entry from existing supply SetDeployment. Free preview; valid confirmation atomically validates membership/availability, side-scoped destination, wallet and 30 reserved-plus-living slots, then deducts and reserves. Use stable reservation IDs distinct from UnitId. Rejected confirmations change neither credits nor cap. Pre-spawn cancellation restores full price/slot; repeated cancellation reaches the same end state without a duplicate refund.

Spawn FIFO at one-second cadence only when the own road-edge footprint is clear. Wait visibly when blocked; convert reservation to live slot once, retain original price/resource denominators and issue ordinary fastest movement to destination. Hidden enemy occupancy is not a preview oracle. No teleport, overlapping bodies or paid-but-lost reservation.

Publish own wallet/reservations/rejections, public phase/readiness and later objective state through existing observation/packed publication/worker decoding. Include all authoritative state and fractional carry in digest/replay. Keep developer authored encounters as their intentional scenario mode; do not turn their fixture units into purchased units.

## API seam and ownership

Side command envelope → atomic purchase result/reservation → physical spawn. Existing simulation admission, movement placement, publication layout and WASM bridge change together; UI never debits locally or maintains a separate queue.

## Narrow verification

Invoke write-tests and pin red behavior at the authority seam: zero start/prep freeze, both-ready/timeout, exact income, unavailable/wrong-faction purchase, overspend, simultaneous confirmations, 30-cap, cancellation idempotence, blocked entry and one spawn only. Extend codec and paired native/WASM records plus replay digest checks for accepted and rejected commands. Narrow timeline/route fixture first; no full browser suite.


This is primarily a contract/data slice. No battle or screenshot is required for
pure documentation or metadata. If an actual visual change is produced, it inherits
the README's comparison, Preview and final unprimed screenshot-critique gates.


## Review surface and verdict

Native timeline probe plus an existing battle-lab fixture with zero army, pending queue and edge dispatch. Verdict: credits/slots/lifecycle remain authoritative across commands, ticks, replay and worker publication.

## Delegated freedoms and invariants

Internal type/function/file names and clean decomposition within the named owner
are delegated. Starter numerical tuning is delegated, with rationale and narrow
proof recorded in choices.md; physical/model facts require references. Cosmetic
spacing/icon fit may be adjusted reversibly under game-ui. Do not add mechanics,
change selected economy/victory/visibility rules or expand excluded capabilities.
If this slice exposes an unlisted material choice, record it in the choices ledger
and reslice the focused uncertainty rather than broadening implementation silently.

Keep existing developer fixtures, command sequencing, native/WASM agreement,
side-only knowledge and asset admission green. Verify changed contracts test-first;
non-behavior refactors preserve digest/replay outcomes. Review locally, update the
README pickup and commit the coherent pass, then proceed to the next independent
ready slice. No adversarial review or legacy compatibility scaffolding.

## Handoff evidence

- [ ] Red behavior/admission case observed where applicable.
- [ ] Narrow green proof and any allowed digest change recorded.
- [ ] Artifact/visual gates resolved where applicable.
- [ ] Choices, status and next pickup updated; focused pass committed.
