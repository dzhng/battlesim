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
              (-3.0, 0.82, 1.5, 2.45, 1.9, 2.8), (-5.0, 0.76, 1.56, 2.5, 1.98, 2.6), (-6.05, 0.72, 1.62, 2.42, 2.02)],
    # The side profile photo (a C taking off): the long canopy from 2.3 m
    # behind the nose, its top 3.2 m up; the crescent intakes under its
    # front half, their lips 4.7 m behind the nose; the canards high over
    # them; the fin's root leaving the spine 10 m back, its tip and fairing
    # overhanging the two nozzles, which end 0.9 m short of it.
    # Each intake on the lower side of the body, outboard of the belly, its
    # trunk faired back into the wing's root.
    bodies=[*[(f"fuselage_intake_{s}", [(2.85, 0.3, 1.36, 1.98, 1.66, 2.6), (0.9, 0.33, 1.34, 2.0, 1.66, 2.6),
                                         (-1.2, 0.28, 1.42, 1.98, 1.72, 2.4), (-3.0, 0.0, 1.8, 1.8)],
               0.86 if s == "L" else -0.86, 14) for s in "LR"],
            ("fuselage_spine", [(1.7, 0.38, 2.5, 2.98, 2.6, 2.4), (-0.8, 0.44, 2.5, 2.88, 2.6, 2.6),
                                (-4.4, 0.0, 2.55, 2.55)])],
    canopy=dict(x_front=5.35, x_back=1.7, sill=2.58, top=3.2, half_width=0.47, bows=(4.75,), peak=0.42, tail=0.7),
    canard=[(3.05, 0.78, 2.36, 1.7, 0.08), (2.0, 2.3, 2.42, 0.7, 0.03)],
    canard_hinge=(0.7,),
    wing_hinge=((0.8, 0, 1), 0.08),
    nav=dict(left=(-4.4, 5.2, WING_Z + 0.06), right=(-4.4, -5.2, WING_Z + 0.06), tail=(-7.45, 0.0, 5.1)),
    # The cranked delta: its root from behind the intakes back to the
    # nozzles, the leading edge 48 degrees back to the tip rails.
    wing=[(0.8, 0.8, WING_Z, 6.2, 0.28), (-4.1, 5.24, WING_Z, 1.2, 0.05)],
    fins=[dict(root_x=-2.4, root_z=2.75, height=2.59, root_chord=4.3, tip_chord=1.7, sweep_m=3.5)],
    intakes=[("rect", (2.92, 0.86, 1.67), 0.48, 0.6, 0.2), ("rect", (2.92, -0.86, 1.67), 0.48, 0.6, 0.2)],
    nozzles=[dict(loc=(-6.0, 0.42, 2.02), r_front=0.42, r_exit=0.4, length=0.75, petals=12),
             dict(loc=(-6.0, -0.42, 2.02), r_front=0.42, r_exit=0.4, length=0.75, petals=12)],
    gear=[dict(name="nose", x=3.05, y=0.0, top=1.3, radius=0.27, width=0.15, wheels=2, spread=0.22, light=True),
          dict(name="main_L", x=-1.8, y=1.32, top=1.35, radius=0.4, width=0.2, door=(1.0, 0.55), rake=0.1),
          dict(name="main_R", x=-1.8, y=-1.32, top=1.35, radius=0.4, width=0.2, door=(1.0, 0.55), rake=0.1)],
    pylons=[((-2.0, 2.0, WING_Z - 0.06), 1.1, 0.3), ((-3.0, 3.3, WING_Z - 0.04), 1.0, 0.24)],
    stores=[("tank", (-2.1, 2.0, 1.15), 4.2, 0.38), ("missile", (-3.0, 3.3, 1.46), 3.65, 0.09),
            ("missile", (-4.3, 5.32, WING_Z), 3.1, 0.08)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The fixed refuelling probe ahead of the windscreen, right of the nose:
    # raked up off the nose, its tip level.
    cyl("probe_base", 0.08, 0.7, (5.75, -0.32, 2.6), "X", m["paint"], hull, seg=10, rot=(0, -0.5, 0), lods=MID)
    cyl("probe", 0.045, 0.8, (6.35, -0.32, 2.76), "X", m["steel"], hull, seg=8, lods=MID)
    box("antiglare", (1.0, 0.5, 0.03), (6.4, 0, 2.35), m["dark"], hull, rot=(0, 0.2, 0), lods=MID)
    box("osf_window", (0.3, 0.25, 0.2), (5.85, 0, 2.6), m["glass"], hull, lods=MID)
    A.blade_antenna("antenna_spine", (-1.4, 0, 2.7), 0.22, m, hull)
    # The Armée de l'air's roundels and fin title, the 30-GT code of the photo.
    A.markings(v, [
        ("insignia", dict(kind="fr", centre=(1.2, 1.2, 2.2), normal=(0, 1, 0), up=(0, 0, 1), size=0.2,
                          onto=("fuselage",))),
        ("insignia", dict(kind="fr", centre=(-3.6, 2.6, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.4,
                          onto=("wing_",))),
        ("text", dict(text="ARMEE DE L'AIR", height=0.12, centre=(-4.7, 0.3, 2.95), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_0",), colour="lowvis_dark")),
        ("text", dict(text="30-GT", height=0.22, centre=(3.3, 1.0, 1.85), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="lowvis_dark")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-3.4, wing_y=3.0, wing_side=1, tail_yaw=0.3, seed=7)


run_disabled("rafale", CARDS, "nato_air_grey", build, wreck)
