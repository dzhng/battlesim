# Parallel unit-model production contract

This is a requirement for future implementation, not authorization to launch
model production during the unknowns walk. The [roster spec](README.md) and
[starter balance](starter-balance.md) define identities and provisional gameplay
data. The existing [scene-assets owner](../../packages/scene-assets/README.md)
owns source, bake, validation, appearance bindings and icons.

## One family per assignment, one owner per shared platform

Use independent model subagents in a bounded, continuously refilled worker pool.
With the current four total agent slots, use one coordinator and up to three
model workers; increase the pool only when the harness supports it. A worker owns
a platform family and its variants, never arbitrary edits throughout the catalog.
Group large families into smaller assignments only at explicit variant boundaries.
Shared U.S./European platforms get one assignment and reused bindings, not two
competing model versions. Infantry reuse the shared rig and equipment components;
do not build nationality variants the roster intentionally removed.

Initial assignments cover ground reconnaissance, infantry/equipment, vehicles
and the three supply trucks. Future air/drone/non-resupply support assignments
remain deferred with their mechanics; their cards may use clear placeholders.

## Freeze the assignment before dispatch

The coordinator prepares a task manifest containing:

- Canonical platform/variant IDs and faction memberships; exact display names.
- Reference images and source provenance, recognizable silhouette and equipment.
- Physical hull dimensions, ground origin, eye height and collision fit; reference
  facts are distinct from gameplay numbers. A squad uses the existing soldier frame.
- Engine-space convention: meters, +X forward, +Y left, +Z up, origin at ground.
- Every mount's name, parent, pivot, muzzle, articulation limits and required model
  nodes, including Trophy hardware and carried/active infantry weapons.
- Infantry skeleton, clip and socket contracts from existing scene-assets owners.
- Exclusive source/export/binding paths and which files only the coordinator edits.
- Existing per-bundle geometry/texture/rig/fit budget gates; no invented alternate
  thresholds and no tolerance changes made by a worker merely to get a pass.
- Variants that may share geometry and the hardware that must remain visibly
  different. Separate variant model identity does not require duplicated geometry.

Platform dimensions and mount measurements are **OPEN research work** until the
coordinator freezes these manifests. Unblock with platform references and existing
physical-fit admission before dispatch. A worker must report a bad contract rather
than resize physics, shift weapon positions or edit a shared rig independently.

## Worker output and integration

Each worker returns reproducible source, exported model, provenance, variant-node
manifest and local validation evidence, using its assigned paths. Scratch renders
and logs go in ignored `throwaway/`, never in this spec. Share installed dependencies;
do not share build output between checkouts with different sources. Keep shared
helper changes coordinator-owned to avoid silently changing other models.

The coordinator integrates identities and bindings, performs source/bake checks,
derives unit icons from models, verifies physical/mount fit and judges variants in
the actual battle renderer. Use the repository's game-UI/renderer workflow and
unprimed screenshot critique, reference comparison and Preview presentation for
visual work. A Blender preview alone is not proof of in-game fit or readability.

Do not mark a named ground unit finished while it still uses an interchangeable
generic tank/jeep model. Placeholder future units stay disabled. At spec closeout,
catalog and assets become authoritative; retain rationale and links instead of a
second finished-model inventory in the spec.
