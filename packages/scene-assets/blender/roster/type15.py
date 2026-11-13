"""ZTQ-15 (Type 15) light tank, from assets/references/type15/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/type15.py -- [--wreck]

The ZTQ-15 wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's
build script after its second round, edited since (its folder's README lists
the edits). This script adds the rig, the whip antennas and the wreck.

What the photos settle (display and VT-5 export photos): six road wheels a
side on hydropneumatic arms, the sprocket at the rear and the idler at the
front, no return rollers showing under the skirts; a short sharp glacis to a
flat deck, the driver's hatch on the left of it; side skirts in sections. The
welded turret is boxed in modular armour, a wedge of blocks on each cheek and
a deep bustle box behind, a slatted rack round its rear; the 105 mm gun has a
thermal sleeve and a fume extractor; the commander's panoramic sight on the
right rear roof, the remote heavy machine gun station beside it, four smoke
tubes a side. Its paint here is the PLA's digital woodland, one scheme a nation.

Built to the catalog frame (hull 7.3 x 3.35 x 2.5 m, gun pivot 1.68 m, the remote HMG on the turret roof at 2.44 m): nothing here moves it.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The ZTQ-15's body (`scripted_body`) and its track's link pitch.
BODY = "assets/source/skins/type15/argon-r2/build.py"
LINK_PITCH = 0.15
# The 105 mm gun's trunnions, this far ahead of the turret ring along the bore.
TRUNNION = 0.85
# The whips' bases on the turret's rear roof, either side (turret frame).
ANTENNA = (-1.60, 0.85, 0.80)


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    RF.whip_pair(turret, ANTENNA, v.mats, height=2.0)


def wreck(variant, v):
    SB.wreck(v, "L", (3, 4))


if __name__ == "__main__":
    run("type15", "chinese_digital", build, wreck, chip=1.0)
