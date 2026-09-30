# C37: shared house appearance

**Depends on:** C01, C13, C32/C14 and C22. **Kind:** slice.

## Question
Can all existing house instances use the template appearance owner with their original physical shape and look?

## Contract it unlocks
Use C01's original house descriptors and the existing house source to produce reusable template placements. Cut over every current `drawn_by: building` consumer, including village and labs, through C32/C22. Delete that appearance branch, its per-house bundle plumbing and any unreferenced bundles in the same pass. Keep source meshes still shared by the template/module owner. No surroundings or house dimensions change here.

This is the removal condition for C22's temporary coexistence with the house path. C34–C36 subsequently add surroundings; they cannot inherit an appearance branch whose remaining consumers were forgotten.

## API seam
Existing house appearance source and C01 physical descriptor → C13/C32 reusable placements → C22 chunks. One property/loader/render cutover across existing consumers; no compatibility alias.

## What the human can run or see
Matched village/lab house crops and a consumer inventory showing the old appearance path is unused and deleted.

## Verification
- Preserve original physical geometry, PropIds and terminal/side-knowledge behavior; name any presentation-schema/config identity change separately from physical outcomes.
- Every existing house consumer resolves supported intact/terminal templates before deleting the shared branch.
- Matched frames preserve the accepted house look and remain within frame/startup budgets.
- Judge house appearance parity only; town layout and surroundings are later slices. Use compare-screenshots against current house crops, run unprimed screenshot-critique last and preview-shots non-blocking.

## Delegated to the implementer
Source/module grouping within the accepted house fit and C32 codec. Geometry changes and retaining duplicate appearance owners are not delegated.

## Must stay green
Original physical outcomes, house visuals, learned damage timing and one template appearance path.

## Feedback that would change this slice
An appearance/fit regression reopens its reusable source mapping before surroundings migration.
