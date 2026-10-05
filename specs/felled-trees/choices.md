# Felled trees: choices

Decisions made where the plan was silent. Consolidated at close.

- **Every toppling body gets a record, not only trees.** Walls and railings
  that topple also publish falls; the renderer draws trees only. One rule, no
  tree special case; later presentation can animate the others.
- **Digest folds the fall log only once non-empty.** Keeps every battle where
  nothing topples bit-identical to before. Battles where trees fall change
  digest (named decision).
- **A pass's fall direction is the shooter's line to the entry point.** The
  flight `Pass` event carries no velocity; the shooter's position is close
  enough for a fall direction and avoids widening the flight event.
- **Degenerate direction falls back to one of eight compass directions by prop
  id.** No trigonometry, so native and Wasm agree bit for bit.
- **Felled trees are a separate small pass, not tilt on the forest instances.**
  Tilt would break static far buffers, chunk bounds and the upright fog heart.
  The felled pass is unculled and repacks every frame while a tree falls;
  felled trees are few.
- **The tree hinges at the stump's top; landing kicks its foot two trunk radii
  along the fall and lowers it onto the ground, and its top rests on the
  ground.** So the stump stands bare beside the log, as the critique asked.
- **Only leaves (and bark above the crown's base) are pressed; the trunk keeps
  its girth.** The press is per tree, bounded where the crown actually lies
  (near the ground at the far end), so the crown keeps a little volume under
  `rest_height_m`. Pressing the whole tree at the hinge left a paper crown and
  a plank trunk.
- **`rest_height_m` is 1 m.** The trunks' own girth near the foot is ~0.9 m,
  so a lower bound could not hold without shrinking the trunk.
- **The stump is a prism at the trunk's radius at its height (interpolated
  between the mesh's foot ring and next ring), in the bark's mean colour, with
  a data cut colour.** The trunk meshes have no vertices between rings, so a
  height band found nothing.
- **What stood below the cut collapses onto it.** The stump stands there from
  the first frame instead of appearing on landing.
- **The fall starts at the start of its tick** (`(tick − 1) / tick_hz`), as
  combat effects do.
- **The consequences scene frames the felled tree from above.** Inside a wood
  anything lower is hidden by the neighbours' crowns.
