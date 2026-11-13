"""Challenger 3, from assets/references/challenger_3/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/challenger_3.py -- [--wreck]

No Challenger 3 photo is under an allowed licence, so the library is the
Challenger 2 TES photos of the hull it keeps and two generated views of the
new turret (labelled as such). What they settle: the Challenger 2's hull,
six road wheels a side with the sprocket at the rear, deep skirts in bolted
appliqué panels, a shallow upper glacis (the driver's hatch at its head)
over a short upright nose plate and a steep lower glacis, the
engine deck behind the turret; the new turret is angular and slab-sided,
its front a sloped wedge, the 120 mm L55A1 smoothbore without fume
extractor, a Trophy radar panel and launcher on each side, the commander's
panoramic sight on its right, a remote machine gun station, a slatted
bustle cage. British green.

Built to the catalog frame, the Challenger 2's hull (8.3 x 4.2 m, gun pivot
0.2 m ahead of the hull's middle and 1.494 m up) under a 2.70 m box that
the commander's sight sets: nothing here moves it.

It wears the Challengers' scripted body (`challenger.BODY`), its
`challenger_3` variant, and places the Trophy part's nodes on its housings.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import challenger as C  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import run  # noqa: E402

# The whips on the script's antenna bases (turret frame, left side).
WHIPS = (-2.08, 1.16, 1.486)
# The Trophy part's nodes (`trophy_aps`) on the script's housings (world, left
# side): the front radar panel's centre and the launcher head's.
TROPHY_NODES = {"radar": (1.68, 1.62, 2.08), "launcher": (-0.62, 1.50, 2.58)}


def build(variant, v):
    C.wear(v, "challenger_3", WHIPS)
    SB.trophy_nodes(TROPHY_NODES)


if __name__ == "__main__":
    run("challenger_3", "british_green", build, C.wreck, chip=1.0)
