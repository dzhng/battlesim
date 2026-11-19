"""AH-64E Guardian. The fitted donor supplies the tandem cockpit, airframe,
undercarriage, wings and stores. The game supplies the crew, articulated
rotors, chin rig, markings and wreck."""
import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_donor import fitted_airframe  # noqa: E402
from parts import cyl, flat_paint, mesh_part  # noqa: E402
from vehicle_export import rig, run  # noqa: E402


def cheek_panels(hull):
    shell = bpy.data.objects["fuselage_LOD0"]
    seal = flat_paint("access_seals", (.034, .031, .024), rough=.85, grime=0, role="paint")
    fastener = flat_paint("flush_fasteners", (.085, .079, .063), rough=.59, grime=0, role="paint")
    for side in (1, -1):
        def surface(x, z, lift=.003):
            hit, point, _, _ = shell.ray_cast((x, 1.2 * side, z), (0, -side, 0))
            if not hit or point.y * side < .6:
                raise ValueError(f"Apache cheek fitting misses the fairing at {x}, {z}, side {side}")
            return (x, point.y + side * lift, z)

        # Project onto each side independently. The cheek fairings are asymmetric.
        strips = []
        for x, top, bottom in ((2.98, 1.49, .98), (3.27, 1.46, .98), (4.16, 1.51, 1.02), (5.41, 1.48, 1.11)):
            strip = [surface(x-.009, bottom), surface(x+.009, bottom), surface(x+.009, top), surface(x-.009, top)]
            strips.append(strip[::-1] if side > 0 else strip)

        def seals(bm, lod):
            for strip in strips:
                bm.faces.new([bm.verts.new(point) for point in strip])

        mesh_part(f"cheek_access_seals_{side}", seals, seal, hull, lods=A.NEAR)
        locks = ((3.42, 1.43), (3.99, 1.46), (4.32, 1.52), (5.21, 1.43),
                 (3.42, 1.06), (3.99, 1.06), (4.32, 1.10), (5.21, 1.14))
        for i, (x, z) in enumerate(locks):
            cyl(f"cheek_lock_{side}_{i}", .034, .008, surface(x, z, .006), "Y", fastener, hull, seg=8, lods=A.NEAR)


def build(variant, v):
    m = A.fit(v)
    hull = v.hull
    m["canopy"] = flat_paint("apache_laminated_glass", (.019, .027, .030), rough=.15,
                             grime=0, coverage=("blended", .28), role="glass")
    m["blade"] = flat_paint("composite_blades", (.026, .028, .027), rough=.59, grime=.12, role="paint")
    fitted_airframe(v, "ah_64_apache", m)
    if not v.wreck:
        cheek_panels(hull)
        for name, x, z in (("gunner", 5.10, 1.38), ("pilot", 3.80, 1.80)):
            v.crew.append((name, hull, (x, 0, z),
                           ((x+.30, .17, z+.25), (x+.30, -.14, z+.24)),
                           ((x+.53, .17, z-.35), (x+.53, -.17, z-.35))))
    A.rotor("main", (2.60, .025, 3.46), 7.32, 4, .53, m, hull, hub=.40, mast=.38, droop=.025, phase=.3)
    A.rotor("tail", (-6.53, .33, 2.92), 1.4, 4, .25, m, hull, hub=.14, mast=.12, droop=0,
            rot=(-math.pi/2, 0, 0), thick=.12, phase=.35)
    A.chin_gun(v, rig(v.frame, v.root), m, (-60, 11), .06)
    A.markings(v, [
        ("text", dict(text="U.S. ARMY", height=.19, centre=(-3.50, .8, 1.52), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="black")),
        ("text", dict(text="53", height=.23, centre=(1.55, 1.65, 2.13), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="lowvis_light")),
    ])


def wreck(variant, v):
    A.rotorcraft_crash(v, tail_x=-2.6, tail_yaw=-.4, tail_drop=.1, seed=64)


run("apache", "us_army_aviation", build, wreck, crew_army="us_army_ocp",
    references="assets/references/ah_64_apache/references.json",
    authoring="Adapted from Muhamad Mirza Arrafi’s CC BY 4.0 AH-64D for the AH-64E roster.")
