"""Mi-35M, from assets/references/mi_24_mi_35_hind/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/hind.py -- [--wreck]

What the photos settle: the Hind's two bubble canopies stepped in tandem
over a deep fuselage with the troop cabin behind them, the twin-barrel gun
turret under the chin beside the sensor turret, engines on the roof with
their intakes forward of the gearbox, stub wings with marked anhedral and
three stations each plus wing-tip launchers, the long boom with the tail
fin and the tail rotor on its left; the 35M's fixed tricycle gear and
shortened wings. Grey.

Dimensions stated from published figures (rotorcraft rule: fuselage length
without blades, width over the wing-tip launchers, height to the top of the
rotor head): 17.5 x 5.27 x 3.97 m, the Mi-24VM's (the sources are in the
library's gaps).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"eastern_mi_24_mi_35_hind_mi_35m": (17.5, 5.27, 3.97)}
SPEC = dict(
    fuselage=[(8.75, 0.0, 1.2, 1.2), (8.4, 0.5, 0.8, 1.7, 1.2, 2.2), (7.2, 0.72, 0.6, 2.2, 1.3, 2.8),
              (5.0, 0.95, 0.55, 2.55, 1.45, 3.2), (1.0, 1.0, 0.6, 2.6, 1.5, 3.4), (-1.6, 0.85, 0.85, 2.55, 1.65, 3.0),
              (-3.0, 0.35, 1.6, 2.4, 1.95, 2.4), (-7.8, 0.22, 1.8, 2.3, 2.05, 2.2), (-8.75, 0.2, 1.85, 2.6, 2.1)],
    bodies=[("fuselage_roof", [(4.4, 0.0, 2.55, 2.55), (3.8, 0.85, 2.45, 3.15, 2.7, 3.0), (-1.0, 0.85, 2.45, 3.2, 2.7, 3.0),
                               (-2.0, 0.0, 2.6, 2.6)])],
    wing=[(1.2, 0.9, 2.15, 1.6, 0.2), (1.0, 2.5, 1.83, 1.25, 0.12)],
    stab=[(-7.0, 0.22, 2.0, 0.9, 0.1), (-7.1, 1.5, 2.0, 0.7, 0.06)],
    fins=[dict(root_x=-7.2, root_z=2.25, height=1.65, root_chord=1.4, tip_chord=0.9, sweep_m=0.6, thick=0.16)],
    gear=[dict(name="nose", x=6.4, y=0.0, top=0.75, radius=0.3, width=0.18, wheels=2, spread=0.3),
          dict(name="main_L", x=-0.3, y=1.5, top=1.0, radius=0.38, width=0.24),
          dict(name="main_R", x=-0.3, y=-1.5, top=1.0, radius=0.38, width=0.24)],
    pylons=[((0.9, 1.5, 1.99), 0.8, 0.2), ((0.85, 2.05, 1.88), 0.8, 0.2)],
    stores=[("pod", (0.8, 1.5, 1.54), 1.7, 0.25), ("pod", (0.75, 2.05, 1.43), 1.7, 0.25),
            ("missile", (0.7, 2.46, 1.61), 1.8, 0.1)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    A.canopy("canopy", 8.4, 7.1, 1.55, 2.12, 0.55, m, hull, bows=(7.7,), peak=0.6)
    A.canopy("canopy_glass_rear", 7.0, 5.4, 2.0, 2.68, 0.65, m, hull, bows=(6.2,), peak=0.5)
    A.sensor_ball("sensor", (8.35, 0.35, 0.78), 0.24, m, hull)
    cyl("gun_turret", 0.26, 0.35, (8.0, -0.3, 0.7), "Z", m["dark"], hull, seg=12)
    for j in (-1, 1):
        cyl(f"gun_barrel_{j}", 0.045, 1.0, (8.2, -0.3 + j * 0.07, 0.65), "X", m["steel"], hull, seg=8, lods=MID)
    for side, k in ((1, "L"), (-1, "R")):
        cyl(f"nacelle_{k}_intake", 0.32, 0.12, (3.85, side * 0.48, 2.85), "X", m["black"], hull, seg=14, lods=MID)
        box(f"exhaust_{k}", (0.9, 0.45, 0.45), (-0.6, side * 0.95, 2.85), m["nozzle"], hull, rot=(0, 0, side * 0.5),
            lods=MID)
        box(f"cabin_door_{k}", (1.2, 0.04, 1.0), (1.2, side * 1.0, 1.35), m["dark"], hull, lods=MID)
        for j in range(3):
            box(f"cabin_window_{k}_{j}", (0.3, 0.03, 0.3), (2.4 - j * 0.9, side * 1.01, 1.9), m["glass"], hull,
                lods=MID)
        box(f"wing_tip_launcher_{k}", (1.4, 0.18, 0.4), (0.7, side * 2.46, 1.73), m["dark"], hull, lods=MID)
    cyl("mast", 0.22, 0.8, (0.6, 0, 3.2), "Z", m["dark"], hull, seg=12)
    A.rotor("main", (0.6, 0, 3.67), 8.65, 5, 0.58, m, hull, hub=0.46, mast=0.0, droop=0.03, phase=0.1)
    A.rotor("tail", (-8.55, 0.3, 3.3), 1.95, 3, 0.3, m, hull, hub=0.18, mast=0.0, droop=0.0,
            rot=(-math.pi / 2, 0, 0), thick=0.12)

    # The VVS's red star on the boom, a red bort number on the nose.
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=(-5.0, 1.2, 2.1), normal=(0, 1, 0), up=(0, 0, 1), size=0.3, onto=("fuselage",))),
        ("text", dict(text="21", height=0.45, centre=(5.4, 1.2, 1.65), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="red")),
    ])

def wreck(variant, v):
    A.crash(v, tail_x=-3.4, tail_yaw=-0.38, tail_drop=0.08, blades_broken=(("main", 0), ("main", 3)), seed=24)


run_disabled("hind", CARDS, "russian_air_grey", build, wreck, skip=("dressing_", "blade_"))
