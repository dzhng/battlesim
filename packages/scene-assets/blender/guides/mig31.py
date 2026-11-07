"""MiG-31BM's guide: the lofted model, measured from its three-view, whose
renders the skin (`roster/mig31.py`) is generated from. Never shipped:

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/guides/mig31.py -- --variant=<id> --out=<path> --no-lint

What the photos settle: a long heavy interceptor. A long radome, the tandem
two-seat canopy, two huge rectangular intakes along the forward fuselage, a
shoulder wing, twin fins canted a little outward over ventral fins,
stabilators, two big nozzles; offset twin-wheel main gear; four long-range
missiles under the body. Light grey.

The references' dimensions (MiG figures): length 22.69 m, span
13.46 m, height 6.15 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402

# Measured from Kaboldy's three-view (`eastern_mig_31_foxhound_mig_31bm-
# top-2.png` in its references, CC BY-SA 3.0, a measurement reference only)
# at one scale, 129.3 px/m: 22.65 m from the pitot's tip (at the frame's
# 11.35) to the nozzles, the airframe itself 21.58 m (10.3 to -11.28), the
# fins' tips at the frame's 6.16 m. The pitot is left to the roster script:
# a generator can't make a 1 m wire.
WING_Z = 3.2
# The fins on the engine bay's shoulders, 56 degrees at the leading edge,
# the trailing edge upright; the ventrals under them, splayed.
FIN = dict(root_x=-5.6, root_z=3.3, height=2.89, root_chord=4.55, tip_chord=0.72, sweep_m=4.25, cant=0.14)
VENTRAL = dict(root_x=-5.88, root_z=1.68, height=0.62, root_chord=3.56, tip_chord=0.3, sweep_m=0.95, cant=2.8,
               rudder=False)
# Each intake's raked side: its top lip 4.6 m forward of its foot.
INTAKE_TOP, INTAKE_FOOT = (4.62, 3.31), (1.76, 1.93)
SPEC = dict(
    # The side view: the long radome to the cockpit's foot, the body deep
    # and boxy from the intakes back, its belly rising aft to the nozzles.
    fuselage=[(10.3, 0.0, 2.5, 2.5), (9.6, 0.27, 2.28, 2.75, 2.5, 2.0), (9.0, 0.41, 2.17, 2.9, 2.52, 2.0),
              (8.4, 0.52, 2.08, 3.04, 2.55, 2.0), (7.8, 0.6, 2.01, 3.16, 2.58, 2.2),
              (7.2, 0.67, 1.94, 3.36, 2.62, 2.4), (6.0, 0.69, 1.89, 3.52, 2.65, 2.8),
              (4.8, 0.7, 1.86, 3.55, 2.65, 3.0), (3.0, 0.72, 1.8, 3.58, 2.6, 3.0),
              (1.5, 1.4, 1.75, 3.5, 2.6, 3.5), (0.0, 1.7, 1.7, 3.45, 2.55, 4.0),
              (-3.0, 1.7, 1.64, 3.45, 2.55, 4.0), (-6.0, 1.6, 1.55, 3.42, 2.55, 4.0),
              (-8.0, 1.5, 1.7, 3.35, 2.55, 3.6), (-10.2, 1.45, 1.79, 3.3, 2.55, 3.2)],
    # The tandem canopy from the windscreen's foot at 6.2 to the spine at
    # 2.5, highest over the rear seat; bows at the windscreen and between
    # the seats.
    canopy=dict(x_front=6.2, x_back=2.5, sill=3.45, top=4.06, half_width=0.5, bows=(5.25, 3.65), peak=0.38,
                tail=0.85),
    # The dorsal spine from the canopy back to the fins, and the intakes'
    # ducts, their sides raked from the top lip down and aft to the foot.
    bodies=[("fuselage_spine", [(2.6, 0.5, 3.4, 3.95, 3.5, 2.6), (0.0, 0.5, 3.4, 3.78, 3.5, 2.8),
                                (-4.0, 0.5, 3.4, 3.7, 3.5, 2.8), (-7.5, 0.45, 3.3, 3.62, 3.45, 2.6),
                                (-9.6, 0.0, 3.4, 3.4)]),
            *[(f"fuselage_intake_{k}", [(4.62, 0.5, 3.2, 3.36, 3.28, 4.0), (3.2, 0.53, 2.6, 3.36, 2.98, 4.0),
                                        (1.76, 0.54, 1.86, 3.36, 2.6, 4.0), (0.0, 0.5, 1.8, 3.3, 2.55, 4.0),
                                        (-2.0, 0.45, 1.8, 3.2, 2.5, 4.0)], y, 16)
              for k, y in (("L", 1.22), ("R", -1.22))]],
    wing_hinge=((0.78, 0, 1), 0.08),
    nav=dict(left=(-6.3, 6.68, WING_Z - 0.22), right=(-6.3, -6.68, WING_Z - 0.22)),
    # The top view: the root extension from the intakes' sides, then 41
    # degrees at the leading edge to a broad tip, the trailing edge swept
    # forward a little to the root.
    wing=[(0.3, 1.7, WING_Z, 6.94, 0.34), (-1.5, 2.45, WING_Z - 0.01, 5.26, 0.3),
          (-5.25, 6.71, WING_Z - 0.2, 2.14, 0.08)],
    # The stabilators' tips cropped on the diagonal.
    stab=[(-7.25, 1.55, 2.45, 3.05, 0.14), (-10.15, 3.9, 2.42, 1.07, 0.05), (-10.63, 4.28, 2.42, 0.3, 0.04)],
    fins=[dict(FIN, y=1.45), dict(FIN, y=-1.45), dict(VENTRAL, y=1.2), dict(VENTRAL, y=-1.2)],
    # Each intake's mouth on its raked side, facing forward and down.
    intakes=[("rect", ((INTAKE_TOP[0] + INTAKE_FOOT[0]) / 2 - 0.05, y, (INTAKE_TOP[1] + INTAKE_FOOT[1]) / 2), 0.95,
              3.1, 0.25, (0, 1.12, 0)) for y in (1.22, -1.22)],
    # The rear view: two big D-30F6 pipes side by side, nearly touching.
    nozzles=[dict(loc=(-10.15, y, 2.56), r_front=0.72, r_exit=0.68, length=1.13, petals=14) for y in (0.72, -0.72)],
    # The side view: the nose wheels under the rear seat, the mains' tandem
    # bogies behind the wing's root.
    gear=[dict(name="nose", x=3.68, y=0.0, top=1.8, radius=0.35, width=0.18, wheels=2, spread=0.3,
               door=(1.0, 0.6), light=True),
          dict(name="main_L", x=-3.3, y=2.1, top=1.66, radius=0.47, width=0.26, tandem=1.15, door=(1.6, 0.8)),
          dict(name="main_R", x=-3.3, y=-2.1, top=1.66, radius=0.47, width=0.26, tandem=1.15, door=(1.6, 0.8))],
    pylons=[((-3.0, 3.4, WING_Z - 0.12), 1.2, 0.34)],
    stores=[("tank", (-3.1, 3.4, 2.32), 4.6, 0.42), ("missile", (2.2, 0.8, 1.56), 4.2, 0.19),
            ("missile", (-3.2, 0.8, 1.42), 4.2, 0.19)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The wing fence a third back along the chord, 4.6 m out.
    A.mirrored(lambda side, k: box(f"wing_fence_{k}", (0.8, 0.04, 0.12), (-4.58, side * 4.6, 3.26), m["paint"],
                                     hull))
    A.blade_antenna("antenna_spine", (0.3, 0, 3.78), 0.25, m, hull)
    # The photo's markings: the red star and VVS ROSSII on the fins' outer
    # faces, the red star under the wings, the blue 86 on the nose.
    fin_x, fin_z = -8.9, 4.65
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=(fin_x, 1.95, fin_z), normal=(0, 1, 0), up=(0, 0, 1), size=0.5,
                          onto=("tail_fin_",))),
        ("text", dict(text="ВВС РОССИИ", height=0.2, centre=(fin_x - 0.1, 1.95, fin_z - 0.75), normal=(0, 1, 0),
                      up=(0, 0, 1), onto=("tail_fin_",), colour="lowvis_dark")),
        ("text", dict(text="RF 92369", height=0.13, centre=(fin_x + 0.3, 1.95, fin_z - 0.5), normal=(0, 1, 0),
                      up=(0, 0, 1), onto=("tail_fin_",), colour="lowvis_dark")),
        *[("insignia", dict(kind="ru_star", centre=(-4.2, y, WING_Z - 0.4), normal=(0, 0, -1), up=(1, 0, 0),
                            size=0.75, onto=("wing_",), mirror=False)) for y in (4.2, -4.2)],
        ("text", dict(text="86", height=0.55, centre=(7.6, 1.2, 2.75), normal=(0, 1, 0), up=(0, 0, 1),
                      onto=("fuselage",), colour="blue")),
    ])


def wreck(variant, v):
    A.crash(v, tail_x=-5.8, wing_y=4.2, wing_side=1, tail_yaw=0.25, seed=31)


run("mig_31_foxhound", "russian_air_grey", build, wreck, script="mig31.py")
