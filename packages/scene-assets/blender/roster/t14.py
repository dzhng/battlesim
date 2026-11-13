"""T-14 Armata, from assets/references/t14/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/t14.py -- [--wreck]

The T-14 wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's
build script after its second round, edited since (its folder's README lists
the edits). This script adds the rig, the whip antennas and the wreck.

What the photos settle: engine at the rear. The crew capsule's three hatches
sit in a row on the deck ahead of the turret; the unmanned turret is a low,
wide faceted shell, half the hull's length, with an arrowhead front and a
long overhanging bustle, the gunner's sight box on its right front, the
commander's panoramic sight on a mast over its left, a remote Kord on the
roof, the Afganit launcher racks low on each flank, a slatted cage round the
bustle. The long 125 mm 2A82-1M, its muzzle about 2.1 m past the bow, has a
thermal sleeve and no fume extractor.

Built to the catalog frame (hull 8.7 x 3.5 x 2.8 m, gun pivot 1.8 m): nothing here moves it.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The T-14's body (`scripted_body`) and its track's link pitch.
BODY = "assets/source/skins/t14/argon-r2/build.py"
LINK_PITCH = 0.16
# The 2A82-1M's trunnions, this far ahead of the turret ring along the bore.
TRUNNION = 1.05
# The whips' bases on the turret's rear roof, either side (turret frame).
ANTENNA = (-2.20, 0.95, 0.91)


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    RF.whip_pair(turret, ANTENNA, v.mats, height=2.0)


def wreck(variant, v):
    SB.wreck(v, "L", (3, 4))


if __name__ == "__main__":
    run("t14", "russian_green", build, wreck, chip=1.0)
