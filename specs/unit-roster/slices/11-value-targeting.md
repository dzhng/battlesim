# 11 — Threat-first automatic selection with intentional price priority

Status: not started. Dependencies: 03. Follow the README handoff and repository
skills before editing. This slice is a committed checkpoint, not a stopping point.

## Contract and scope

Extend existing weapons::select; do not create a separate AI or HUD target-priority owner. Among targets the firing mount can shoot/effect under current admission, prefer enemies whose known weapons can return fire and damage the shooter at the current observed range/geometry. Then descending cost, distance and stable observed ID. Known weapon reload does not remove threat classification; never inspect hidden enemy ammo or private orders. Preserve existing minimum range, line of fire, armor and directed player attacks.

Use a single authored cost for purchase and automatic value priority: changing price intentionally changes whom units select. Keep existing contact/default-gun suppression fallbacks outside the new threat tier; do not silently eliminate legitimate existing behavior while adding priority.

## API seam and ownership

Existing mount targeting assessment + side knowledge → ranked permitted targets. Catalog cost remains the single value signal; no second priority coefficient or per-platform named exception.

## Narrow verification

Focused weapon tests: expensive harmless vs cheaper damaging target, two threatening targets with price change, target too close/out of range/blocked, target cannot damage shooter, shooter cannot affect target, identical observation with different hidden ammo, explicit attack and contact fallback. Quick affected rule sample only; no full balance until closeout.


This is primarily a contract/data slice. No battle or screenshot is required for
pure documentation or metadata. If an actual visual change is produced, it inherits
the README's comparison, Preview and final unprimed screenshot-critique gates.


## Review surface and verdict

Deterministic target-choice fixture plus one declared physical moment. Verdict: threat outranks value, value changes intentionally alter tie-group choices, and knowledge/shot admission remain intact.

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
