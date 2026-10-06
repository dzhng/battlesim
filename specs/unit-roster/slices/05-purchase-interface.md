# 05 — Faction picker, disabled variants, ghost and persistent army bar

Status: not started. Dependencies: 04. Follow the README handoff and repository
skills before editing. This slice is a committed checkpoint, not a stopping point.

## Contract and scope

Compose faction selection in the existing skirmish menu and a persistent bottom purchase surface even at zero owned units. Use the Broken Arrow reference for category/family/variant grouping and military card vocabulary, not a dashboard or exact pixel clone. All 150 memberships are visible; unavailable entries cannot be selected, purchased or deployed. Shared entries are filtered by memberships from the catalog owner.

Show cost/affordability and enabled availability, free unit ghost, left-click confirmation, Escape cancellation, pending cards and actual owned cards. A successful placement returns to ordinary selection; choose a card again for another purchase. Preview calls the authority-owned type placement against side knowledge, including current variant/facing/route intent, never temporarily spawns a private unit. Destination is not physical spawn. Pending queue blockage/readiness/wallet are observed state. Make 30-instance scrolling readable. Preserve existing unit selection and weapon-panel structure.

Input produces ordinary recorded intents. No browser-only wallet, queue, readiness clock or inferred hidden enemy occupancy. Disabled UI messaging is concise and player-facing; implementation subsystem names do not belong in the menu.

## API seam and ownership

Resolved catalog + own/public match observation → picker/army UI; placement gesture → authority preview/confirm/cancel intents. Existing input/presentation owners and shared card/panel components are the seam.

## Narrow verification

Focused DOM/input tests for zero-unit access, family grouping, disabled selection, variant identity, free cancel, rejected confirm, pending-to-owned continuity and cap/affordability updates. One focused browser scene on the production skirmish route exercises choose → ghost → cancel/confirm → entry. Use actual rendered card/ghost states, not a mock as final proof.


For every visual shot produced in this slice: load game-ui and renderer as
applicable; compare candidate against its declared crop/reference and pre-change
baseline with compare-screenshots. Show the real artifact with preview-shots.
Run an **unprimed screenshot-critique as the last visual check before acceptance**.
The human checkpoint is non-blocking: leave a short (~5 minute) opportunity to
correct reversible choices while continuing independent work, decide from evidence
if silent, record rationale and close opened Preview shots. Never infer permission
for a new mechanic from silence. GPU jobs serialize under the existing harness.


## Review surface and verdict

Picker and bottom-bar specimen in every key state, then production battle gesture. Compare the reference category/family/card crop and before/after ghost framing; terrain/model fidelity is out of scope. Verdict: every player action matches the authoritative lifecycle and disabled entries cannot slip through input.

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
