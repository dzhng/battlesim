"""Su-57, from assets/references/su_57_felon/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/su57.py -- [--wreck]

What the photos settle: a chined nose under a long canopy, the movable
leading-edge extensions (LEVCONs) ahead of the wing root, two engine
nacelles set wide apart under a flat body with a tunnel between them, their
intakes raked under the LEVCONs, a swept wing, stabilators, all-moving fins
canted outward, the long stinger between widely spaced nozzles. Splinter
greys.

Dimensions stated from the references (published figures): length 20.1 m,
span 14.1 m, height 4.6 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"eastern_su_57_felon_stealth_multirole_fighter": (20.1, 14.1, 4.6)}
WING_Z = 2.1
FIN = dict(root_x=-5.0, root_z=2.4, height=2.44, root_chord=3.2, tip_chord=1.3, sweep_m=2.0, cant=0.45, rudder=False)
SPEC = dict(
    fuselage=[(10.0, 0.0, 1.95, 1.95), (9.2, 0.42, 1.72, 2.22, 1.95, 1.4), (7.8, 0.78, 1.58, 2.48, 1.92, 1.5),
              (6.2, 1.0, 1.5, 2.6, 1.9, 1.7), (4.4, 1.6, 1.62, 2.55, 1.95, 2.6), (1.5, 2.1, 1.7, 2.5, 2.0, 3.5),
              (-2.0, 2.0, 1.75, 2.45, 2.0, 3.5), (-5.5, 1.3, 1.8, 2.4, 2.05, 3.0), (-8.5, 0.5, 1.95, 2.3, 2.1, 2.4),
              (-10.0, 0.25, 2.0, 2.25, 2.1)],
    # The side photo: the canopy 2.5 m behind the nose and 3 m long, the
    # nose gear under the intakes' mouths, the mains 1.6 m further aft.
    canopy=dict(x_front=7.5, x_back=4.5, sill=2.5, top=3.12, half_width=0.52, bows=(6.9,), peak=0.45, tail=0.7),
    bodies=[("fuselage_spine", [(4.6, 0.42, 2.4, 2.94, 2.55, 2.4), (1.0, 0.5, 2.4, 2.72, 2.55, 2.6),
                                (-4.4, 0.0, 2.35, 2.35)])],
    wing_hinge=((0.8, 0, 1), 0.08),
    nav=dict(left=(-2.8, 6.95, WING_Z - 0.04), right=(-2.8, -6.95, WING_Z - 0.04)),
    strake=[(6.8, 0.9, 2.1, 3.0, 0.08), (4.3, 2.1, 2.05, 0.6, 0.05)],
    wing=[(3.6, 2.0, WING_Z, 7.0, 0.35), (-2.0, 7.05, WING_Z - 0.06, 1.9, 0.06)],
    stab=[(-6.0, 2.0, 2.0, 3.0, 0.12), (-8.0, 4.6, 1.98, 1.4, 0.05)],
    fins=[dict(FIN, y=1.6), dict(FIN, y=-1.6)],
    intakes=[("rect", (4.08, 1.25, 1.5), 0.9, 0.8, 0.24, (0.2, 0.25, 0)),
             ("rect", (4.08, -1.25, 1.5), 0.9, 0.8, 0.24, (-0.2, 0.25, 0))],
    nozzles=[dict(loc=(-8.0, 1.25, 1.5), r_front=0.55, r_exit=0.5, length=1.1, petals=14),
             dict(loc=(-8.0, -1.25, 1.5), r_front=0.55, r_exit=0.5, length=1.1, petals=14)],
    gear=[dict(name="nose", x=3.6, y=0.0, top=1.6, radius=0.34, width=0.18, wheels=2, spread=0.3, door=(1.0, 0.5),
               light=True),
          dict(name="main_L", x=-2.6, y=2.0, top=1.5, radius=0.5, width=0.26, door=(1.3, 0.6), rake=0.1),
          dict(name="main_R", x=-2.6, y=-2.0, top=1.5, radius=0.5, width=0.26, door=(1.3, 0.6), rake=0.1)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(4.0, 0.5, 1.08, 1.94, 1.5, 3.5), (0.0, 0.56, 1.05, 1.98, 1.5, 3.5),
                                (-5.0, 0.56, 1.05, 1.98, 1.5, 3.0), (-8.0, 0.54, 0.98, 2.02, 1.5)],
               m["paint"], hull, seg=20, loc=(0, side * 1.25, 0))
    A.sensor_ball("irst", (7.75, 0.3, 2.46), 0.14, m, hull)
    box("antiglare", (1.0, 0.7, 0.03), (8.4, 0, 2.36), m["dark"], hull, rot=(0, 0.2, 0), lods=MID)
    # The photo's markings: the red star on the fins' outer faces and under
    # the wings, the blue 054 on the intakes.
    fin_n, fin_up = (0, math.cos(0.45), -math.sin(0.45)), (0, math.sin(0.45), math.cos(0.45))
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=(-6.6, 1.6 + 1.3 * fin_up[1] + 0.3, 2.4 + 1.3 * fin_up[2]),
                          normal=fin_n, up=fin_up, size=0.32, onto=("tail_fin_",))),
        *[("insignia", dict(kind="ru_star", centre=(-0.5, y, WING_Z - 0.4), normal=(0, 0, -1), up=(1, 0, 0),
                            size=0.75, onto=("wing_",), mirror=False)) for y in (4.6, -4.6)],
        ("text", dict(text="054", height=0.42, centre=(4.6, 2.2, 2.05), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage", "nacelle_"), colour="blue")),
    ])
    box("main_bay", (4.4, 1.2, 0.03), (0.5, 0, 1.69), m["dark"], hull, lods=MID)


def wreck(variant, v):
    A.crash(v, tail_x=-5.4, wing_y=4.4, wing_side=1, tail_yaw=0.28, seed=57)


run_disabled("su57", CARDS, "russian_air_grey", build, wreck)
