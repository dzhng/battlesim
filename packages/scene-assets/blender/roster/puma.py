"""Puma (protection level C), from assets/references/puma/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/puma.py -- [--variant=<id>] [--wreck]

The Puma wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's
build script after its second round, edited since (its folder's README
lists the edits). This script adds the rig, the whip antenna and the wreck.

What the photos settle (Tag der Bundeswehr 2026, Augustdorf 2019): six road
wheels a side, three return rollers behind the armour, the drive sprocket
raised at the front and the idler at the rear; a sharp prow, its lower plate
and long upper glacis meeting in a chined nose; tall flat sides under the
level C armour modules; the rear in stowage cages round the ramp. The
unmanned turret is a low box: the 30 mm gun in its long perforated cradle,
the MELLS Spike launcher on its arm on the left (the frame's `launcher`,
riding the turret), the PERI commander's sight on its mast at the left rear,
smoke discharger banks and the MUSS sensors on the corners.

Built to the catalog frame (hull 7.4 x 4.0 x 3.3 m, turret pivot 0.45 m
behind the hull's middle and 2.55 m up, autocannon muzzle 4.1 m ahead,
launcher on the left at 1.08 m): nothing here moves it.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from parts import empty  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The body (`scripted_body`), its track's link pitch, and the cannon's
# trunnion ahead of the ring, where its cradle pitches.
BODY = "assets/source/skins/puma/argon-r2/build.py"
LINK_PITCH = 0.16
TRUNNION = 0.60
# The whip's base on the turret's right rear roof (turret frame).
ANTENNA = (-1.10, -0.65, 0.55)


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"autocannon": TRUNNION})["autocannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    whip = empty("dressing_antenna", parent=turret)
    RF.antenna("antenna", ANTENNA, v.mats, whip, height=2.0)


def wreck(variant, v):
    SB.wreck(v, "L", (2, 5))


run("puma", "german_three_tone", build, wreck, chip=1.0)
