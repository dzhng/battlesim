"""Su-57's guide: the lofted model, measured from its three-view, whose
renders the skin (`roster/su57.py`) is generated from. Never shipped:

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/guides/su57.py -- --variant=<id> --out=<path> --no-lint

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

# Measured from XaHyMaH's two-view of the T-50 (`eastern_su_57_felon_stealth_
# multirole_fighter-top-2.png` in its references, CC BY-SA 3.0, a measurement
# reference only) at one scale, 20 m nose to stinger: the fin tops at 4.6 m,
# the nacelles' bellies 1.3 m off the ground.
WING_Z = 2.2
# The engines' axes and the tail booms outboard of them, under the fins.
ENGINE_Y, ENGINE_Z, BOOM_Y = 1.45, 2.17, 2.4
# The fins stand on the booms, canted outward 0.9 m over their 1.76 m rise.
FIN_CANT, FIN_ROOT_Z = 0.47, 2.8
FIN = dict(root_x=-4.75, root_z=FIN_ROOT_Z, height=2.0, root_chord=3.0, tip_chord=0.65, sweep_m=1.7, cant=FIN_CANT,
           rudder=False)
SPEC = dict(
    # The side view: the nose droops to its tip on the chine line, the body
    # flat beneath; behind the canopy the top falls straight to the booms
    # while the body spreads flat under the LEVCONs to the wing roots, its
    # crown broad rather than a ridge, and narrows aft to the stinger.
    fuselage=[(10.0, 0.0, 2.17, 2.17), (9.6, 0.18, 2.0, 2.3, 2.18, 2.0), (9.2, 0.31, 1.9, 2.42, 2.19, 2.0),
              (8.6, 0.48, 1.79, 2.58, 2.21, 2.0), (8.0, 0.61, 1.73, 2.74, 2.23, 2.1),
              (7.4, 0.71, 1.71, 2.86, 2.24, 2.2), (6.2, 0.84, 1.72, 2.88, 2.24, 2.3),
              (5.0, 0.92, 1.74, 2.92, 2.24, 2.4), (4.0, 1.05, 1.76, 3.2, 2.24, 2.4),
              (3.2, 1.3, 1.77, 3.28, 2.24, 2.4), (2.0, 1.55, 1.79, 3.16, 2.24, 2.4),
              (0.8, 1.7, 1.76, 3.04, 2.22, 2.5), (-0.4, 1.75, 1.74, 2.92, 2.2, 2.5),
              (-1.6, 1.75, 1.76, 2.8, 2.2, 2.5), (-3.5, 1.65, 1.8, 2.76, 2.2, 2.5),
              (-5.0, 1.4, 1.85, 2.68, 2.22, 2.5), (-6.6, 1.0, 1.9, 2.62, 2.24, 2.4),
              (-7.7, 0.74, 1.95, 2.58, 2.26, 2.2), (-8.6, 0.59, 2.0, 2.57, 2.3, 2.0),
              (-9.4, 0.36, 2.15, 2.5, 2.33, 2.0), (-10.0, 0.0, 2.36, 2.36)],
    # The side and top views: the windscreen 2.5 m behind the nose, the
    # canopy 3.3 m long and highest 1.9 m back, faired into the spine.
    canopy=dict(x_front=7.5, x_back=4.2, sill=2.85, top=3.64, half_width=0.56, bows=(6.0,), peak=0.58, tail=0.7),
    # The tail booms: from a point ahead of the fins back past the nozzles,
    # the stabilators' pivots under them.
    bodies=[(f"fuselage_boom_{k}", [(-3.0, 0.0, 2.6, 2.6), (-4.6, 0.18, 2.2, 2.95, 2.5, 2.5),
                                    (-7.0, 0.16, 2.2, 2.92, 2.5, 2.5), (-8.3, 0.14, 2.3, 2.85, 2.55, 2.2),
                                    (-8.9, 0.0, 2.55, 2.55)], y, 16)
            for k, y in (("L", BOOM_Y), ("R", -BOOM_Y))],
    wing_hinge=((0.8, 0, 1), 0.08),
    nav=dict(left=(-4.9, 7.05, WING_Z - 0.06), right=(-4.9, -7.05, WING_Z - 0.06)),
    # The LEVCONs: from the chine 4 m behind the nose out to the wing's
    # leading edge, their outer edge barely swept.
    strake=[(4.0, 1.0, WING_Z + 0.04, 4.5, 0.1), (2.22, 2.52, WING_Z + 0.02, 2.6, 0.06), (0.2, 3.1, WING_Z, 0.6, 0.03)],
    # The wing: 49 degrees at the leading edge from the LEVCONs' tips, its
    # root a flat glove from the dorsal hump to the booms, a cropped tip, the
    # trailing edge almost straight across.
    wing=[(1.6, 1.0, WING_Z + 0.04, 8.4, 0.36), (0.6, 2.45, WING_Z, 7.28, 0.3), (-0.07, 3.17, WING_Z, 6.48, 0.25),
          (-3.77, 6.42, WING_Z - 0.05, 2.2, 0.08), (-4.55, 7.1, WING_Z - 0.06, 0.92, 0.05)],
    # The top view: the all-moving tailplanes pivot on the booms and reach
    # back past the nozzles, their trailing edges notched forward.
    stab=[(-4.94, 2.45, 2.3, 4.26, 0.12), (-7.9, 5.05, 2.25, 0.88, 0.04)],
    fins=[dict(FIN, y=BOOM_Y), dict(FIN, y=-BOOM_Y)],
    # The side view: the mouths raked steeply, top lip forward under the
    # LEVCONs.
    intakes=[("rect", (1.5, y, 1.78), 1.0, 0.8, 0.3, (0, 0.8, 0)) for y in (ENGINE_Y, -ENGINE_Y)],
    # The rear: the nozzles wide apart, the stinger reaching past them.
    nozzles=[dict(loc=(-7.6, y, ENGINE_Z), r_front=0.5, r_exit=0.42, length=1.15, petals=14)
             for y in (ENGINE_Y, -ENGINE_Y)],
    gear=[dict(name="nose", x=3.6, y=0.0, top=1.76, radius=0.34, width=0.18, wheels=2, spread=0.3, door=(1.0, 0.5),
               light=True),
          dict(name="main_L", x=-2.4, y=2.2, top=1.5, radius=0.5, width=0.26, door=(1.3, 0.6), rake=0.1),
          dict(name="main_R", x=-2.4, y=-2.2, top=1.5, radius=0.5, width=0.26, door=(1.3, 0.6), rake=0.1)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The nacelles: boxy under the intakes, round and rising aft to the
    # nozzles, the tunnel between them under the flat body.
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(1.9, 0.5, 1.5, 2.2, 1.85, 3.5), (0.6, 0.56, 1.33, 2.2, 1.8, 3.5),
                                (-0.4, 0.58, 1.31, 2.25, 1.8, 3.5), (-2.2, 0.58, 1.4, 2.3, 1.85, 3.2),
                                (-4.6, 0.56, 1.52, 2.4, 1.95, 3.0), (-6.6, 0.53, 1.6, 2.6, 2.1, 2.6),
                                (-7.6, 0.5, ENGINE_Z - 0.5, ENGINE_Z + 0.5, ENGINE_Z)],
               m["paint"], hull, seg=20, loc=(0, side * ENGINE_Y, 0))
    # The IRST ball ahead of the windscreen, offset to starboard.
    A.sensor_ball("irst", (7.85, -0.3, 2.72), 0.14, m, hull)
    # The photo's markings: the red star on the fins' outer faces and under
    # the wings, the blue 054 on the body's side behind the canopy.
    fin_n = (0, math.cos(FIN_CANT), -math.sin(FIN_CANT))
    fin_up = (0, math.sin(FIN_CANT), math.cos(FIN_CANT))
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=(-6.3, BOOM_Y + 1.1 * fin_up[1] + 0.3, FIN_ROOT_Z + 1.1 * fin_up[2]),
                          normal=fin_n, up=fin_up, size=0.32, onto=("tail_fin_",))),
        *[("insignia", dict(kind="ru_star", centre=(-3.4, y, WING_Z - 0.4), normal=(0, 0, -1), up=(1, 0, 0),
                            size=0.75, onto=("wing_",), mirror=False)) for y in (4.6, -4.6)],
        ("text", dict(text="054", height=0.3, centre=(3.4, 1.9, 2.5), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="blue")),
    ])
    box("main_bay", (4.4, 1.2, 0.03), (-1.5, 0, 1.735), m["dark"], hull, lods=MID)


def wreck(variant, v):
    A.crash(v, tail_x=-4.6, wing_y=4.4, wing_side=1, tail_yaw=0.28, seed=57)


run("su_57_felon", "russian_air_grey", build, wreck, script="su57.py")
