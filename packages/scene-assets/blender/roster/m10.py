"""M10 Booker light tank, from assets/references/m10/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/m10.py -- [--wreck]

What the photos settle (unveiling, C-17 loading, Yuma and Arctic testing):
six road wheels a side, the drive sprocket raised at the front and the idler
at the rear, side skirts in bolted panels over the top run; a tall blunt nose,
a long flat deck over the engine at the front; the turret set back, long and
low, with a blunt faceted front, upright slab sides and a long bustle box;
the commander's panoramic sight dome on the left rear; the 105 mm with a
bore evacuator mid-barrel. US desert tan (the Yuma photos; the unveiling
vehicles are green).

Built to the catalog frame (hull 7.0 x 3.4 m, box top 2.6 m with the sight
dome over it, gun axis 2.10 m up, turret pivot 0.3 m behind the hull's
middle): nothing here moves it.

It wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's build
script after its second round, edited since (its folder's README lists the
edits). This script adds its rig, whip antennas and wreck.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

BODY = "assets/source/skins/m10/argon-r2/build.py"
LINK_PITCH = 0.15
# The gun's trunnion, ahead of the ring along the bore: the script's.
TRUNNION = 1.00


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    # The whips on the bustle roof's rear corners (turret frame).
    RF.whip_pair(turret, (-1.7, 1.1, 0.705), v.mats, height=2.0)


def wreck(variant, v):
    SB.wreck(v, "L", (2, 4), seed=2)


if __name__ == "__main__":
    run("m10", "us_desert_tan", build, wreck, chip=1.0)
