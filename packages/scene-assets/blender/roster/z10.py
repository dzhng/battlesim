"""Z-10, from assets/references/z_10/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/z10.py -- [--wreck]

What the photos settle: a slim tandem attack helicopter, the gunner forward
and the pilot raised behind under angular flat-paned canopies, the sensor
turret in the nose, the cannon turret under the chin, engines either side of
the gearbox with exhausts turned out, stub wings with two stations each, the
tail with the fin, a stabilator and the tail rotor on the right; fixed main
wheels and a tail wheel. Grey.

Dimensions stated from published figures (rotorcraft rule: fuselage length
without blades, width over the stub wings' stores, height to the top of the
rotor head): 14.15 x 4.32 x 3.85 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"eastern_z_10_attack_helicopter": (14.15, 4.32, 3.85)}
SPEC = dict(
    fuselage=[(7.07, 0.0, 1.3, 1.3), (6.7, 0.42, 0.95, 1.64, 1.3, 1.8), (5.7, 0.54, 0.8, 1.8, 1.35, 2.4),
              (4.1, 0.58, 0.8, 2.1, 1.45, 2.6), (2.4, 0.62, 0.85, 2.38, 1.5, 3.0), (0.4, 0.6, 0.9, 2.28, 1.55, 3.0),
              (-1.2, 0.42, 1.15, 2.1, 1.65, 2.6), (-4.2, 0.22, 1.4, 1.9, 1.65, 2.2), (-6.4, 0.18, 1.45, 1.95, 1.7),
              (-7.08, 0.16, 1.5, 2.1, 1.75)],
    wing=[(1.6, 0.55, 1.6, 1.2, 0.16), (1.4, 2.06, 1.52, 1.0, 0.12)],
    stab=[(-5.8, 0.2, 1.8, 0.9, 0.1), (-5.9, 1.3, 1.8, 0.7, 0.06)],
    fins=[dict(root_x=-5.6, root_z=1.95, height=1.5, root_chord=1.4, tip_chord=0.9, sweep_m=0.5, thick=0.16)],
    gear=[dict(name="main_L", x=2.6, y=1.0, top=0.95, radius=0.32, width=0.18),
          dict(name="main_R", x=2.6, y=-1.0, top=0.95, radius=0.32, width=0.18),
          dict(name="tail", x=-5.4, y=0.0, top=1.5, radius=0.16, width=0.1)],
    pylons=[((1.3, 1.1, 1.52), 0.8, 0.22), ((1.25, 1.85, 1.48), 0.8, 0.22)],
    stores=[("pod", (1.2, 1.1, 1.08), 1.6, 0.22), ("missile", (1.1, 2.0, 1.1), 1.7, 0.1)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The three-view's stepped glass, tall and standing out of the body.
    A.canopy("canopy", 6.45, 4.95, 1.55, 2.35, 0.52, m, hull, bows=(6.0, 5.5), peak=0.6, tail=0.75)
    A.canopy("canopy_glass_rear", 4.95, 3.35, 1.85, 2.82, 0.56, m, hull, bows=(4.45, 3.85), peak=0.5, tail=0.8)
    A.sensor_ball("sensor", (6.85, 0, 1.12), 0.24, m, hull)
    cyl("gun_turret", 0.2, 0.3, (5.6, 0, 0.66), "Z", m["dark"], hull, seg=12)
    cyl("gun_barrel", 0.05, 1.2, (6.2, 0, 0.58), "X", m["steel"], hull, seg=8, lods=MID)
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(1.9, 0.0, 2.3, 2.3), (1.6, 0.3, 2.02, 2.58, 2.3), (-0.9, 0.3, 2.02, 2.56, 2.3),
                                (-1.4, 0.15, 2.14, 2.46, 2.3)], m["paint"], hull, seg=14, loc=(0, side * 0.62, 0))
        box(f"exhaust_{k}", (0.6, 0.22, 0.3), (-1.2, side * 0.86, 2.4), m["nozzle"], hull, rot=(0, 0, side * 0.6),
            lods=MID)
        A.store(f"atgm_{k}", "missile", (1.1, side * 2.0, 1.32), 1.7, 0.1, m, hull)
    cyl("mast", 0.15, 0.9, (0.3, 0, 3.05), "Z", m["dark"], hull, seg=12)
    A.rotor("main", (0.3, 0, 3.55), 6.5, 5, 0.46, m, hull, hub=0.38, mast=0.0, droop=0.025, phase=0.4)
    A.rotor("tail", (-6.65, -0.24, 3.0), 1.4, 4, 0.24, m, hull, hub=0.13, mast=0.0, droop=0.0,
            rot=(math.pi / 2, 0, 0), thick=0.12)

    # The PLA's star on the boom, a red number on the nose.
    A.markings(v, [
        ("insignia", dict(kind="cn_star", centre=(-3.5, 1.2, 1.68), normal=(0, 1, 0), up=(0, 0, 1), size=0.3, onto=("fuselage",))),
        ("text", dict(text="07", height=0.36, centre=(4.2, 1.2, 1.4), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="red")),
    ])

def wreck(variant, v):
    A.crash(v, tail_x=-2.4, tail_yaw=-0.42, tail_drop=0.1, blades_broken=(("main", 3),), seed=10)


run_disabled("z10", CARDS, "chinese_air_grey", build, wreck, skip=("dressing_", "blade_"))
