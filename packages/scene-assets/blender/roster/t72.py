"""T-72B3 (mod. 2016), from assets/references/t72/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/t72.py -- [--variant=<id>] [--wreck]

The T-72B3 wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's
build script after its second round, edited since (its folder's README lists
the edits). This script adds the rig, the whip antennas and the wreck.

What the photos settle (Alabino 2017, Moscow 2018): six large road wheels a
side with ribbed discs, three small return rollers mostly hidden, the
sprocket at the rear and the idler at the front; a steep glacis in rows of
explosive reactive armour under a V splash board; the hull sides in two rows
of bolted Relikt panels over the front two-thirds above a scalloped rubber
skirt, slat panels over the rear third and a slat cage round the engine deck.
The cast dome turret wears a fan of sloped Relikt blocks round each front
cheek, a row of rounded side modules wrapping its flanks, two rows of smaller
blocks on the roof front, four smoke tubes a side, the gunner's Sosna-U sight
box on the left, the commander's cupola with its heavy machine gun on the
right, a stowage box and snorkel on the rear and a slat frame round the
bustle.

Built to the catalog frame (hull 6.86 x 3.59 x 2.23 m, turret pivot 1.338 m,
cannon muzzle 6.1 m ahead): nothing here moves it.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The T-72B3's body (`scripted_body`) and its track's link pitch.
BODY = "assets/source/skins/t72/argon-r2/build.py"
LINK_PITCH = 0.15
# The 2A46M-5's trunnions, this far ahead of the turret ring along the bore.
TRUNNION = 0.95
# The whips' bases on the turret's rear roof, either side (turret frame).
ANTENNA = (-1.05, 0.70, 0.72)


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    RF.whip_pair(turret, ANTENNA, v.mats, height=2.2)


def wreck(variant, v):
    SB.wreck(v, "L", (3, 4))


if __name__ == "__main__":
    run("t72", "russian_green", build, wreck, chip=1.0)
