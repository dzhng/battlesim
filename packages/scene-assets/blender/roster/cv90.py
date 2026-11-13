"""CV9040C and CV90 Mk IV, from assets/references/cv90/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/cv90.py -- [--variant=<id>] [--wreck]

Both wear one scripted body (`BODY`, `scripted_body`), the variant chosen by
`VARIANTS`: Gemini Argon's build script after its second round, edited since
(its folder's README lists the edits). This script adds the rig, the whip
antennas and the wreck.

What the photos settle: seven road wheels a side with no return rollers,
the track's top run lying on them, the drive sprocket at the front and the
idler at the rear; the nose plate under a long shallow glacis; tall flat
hull sides in bolted armour panels; the rear door. The turret is low and
angular. CV9040C (Swedish and Ukrainian vehicles): the 40 mm Bofors in a
perforated jacket, add-on armour on the hull and turret sides, the
commander's sight on the roof. Mk IV (Czech and Slovak vehicles): an
angular faceted turret with the 30 mm, the commander's panoramic sight on
its mast, a Spike launcher on the left.

Built to the catalog frames (hull 6.8 m long; the C 3.38 m wide and 2.5 m
to its roof armour, the Mk IV 3.2 m and 2.8 m to its roof fittings; turret
pivot 0.35 m behind the hull's middle and 1.82 m up): nothing here moves
them.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The body (`scripted_body`) and the variant it builds for each appearance;
# its track's link pitch; the autocannon's trunnion ahead of the ring, where
# the mantlet pitches.
BODY = "assets/source/skins/cv90/argon-r2/build.py"
VARIANTS = {"europe_cv90_cv9040c": "cv9040c", "europe_cv90_mk_iv": "mk_iv"}
LINK_PITCH = 0.15
TRUNNION = 0.90
# The whips' bases on the turret's rear roof (turret frame), by variant.
ANTENNA = {"cv9040c": (-1.10, 0.70, 0.66), "mk_iv": (-1.20, 0.70, 0.78)}


def build(variant, v):
    body = VARIANTS[variant["id"]]
    turret = rig(v.frame, v.root, trunnion={"autocannon": TRUNNION})["autocannon"][0]
    SB.wear(v, BODY, LINK_PITCH, variant=body)
    RF.whip_pair(turret, ANTENNA[body], v.mats, height=2.0)


def wreck(variant, v):
    SB.wreck(v, "R", (3, 5))


if __name__ == "__main__":
    run("cv90", "swedish_splinter", build, wreck, chip=1.0)
