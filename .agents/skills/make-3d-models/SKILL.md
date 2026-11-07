---
name: make-3d-models
description: Make or improve a unit's 3D model (a jet, vehicle, helicopter) from its references, procedurally and optionally under a generated skin. Use when a model looks wrong or plain against its photos or a target game's quality, or when a new unit needs one.
---

# Make 3D Models

Code owns proportions, fittings and the game's contracts; a generator can
add a smooth, blended surface over a shape the code has already got right.
Measure, don't eyeball, and look at every render.

## Workflow

1. Gather references from every side, and a three-view drawing if one
   exists, each with its licence. Measuring any drawing is fine (dimensions
   are facts), but only inputs we may ship (public domain, CC0, our own
   renders) may go into a generator; share-alike images stay review-only.
2. Get the shape right in code first: measure the drawing at one scale and
   overlay the model on it (`scripts/drawing_overlay.py`) until the outlines
   agree.
3. If lofting can't give the surface quality wanted, generate a skin from
   renders of that measured model, register it against the model, and seat
   everything else on it. The scene-assets Blender README says how.
4. Look: the sheet, close-ups of every joint, edge and canopy, and a shiny
   matcap for the surface. Then get an unprimed critique against the target.
5. Done when lint, the sheet, the wreck, the bake and the check are clean and
   every high-confidence critique finding is fixed or named to the user.

## Smells

- **Generators guess what they can't see.** One photo means wrong
  proportions. Multi-view generators scale each view to fill it, so mixing a
  nose-on view with a side view reads span as length.
- **Crumpled surface.** Wrinkles at the scale of a panel show under a matcap,
  and decimating or smoothing doesn't remove them. Try another generator.
- **The skin inherits the guide.** A boxy intake, a floating fin or a
  generic tube in the renders comes back in the skin, and so does any
  fitting left on the guide (a rail, a pod); fix the model first.
  A new skin also moves everything measured on the old one.
- **Drawings lie at the edges.** Drawn gear and ground lines spoil a side
  profile, a loose crop catches the neighbouring view, and the drawing may
  be a sibling variant (a two-seater for a single-seater).
- **Symmetry and fusing.** One-sided details get mirrored, and neighbours
  fuse (rails into tips, two nozzles from one). Expect it and check.
- **Paint that looks blotchy** can be geometry: reflections off undulations,
  or a generator's own UVs. Turn texture channels off to tell which.
- **Ragged edges** come from cutting faces out of a decimated mesh.
  Flattening, or a frame over the seam, hides a boundary better.
- **Floating parts.** Anything placed at a number instead of on the surface
  floats once the shape changes.
- **Fetching references.** Wikimedia refuses clients without a descriptive
  user agent, and serves its SVG drawings as PNG renders on request.
- **Silent budgets.** Triangles add up against the catalog's load limit, and
  a failed commit hook can leave LFS files as pointers.
