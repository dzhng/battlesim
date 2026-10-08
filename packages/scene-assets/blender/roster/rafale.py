"""Dassault Rafale C (F4), from assets/references/rafale/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/rafale.py -- [--wreck]

What the photos settle: the canards mounted high, above and behind the two
crescent intakes set low under the cockpit, the fixed refuelling probe on the
right of the nose, the cranked delta wing, one tall swept fin, two M88
nozzles; wing-tip missiles, tanks and Meteors under the wing. Mid grey.

Dimensions stated from the references (Dassault figures): length 15.27 m,
span 10.80 m, height 5.34 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"europe_rafale_c_f4": (15.27, 10.8, 5.34)}
WING_Z = 1.85
SPEC = dict(
    fuselage=[(7.63, 0.0, 2.0, 2.0), (6.9, 0.36, 1.75, 2.26, 2.0), (5.8, 0.52, 1.58, 2.46, 1.95, 2.2),
              (4.3, 0.6, 1.5, 2.6, 1.92, 2.4), (2.5, 0.68, 1.46, 2.6, 1.9, 2.6), (0.0, 0.8, 1.46, 2.55, 1.9, 2.8),
              (-3.0, 0.82, 1.5, 2.45, 1.9, 2.8), (-5.6, 0.75, 1.6, 2.35, 1.92, 2.6), (-6.6, 0.7, 1.65, 2.28, 1.95)],
    # The side photo: the long canopy from 2.4 m behind the nose, the
    # crescent intakes under its back, the canards over them, the fin and
    # both gear about a metre (the mains two) further aft than drawn before.
    bodies=[*[(f"fuselage_intake_{s}", [(2.25, 0.32, 1.12, 1.86, 1.5, 2.6), (0.5, 0.34, 1.12, 1.9, 1.52, 2.6),
                                         (-1.6, 0.0, 1.6, 1.6)], 0.78 if s == "L" else -0.78, 14) for s in "LR"],
            ("fuselage_spine", [(1.7, 0.36, 2.45, 2.88, 2.55, 2.4), (-0.8, 0.42, 2.45, 2.72, 2.55, 2.6),
                                (-4.0, 0.0, 2.4, 2.4)])],
    canopy=dict(x_front=5.3, x_back=1.6, sill=2.46, top=3.04, half_width=0.47, bows=(4.7,), peak=0.42, tail=0.7),
    canard=[(2.7, 0.78, 2.06, 1.6, 0.08), (1.7, 2.3, 2.1, 0.7, 0.03)],
    canard_hinge=(0.7,),
    wing_hinge=((0.8, 0, 1), 0.08),
    nav=dict(left=(-2.6, 5.2, WING_Z + 0.06), right=(-2.6, -5.2, WING_Z + 0.06), tail=(-7.2, 0.0, 5.2)),
    wing=[(1.3, 0.8, WING_Z, 5.5, 0.28), (-3.2, 5.24, WING_Z, 1.2, 0.05)],
    fins=[dict(root_x=-3.5, root_z=2.45, height=2.89, root_chord=3.6, tip_chord=1.0, sweep_m=2.6)],
    intakes=[("rect", (2.28, 0.78, 1.5), 0.5, 0.62, 0.2), ("rect", (2.28, -0.78, 1.5), 0.5, 0.62, 0.2)],
    nozzles=[dict(loc=(-6.6, 0.42, 1.95), r_front=0.42, r_exit=0.4, length=1.0, petals=12),
             dict(loc=(-6.6, -0.42, 1.95), r_front=0.42, r_exit=0.4, length=1.0, petals=12)],
    gear=[dict(name="nose", x=2.9, y=0.0, top=1.25, radius=0.27, width=0.15, wheels=2, spread=0.22, light=True),
          dict(name="main_L", x=-2.5, y=1.32, top=1.35, radius=0.4, width=0.2, door=(1.0, 0.55), rake=0.1),
          dict(name="main_R", x=-2.5, y=-1.32, top=1.35, radius=0.4, width=0.2, door=(1.0, 0.55), rake=0.1)],
    pylons=[((-0.4, 2.0, WING_Z - 0.06), 1.1, 0.3), ((-1.4, 3.3, WING_Z - 0.04), 1.0, 0.24)],
    stores=[("tank", (-0.5, 2.0, 1.15), 4.2, 0.38), ("missile", (-1.4, 3.3, 1.46), 3.65, 0.09),
            ("missile", (-2.4, 5.32, WING_Z), 3.1, 0.08)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The fixed refuelling probe ahead of the windscreen, right of the nose.
    cyl("probe", 0.05, 1.3, (6.3, -0.32, 2.42), "X", m["steel"], hull, seg=8, lods=MID)
    cyl("probe_base", 0.09, 0.4, (5.6, -0.32, 2.38), "X", m["paint"], hull, seg=10, lods=MID)
    box("antiglare", (1.0, 0.5, 0.03), (6.4, 0, 2.35), m["dark"], hull, rot=(0, 0.2, 0), lods=MID)
    box("osf_window", (0.3, 0.25, 0.2), (5.85, 0, 2.6), m["glass"], hull, lods=MID)
    A.blade_antenna("antenna_spine", (-1.4, 0, 2.7), 0.22, m, hull)
    # The Armée de l'air's roundels and fin title, the 30-GT code of the photo.
    A.markings(v, [
        ("insignia", dict(kind="fr", centre=(0.0, 1.2, 2.0), normal=(0, 1, 0), up=(0, 0, 1), size=0.2,
                          onto=("fuselage",))),
        ("insignia", dict(kind="fr", centre=(-1.8, 3.2, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.4,
                          onto=("wing_",))),
        ("text", dict(text="ARMEE DE L'AIR", height=0.12, centre=(-4.7, 0.3, 2.95), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_0",), colour="lowvis_dark")),
        ("text", dict(text="30-GT", height=0.22, centre=(3.3, 1.0, 1.85), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="lowvis_dark")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-3.4, wing_y=3.0, wing_side=1, tail_yaw=0.3, seed=7)


run_disabled("rafale", CARDS, "nato_air_grey", build, wreck)
