"""Eurofighter Typhoon (Tranche 3), from assets/references/eurofighter_typhoon/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/typhoon.py -- [--wreck]

What the photos settle: the long pointed nose, canards just behind and below
the cockpit, the big rectangular chin intake under the forward body with its
variable lip, the cranked delta wing with no tailplane, one swept fin, two
EJ200 nozzles side by side; wing-tip defensive-aids pods, tanks and missiles
under the wing, missiles semi-recessed under the body. Light grey.

Dimensions stated from the references (Eurofighter figures agree): length
15.96 m, span 10.95 m, height 5.28 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"europe_eurofighter_typhoon_tranche_3": (15.96, 10.95, 5.28)}
WING_Z = 1.82
SPEC = dict(
    fuselage=[(7.98, 0.0, 2.0, 2.0), (7.2, 0.36, 1.75, 2.26, 2.0), (6.0, 0.55, 1.55, 2.5, 1.95, 2.2),
              (4.4, 0.62, 1.5, 2.62, 1.95, 2.4), (2.6, 0.72, 1.55, 2.64, 1.95, 2.6), (0.0, 0.86, 1.5, 2.56, 1.95, 2.8),
              (-3.0, 0.86, 1.5, 2.46, 1.95, 2.8), (-5.6, 0.8, 1.6, 2.36, 1.95, 2.6), (-6.95, 0.72, 1.65, 2.26, 1.95)],
    bodies=[("fuselage_intake", [(3.95, 0.6, 0.95, 1.56, 1.26, 4.0), (2.6, 0.64, 0.98, 1.62, 1.3, 4.0),
                                 (0.5, 0.62, 1.1, 1.7, 1.4, 3.0), (-1.5, 0.5, 1.35, 1.75, 1.55, 2.4)], 0.0, 16)],
    canopy=dict(x_front=5.65, x_back=3.15, sill=2.52, top=3.1, half_width=0.46, peak=0.45),
    canard=[(5.05, 0.6, 2.08, 1.3, 0.08), (4.2, 2.1, 2.06, 0.55, 0.03)],
    wing=[(1.6, 0.86, WING_Z, 6.4, 0.3), (-3.8, 5.32, WING_Z, 1.3, 0.05)],
    fins=[dict(root_x=-3.3, root_z=2.45, height=2.83, root_chord=3.4, tip_chord=1.0, sweep_m=2.6)],
    intakes=[("rect", (3.98, 0.0, 1.26), 1.1, 0.5, 0.22, (0, -0.1, 0))],
    nozzles=[dict(loc=(-6.95, 0.42, 1.95), r_front=0.45, r_exit=0.42, length=1.0, petals=12),
             dict(loc=(-6.95, -0.42, 1.95), r_front=0.45, r_exit=0.42, length=1.0, petals=12)],
    gear=[dict(name="nose", x=4.5, y=0.0, top=1.0, radius=0.26, width=0.16, door=(0.7, 0.4)),
          dict(name="main_L", x=-1.2, y=1.2, top=1.4, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1),
          dict(name="main_R", x=-1.2, y=-1.2, top=1.4, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1)],
    pylons=[((-1.0, 2.0, WING_Z - 0.06), 1.1, 0.3), ((-1.8, 3.3, WING_Z - 0.04), 1.0, 0.24)],
    stores=[("tank", (-1.1, 2.0, 1.1), 4.0, 0.36), ("missile", (-1.8, 3.3, 1.42), 3.65, 0.09),
            ("pod", (-3.0, 5.42, WING_Z), 1.6, 0.12), ("missile", (0.4, 0.55, 1.05), 3.65, 0.09),
            ("missile", (-2.6, 0.55, 1.15), 3.65, 0.09)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    box("intake_splitter", (0.5, 0.04, 0.5), (3.8, 0, 1.3), m["paint"], hull, lods=MID)
    box("antiglare", (1.1, 0.55, 0.03), (6.5, 0, 2.38), m["dark"], hull, rot=(0, 0.18, 0), lods=MID)
    box("probe_fairing", (1.2, 0.12, 0.12), (5.6, -0.52, 2.4), m["paint"], hull, bevel=0.03, lods=MID)
    A.blade_antenna("antenna_spine", (0.0, 0, 2.56), 0.22, m, hull)


def wreck(variant, v):
    A.crash(v, tail_x=-3.6, wing_y=3.2, wing_side=1, tail_yaw=0.28, seed=3)


run_disabled("typhoon", CARDS, "nato_air_grey", build, wreck)
