"""J-10C, from assets/references/j_10_vigorous_dragon/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/j10.py -- [--wreck]

What the photos settle: a single-engine canard delta. The pointed nose and
bubble canopy, the chin intake under the cockpit (the C's diverterless bump,
where the A had a splitter plate), canards just behind the cockpit, the delta
wing, one tall fin, two ventral fins canted outward under the tail, one
nozzle. Light grey.

Dimensions stated from the references (published figures): length 16.9 m,
span 9.75 m, height 5.43 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"eastern_j_10_vigorous_dragon_j_10c": (16.9, 9.75, 5.43)}
WING_Z = 1.85
VENTRAL = dict(root_x=-4.2, root_z=1.75, height=0.7, root_chord=1.6, tip_chord=0.9, sweep_m=0.5, cant=2.7,
               rudder=False)
SPEC = dict(
    fuselage=[(8.45, 0.0, 2.15, 2.15), (7.6, 0.38, 1.88, 2.42, 2.15), (6.4, 0.55, 1.7, 2.65, 2.12, 2.2),
              (5.0, 0.6, 1.65, 2.8, 2.1, 2.4), (3.2, 0.7, 1.7, 2.78, 2.1, 2.6), (0.0, 0.85, 1.65, 2.7, 2.1, 2.8),
              (-3.0, 0.8, 1.7, 2.6, 2.1, 2.6), (-5.6, 0.66, 1.8, 2.46, 2.12, 2.4), (-6.8, 0.6, 1.85, 2.4, 2.12)],
    bodies=[("fuselage_intake", [(4.45, 0.62, 0.98, 1.66, 1.32, 3.0), (3.0, 0.72, 0.96, 1.72, 1.34, 3.0),
                                 (0.5, 0.7, 1.1, 1.8, 1.45, 2.8), (-1.5, 0.5, 1.4, 1.85, 1.62, 2.4)], 0.0, 16),
            ("fuselage_dsi", [(4.3, 0.0, 1.5, 1.5), (3.8, 0.26, 1.42, 1.66, 1.5), (3.0, 0.0, 1.55, 1.55)], 0.0, 16),
            # The spine the canopy fairs into, back to the fin.
            ("fuselage_spine", [(4.2, 0.36, 2.7, 3.18, 2.8, 2.4), (1.0, 0.42, 2.65, 2.94, 2.75, 2.6),
                                (-3.2, 0.0, 2.6, 2.6)])],
    canopy=dict(x_front=6.5, x_back=4.0, sill=2.76, top=3.34, half_width=0.46, bows=(5.85,), peak=0.45, tail=0.75),
    canard_hinge=(0.7,),
    wing_hinge=((0.8, 0, 1), 0.08),
    nav=dict(left=(-3.3, 4.75, WING_Z + 0.05), right=(-3.3, -4.75, WING_Z + 0.05), tail=(-8.3, 0.0, 5.3)),
    canard=[(5.0, 0.62, 2.0, 1.4, 0.08), (4.1, 2.4, 2.06, 0.55, 0.03)],
    wing=[(2.0, 0.85, WING_Z, 6.6, 0.3), (-3.8, 4.84, WING_Z, 1.0, 0.05)],
    fins=[dict(root_x=-3.0, root_z=2.6, height=2.83, root_chord=3.8, tip_chord=1.2, sweep_m=2.9),
          dict(VENTRAL, y=0.55), dict(VENTRAL, y=-0.55)],
    intakes=[("rect", (4.48, 0.0, 1.3), 1.12, 0.56, 0.22, (0, -0.1, 0))],
    nozzles=[dict(loc=(-6.8, 0.0, 2.12), r_front=0.56, r_exit=0.5, length=1.25, petals=14)],
    gear=[dict(name="nose", x=5.0, y=0.0, top=1.0, radius=0.26, width=0.15, door=(0.7, 0.4), light=True),
          dict(name="main_L", x=-0.8, y=1.3, top=1.5, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1),
          dict(name="main_R", x=-0.8, y=-1.3, top=1.5, radius=0.38, width=0.2, door=(1.0, 0.55), rake=0.1)],
    pylons=[((-0.6, 2.2, WING_Z - 0.06), 1.1, 0.3), ((-1.4, 3.3, WING_Z - 0.04), 0.9, 0.24)],
    stores=[("tank", (-0.7, 2.2, 1.14), 4.0, 0.36), ("missile", (-1.4, 3.3, 1.46), 3.7, 0.1),
            ("missile", (-2.6, 4.3, 1.5), 2.9, 0.08)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    box("antiglare", (1.0, 0.5, 0.03), (7.0, 0, 2.5), m["dark"], hull, rot=(0, 0.22, 0), lods=MID)
    A.blade_antenna("antenna_spine", (0.2, 0, 2.94), 0.22, m, hull)
    # The PLA's red star and bars aft on the body and on the wings, the
    # photo's red 05 on the nose and the fin.
    A.markings(v, [
        ("insignia", dict(kind="cn_star", centre=(-3.9, 1.2, 2.15), normal=(0, 1, 0), up=(0, 0, 1), size=0.6,
                          onto=("fuselage",))),
        ("insignia", dict(kind="cn_star", centre=(-1.4, 3.0, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0),
                          size=0.55, onto=("wing_",))),
        ("text", dict(text="05", height=0.42, centre=(6.6, 1.0, 2.2), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="red")),
        ("text", dict(text="05", height=0.4, centre=(-5.6, 0.3, 4.6), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("tail_fin_0",), colour="red")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-3.4, wing_y=3.0, wing_side=1, tail_yaw=0.3, seed=10)


run_disabled("j10", CARDS, "chinese_air_grey", build, wreck)
