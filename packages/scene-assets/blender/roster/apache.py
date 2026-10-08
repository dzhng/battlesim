"""AH-64E Apache Guardian, from assets/references/ah_64_apache/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/apache.py -- [--wreck]

What the photos settle: the narrow fuselage with the gunner forward and the
pilot raised behind under flat-paned canopies, the TADS/PNVS turret on the
nose, the M230 chain gun under the chin, engine nacelles on both flanks
behind the cockpit with their exhaust suppressors, stub wings with Hellfire
racks and rocket pods, the Longbow radar dome over the rotor head, four
blades, the tail fin with the scissor tail rotor on its left and the
stabilator at its foot; trailing-arm main wheels and a tail wheel. Army green.

Dimensions stated from published figures (rule: fuselage length without
blades, width over the stub wings' stores, height to the top of the Longbow
dome): 14.7 x 5.23 x 4.72 m, Boeing's length and height (the sources are in
the library's gaps).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"ah_64e_guardian": (14.7, 5.23, 4.72)}
SPEC = dict(
    fuselage=[(6.65, 0.0, 1.42, 1.42), (6.38, 0.44, 1.05, 1.86, 1.4, 2.4), (5.36, 0.56, 0.95, 2.3, 1.45, 3.0),
              (3.7, 0.62, 0.95, 2.72, 1.6, 3.0), (1.85, 0.75, 0.95, 2.62, 1.6, 3.5), (0.0, 0.7, 1.0, 2.52, 1.65, 3.5),
              (-1.39, 0.5, 1.2, 2.36, 1.75, 3.0), (-3.7, 0.25, 1.55, 2.16, 1.85, 2.4), (-6.28, 0.2, 1.7, 2.2, 1.95),
              (-7.02, 0.18, 1.75, 2.42, 2.0)],
    wing=[(1.62, 0.65, 1.95, 1.29, 0.18), (1.34, 2.42, 1.86, 1.02, 0.12)],
    stab=[(-6.1, 0.2, 1.95, 0.92, 0.1), (-6.24, 1.7, 1.95, 0.74, 0.06)],
    fins=[dict(root_x=-5.82, root_z=2.15, height=1.8, root_chord=1.48, tip_chord=1.02, sweep_m=0.83, thick=0.16)],
    gear=[dict(name="main_L", x=2.4, y=1.0, top=1.1, radius=0.4, width=0.22),
          dict(name="main_R", x=2.4, y=-1.0, top=1.1, radius=0.4, width=0.22),
          dict(name="tail", x=-5.64, y=0.0, top=1.7, radius=0.2, width=0.12)],
    pylons=[((1.29, 1.35, 1.86), 0.83, 0.3), ((1.2, 2.15, 1.82), 0.83, 0.3)],
    stores=[("pod", (1.2, 1.35, 1.32), 1.57, 0.27)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    A.canopy("canopy", 6.24, 4.9, 1.92, 2.42, 0.5, m, hull, bows=(5.54,), peak=0.6)
    A.canopy("canopy_glass_rear", 4.85, 3.51, 2.3, 2.86, 0.52, m, hull, bows=(4.16,), peak=0.5)
    # TADS/PNVS on the nose: the turret and its sensor windows.
    A.sensor_ball("tads", (6.7, 0, 1.25), 0.36, m, hull)
    box("pnvs", (0.4, 0.3, 0.3), (6.51, 0, 1.78), m["dark"], hull, bevel=0.03, lods=MID)
    # The M230 under the chin.
    cyl("gun_turret", 0.2, 0.3, (4.34, 0, 0.82), "Z", m["dark"], hull, seg=12)
    cyl("gun_barrel", 0.05, 1.39, (5.08, 0, 0.72), "X", m["steel"], hull, seg=8, lods=MID)
    # Engines on the flanks with their exhaust suppressors, and the rotor
    # mast's fairing.
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(1.94, 0.0, 2.2, 2.2), (1.66, 0.32, 1.88, 2.5, 2.2), (-0.55, 0.32, 1.88, 2.5, 2.2),
                                (-1.11, 0.2, 2.0, 2.4, 2.2)], m["paint"], hull, seg=14, loc=(0, side * 0.92, 0))
        box(f"exhaust_{k}", (0.65, 0.2, 0.3), (-1.02, side * 1.15, 2.22), m["nozzle"], hull, rot=(0, 0, side * 0.5),
            lods=MID)
        # Hellfire rack on the outer pylon: four rails.
        for j, (dy, dz) in enumerate(((0.16, 0.18), (-0.16, 0.18), (0.16, -0.12), (-0.16, -0.12))):
            A.store(f"hellfire_{k}_{j}", "missile", (1.2, side * 2.15 + dy, 1.36 + dz), 1.63, 0.09, m, hull)
        box(f"hellfire_rack_{k}", (0.92, 0.1, 0.5), (1.2, side * 2.15, 1.42), m["dark"], hull, lods=MID)
        box(f"wing_tip_station_{k}", (0.5, 0.18, 0.2), (1.11, side * 2.52, 1.86), m["dark"], hull, lods=MID)
    A.body("mast_fairing", [(1.29, 0.0, 2.55, 2.55), (0.92, 0.45, 2.45, 2.95, 2.6), (-0.55, 0.45, 2.45, 2.9, 2.6),
                            (-1.11, 0.0, 2.6, 2.6)], m["paint"], hull, seg=14)
    cyl("mast", 0.15, 0.85, (0.18, 0, 3.11), "Z", m["dark"], hull, seg=12)
    A.rotor("main", (0.18, 0, 3.61), 7.32, 4, 0.53, m, hull, hub=0.4, mast=0.0, droop=0.025, phase=0.3)
    # The Longbow fire-control radar's dome over the hub.
    A.body("longbow", [(0.88, 0.0, 4.31, 4.31), (0.69, 0.5, 4.06, 4.61, 4.34), (-0.32, 0.55, 4.01, 4.71, 4.36),
                       (-0.55, 0.0, 4.36, 4.36)], m["paint"], hull, seg=16)
    cyl("longbow_post", 0.12, 0.3, (0.18, 0, 3.96), "Z", m["dark"], hull, seg=10)
    A.rotor("tail", (-6.61, 0.3, 3.5), 1.4, 4, 0.25, m, hull, hub=0.14, mast=0.0, droop=0.0,
            rot=(-math.pi / 2, 0, 0), thick=0.12, phase=0.35)
    A.blade_antenna("antenna_belly", (0.0, 0, 1.0), 0.25, m, hull, down=True)


def wreck(variant, v):
    A.crash(v, tail_x=-2.6, tail_yaw=-0.4, tail_drop=0.1, blades_broken=(("main", 0), ("main", 2)), seed=64)


run_disabled("apache", CARDS, "us_army_aviation", build, wreck, skip=("dressing_", "blade_"))
