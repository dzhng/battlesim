"""J-20, from assets/references/j_20_mighty_dragon/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/j20.py -- [--wreck]

What the photos settle: a long chined body, the canopy far forward, the
diverterless intakes either side of the cockpit, canards ahead of the
leading-edge extensions and the delta wing, all-moving fins canted outward,
two ventral fins, two nozzles; weapon bays under the body. Dark grey.

Dimensions stated from the references: about 21.2 m long, 13.01 m span and
4.69 m high (published estimates; no official figures).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"eastern_j_20_mighty_dragon_stealth_fighter": (21.2, 13.01, 4.69)}
WING_Z = 2.0
FIN = dict(root_x=-5.0, root_z=2.35, height=2.54, root_chord=3.0, tip_chord=1.0, sweep_m=1.9, cant=0.4, rudder=False)
VENTRAL = dict(root_x=-5.6, root_z=1.5, height=0.65, root_chord=1.6, tip_chord=0.8, sweep_m=0.5, cant=2.6,
               rudder=False)
SPEC = dict(
    fuselage=[(10.6, 0.0, 1.9, 1.9), (9.6, 0.42, 1.7, 2.12, 1.9, 1.4), (8.0, 0.8, 1.55, 2.4, 1.88, 1.5),
              (6.4, 1.0, 1.45, 2.55, 1.85, 1.7), (4.6, 1.5, 1.4, 2.5, 1.85, 2.4), (1.5, 1.8, 1.38, 2.45, 1.85, 3.2),
              (-2.5, 1.7, 1.4, 2.4, 1.85, 3.2), (-6.5, 1.3, 1.5, 2.3, 1.88, 3.0), (-8.6, 1.1, 1.55, 2.25, 1.9)],
    # The side photo: the canopy 2.9 m behind the nose and 3.3 m long.
    canopy=dict(x_front=7.7, x_back=4.4, sill=2.46, top=3.08, half_width=0.52, bows=(7.1,), peak=0.45, tail=0.7),
    bodies=[("fuselage_spine", [(4.5, 0.42, 2.35, 2.9, 2.5, 2.4), (0.5, 0.5, 2.35, 2.66, 2.5, 2.6),
                                (-5.0, 0.0, 2.3, 2.3)])],
    canard_hinge=(0.75,),
    wing_hinge=((0.8, 0, 1), 0.08),
    nav=dict(left=(-3.6, 6.4, WING_Z - 0.02), right=(-3.6, -6.4, WING_Z - 0.02)),
    canard=[(5.6, 1.4, 2.25, 2.0, 0.08), (4.0, 3.4, 2.25, 0.7, 0.03)],
    strake=[(4.0, 1.6, 2.1, 2.5, 0.06), (1.8, 2.2, 2.05, 0.4, 0.04)],
    wing=[(2.0, 1.8, WING_Z, 6.5, 0.32), (-3.6, 6.5, WING_Z - 0.04, 1.2, 0.05)],
    fins=[dict(FIN, y=1.1), dict(FIN, y=-1.1), dict(VENTRAL, y=1.0), dict(VENTRAL, y=-1.0)],
    intakes=[("rect", (5.05, 1.3, 1.85), 0.6, 0.85, 0.22, (0.2, 0.3, -0.15)),
             ("rect", (5.05, -1.3, 1.85), 0.6, 0.85, 0.22, (-0.2, 0.3, 0.15))],
    nozzles=[dict(loc=(-8.6, 0.55, 1.9), r_front=0.52, r_exit=0.48, length=1.62, petals=14),
             dict(loc=(-8.6, -0.55, 1.9), r_front=0.52, r_exit=0.48, length=1.62, petals=14)],
    gear=[dict(name="nose", x=6.0, y=0.0, top=1.5, radius=0.3, width=0.17, door=(1.0, 0.5), light=True),
          dict(name="main_L", x=-1.0, y=1.6, top=1.4, radius=0.48, width=0.24, door=(1.3, 0.55), rake=0.1),
          dict(name="main_R", x=-1.0, y=-1.6, top=1.4, radius=0.48, width=0.24, door=(1.3, 0.55), rake=0.1)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    box("antiglare", (1.0, 0.7, 0.03), (8.5, 0, 2.28), m["dark"], hull, rot=(0, 0.18, 0), lods=MID)
    # The photo's markings: the PLA's star on the fins' outer faces, the
    # star and bars under the wings, the white 2011 on the nose.
    fin_n, fin_up = (0, math.cos(0.4), -math.sin(0.4)), (0, math.sin(0.4), math.cos(0.4))
    A.markings(v, [
        ("insignia", dict(kind="cn_fin", centre=(-6.6, 1.1 + 1.4 * fin_up[1] + 0.3, 2.35 + 1.4 * fin_up[2]),
                          normal=fin_n, up=fin_up, size=0.38, onto=("tail_fin_0", "tail_fin_1"))),
        *[("insignia", dict(kind="cn_star", centre=(-1.6, y, WING_Z - 0.3), normal=(0, 0, -1), up=(1, 0, 0),
                            size=0.7, onto=("wing_",), mirror=False)) for y in (4.0, -4.0)],
        ("text", dict(text="2011", height=0.36, centre=(7.5, 1.3, 1.95), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="white")),
    ])
    box("main_bay", (4.6, 1.4, 0.03), (0.6, 0, 1.37), m["dark"], hull, lods=MID)
    A.mirrored(lambda side, k: box(f"side_bay_{k}", (1.8, 0.03, 0.35), (3.2, side * 1.62, 1.6), m["dark"], hull,
                                   lods=MID))


def wreck(variant, v):
    A.crash(v, tail_x=-5.4, wing_y=4.2, wing_side=-1, tail_yaw=-0.28, seed=20)


run_disabled("j20", CARDS, "chinese_air_grey", build, wreck)
