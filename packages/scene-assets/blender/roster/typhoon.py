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
# The side photo (an RAF jet taxiing): the body deep, its spine 3.1 m up and
# its intake's lip 1.1 m off the ground; the long canopy 2.5 m back from the
# nose, the canards under its front, both gear well aft of the old stations.
WING_Z = 2.0
SPEC = dict(
    fuselage=[(7.98, 0.0, 2.2, 2.2), (7.2, 0.38, 1.95, 2.5, 2.2), (6.0, 0.58, 1.8, 2.78, 2.25, 2.2),
              (4.4, 0.66, 1.75, 2.86, 2.3, 2.4), (2.6, 0.76, 1.7, 3.0, 2.35, 2.6), (0.0, 0.88, 1.62, 3.12, 2.4, 2.8),
              (-3.0, 0.88, 1.62, 3.04, 2.4, 2.8), (-5.6, 0.8, 1.72, 2.9, 2.35, 2.6), (-6.95, 0.72, 1.78, 2.74, 2.3)],
    bodies=[("fuselage_intake", [(3.15, 0.6, 1.1, 1.71, 1.41, 4.0), (1.8, 0.64, 1.13, 1.77, 1.45, 4.0),
                                 (-0.3, 0.62, 1.25, 1.85, 1.55, 3.0), (-2.3, 0.5, 1.5, 1.9, 1.7, 2.4)], 0.0, 16),
            ("fuselage_spine", [(2.1, 0.4, 2.85, 3.45, 2.95, 2.4), (-0.5, 0.46, 2.9, 3.24, 3.0, 2.6),
                                (-4.4, 0.0, 2.9, 2.9)])],
    canopy=dict(x_front=5.45, x_back=2.0, sill=2.8, top=3.62, half_width=0.5, bows=(4.85,), peak=0.42, tail=0.75),
    canard=[(5.3, 0.62, 2.3, 1.3, 0.08), (4.45, 2.1, 2.28, 0.55, 0.03)],
    wing=[(1.6, 0.86, WING_Z, 6.4, 0.3), (-3.8, 5.32, WING_Z, 1.3, 0.05)],
    wing_hinge=((0.82, 0, 1),),
    fins=[dict(root_x=-4.6, root_z=2.92, height=2.36, root_chord=3.2, tip_chord=1.0, sweep_m=2.38)],
    intakes=[("rect", (3.18, 0.0, 1.41), 1.1, 0.5, 0.22, (0, -0.1, 0))],
    nozzles=[dict(loc=(-6.95, 0.42, 2.25), r_front=0.45, r_exit=0.42, length=1.0, petals=12),
             dict(loc=(-6.95, -0.42, 2.25), r_front=0.45, r_exit=0.42, length=1.0, petals=12)],
    gear=[dict(name="nose", x=2.4, y=0.0, top=1.15, radius=0.26, width=0.16, door=(0.7, 0.4), light=True),
          dict(name="main_L", x=-2.0, y=1.2, top=1.6, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1),
          dict(name="main_R", x=-2.0, y=-1.2, top=1.6, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1)],
    pylons=[((-1.0, 2.0, WING_Z - 0.06), 1.1, 0.3), ((-1.8, 3.3, WING_Z - 0.04), 1.0, 0.24)],
    stores=[("tank", (-1.1, 2.0, 1.28), 4.0, 0.36), ("missile", (-1.8, 3.3, 1.6), 3.65, 0.09),
            ("pod", (-3.0, 5.42, WING_Z), 1.6, 0.12), ("missile", (0.4, 0.55, 1.42), 3.65, 0.09, False),
            ("missile", (-2.6, 0.55, 1.5), 3.65, 0.09, False)],
    nav=dict(left=(-3.2, 5.2, WING_Z + 0.04), right=(-3.2, -5.2, WING_Z + 0.04), tail=(-7.98, 0.0, 5.0)),
)
# The RAF's low-visibility roundels and fin flash, as the side photo.
MARKINGS = [
    ("insignia", dict(kind="uk_lowvis", centre=(0.9, 1.2, 2.6), normal=(0, 1, 0), up=(0, 0, 1), size=0.32,
                      onto=("fuselage",))),
    ("insignia", dict(kind="uk_flash", centre=(-6.3, 0.3, 4.4), normal=(0, 1, 0), up=(0, 0, 1), size=0.22,
                      onto=("tail_fin_0",))),
    ("insignia", dict(kind="uk_lowvis", centre=(-2.0, 3.8, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0), size=0.45,
                      onto=("wing_",))),
    ("text", dict(text="ZK353", height=0.2, centre=(-4.2, 1.2, 2.5), normal=(0, 1, 0), up=(0, 0, 1),
                  onto=("fuselage",))),
]


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    box("intake_splitter", (0.5, 0.04, 0.5), (3.0, 0, 1.45), m["paint"], hull, lods=MID)
    box("antiglare", (1.1, 0.55, 0.03), (6.0, 0, 2.78), m["dark"], hull, rot=(0, 0.12, 0), lods=MID)
    box("probe_fairing", (1.2, 0.12, 0.12), (5.0, -0.56, 2.74), m["paint"], hull, bevel=0.03, lods=MID)
    A.blade_antenna("antenna_spine", (-1.2, 0, 3.18), 0.22, m, hull)
    A.markings(v, MARKINGS)


def wreck(variant, v):
    A.crash(v, tail_x=-3.6, wing_y=3.2, wing_side=1, tail_yaw=0.28, seed=3)


run_disabled("typhoon", CARDS, "nato_air_grey", build, wreck)
