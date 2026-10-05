# Felled trees: choices

Decisions made where the request and plan were silent, checked against the
shipped code. Least confident first within each group.

## Worth your look

- **The lying height bound is 1 m, not lower.** You asked for the fallen tree to
  be flat so it never hides units. The crown is pressed to a low mat, but the
  trunk near its foot is ~0.9 m thick in the current tree art. Pressing the
  trunk too made it a floating plank, so the bound sits just above it.
  Lowering it means thinner trunk art or a pressed trunk.
- **The fallen crown reads as a low green mat, not a heap of branches.** That
  is what "flat and smaller" gives with the existing tree meshes. A shaped
  broken-crown look would need new art.
- **Battles where a tree falls changed digest.** The fall log is new battle
  state. It folds into the digest only once something has fallen, so every
  battle without a fall keeps its digest; the publication parity streams keep
  every digest, though their published bytes changed with the new group and
  header word. Saved replays store no battle digest; whether an old one loads
  is decided by the engine build identity, as for any engine change.
- **Every toppling body gets a fall record, not only trees.** Walls and
  railings that topple publish falls too (shelled or pushed); nothing draws
  them yet. One rule rather than a tree special case; the cost is records
  nobody draws.

## Settled

- **A forest tree leaves the drawn forest only by its fall record.** It no
  longer also leaves by seen-cleared ground, so it can never vanish without a
  stump. Shrubs and the forest-floor dressing still follow cleared ground. The
  dressing is laid again when a fall arrives, since a wood has no understorey
  and falls are what clear its ground.
- **Felled trees have their own small drawing pass.** Tilting the forest's
  instances would break its static buffers, chunk bounds and fog probe. The
  pass is not culled and repacks every frame while a tree falls; felled trees
  are few.
- **The tree hinges at its stump's top; on landing its foot kicks two trunk
  radii along the fall onto the ground, and its top rests on the ground.** So
  the stump stands bare beside the log.
- **What stood below the cut closes onto it while the tree falls.** The stump
  is visible from the first frame rather than appearing on landing.
- **The stump is cut from the species' own trunk:** a prism at the trunk's
  radius at stump height (between the mesh's foot ring and next ring), in its
  bark's mean colour, with a dull cut-wood face from data. Every species gets a
  matching stump with no new art.
- **A round passing through a body pushes it along its own flight.** The
  flight's pass event carries the round's direction, as a hit carries its
  velocity.
- **A burst at a tree's very foot gives no direction, so the tree falls toward
  one of eight compass points chosen by its id.** No trigonometry, so the
  browser and native builds agree exactly.
- **A fall starts at the start of the tick it happened in**, as combat effects
  do.
- **The browser check frames the felled tree from above.** Inside a wood, any
  lower camera sees only the neighbours' crowns.
