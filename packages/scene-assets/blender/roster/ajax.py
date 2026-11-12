"""Ajax tracked reconnaissance vehicle, from assets/references/ajax/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/ajax.py -- [--variant=<id>] [--wreck]

The Ajax wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's
build script after its second round, edited since (its folder's README
lists the edits). This script adds the rig, the whip antennas and the wreck.

What the photos settle (Ajax at Bovington; the Ares and an Ajax-family
vehicle for the hull front and rear): seven road wheels a side, three return
rollers, the drive sprocket at the front and the idler at the rear; a long
boxy hull, its flat glacis carrying the driver's hatch and periscope block,
the lamp clusters set into the nose corners; the sides in big flat armour
modules, large stowage bins on the rear quarters. The turret carries the
40 mm CTA cannon in its mantlet, smoke discharger clusters on the front
corners, the remote weapon station on the right front (the frame's `HMG`,
riding the turret) and the commander's ORION sight on the right rear.

Built to the catalog frame (hull 7.62 x 3.35 x 2.7 m, turret pivot 0.4 m
behind the hull's middle and 1.95 m up, autocannon muzzle 2.65 m ahead):
nothing here moves it.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from parts import empty  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The body (`scripted_body`), its track's link pitch, and the cannon's
# trunnion ahead of the ring, where the mantlet pitches.
BODY = "assets/source/skins/ajax/argon-r2/build.py"
LINK_PITCH = 0.15
TRUNNION = 0.80
# The whips' bases on the turret's rear roof (turret frame).
ANTENNA = (-1.05, 0.70, 0.62)


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"autocannon": TRUNNION})["autocannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    x, y, z = ANTENNA
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        RF.antenna(f"antenna_{s}", (x, side * y, z), v.mats, whip, height=2.2)


def wreck(variant, v):
    SB.wreck(v, "R", (3, 6))


run("ajax", "british_green", build, wreck, chip=1.0)
