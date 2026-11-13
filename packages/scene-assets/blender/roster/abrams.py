"""M1A2 Abrams SEPv2, SEPv2 Trophy and SEPv3 Trophy, from assets/references/abrams/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/abrams.py -- [--variant=<id>] [--wreck]

What the photos settle: seven dual road wheels a side on torsion arms, a rear
sprocket and front idler both raised, two return rollers hidden by the
skirts; skirts in eight panels, the first a long wedge over the idler and the
last cut up over the sprocket; a long shallow glacis with the driver's hatch
and three periscopes at its head; the rear plate one wide exhaust grille
between two smaller ones, tail lights in guards at its corners; the turret's
flat, swept-back cheeks either side of a recessed gun shield, a squared
bustle with its rack and side stowage bins, six-tube smoke banks ahead of the
bins, the GPS doghouse on the right front roof, the CITV on the left, the
commander under a CROWS on the right and the loader's hatch on the left.
SEPv3: the auxiliary power unit's armoured box on the left rear deck, CROWS-LP
and the ammunition data link over the muzzle. Trophy: a radar and launcher
housing along each side of the bustle in place of the side bin, standing
proud of the roof, its launcher head on top (from the SEPv2 Trophy photos;
no licensable photo shows a SEPv3 with it).

Built to the catalog frame (hull 7.93 x 3.66 x 2.44 m; turret pivot 0.25 m
ahead of the hull's middle, 1.464 m up, as the side photos show; cannon muzzle
5.555 m ahead of it): nothing here moves it.

All three wear one scripted body (`BODY`, `scripted_body`): Gemini Argon's
SEPv2 build script after its second round, with the Trophy and SEPv3 added
as its variants in a third session, edited since (its folder's README lists
the edits). This script adds its rig, Trophy nodes, whip antennas and wreck.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The scripted body every variant wears, each variant's id in it, its
# track's link pitch, and the gun's trunnion ahead of the ring along the bore.
BODY = "assets/source/skins/abrams/argon-variants-r1/build.py"
BODIES = {"us_m1_abrams_sep_v2": "sep_v2", "us_m1_abrams_sep_v2_trophy": "sep_v2_trophy",
          "us_m1_abrams_sep_v3_trophy": "sep_v3_trophy"}
LINK_PITCH = 0.19
TRUNNION = 1.55
# The Trophy part's nodes (`trophy_aps`), on the script's housings (world,
# left side): each forward radar panel's centre and each launcher head's.
TROPHY_NODES = {"radar": (-0.62, 1.925, 2.30), "launcher": (-0.70, 1.72, 2.645)}
# The long 120 mm droops further than the burn's sag, down towards the
# glacis, so the heaved turret's gun stays over the wreck's ground.
GUN_DROOP_DEG = 10.0


def build(variant, v):
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH, variant=BODIES[variant["id"]])
    SB.trophy_nodes(TROPHY_NODES)
    # The whips on the bustle roof's rear corners (turret frame).
    RF.whip_pair(turret, (-2.47, 1.20, 0.956), v.mats, height=2.2)


def wreck(variant, v):
    """The tank after its cook-off (`scripted_body.wreck`): the right track
    thrown flat beside it with three road wheels torn off, plates warped and
    the glacis dented; the gun droops and `wreckage.burn` heaves the turret."""
    import bpy
    bpy.data.objects["gun"].rotation_euler.y += math.radians(GUN_DROOP_DEG)
    SB.wreck(v, "R", (2, 3, 6), seed=5)


if __name__ == "__main__":
    run("abrams", "us_desert_tan", build, wreck, chip=1.0)
