"""Challenger 2 TES, from assets/references/challenger/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/challenger.py -- [--variant=<id>] [--wreck]

What the photos settle (TES "Megatron", Tankfest 2023, and the Op Telic TES):
six large road wheels a side on hydrogas arms, the drive sprocket raised at the
rear and the idler raised at the front; the side armour packs over the front
two-thirds of the hull and bar armour over the rear third and across the rear
plate; heavy armoured boxes on the nose corners over the tracks with the lamps
in them; a long sloped glacis with the driver's hatch at its head on the centre
line. The turret's front faces slope back steeply above the mantlet, its flat
cheeks angle in, five smoke tubes a side sit on the front corners, deep
stowage bins run along the bustle sides with bar armour round its rear; the
gunner's sight barbette on the right front roof, the commander's sight and
cupola behind it, the loader's hatch on the left, the remote weapon station
on the commander's side and the electronic countermeasures table on posts at
the rear of the roof.

Built to the catalog frame (hull 8.3 x 4.2 x 2.49 m, turret pivot 0.2 m ahead
of the hull's middle and 1.494 m up, cannon muzzle 7.2 m ahead): nothing
here moves it.

It wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's build
script after its second round, which builds the Challenger 3 too
(`challenger_3.py`), edited since (its folder's README lists the edits).
This script adds its rig, whip antennas and wreck.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from parts import empty  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The Challengers' body, which builds both (`build(variant)`), its tracks' link
# pitch and the gun's trunnion ahead of the ring along the bore: the script's.
BODY = "assets/source/skins/challenger/argon-r2/build.py"
LINK_PITCH = 0.17
TRUNNION = 1.90
# The whips stand on the script's antenna bases at the rear of the roof (turret frame).
WHIPS = (-2.04, 1.08, 1.286)


def wear(v, variant, whips):
    """The Challenger `variant` of `BODY` worn on `v`, its whips at `whips`
    (turret frame, left side); returns the turret's node."""
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH, variant=variant)
    x, y, z = whips
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        RF.antenna(f"antenna_{s}", (x, side * y, z), v.mats, whip, height=2.2)
    return turret


def build(variant, v):
    wear(v, "challenger_2_tes", WHIPS)


def wreck(variant, v):
    import bpy
    # The turret died traversed to the right: with the ring ahead of the
    # hull's middle, the long gun laid near the bow would reach past the
    # ground the wreck may cover (its footprint allowance).
    bpy.data.objects["turret"].rotation_euler.z -= math.radians(54.0)
    SB.wreck(v, "R", (3, 4), seed=4)


if __name__ == "__main__":
    run("challenger", "british_green", build, wreck, chip=1.0)
