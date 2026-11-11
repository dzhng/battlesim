---
name: make-3d-models
description: Make or improve a unit's 3D model (a jet, vehicle, helicopter, prop) from its references, procedurally and optionally under a generated image-to-3D skin. Use when a model looks wrong, plain or lumpy against its photos or a target game's quality; when a new unit needs a model; or when choosing, feeding or cleaning up a 3D generator (Hunyuan3D, Meshy, Tripo, Rodin).
---

# Make 3D Models

A model is done when it reads as its real subject at game distance, holds up
close, and sits in the roster as one set: the same finish, level of detail and
scale as its siblings. Code owns proportions, fittings and the game's
contracts. A generator can add a smooth, blended surface over a shape the code
has already got right. It is never the source of proportions.

## The pieces

- A family's **roster script** builds what ships: every variant, tier and
  wreck.
- A **guide** is an airframe's measured, lofted model. Its renders make the
  skin, and it ships nowhere.
- A **skin** is the generated surface, pinned with what made it.
  - On a skinned unit the body comes from the skin, not the roster script.
  - A shape fix there goes into the guide, then a new skin, then a refit of
    the roster script around it.
- A family's **reference library** holds its photos and drawings with their
  licences. Record a drawing's crop and measured scale in its note so the
  next measurement reuses them.
- The scene-assets readmes own the commands: exporting a family, lint, the
  sheet, validating a wreck, bake and check. All run through
  `bun run --cwd web asset -- <command>`, whose usage text lists them. Read
  those readmes before the first export.

## The loop

Good results come from running this loop many times, not from a model or a
setting:

1. **Render it and look.** A script that ran proves nothing about how the
   model looks.
2. **Name the problem precisely.** "The tip missile sits 0.3 m ahead of its
   rail", not "the wing looks off". Measure it when you can.
3. **Fix it in code** (the family's script, its guide, a shared helper), so a
   rebuild keeps the fix. Never hand-edit an exported file.
4. **Check from a view that shows it.** A three-quarter thumbnail hides a
   floating part; a close, low view shows it at once.
5. **Get an outside opinion.** Whoever built a model stops seeing its flaws.

Before you finish, add any lasting lesson to the smells below, as the problem
and its tell.

## Workflow

Order is for building. Say what you find when you find it.

1. **References.** Collect photos from every side, and a three-view drawing if
   one exists, each with its licence. Look at them together, on one sheet,
   before modelling. Check for the right variant, every side covered, and
   nothing cropped. Anything may be measured, since dimensions are facts. Only
   inputs we may ship (public domain, CC0, our own renders) may feed a
   generator or ship as art; share-alike images stay review-only.
   *Done:* the reference library passes the asset check, and every view you
   will judge has a reference.
2. **Shape in code.** Look up the real overall dimensions rather than
   guessing. Measure the drawing at one scale, then overlay the model on each
   view with `scripts/drawing_overlay.py` (below) until the outlines agree.
   The procedural route is dimensions, then lofted sections, then fittings.
   *Done:* the remaining pink and green on every overlaid view is something
   you can name and accept (drawn gear, a sibling variant's detail).
3. **Surface, if lofting can't reach it.** Generate a skin from renders of the
   measured model, never from the drawing or one photo. Then register it
   against the model and strip the generator's paint. The scene-assets
   Blender readme owns this pipeline. Read
   [references/generators.md](references/generators.md) before your first
   paid call.
   *Done:* the skin's measured lengths agree with the model's, and it is
   pinned with its inputs, request and terms.
4. **Fit and finish.** Seat every fitting (gear, canopy, intakes, stores,
   pylons) on the surface it attaches to, and paint by material role. Then
   look at the unit beside its roster siblings.
   *Done:* nothing hovers or intersects in the close-ups, and the unit
   doesn't stand out in the lineup.
5. **Review.** Look at the asset sheet with its references, then
   `scripts/closeups.py` shaded and as a matcap. Then get an unprimed
   critique (`screenshot-critique`) against the target. Have it score
   silhouette, surface, fittings, materials and cohesion out of 10, and list
   at most five fixes. A 6 means a studio would take the model as a base to
   refine, judged at normal viewing distance. A finding is only what would
   stop that. Fix the findings; don't argue. Re-render, and critique again.
   *Done:* no score is below 6 and the average is 7 or more. Otherwise
   whatever is still wrong is named to the user.
6. **Ship.** *Done:* lint, the sheet, the wreck, the bake and the check are
   clean, and the generated output, the catalog budget and the LFS files are
   accounted for.

## Tools

Run both scripts on the repo's pinned Blender, with absolute paths:
`bun run --cwd web asset -- blender <script> <args>`. Each script's docstring
gives its arguments.

- **`scripts/drawing_overlay.py`** puts the model's silhouette on one cropped
  view of a three-view drawing, at the drawing's own scale (from the long axis
  you give it). It writes an image coloured green where only the model is,
  pink where only the drawing is, and grey where they agree. You give it the
  view's pixel box, read off the drawing, and which way its nose points. It
  also prints the scale and both outlines' widths at 19 stations along the
  length.
  - Read the stations to size a fix in metres.
  - Read the image to see where the fix goes.
  - Its last argument picks the parts compared. `aircraft` (the default)
    leaves gear, stores and pylons out of the model. `vehicle` keeps the
    running gear (wheels and track belts) and leaves every `dressing_*`
    subtree out, so the outline is the one the battle frame holds, and it
    stands side and front views on the ground line. Either way the
    drawing's own gear or antennas show in pink.
- **`scripts/closeups.py`** renders tier 0 of each model from six framed
  close views, or your own (`--shot`, given as fractions of the bounds so one
  list fits every model, or in metres for a camera that keeps its distance).
  Each view comes out shaded and as a matcap.
  - The matcap shows the surface alone: wrinkles, dents and undulation that
    paint hides.
  - The shaded view shows seams, floating parts and paint.

Batch your Blender work: a launch plus an import costs more than a render.
Start long jobs (generations take minutes) in the background. Review each
result as it lands, polling briefly, and keep test renders small.

## Smells

- **The skin inherits the model.** A boxy intake, a floating fin or a generic
  tube in the renders comes back in the skin. So does any fitting left on the
  model (a rail, a pod). Fix the model first. A new skin also moves everything
  measured on the old one.
- **Drawings lie at the edges.** Drawn gear and ground lines spoil a side
  profile, and a loose crop catches the neighbouring view. The drawing may
  also be of a sibling variant (a two-seater for a single-seater).
- **Floating parts.** Anything placed at a number instead of on the surface
  floats once the shape changes.
- **Ragged edges** come from cutting faces out of a decimated mesh, and
  decimating after a cut reshapes the cut. Flattening the region on each tier,
  or a frame over the seam, hides a boundary better.
- **Paint that looks blotchy** can be geometry: reflections off undulations,
  or a generator's own UVs. Turn texture channels off to tell which.
- **Cohesion breaks early.** Siblings drawn under different light, views or
  finish never match later. Make every unit's inputs the same way.
- **Thumbnails flatter.** Judge joints, edges, the canopy and the belly from
  close-ups, at the tier the player sees.
- **Fetching references.** Wikimedia refuses clients without a descriptive
  user agent, and serves its SVG drawings as PNG renders on request.
- **Silent budgets.** Triangles add up against the catalog's load limit. A
  failed commit hook can leave LFS files as pointers.

Blender scripting traps live in [references/blender.md](references/blender.md).
Read it when a script runs but its result looks wrong.
