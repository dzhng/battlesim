"""CV90120 light tank, from assets/references/cv90120/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/cv90120.py -- [--wreck]

The CV90120-T wears a scripted body (`BODY`, `scripted_body`): Gemini
Argon's build script after its second round, edited since (its folder's
README lists the edits). This script adds the rig, the whip antennas and the
wreck.

What the photos settle: the CV90 hull (seven road wheels with no return
rollers, the front drive, the long shallow glacis, bolted side armour, the
rear door; its side and rear are the CV90's own references,
`assets/references/cv90/`) under a low, wide wedge turret set just behind
the hull's middle, its faceted cheeks running out from a narrow gun shield
to nearly the hull's width, then flat sides and a deep bustle; the long
120 mm smoothbore in a sectioned thermal sleeve with a perforated muzzle
brake; the commander's sight ball at the left front of the roof, the
gunner's sight box on the right, a pintle machine gun by the gunner's hatch.
Swedish splinter scheme (the CV90 family's home army; the photos'
exhibition finishes are not followed).

Built to the catalog frame (hull 6.95 x 3.3 x 2.5 m, the turret ring 0.3 m
behind the hull's middle): nothing here moves it.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from parts import empty  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The body (`scripted_body`), its track's link pitch, and the gun's trunnion
# ahead of the ring, where the mantlet pitches.
BODY = "assets/source/skins/cv90120/argon-r2/build.py"
LINK_PITCH = 0.15
TRUNNION = 0.95
# The whips' bases on the bustle roof (turret frame).
ANTENNA = (-2.14, 0.88, 0.51)


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    x, y, z = ANTENNA
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        RF.antenna(f"antenna_{s}", (x, side * y, z), v.mats, whip, height=2.0)


def wreck(variant, v):
    SB.wreck(v, "R", (3, 5))


run("cv90120", "swedish_splinter", build, wreck, chip=1.0)
