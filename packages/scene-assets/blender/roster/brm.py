"""BRM-3K Rys reconnaissance vehicle, from assets/references/brm/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/brm.py -- [--wreck]

The BRM-3K is built on the BMP-3 hull, so it wears the BMP-3's scripted body
as its `brm_3k` variant (`BODY`, `scripted_body`): Gemini Argon's build
script after its second round, edited since (its folder's README lists the
edits). This script adds the rig, the whip antennas and the wreck.

What the photos settle (Russia Arms Expo 2013, the Rubtsovsk plant's
monument): the BMP-3 hull, six road wheels a side, the drive sprocket at the
rear and the idler at the front; the long low glacis with the folded trim
vane; the two-man turret with the 30 mm 2A72 on the centre line, a sight box
each side of the gun, smoke tubes on the cheeks and the 1RL-133 radar's
folded housing on the turret's rear. Russian green (the 2013 car's desert
scheme was an exhibition finish).

Built to the catalog frame, the BMP-3 hull's (7.14 x 3.2 x 2.4 m, gun pivot
1.56 m up and 0.3 m ahead of amidships): nothing here moves it.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import bmp  # noqa: E402
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The body (`scripted_body`, the BMP-3's), its track's link pitch, and the
# cannon's trunnion ahead of the ring, where its mantlet pitches.
BODY = bmp.SCRIPTED["eastern_bmp_ifv_family_bmp_3"]["body"]
LINK_PITCH = bmp.LINK_PITCH
TRUNNION = 0.60
# The whips' bases on the turret's rear roof (turret frame).
ANTENNA = (-0.95, 0.62, 0.85)


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"autocannon": TRUNNION})["autocannon"][0]
    SB.wear(v, BODY, LINK_PITCH, variant="brm_3k")
    RF.whip_pair(turret, ANTENNA, v.mats, height=2.2)


def wreck(variant, v):
    SB.wreck(v, "L", (1, 4))


if __name__ == "__main__":
    run("brm", "russian_green", build, wreck, chip=1.0)
