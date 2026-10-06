# 01 — Canonical identities and planned availability

Status: not started. Dependencies: —. Follow the README handoff and repository
skills before editing. This slice is a committed checkpoint, not a stopping point.

## Contract and scope

Extend `contract::catalog` as the single authored/resolved owner of canonical variant identity, faction memberships, REC/INF/VEH/SUP/HEL/AIR family grouping, one cost, and planned versus available state. Planned records have visible card metadata, provisional profiles and a disabled reason, but never become physical `TypeIndex` entries or demand a playable appearance. Available records retain strict body, mount, mobility, weapon and capability admission. Shared U.S./European platforms are one identity with multiple memberships. Developer generic types can remain valid scenario content without player-faction membership; they are not a second player roster.

Author all 150 memberships from the roster and starter tables; preserve 50 per faction and at most ten families/category. Cut producer, generated catalog, consumers and fixtures to the new shape together. No alias catalog, legacy parser or migration readers. Do not add speculative categories or nationality variants.

Produce `capability-admission.md`: for each phase-one family/loadout, state existing supported physics, required assets and blocked capabilities. Inspect carriage and authentic top attack before enabling their promises. Missing mechanics remain visible/disabled; future air/drone/artillery/EW/transport rules do not enter this feature merely to make a card selectable. Ground carrier platforms may be admitted for their supported ground weapons only when their description does not promise unimplemented passenger behavior; record that limited role explicitly.

## API seam and ownership

Catalog resolution and available-type lookup are the seam. Resolved card data carries canonical variant ID, display name, family/category, faction memberships, cost and availability. Runtime instantiation asks this owner for an admitted physical type; neither UI nor AI decides availability independently.

## Narrow verification

Extend existing contract/sim catalog tests first: planned aircraft loads as metadata but cannot spawn/purchase; malformed available physics still refuses; shared variants do not duplicate tuning; all memberships/counts resolve. Regenerate the browser catalog with the existing owner. Test public admission/lookup behavior rather than freezing JSON formatting or mirroring the table.


This is primarily a contract/data slice. No battle or screenshot is required for
pure documentation or metadata. If an actual visual change is produced, it inherits
the README's comparison, Preview and final unprimed screenshot-critique gates.


## Review surface and verdict

Inspect resolved catalog via existing mechanics/model workbench and a tiny picker specimen. Verdict: every roster identity is represented, unsupported entries cannot reach live construction, and consumers agree on availability.

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
