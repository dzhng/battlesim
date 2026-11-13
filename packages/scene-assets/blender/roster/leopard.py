"""Leopard 2A6, 2A7V and 2A8, from assets/references/leopard/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/leopard.py -- [--variant=<id>] [--wreck]

What the photos settle: seven dual road wheels a side, evenly spaced, four
return rollers behind the skirts, the drive sprocket raised at the rear and the
idler raised at the front; skirts in panel sections, the front three thick
ballistic panels with a sloped leading edge; a long shallow glacis rising to a
flat deck; the engine deck's grilles and the full-width louvred rear plate. The
turret is the A5-and-later arrowhead: wedge add-on armour either side of the
mantlet, meeting in a ridge at mid height, flat bolted sides, eight smoke tubes
a side behind the wedge, side stowage boxes and a rear basket on the bustle,
the gunner's sight box on the right front roof and the commander's PERI behind
it. 2A7V: the slab armour module across the hull front with its lamps let in,
bolted roof armour, the bustle's auxiliary unit and the FLW 100 weapon station.
2A8: Trophy's angular housings on the turret sides over the bustle, the FLW 200
with its heavy machine gun, and the turret front plates (from the Eurosatory
2022 demonstrator; no licensable photo of a series 2A8's turret roof).

Built to the catalog frame (2A6: hull 7.7 x 3.75 m, box top 2.70 m, turret
pivot 0.3 m ahead of the hull's middle and 1.8 m up; 2A7V and 2A8: hull
8.05 m with the front module, box top 2.83 m that the PERI sets, pivot
0.125 m ahead): nothing here moves it.

All three wear one scripted body (`BODY`, `scripted_body`): Gemini Argon's
build script after its third round (the turret front), `build(variant)` for each (its folder's
README lists its rounds and edits). This script adds the rig, the 2A8's
Trophy nodes, whip antennas and wreck.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from parts import empty  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The scripted body every variant wears, each variant's id in it, its
# track's link pitch, and the gun's trunnion ahead of the ring along the bore.
BODY = "assets/source/skins/leopard/argon-r3/build.py"
BODIES = {"europe_leopard_2_2a6": "2a6", "europe_leopard_2_2a7v": "2a7v", "europe_leopard_2_2a8": "2a8"}
LINK_PITCH = 0.18
TRUNNION = 1.10
# The whips on the bustle roof's rear corners (turret frame, left side), on
# each variant's roof.
WHIPS = {"2a6": (-2.8, 1.2, 0.76), "2a7v": (-2.8, 1.2, 0.84), "2a8": (-2.8, 1.2, 0.84)}
# The 2A8's Trophy part's nodes (`trophy_aps`) on the script's housings
# (world, left side): the radar panels' fairing and the launcher drum.
TROPHY_NODES = {"radar": (-0.795, 1.75, 2.16), "launcher": (-1.125, 1.62, 2.72)}


def build(variant, v):
    import bpy
    from mathutils import Vector
    body = BODIES[variant["id"]]
    turret = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})["cannon"][0]
    SB.wear(v, BODY, LINK_PITCH, variant=body)
    bpy.context.view_layer.update()
    for side, s in ((1, "L"), (-1, "R")):
        holder = bpy.data.objects.get(f"dressing_trophy_{s}")
        for part, (x, y, z) in TROPHY_NODES.items() if holder else ():
            empty(f"trophy_{part}_{s}", tuple(holder.matrix_world.inverted() @ Vector((x, side * y, z))), holder)
    x, y, z = WHIPS[body]
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        RF.antenna(f"antenna_{s}", (x, side * y, z), v.mats, whip, height=2.2)


def wreck(variant, v):
    SB.wreck(v, "R", (3, 4), seed=7)


if __name__ == "__main__":
    run("leopard", "german_three_tone", build, wreck, chip=1.0)
