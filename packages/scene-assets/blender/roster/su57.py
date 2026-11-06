"""Su-57, from assets/references/su_57_felon/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/su57.py -- [--wreck]

What the photos settle: a chined nose under a long canopy, the movable
leading-edge extensions (LEVCONs) ahead of the wing root, two engine
nacelles set wide apart under a flat body with a tunnel between them, their
intakes raked under the LEVCONs, a swept wing, stabilators, all-moving fins
canted outward, the long stinger between widely spaced nozzles. Splinter
greys.

The references' dimensions (published figures): length 20.1 m,
span 14.1 m, height 4.6 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

WING_Z = 2.1
# The engines' axes, the nacelles round them and the booms outboard of them.
ENGINE_Y, ENGINE_Z, BOOM_Y = 1.5, 1.85, 2.12
# The fins stand on the booms, canted outward; the top of the fin is the
# airframe's height.
FIN_CANT, FIN_ROOT_Z = 0.45, 2.3
FIN = dict(root_x=-5.2, root_z=FIN_ROOT_Z, height=2.55, root_chord=3.0, tip_chord=1.0, sweep_m=2.0, cant=FIN_CANT,
           rudder=False)
SPEC = dict(
    # The side photo: the nose droops to its tip under a chine, widest low;
    # behind the cockpit the body flattens and spreads under the LEVCONs to
    # the wing roots, and narrows aft between the engines to the stinger.
    fuselage=[(10.0, 0.0, 1.66, 1.66), (9.3, 0.42, 1.52, 1.92, 1.64, 1.4), (8.4, 0.76, 1.42, 2.17, 1.64, 1.5),
              (7.4, 0.98, 1.38, 2.42, 1.7, 1.6), (6.2, 1.06, 1.38, 2.52, 1.85, 1.8),
              (5.0, 1.12, 1.42, 2.54, 2.0, 2.1), (3.5, 1.4, 1.5, 2.52, 2.06, 2.1), (1.5, 1.62, 1.58, 2.5, 2.08, 2.1),
              (-2.0, 1.6, 1.62, 2.46, 2.08, 2.1), (-5.0, 1.35, 1.7, 2.42, 2.05, 2.2),
              (-7.5, 0.75, 1.68, 2.25, 1.95, 2.4), (-8.8, 0.4, 1.7, 2.08, 1.88, 2.0), (-10.0, 0.0, 1.88, 1.88)],
    # The side photo: the windscreen 2.6 m behind the nose, the canopy 3 m
    # long and highest past its middle, the nose gear under the intakes'
    # mouths.
    canopy=dict(x_front=7.4, x_back=4.3, sill=2.45, top=3.28, half_width=0.55, bows=(6.75,), peak=0.55, tail=0.7),
    # The dorsal hump behind the canopy: broad and low, down to the engines.
    bodies=[("fuselage_spine", [(4.5, 0.85, 2.3, 2.88, 2.4, 2.0), (2.0, 1.2, 2.3, 2.7, 2.38, 2.0),
                                (-1.5, 1.2, 2.3, 2.6, 2.36, 2.0), (-5.0, 0.7, 2.3, 2.5, 2.36, 2.0),
                                (-7.0, 0.0, 2.36, 2.36)]),
            *[(f"fuselage_boom_{k}", [(-2.5, 0.42, 1.96, 2.26, 2.1, 5.0), (-7.2, 0.42, 1.96, 2.33, 2.12, 5.0),
                                      (-8.4, 0.2, 2.02, 2.2, 2.1)], y, 16)
              for k, y in (("L", BOOM_Y), ("R", -BOOM_Y))]],
    wing_hinge=((0.8, 0, 1), 0.08),
    nav=dict(left=(-2.8, 6.95, WING_Z - 0.04), right=(-2.8, -6.95, WING_Z - 0.04)),
    # The LEVCONs: from the cockpit's sides out to the wing's leading edge,
    # their trailing edge along it.
    strake=[(6.6, 0.85, WING_Z + 0.03, 1.9, 0.07), (3.1, 2.75, WING_Z, 0.45, 0.04)],
    wing=[(4.15, 1.5, WING_Z, 7.55, 0.36), (-2.0, 7.05, WING_Z - 0.06, 1.9, 0.06)],
    # The top and detail photos: the all-moving tailplanes pivot on the
    # booms under the fins and reach back past the nozzles.
    stab=[(-4.6, 2.45, 1.98, 3.6, 0.12), (-7.9, 5.0, 1.95, 1.7, 0.05)],
    fins=[dict(FIN, y=BOOM_Y), dict(FIN, y=-BOOM_Y)],
    intakes=[("rect", (4.1, ENGINE_Y, 1.55), 1.0, 1.0, 0.24, (0.15, 0.3, 0)),
             ("rect", (4.1, -ENGINE_Y, 1.55), 1.0, 1.0, 0.24, (-0.15, 0.3, 0))],
    # The rear photo: the nozzles wide apart at the wing's height, the
    # stinger reaching well past them.
    nozzles=[dict(loc=(-7.6, y, ENGINE_Z), r_front=0.52, r_exit=0.47, length=1.15, petals=14)
             for y in (ENGINE_Y, -ENGINE_Y)],
    gear=[dict(name="nose", x=3.6, y=0.0, top=1.6, radius=0.34, width=0.18, wheels=2, spread=0.3, door=(1.0, 0.5),
               light=True),
          dict(name="main_L", x=-2.6, y=2.12, top=1.5, radius=0.5, width=0.26, door=(1.3, 0.6), rake=0.1),
          dict(name="main_R", x=-2.6, y=-2.12, top=1.5, radius=0.5, width=0.26, door=(1.3, 0.6), rake=0.1)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The nacelles: boxy under the intakes, round and rising aft to the
    # nozzles, the tunnel between them under the flat body.
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(4.05, 0.52, 1.02, 2.06, 1.55, 3.5), (0.0, 0.6, 1.0, 2.06, 1.53, 3.5),
                                (-4.0, 0.6, 1.08, 2.12, 1.6, 3.0), (-6.5, 0.56, 1.28, 2.32, 1.8, 2.4),
                                (-7.6, 0.53, ENGINE_Z - 0.52, ENGINE_Z + 0.52, ENGINE_Z)],
               m["paint"], hull, seg=20, loc=(0, side * ENGINE_Y, 0))
    # The IRST ball ahead of the windscreen, offset to starboard.
    A.sensor_ball("irst", (7.75, -0.3, 2.42), 0.14, m, hull)
    # The photo's markings: the red star on the fins' outer faces and under
    # the wings, the blue 054 on the body's side behind the canopy.
    fin_n = (0, math.cos(FIN_CANT), -math.sin(FIN_CANT))
    fin_up = (0, math.sin(FIN_CANT), math.cos(FIN_CANT))
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=(-6.9, BOOM_Y + 1.5 * fin_up[1] + 0.3, FIN_ROOT_Z + 1.5 * fin_up[2]),
                          normal=fin_n, up=fin_up, size=0.32, onto=("tail_fin_",))),
        *[("insignia", dict(kind="ru_star", centre=(-0.5, y, WING_Z - 0.4), normal=(0, 0, -1), up=(1, 0, 0),
                            size=0.75, onto=("wing_",), mirror=False)) for y in (4.6, -4.6)],
        ("text", dict(text="054", height=0.3, centre=(3.9, 2.6, 2.32), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="blue")),
    ])
    box("main_bay", (4.4, 1.2, 0.03), (0.5, 0, 1.585), m["dark"], hull, lods=MID)


def wreck(variant, v):
    A.crash(v, tail_x=-5.4, wing_y=4.4, wing_side=1, tail_yaw=0.28, seed=57)


run("su_57_felon", "russian_air_grey", build, wreck, script="su57.py")
