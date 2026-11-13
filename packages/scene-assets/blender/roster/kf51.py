"""KF51 Panther (2022 prototype), from assets/references/kf51/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/kf51.py -- [--variant=<id>] [--wreck]

What the photos settle: two licensable photos of the prototype (front, and
front right in motion) and the KF-51U at Eurosatory 2024 for the hull, which
the U shares. The Leopard 2 running gear: seven dual road wheels a side,
four return rollers behind the armour, the sprocket raised at the rear and
the idler at the front. The turret is the long faceted one, its arrowhead
front running out over the glacis, and the 130 mm gun a thick tube with a
heavy muzzle.

Built to the catalog frame (hull 7.8 x 3.7 x 2.5 m, turret pivot 0.3 m ahead
of the hull's middle and 1.5 m up, as on the Leopard 2 hull it shares, cannon
muzzle 6.1 m ahead): nothing here moves it.

It wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's build
script after its second round, edited since (its folder's README lists the
edits). This script adds its rig, whip antennas and wreck.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

BODY = "assets/source/skins/kf51/argon-r2/build.py"
LINK_PITCH = 0.18
# The gun's trunnion, ahead of the ring along the bore: the script's.
TRUNNION = 1.20


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    # The whips on the bustle roof's rear corners (turret frame).
    RF.whip_pair(turret, (-2.9, 1.2, 1.0), v.mats, height=2.2)


def wreck(variant, v):
    import bpy
    # The turret died traversed a little left of the bow: with the ring ahead
    # of the hull's middle, the 130 mm laid dead ahead would reach past the
    # ground the wreck may cover (its footprint allowance).
    bpy.data.objects["turret"].rotation_euler.z += math.radians(5.0)
    SB.wreck(v, "R", (3, 4), seed=3)


if __name__ == "__main__":
    run("kf51", "german_three_tone", build, wreck, chip=1.0)
