"""ZTZ-99A, from assets/references/type99/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/type99.py -- [--wreck]

The ZTZ-99A wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's
build script after its second round, edited since (its folder's README lists
the edits). This script adds the rig, the whip antennas and the wreck.

What the photos settle (Military Museum Beijing 2017, the 2015 parade): six
large road wheels a side, the sprocket at the rear and the idler at the
front; heavy side armour panels from nose to tail, their lower edges cut in V
scallops between the wheels; rounded front fenders either side of a glacis in
reactive armour blocks, a lamp in a box on the left fender. The turret is the
arrowhead: wedge armour either side of the gun, flat sides, a bank of five
smoke tubes on each side behind the wedge, a long bustle with a cylindrical
stowage container across its rear, the gunner's sight on the left front, the
commander's independent sight on a pedestal at the right rear and the machine
gun by his hatch. The rear is from the earlier Type 99 (gap); the hull rear
is assumed the same.

Built to the catalog frame (hull 7.6 x 3.7 x 2.5 m, turret pivot 1.41 m, cannon muzzle 7.2 m ahead): nothing here moves it.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The ZTZ-99A's body (`scripted_body`) and its track's link pitch.
BODY = "assets/source/skins/type99/argon-r2/build.py"
LINK_PITCH = 0.16
# The ZPT-98's trunnions, this far ahead of the turret ring along the bore.
TRUNNION = 1.05
# The whips' bases on the turret's rear roof, either side (turret frame).
ANTENNA = (-1.90, 1.00, 0.86)


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    RF.whip_pair(turret, ANTENNA, v.mats, height=2.4)


def wreck(variant, v):
    SB.wreck(v, "L", (3, 4))


if __name__ == "__main__":
    run("type99", "chinese_digital", build, wreck, chip=1.0)
