"""A-10C Thunderbolt II, from assets/references/a_10_warthog/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/a10.py -- [--wreck]

What the photos settle: the blunt nose with the GAU-8's seven-barrel muzzle
under it and the nose gear offset to the right, the high bubble canopy on an
armoured tub, the thick straight low wing with the main gear half-retracted
into pods under it and the tips drooped, two engines in round nacelles on
pylons high on the rear fuselage, the tailplane with twin fins at its ends;
eleven pylons. Two-tone grey.

Dimensions stated from the references (USAF fact sheet figures agree):
length 16.26 m, span 17.53 m, height 4.47 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import FINE, MID, NEAR  # noqa: E402

CARDS = {"us_a_10_warthog_a_10c_retained_as_an_iconic_exception_if_needed": (16.26, 17.53, 4.47)}

WING_Z = 1.12
FIN = dict(root_x=-6.1, root_z=1.92, height=2.55, root_chord=2.0, tip_chord=1.45, sweep_m=0.45)
SPEC = dict(
    fuselage=[(7.72, 0.0, 1.5, 1.5), (7.35, 0.46, 1.08, 1.95, 1.5), (6.3, 0.72, 0.96, 2.18, 1.55, 2.4),
              (4.6, 0.8, 0.94, 2.32, 1.6, 2.6), (2.5, 0.82, 0.95, 2.2, 1.6, 2.6), (0.0, 0.76, 1.0, 2.06, 1.55, 2.4),
              (-3.0, 0.56, 1.24, 1.96, 1.6, 2.2), (-6.0, 0.32, 1.55, 1.96, 1.75), (-7.6, 0.16, 1.72, 1.95, 1.84)],
    bodies=[(f"nacelle_gear_{s}", [(2.9, 0.0, 0.9, 0.9), (2.4, 0.34, 0.62, 1.14, 0.9), (0.2, 0.34, 0.62, 1.12, 0.88),
                                   (-0.6, 0.0, 1.0, 1.0)], 2.1 if s == "L" else -2.1, 14) for s in "LR"],
    canopy=dict(x_front=5.4, x_back=3.6, sill=2.24, top=2.86, half_width=0.52, bows=(4.85,), peak=0.5),
    wing=[(1.48, 0.72, WING_Z, 3.1, 0.46), (1.42, 2.7, WING_Z, 3.0, 0.4), (0.92, 8.38, 1.52, 1.7, 0.16),
          (0.86, 8.76, 1.42, 1.6, 0.1)],
    stab=[(-6.25, 0.2, 1.92, 1.95, 0.16), (-6.4, 2.86, 1.92, 1.65, 0.12)],
    fins=[dict(FIN, y=2.86), dict(FIN, y=-2.86)],
    gear=[dict(name="nose", x=6.2, y=-0.36, top=1.0, radius=0.33, width=0.2, door=(0.8, 0.4)),
          dict(name="main_L", x=1.5, y=2.1, top=0.82, radius=0.5, width=0.3),
          dict(name="main_R", x=1.5, y=-2.1, top=0.82, radius=0.5, width=0.3)],
    pylons=[((0.9, 3.4, WING_Z - 0.18), 1.2, 0.26), ((0.9, 4.6, WING_Z - 0.08), 1.1, 0.24),
            ((0.6, 5.8, WING_Z + 0.04), 1.0, 0.22)],
    stores=[("missile", (0.8, 3.4, 0.5), 2.5, 0.15), ("bomb", (0.8, 4.6, 0.62), 2.3, 0.2),
            ("pod", (0.5, 5.8, 0.85), 2.4, 0.15)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The GAU-8 under the nose: its muzzle fairing and seven barrels.
    cyl("gun_fairing", 0.24, 0.5, (7.6, 0.06, 1.3), "X", m["paint"], hull, seg=16, bevel=0.02)
    for k in range(7):
        a = math.tau * k / 7
        cyl(f"gun_barrel_{k}", 0.035, 0.5, (8.0, 0.06 + math.cos(a) * 0.12, 1.3 + math.sin(a) * 0.12), "X",
            m["steel"], hull, seg=8, lods=MID if k % 2 == 0 else NEAR)
    cyl("gun_muzzle", 0.17, 0.04, (8.24, 0.06, 1.3), "X", m["black"], hull, seg=14, lods=MID)
    # The engines: round nacelles on pylons over the rear fuselage.
    for side, s_ in ((1, "L"), (-1, "R")):
        y = side * 1.38
        A.body(f"nacelle_engine_{s_}", [(-1.9, 0.0, 2.68, 2.68), (-2.1, 0.62, 2.06, 3.3, 2.68),
                                        (-4.4, 0.62, 2.06, 3.3, 2.68), (-5.15, 0.44, 2.24, 3.12, 2.68)],
               m["paint"], hull, seg=20, loc=(0, y, 0))
        cyl(f"nacelle_engine_{s_}_fan", 0.52, 0.04, (-2.1, y, 2.68), "X", m["dark"], hull, seg=20, lods=MID)
        cyl(f"nacelle_engine_{s_}_spinner", 0.14, 0.2, (-2.02, y, 2.68), "X", m["steel"], hull, seg=10,
            r2=0.03, lods=NEAR)
        cyl(f"nacelle_engine_{s_}_exhaust", 0.36, 0.05, (-5.17, y, 2.68), "X", m["black"], hull, seg=16, lods=MID)
        box(f"nacelle_pylon_{s_}", (2.2, 0.16, 0.7), (-3.3, side * 0.95, 2.12), m["paint"], hull, bevel=0.03,
            rot=(side * 0.6, 0, 0))
    box("antiglare", (1.1, 0.7, 0.03), (6.5, 0, 2.08), m["dark"], hull, rot=(0, 0.25, 0), lods=MID)
    box("refuel_door", (0.5, 0.3, 0.03), (6.9, 0, 1.97), m["dark"], hull, rot=(0, 0.35, 0), lods=NEAR)
    for side, s_ in ((1, "L"), (-1, "R")):
        box(f"wing_flap_seam_{s_}", (0.03, 5.4, 0.03), (-1.0, side * 5.4, 1.32), m["dark"], hull,
            rot=(0, 0, side * 0.05), lods=FINE)
    A.blade_antenna("antenna_spine", (1.4, 0, 2.08), 0.2, m, hull)


def wreck(variant, v):
    A.crash(v, tail_x=-4.9, wing_y=5.0, wing_side=-1, tail_yaw=0.32, seed=10)


run_disabled("a10", CARDS, "us_gunship_grey", build, wreck)
