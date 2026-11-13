"""BMP-2M Berezhok and BMP-3, from assets/references/bmp/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/bmp.py -- [--variant=<id>] [--wreck]

Each wears a scripted body (`SCRIPTED`, `scripted_body`): Gemini Argon's
build script after its second round, edited since (its folder's README
lists the edits). The BMP-3's script builds the BRM-3K too (`brm.py`). This
script adds the rigs, the whip antennas and the wrecks.

What the photos settle. BMP-2M (Engineering Technologies 2010, Army-2020):
six road wheels a side, three return rollers, the drive sprocket at the
front and the idler at the rear; the long, nearly flat ribbed glacis to a
high, sharp nose over a lower plate rising to it; flat sloped sides; the two
rear doors. The low Berezhok turret: the 30 mm 2A42, the AG-30 grenade
launcher on the right, a Kornet pair on each shoulder (the left one the
frame's `launcher`), sight boxes on the roof, smoke tubes either side.
BMP-3: its own longer hull with the engine at the rear, the sprocket at the
rear and the idler at the front, six road wheels; the high, blunt nose
under the folded trim vane; the turret with the 100 mm 2A70 and the 30 mm
2A72 beside it on the right (the frame's `autocannon`, riding the 2A70).

Built to the catalog frames (BMP-2M hull 6.735 x 3.15 x 2.15 m, the ring
0.1 m ahead of amidships; BMP-3 7.14 x 3.2 x 2.4 m, the ring 0.3 m ahead):
nothing here moves them.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# Each appearance's body (`scripted_body`): its script and the variant it
# builds, the guns' trunnions ahead of the ring (by mount), where the
# mantlets pitch, and the whips' bases on the turret's rear roof (turret
# frame). The track's link pitch.
SCRIPTED = {
    "eastern_bmp_ifv_family_bmp_2m_berezhok": dict(
        body="assets/source/skins/bmp/bmp2m-argon-r2/build.py", variant=None,
        trunnion={"autocannon": 0.50}, antenna=(-0.70, 0.50, 0.55)),
    "eastern_bmp_ifv_family_bmp_3": dict(
        body="assets/source/skins/bmp/bmp3-argon-r2/build.py", variant="bmp_3",
        trunnion={"main_gun": 0.70, "autocannon": 0.70}, antenna=(-0.75, 0.55, 0.88)),
}
LINK_PITCH = 0.15


def build(variant, v):
    look = SCRIPTED[variant["id"]]
    mounts = rig(v.frame, v.root, trunnion=look["trunnion"])
    turret = next(iter(mounts.values()))[0]
    SB.wear(v, look["body"], LINK_PITCH, variant=look["variant"])
    RF.whip_pair(turret, look["antenna"], v.mats, height=2.0)


def wreck(variant, v):
    SB.wreck(v, "R", (2, 4))


if __name__ == "__main__":
    run("bmp", "russian_green", build, wreck, chip=1.0)
