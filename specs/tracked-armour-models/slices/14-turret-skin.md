# 14: turret skin (coordinator; only if slice 03 picks the skin)

If slice 03's verdict is "current" or "smoothed", close this slice as "not
needed", point to the verdict in `decisions.md`, and skip it.

## Contract

The T-72 family's cast turret ships under the generated skin that won
slice 03. It is fitted to named axes, not to the aircraft's "highest point is
the fin" assumption.

## Seam

- **Fitting.** Generalise the skin fit so a caller names the axes and anchor:
  - the turret ring's centre at the mount pivot;
  - the gun's axis along +X.
  - Keep `aircraft_skin.py` (16,643 B) under the ratchet. If the turret
    fitting doesn't fit there, split by concept: the shared registration goes
    in `skin_register.py`, and the aircraft-only orientation stays put.
- **What the skin covers.** Only the turret dome. Code keeps:
  - the proportions;
  - the gun and mantlet;
  - the ERA;
  - the fittings.
- **Which models.** The T-72B3 always. The T-80BVM and T-90M only if their
  turrets share the dome; slice 17 decides that on the overlay.

## The variable and crop

The same as slice 03, on the shipped export.

## Verification

- **Fit:** the skin is seated on the ring, with no gap or overlap with the
  mantlet.
- **Lint.**
- **Closeups**, then **`compare-screenshots`** against slice 03's winning shot.
- **Last check: an unprimed `screenshot-critique`.**

## Delegated

The registration mechanics, within the seam above.
