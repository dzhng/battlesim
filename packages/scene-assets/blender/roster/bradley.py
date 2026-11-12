"""M2A4 Bradley IFV and M3A3 Bradley CFV, from assets/references/bradley/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/bradley.py -- [--variant=<id>] [--wreck]

Both wear one scripted body (`BODY`, `scripted_body`), the variant chosen by
`VARIANTS`: Gemini Argon's build script after its second round, edited since
(its folder's README lists the edits). This script adds the rig, the whip
antennas and the wreck.

What the photos settle: six dual road wheels a side, three return rollers,
the drive sprocket at the front and the idler at the rear; the upper hull
sides in bolted applique tiles over a skirt of panels; a steep glacis with
the headlight clusters on its corners and the folded trim vane across it;
the big rear ramp. The turret is a squared box offset right: the 25 mm
Bushmaster in its mantlet, the twin TOW launcher box on its left side (the
frame's `launcher`, riding the turret), the commander's independent viewer
at the rear, stowage baskets round the bustle. The M2A4 keeps its firing
port plates on the rear hull sides; the M3A3 carries more stowage.

Paint: US desert tan (`us_desert_tan`), as the M3A3 photos and the Abrams.

Built to the catalog frame (hull 6.55 x 3.6 x 2.76 m; turret pivot 0.5 m
behind the hull's middle and 0.2 m right of it, 1.937 m up; autocannon
muzzle 2.5 m ahead, launcher 0.972 m left of the pivot): nothing here moves
it.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from parts import empty  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The body (`scripted_body`) and the variant it builds for each appearance;
# its track's link pitch; the Bushmaster's trunnion ahead of the ring, where
# the mantlet pitches.
BODY = "assets/source/skins/bradley/argon-r2/build.py"
VARIANTS = {"us_m2_bradley_ifv_m2a4": "m2a4", "us_m3_bradley_cfv_m3a3": "m3a3"}
LINK_PITCH = 0.15
TRUNNION = 0.80
# The whips' bases on the turret's rear roof (turret frame).
ANTENNA = (-1.15, 0.70, 0.62)


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"autocannon": TRUNNION})["autocannon"][0]
    SB.wear(v, BODY, LINK_PITCH, variant=VARIANTS[variant["id"]])
    x, y, z = ANTENNA
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        RF.antenna(f"antenna_{s}", (x, side * y, z), v.mats, whip, height=2.0)


def wreck(variant, v):
    SB.wreck(v, "L", (2, 5))


run("bradley", "us_desert_tan", build, wreck, chip=1.0)
