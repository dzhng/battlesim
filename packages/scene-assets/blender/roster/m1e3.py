"""M1E3 Abrams prototype, from assets/references/m1e3/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/m1e3.py -- [--wreck]

What the photos of the early prototype (Detroit, January 2026) settle: the
M1 hull and its running gear, the engine deck standing above the front deck
and a blunt nose; long flat skirt panels; a long, slab-sided turret with a
squared bustle over the engine deck, its cheeks either side of the gun
shield; a remote weapon station on a raised base on the roof, a sight box
and active-protection panels and launchers on the turret. The prototype is
green; the model wears US desert tan like every US roster vehicle.

Built to the catalog frame, the SEPv3 Trophy's box and turret pivot (hull
7.93 x 3.66 x 2.44 m, pivot 0.25 m ahead of the hull's middle and 1.464 m
up), with the remote station's mount on its base mid-roof: nothing here
moves it.

It wears a scripted body (`BODY`, `scripted_body`): Gemini Argon's build
script after its second round, edited since (its folder's README lists the
edits). This script adds its rig, the Trophy part's nodes, whip antennas and
wreck.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from parts import empty  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

BODY = "assets/source/skins/m1e3/argon-r2/build.py"
LINK_PITCH = 0.19
# The gun's trunnion, ahead of the ring along the bore: the script's.
TRUNNION = 1.47
# The long 120 mm droops toward the glacis in the wreck, as the Abrams' does.
GUN_DROOP_DEG = 10.0


def build(variant, v):
    import bpy
    from mathutils import Vector
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH)
    # The Trophy part's nodes (`trophy_aps`): each at the middle of its housing.
    bpy.context.view_layer.update()
    for part in ("radar", "launcher"):
        for s in ("L", "R"):
            holder = bpy.data.objects[f"dressing_trophy_{part}_{s}"]
            points = [o.matrix_world @ p.co for o in holder.children_recursive
                      if o.type == "MESH" and o.name.endswith("_LOD0") for p in o.data.vertices]
            mid = sum(points, Vector()) / len(points)
            empty(f"trophy_{part}_{s}", tuple(holder.matrix_world.inverted() @ mid), holder)
    # The whips on the bustle roof's rear corners (turret frame).
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        RF.antenna(f"antenna_{s}", (-2.2, side * 1.2, 0.98), v.mats, whip, height=2.2)


def wreck(variant, v):
    import bpy
    bpy.data.objects["gun"].rotation_euler.y += math.radians(GUN_DROOP_DEG)
    SB.wreck(v, "R", (2, 3, 6), seed=5)


if __name__ == "__main__":
    run("m1e3", "us_desert_tan", build, wreck, chip=1.0)
