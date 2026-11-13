"""T-80BVM, from assets/references/t80/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/t80.py -- [--wreck]

The T-80BVM wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's
build script after its second round, edited since (its folder's README lists
the edits). This script adds the rig, the whip antennas and the wreck.

What the photos settle (Murmansk rehearsal, Army-2020 and 2022): a longer
hull than the T-72's, its six rubber-tyred road wheels smaller and evenly
spaced, five return rollers behind the side armour, the sprocket at the rear
and the idler at the front; the hull sides in two rows of Relikt panels over
the front two-thirds, the lower row's edge scalloped, slat panels over the
rear and a slat cage on the hull rear with fuel drums over it; the turbine
deck's broad grille and the exhaust louvre in the rear plate. The cast turret
carries the broad Relikt cheek kit, a band of flat panels round its front arc
continuing as modules down its flanks, Relikt tiles on the roof front, smoke
tubes, the gunner's sight on the left, the commander's cupola with its
machine gun on the right, and stowage boxes and a slat cage on the rear. The
commander rides head out.

Built to the catalog frame (hull 7.01 x 3.603 x 2.202 m, turret pivot 1.321 m, cannon muzzle 6.2 m ahead): nothing here moves it.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The T-80BVM's body (`scripted_body`) and its track's link pitch.
BODY = "assets/source/skins/t80/argon-r2/build.py"
LINK_PITCH = 0.15
# The 2A46M-4's trunnions, this far ahead of the turret ring along the bore.
TRUNNION = 0.95
# The whips' bases on the turret's rear roof, either side (turret frame).
ANTENNA = (-0.95, 0.65, 0.39)


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    RF.whip_pair(turret, ANTENNA, v.mats, height=2.2)


def wreck(variant, v):
    SB.wreck(v, "L", (3, 4))


if __name__ == "__main__":
    run("t80", "russian_green", build, wreck, chip=1.0)
