"""J-20's guide: the lofted model, measured from its three-view, whose
renders the skin (`roster/j20.py`) was generated from. Never shipped:

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/guides/j20.py -- --variant=<id> --out=<path> --no-lint

What the photos settle: a long chined body, the canopy far forward, the
diverterless intakes either side of the cockpit, canards ahead of the
leading-edge extensions and the delta wing, all-moving fins canted outward,
two ventral fins, two nozzles; weapon bays under the body. Dark grey.

The references' dimensions: about 21.2 m long, 13.01 m span and
4.69 m high (published estimates; no official figures).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

# Measured from Kaboldy's three-view (`eastern_j_20_mighty_dragon_stealth_fighter-
# top-3.png` in its references, CC BY-SA 3.0, a measurement reference only) at one scale,
# its length nose to tail (no pitot) the frame's 20.82 m, its fins' top 4.70 m
# over its ground line (the published 4.69 m): the body's outline station by
# station from the side and the plan, the canards, extensions, wing and the
# fins (canted 34 degrees, as the front view shows them) from the plan, the
# big caret intakes either side of the cockpit from the front.
WING_Z = 2.05
FIN = dict(root_x=-6.75, root_z=2.8, height=2.04, root_chord=2.7, tip_chord=1.35, sweep_m=2.0, cant=0.59, rudder=False)
VENTRAL = dict(root_x=-6.9, root_z=1.2, height=0.72, root_chord=1.5, tip_chord=0.8, sweep_m=0.5, cant=2.79,
               rudder=False)
SPEC = dict(
    fuselage=[(10.6, 0.0, 2.22, 2.22), (9.8, 0.34, 1.83, 2.42, 2.1, 1.6), (9.2, 0.53, 1.67, 2.54, 2.05, 1.6),
              (8.0, 0.73, 1.45, 2.8, 2.0, 1.7), (6.8, 0.85, 1.32, 2.9, 2.0, 1.8), (5.6, 1.02, 1.23, 2.95, 2.0, 2.0),
              (4.4, 1.76, 1.18, 2.95, 2.0, 2.6), (2.6, 1.86, 1.11, 2.95, 2.0, 3.0), (-1.0, 1.88, 1.04, 2.93, 2.0, 3.2),
              (-4.0, 2.15, 1.06, 2.85, 2.05, 3.2), (-6.4, 2.3, 1.09, 2.76, 2.1, 3.0), (-8.2, 2.0, 1.16, 2.4, 2.1, 2.6),
              (-9.6, 1.6, 1.4, 2.25, 2.0)],
    # The canopy: its windscreen's foot at 7.9 m, its peak 3.51 m over 6.2, its glass ending on the spine by 4.6.
    canopy=dict(x_front=7.9, x_back=4.6, sill=2.8, top=3.51, half_width=0.55, bows=(7.3,), peak=0.52, tail=0.7),
    canard_hinge=(0.75,),
    wing_hinge=((0.8, 0, 1), 0.08),
    nav=dict(left=(-6.6, 6.8, WING_Z - 0.02), right=(-6.6, -6.8, WING_Z - 0.02)),
    # The plan: the canards from the intakes' lips aft to the extensions, the
    # extensions blending into the wing's root, the wing's leading edge swept
    # 46 degrees to its cropped tip, its trailing edge swept forward to it.
    canard=[(4.6, 1.75, 2.45, 3.6, 0.07), (1.7, 3.97, 2.4, 0.8, 0.03)],
    strake=[(1.2, 1.88, 2.1, 2.6, 0.06), (-1.2, 2.5, 2.06, 0.4, 0.04)],
    wing=[(-1.0, 2.2, WING_Z, 7.5, 0.3), (-5.6, 6.87, WING_Z - 0.05, 1.7, 0.05)],
    fins=[dict(FIN, y=2.15), dict(FIN, y=-2.15), dict(VENTRAL, y=1.35), dict(VENTRAL, y=-1.35)],
    # The caret intakes, a metre wide and 1.2 m tall, beside the cockpit.
    intakes=[("rect", (5.2, 1.05, 2.05), 1.0, 1.2, 0.3, (0.2, 0.3, -0.15)),
             ("rect", (5.2, -1.05, 2.05), 1.0, 1.2, 0.3, (-0.2, 0.3, 0.15))],
    nozzles=[dict(loc=(-8.4, 0.72, 1.95), r_front=0.55, r_exit=0.5, length=1.1, petals=14),
             dict(loc=(-8.4, -0.72, 1.95), r_front=0.55, r_exit=0.5, length=1.1, petals=14)],
    gear=[dict(name="nose", x=6.0, y=0.0, top=1.32, radius=0.3, width=0.17, door=(1.0, 0.5), light=True),
          dict(name="main_L", x=-2.0, y=1.6, top=1.12, radius=0.48, width=0.24, door=(1.3, 0.55), rake=0.1),
          dict(name="main_R", x=-2.0, y=-1.6, top=1.12, radius=0.48, width=0.24, door=(1.3, 0.55), rake=0.1)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The photo's markings: the PLA's star on the fins' outer faces, the
    # star and bars under the wings, the white 2011 on the nose.
    cant = FIN["cant"]
    fin_n, fin_up = (0, math.cos(cant), -math.sin(cant)), (0, math.sin(cant), math.cos(cant))
    A.markings(v, [
        ("insignia", dict(kind="cn_fin", centre=(-8.2, 2.15 + 1.0 * fin_up[1] + 0.3, 2.8 + 1.0 * fin_up[2]),
                          normal=fin_n, up=fin_up, size=0.38, onto=("tail_fin_0", "tail_fin_1"))),
        *[("insignia", dict(kind="cn_star", centre=(-5.0, y, WING_Z - 0.3), normal=(0, 0, -1), up=(1, 0, 0),
                            size=0.7, onto=("wing_",), mirror=False)) for y in (4.0, -4.0)],
        ("text", dict(text="2011", height=0.36, centre=(7.5, 1.3, 1.95), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="white")),
    ])
    box("main_bay", (4.6, 1.4, 0.03), (0.6, 0, 1.04), m["dark"], hull, lods=MID)
    A.mirrored(lambda side, k: box(f"side_bay_{k}", (1.8, 0.03, 0.35), (3.2, side * 1.86, 1.6), m["dark"], hull,
                                   lods=MID))


def wreck(variant, v):
    A.crash(v, tail_x=-5.4, wing_y=4.2, wing_side=-1, tail_yaw=-0.28, seed=20)


run("j_20_mighty_dragon", "chinese_air_grey", build, wreck, script="j20.py")
