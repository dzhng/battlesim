"""T-90M Proryv-3, from assets/references/t90/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/t90.py -- [--wreck]

The T-90M wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's
build script after its second round, edited since (its folder's README lists
the edits). This script adds the rig, the whip antennas and the wreck.

What the photos settle (Army-2020 to 2023): the T-72-derived hull and running
gear (six ribbed road wheels, three return rollers, Relikt panels in two rows
over the front of the hull sides above a scalloped rubber skirt, slat panels
over the rear), Relikt in rows on the glacis, and two external fuel drums
across the hull rear over a slat panel. The welded turret is compact and
angular, standing clear of the hull over a shadowed ring: Relikt wedges
pointing forward either side of the gun, its flat sides hung with net
screens, a short bustle in a net cage leaving the engine deck bare behind it,
smoke dischargers on the front roof corners, the gunner's Sosna-U box on the
left, the commander's tall panoramic sight on the right and the remote
machine gun beside it.

Built to the catalog frame (hull 6.86 x 3.78 x 2.22 m, turret pivot 1.332 m, cannon muzzle 6.2 m ahead): nothing here moves it.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The T-90M's body (`scripted_body`) and its track's link pitch.
BODY = "assets/source/skins/t90/argon-r2/build.py"
LINK_PITCH = 0.15
# The 2A46M-5's trunnions, this far ahead of the turret ring along the bore.
TRUNNION = 0.95
# The whips' bases on the turret's rear roof, either side (turret frame).
ANTENNA = (-1.60, 0.95, 0.75)


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    RF.whip_pair(turret, ANTENNA, v.mats, height=2.2)


def wreck(variant, v):
    SB.wreck(v, "L", (3, 4))


if __name__ == "__main__":
    run("t90", "russian_green", build, wreck, chip=1.0)
