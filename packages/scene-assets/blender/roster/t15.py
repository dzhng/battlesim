"""T-15 Armata heavy IFV, from assets/references/t15/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/t15.py -- [--wreck]

The T-15 wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's
build script after its second round, edited since (its folder's README lists
the edits). Its Kornet-EM pair is the `launcher` mount, drawn as the roof gun's
`hmg` and `hmg_gun`. This script adds the rig, the whip antennas and the wreck.

Built to the catalog frame (hull 9.5 x 3.5 x 3.2 m, module pivot 2.22 m on the troop compartment's roof): nothing here moves it.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from parts import empty  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The T-15's body (`scripted_body`) and its track's link pitch.
BODY = "assets/source/skins/t15/argon-r2/build.py"
LINK_PITCH = 0.16
# The whips' bases on the turret's rear roof, either side (turret frame).
ANTENNA = (-1.10, 0.80, 0.98)


def build(variant, v):
    turret = rig(v.frame, v.root)["autocannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    x, y, z = ANTENNA
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        RF.antenna(f"antenna_{s}", (x, side * y, z), v.mats, whip, height=2.0)


def wreck(variant, v):
    SB.wreck(v, "L", (3, 4))


if __name__ == "__main__":
    run("t15", "russian_green", build, wreck, chip=1.0)
