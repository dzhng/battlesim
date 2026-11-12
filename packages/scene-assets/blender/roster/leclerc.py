"""Leclerc XLR, from assets/references/leclerc/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/leclerc.py -- [--variant=<id>] [--wreck]

What the photos settle: one XLR photo (Eurosatory 2022, front left) and
pre-XLR Leclercs for the shared hull, running gear and turret core. Six large
evenly spaced road wheels a side with eight-bolt hubs, the sprocket raised at
the rear and the idler raised at the front, thin skirts with a top band; a
shallow upper glacis, a short nose plate and a steep lower glacis, the lamp
clusters in boxes on the fenders' front corners and shackles on the lower
glacis. The turret is low and flat-faced, its armour modules either
side of the mantlet; the commander's panoramic sight tower on the left of the
roof with his cupola beside it, the gunner's sight box on the right front, the
long autoloader bustle behind, GALIX launchers along the bustle sides. XLR
(that one photo): stacked armour modules over the front of the hull sides,
bar armour over the rear and round the bustle, armour blocks on the turret
sides and the PL1 remote weapon station on the right of the roof.

Built to the catalog frame (hull 6.88 x 3.6 x 2.53 m, turret pivot 1.518 m,
cannon muzzle 6.43 m ahead): nothing here moves it.

It wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's build
script (its folder's README lists its rounds and edits). This script adds
its rig, whip antennas and wreck.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from parts import empty  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

BODY = "assets/source/skins/leclerc/argon-r1/build.py"
LINK_PITCH = 0.16
# The gun's trunnion, ahead of the ring along the bore: the script's.
TRUNNION = 1.30


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    # The whips on the bustle roof's rear corners (turret frame).
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        RF.antenna(f"antenna_{s}", (-2.4, side * 1.1, 0.862), v.mats, whip, height=2.2)


def wreck(variant, v):
    SB.wreck(v, "L", (2, 5), seed=6)


if __name__ == "__main__":
    run("leclerc", "french_three_tone", build, wreck, chip=1.0)
