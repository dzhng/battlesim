# 12 — Parallel family models, variants and runtime appearance admission

Status: authored runtime integration complete; visual acceptance pending. Dependencies: 03; integrate Trophy families after 10. Follow the README handoff and repository
skills before editing. This slice is a committed checkpoint, not a stopping point.

## Contract and scope

Follow model-production.md using a continuously refilled coordinator+three-worker pool at current concurrency. Begin independent families as soon as their manifests freeze; do not serialize all models behind gameplay. Workers only edit exclusive family source/export paths; coordinator owns shared helpers, rig contracts, binding catalogs and integration. Shared platforms get one assignment. Reuse soldier rigs and equipment, never nationality filler.

Each family has separate review passes: silhouette first, then visible equipment/mount fit, then material/variant readability. Keep physics/manifests fixed; workers report contradictions rather than move muzzles or relax budgets. Return reproducible source, exported model, variant/node manifest, pinned references and local validation. Coordinator bakes, derives icons, checks joints/pivots/muzzles/equipment and installs appearance bindings.

All admitted first-phase variants require correct named art before completion; generic tank/jeep placeholders are temporary. Future unsupported entries keep placeholder cards/models and stay disabled. Do not fetch every future model at startup merely because its card exists. Scratch renders/logs live under ignored throwaway; no new spec/source dump. Use existing asset/model workbench, not another authoring application.

## API seam and ownership

Frozen family manifest → source/GLB/node bindings → admitted appearance bundle and model-derived icon. Existing scene-assets resolver/loader owns the runtime generation. Each family is a bounded sub-pass with exclusive output ownership.

## Narrow verification

Existing source/bake/physical-fit/rig/geometry/texture budget gates per family. Verify asymmetric orientation, reachable articulated poses and visible upgrade hardware in the actual renderer. Match shared variants once across memberships. Final available catalog has no unfinished generic named-platform appearance; unavailable placeholders do not fail playable asset admission.


For every visual shot produced in this slice: load game-ui and renderer as
applicable; compare candidate against its declared crop/reference and pre-change
baseline with compare-screenshots. Show the real artifact with preview-shots.
Run an **unprimed screenshot-critique as the last visual check before acceptance**.
The human checkpoint is non-blocking: leave a short (~5 minute) opportunity to
correct reversible choices while continuing independent work, decide from evidence
if silent, record rationale and close opened Preview shots. Never infer permission
for a new mechanic from silence. GPU jobs serialize under the existing harness.


## Review surface and verdict

Existing model workbench and actual battle renderer. Compare a family silhouette crop to its references in pass one, mount/equipment crop in pass two and integrated variant view in pass three. Treat unrelated map/HUD polish as out of scope. Verdict is per-family in-game readability/fit, never Blender-only approval.

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


Runtime production is baked and checked: 216 content-addressed files, 72 bound
appearance entries, 60 infantry sources with exact physical inheritance and grounded
launcher socket validation. GPU ghost probes pass after fog composition. Representative
model sheets and fresh unprimed review remain before final acceptance.
