"""MiG-31BM, from assets/references/mig_31_foxhound/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/mig31.py -- [--wreck]

What the photos settle: a long heavy interceptor. A long radome, the tandem
two-seat canopy, two huge rectangular intakes along the forward fuselage, a
shoulder wing, twin fins canted a little outward over ventral fins,
stabilators, two big nozzles; offset twin-wheel main gear; four long-range
missiles under the body. Light grey.

Dimensions stated from the references (MiG figures): length 22.69 m, span
13.46 m, height 6.15 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"eastern_mig_31_foxhound_mig_31bm": (22.69, 13.46, 6.15)}
WING_Z = 3.2
FIN = dict(root_x=-5.7, root_z=3.25, height=2.93, root_chord=3.6, tip_chord=1.3, sweep_m=2.4, cant=0.14)
VENTRAL = dict(root_x=-6.4, root_z=1.95, height=0.85, root_chord=1.8, tip_chord=1.0, sweep_m=0.6, cant=2.8,
               rudder=False)
SPEC = dict(
    fuselage=[(11.35, 0.0, 2.6, 2.6), (10.4, 0.5, 2.25, 2.95, 2.6), (9.0, 0.75, 2.05, 3.25, 2.6, 2.2),
              (7.0, 0.82, 2.0, 3.42, 2.6, 2.4), (5.0, 1.5, 1.85, 3.38, 2.6, 3.8), (1.0, 1.6, 1.8, 3.32, 2.55, 4.5),
              (-4.0, 1.55, 1.85, 3.22, 2.55, 4.5), (-8.0, 1.3, 1.95, 3.06, 2.55, 4.0), (-10.2, 1.15, 2.05, 2.96, 2.55)],
    canopy=dict(x_front=8.6, x_back=5.4, sill=3.36, top=3.92, half_width=0.5, bows=(8.1, 7.1), peak=0.3, tail=0.8),
    # The flat spine the rear canopy fairs into.
    bodies=[("fuselage_spine", [(5.6, 0.42, 3.3, 3.84, 3.4, 2.4), (2.0, 0.5, 3.3, 3.58, 3.4, 2.6),
                                (-4.0, 0.0, 3.2, 3.2)])],
    wing_hinge=((0.78, 0, 1), 0.08),
    nav=dict(left=(-4.4, 6.6, WING_Z - 0.18), right=(-4.4, -6.6, WING_Z - 0.18)),
    wing=[(0.8, 1.6, WING_Z, 5.4, 0.34), (-3.3, 6.73, WING_Z - 0.2, 2.0, 0.08)],
    stab=[(-8.0, 1.5, 2.62, 3.0, 0.14), (-9.8, 4.4, 2.6, 1.3, 0.05)],
    fins=[dict(FIN, y=1.3), dict(FIN, y=-1.3), dict(VENTRAL, y=1.2), dict(VENTRAL, y=-1.2)],
    intakes=[("rect", (5.25, 1.12, 2.55), 0.9, 1.3, 0.26, (0, 0.15, 0)),
             ("rect", (5.25, -1.12, 2.55), 0.9, 1.3, 0.26, (0, 0.15, 0))],
    nozzles=[dict(loc=(-10.2, 0.62, 2.5), r_front=0.62, r_exit=0.58, length=1.15, petals=14),
             dict(loc=(-10.2, -0.62, 2.5), r_front=0.62, r_exit=0.58, length=1.15, petals=14)],
    # The nose gear under the rear seat, the mains' tandem bogies aft of
    # the wing's root (the side photo).
    gear=[dict(name="nose", x=6.5, y=0.0, top=2.0, radius=0.34, width=0.18, wheels=2, spread=0.3, door=(1.0, 0.6),
               light=True),
          dict(name="main_L", x=-2.6, y=2.1, top=1.9, radius=0.48, width=0.26, tandem=0.95, door=(1.6, 0.8)),
          dict(name="main_R", x=-2.6, y=-2.1, top=1.9, radius=0.48, width=0.26, tandem=0.95, door=(1.6, 0.8))],
    pylons=[((0.4, 3.4, WING_Z - 0.12), 1.2, 0.34)],
    stores=[("tank", (0.3, 3.4, 2.32), 4.6, 0.42), ("missile", (2.2, 0.8, 1.58), 4.2, 0.19),
            ("missile", (-3.2, 0.8, 1.62), 4.2, 0.19)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    box("antiglare", (1.2, 0.7, 0.03), (9.3, 0, 3.15), m["dark"], hull, rot=(0, 0.22, 0), lods=MID)
    box("intake_ramp_seam", (2.4, 2.6, 0.03), (4.2, 0, 3.34), m["dark"], hull, lods=MID)
    A.mirrored(lambda side, k: box(f"wing_fence_{k}", (1.6, 0.04, 0.12), (-0.4, side * 4.6, WING_Z + 0.04), m["paint"],
                                   hull, lods=MID))
    A.blade_antenna("antenna_spine", (0.0, 0, 3.58), 0.25, m, hull)
    # The photo's markings: the red star and VVS ROSSII on the fins' outer
    # faces, the red star under the wings, the blue 86 on the nose.
    fin_x, fin_z = -5.7 - 2.4 * 0.45 - 1.2, 3.25 + 2.93 * 0.5
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=(fin_x, 1.8, fin_z), normal=(0, 1, 0), up=(0, 0, 1), size=0.5,
                          onto=("tail_fin_",))),
        ("text", dict(text="ВВС РОССИИ", height=0.2, centre=(fin_x - 0.1, 1.8, fin_z - 0.75), normal=(0, 1, 0),
                      up=(0, 0, 1), onto=("tail_fin_",), colour="lowvis_dark")),
        ("text", dict(text="RF 92369", height=0.13, centre=(fin_x + 0.3, 1.8, fin_z - 0.5), normal=(0, 1, 0),
                      up=(0, 0, 1), onto=("tail_fin_",), colour="lowvis_dark")),
        *[("insignia", dict(kind="ru_star", centre=(-1.5, y, WING_Z - 0.4), normal=(0, 0, -1), up=(1, 0, 0),
                            size=0.75, onto=("wing_",), mirror=False)) for y in (4.2, -4.2)],
        ("text", dict(text="86", height=0.55, centre=(7.6, 1.2, 2.75), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="blue")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-5.8, wing_y=4.2, wing_side=1, tail_yaw=0.25, seed=31)


run_disabled("mig31", CARDS, "russian_air_grey", build, wreck)
