"""Mi-8AMTSh, from assets/references/mi_8_hip/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/hip.py -- [--wreck]

What the photos settle: the long rounded cabin with its glazed nose and
blister side windows, two engines side by side on the roof ahead of the
gearbox with their exhausts out the flanks, the long tail boom rising to
the tail pylon with the three-blade tail rotor on its right, a small
stabilator, the clamshell doors at the rear, fixed tricycle gear; the
AMTSh's outriggers with rocket pods, the armour plates and the exhaust
suppressors. Army camouflage.

Dimensions stated from published figures (rotorcraft rule: fuselage length
without blades, width over the outrigger stores, height to the top of the
rotor head): 18.17 x 5.6 x 4.76 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"eastern_mi_8_hip_mi_8amtsh": (18.17, 5.6, 4.76)}
SPEC = dict(
    fuselage=[(9.08, 0.0, 1.35, 1.35), (8.7, 0.7, 0.75, 2.05, 1.35, 2.2), (7.6, 1.05, 0.55, 2.6, 1.45, 2.8),
              (5.6, 1.18, 0.5, 2.85, 1.55, 3.5), (0.0, 1.18, 0.5, 2.85, 1.55, 3.5), (-1.9, 1.05, 0.7, 2.8, 1.7, 3.0),
              (-2.6, 0.45, 1.6, 2.6, 2.05, 2.4), (-7.6, 0.24, 1.95, 2.5, 2.2, 2.2), (-8.6, 0.24, 2.0, 3.6, 2.3),
              (-9.08, 0.2, 2.2, 3.6, 2.6)],
    bodies=[("fuselage_roof", [(6.0, 0.0, 2.85, 2.85), (5.2, 0.8, 2.7, 3.45, 2.95, 3.0), (-0.6, 0.8, 2.7, 3.55, 3.0, 3.0),
                               (-1.6, 0.0, 2.9, 2.9)])],
    canopy=dict(x_front=8.95, x_back=7.0, sill=1.2, top=2.55, half_width=1.0, peak=0.9, bows=(8.2,)),
    stab=[(-7.6, 0.25, 2.1, 0.9, 0.1), (-7.7, 1.4, 2.1, 0.7, 0.06)],
    wing=[(1.4, 1.15, 1.8, 0.9, 0.1), (1.3, 2.6, 1.85, 0.8, 0.08)],
    gear=[dict(name="nose", x=6.4, y=0.0, top=0.8, radius=0.3, width=0.18, wheels=2, spread=0.34),
          dict(name="main_L", x=-0.4, y=2.0, top=1.1, radius=0.4, width=0.25),
          dict(name="main_R", x=-0.4, y=-2.0, top=1.1, radius=0.4, width=0.25)],
    pylons=[((1.2, 1.7, 1.75), 0.7, 0.2), ((1.2, 2.45, 1.8), 0.7, 0.2)],
    stores=[("pod", (1.1, 1.7, 1.33), 1.7, 0.25), ("pod", (1.1, 2.45, 1.38), 1.7, 0.25)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    for side, k in ((1, "L"), (-1, "R")):
        cyl(f"nacelle_{k}_intake", 0.3, 0.1, (5.7, side * 0.45, 3.1), "X", m["black"], hull, seg=14, lods=MID)
        box(f"exhaust_{k}", (0.8, 0.5, 0.45), (2.2, side * 1.0, 3.1), m["nozzle"], hull, rot=(0, 0, side * 0.6),
            lods=MID)
        for j in range(5):
            cyl(f"blister_{k}_{j}", 0.24, 0.06, (4.6 - j * 1.1, side * 1.18, 1.85), "Y", m["glass"], hull, seg=14,
                lods=MID)
        box(f"armour_{k}", (1.2, 0.05, 0.9), (7.0, side * 1.15, 1.45), m["paint"], hull, bevel=0.02, lods=MID)
        box(f"clamshell_seam_{k}", (0.03, 0.04, 1.8), (-1.85, side * 0.6, 1.6), m["dark"], hull, lods=MID)
        # The outrigger struts to the stores.
        box(f"outrigger_strut_{k}", (0.12, 1.4, 0.1), (1.3, side * 1.7, 1.5), m["dark"], hull, rot=(side * 0.3, 0, 0))
    cyl("mast", 0.22, 0.9, (0.3, 0, 3.95), "Z", m["dark"], hull, seg=12)
    A.rotor("main", (0.3, 0, 4.5), 10.65, 5, 0.52, m, hull, hub=0.48, mast=0.0, droop=0.03, phase=0.2)
    A.rotor("tail", (-8.75, -0.32, 3.25), 1.95, 3, 0.3, m, hull, hub=0.18, mast=0.0, droop=0.0,
            rot=(math.pi / 2, 0, 0), thick=0.12)
    box("tail_pylon", (0.9, 0.36, 1.4), (-8.55, 0, 2.95), m["paint"], hull, bevel=0.04, rot=(0, -0.3, 0))
    A.blade_antenna("antenna_belly", (-1.0, 0, 0.55), 0.3, m, hull, down=True)


def wreck(variant, v):
    A.crash(v, tail_x=-3.4, tail_yaw=0.35, tail_drop=0.08, blades_broken=(("main", 1), ("main", 4)), seed=8)


run_disabled("hip", CARDS, "russian_helicopter_camo", build, wreck, skip=("dressing_", "blade_"))
