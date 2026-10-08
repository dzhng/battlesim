"""MiG-29SMT, from assets/references/mig_29_fulcrum/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/mig29.py -- [--wreck]

What the photos settle: the drooping radome, the bubble canopy on the SMT's
humped spine (its extra fuel), the wide flat centre body with the engine
nacelles slung apart under it and their raked box intakes under the
leading-edge extensions, a swept wing, twin fins on the nacelles canted a
little outward, stabilators, two nozzles; twin nose wheels. Two greys.

Dimensions stated from the references (MiG figures): length 17.32 m, span
11.36 m, height 4.73 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"eastern_mig_29_fulcrum_mig_29smt": (17.32, 11.36, 4.73)}
WING_Z = 2.3
FIN = dict(root_x=-3.6, root_z=2.75, height=1.99, root_chord=3.0, tip_chord=1.0, sweep_m=1.9, cant=0.1)
SPEC = dict(
    fuselage=[(8.0, 0.0, 2.2, 2.2), (7.6, 0.4, 1.95, 2.48, 2.2), (6.6, 0.56, 1.78, 2.72, 2.18, 2.2),
              (5.2, 0.62, 1.72, 2.88, 2.16, 2.4), (3.6, 0.78, 1.82, 3.02, 2.22, 2.6), (2.0, 1.3, 1.95, 2.98, 2.25, 3.5),
              (0.0, 1.5, 1.95, 2.9, 2.25, 4.0), (-3.0, 1.4, 2.0, 2.82, 2.25, 4.0), (-5.5, 0.9, 2.05, 2.72, 2.3, 3.0),
              (-7.6, 0.35, 2.15, 2.56, 2.3)],
    # The side photo: the canopy 4 m behind the probe's tip, the SMT's fuel
    # hump behind it, the intakes and the nose gear under the LERX's root,
    # the mains a metre further aft than drawn before.
    canopy=dict(x_front=4.7, x_back=2.45, sill=2.86, top=3.44, half_width=0.47, bows=(4.15,), peak=0.42, tail=0.75),
    bodies=[("fuselage_spine", [(2.6, 0.48, 2.8, 3.3, 2.95, 2.4), (0.0, 0.62, 2.8, 3.3, 2.95, 2.6),
                                (-3.5, 0.42, 2.7, 3.0, 2.8, 2.4), (-5.6, 0.0, 2.65, 2.65)])],
    wing_hinge=((0.8, 0, 1), 0.1),
    nav=dict(left=(-2.6, 5.55, WING_Z), right=(-2.6, -5.55, WING_Z)),
    strake=[(5.6, 0.75, 2.36, 3.6, 0.08), (2.0, 1.5, WING_Z, 0.4, 0.05)],
    wing=[(1.7, 1.5, WING_Z, 4.3, 0.26), (-2.4, 5.66, WING_Z - 0.06, 1.1, 0.05)],
    stab=[(-5.5, 1.2, 2.15, 2.6, 0.12), (-7.4, 3.9, 2.12, 1.1, 0.04)],
    fins=[dict(FIN, y=1.25), dict(FIN, y=-1.25)],
    intakes=[("rect", (1.58, 0.95, 1.62), 0.68, 0.6, 0.22, (0, 0.3, 0)),
             ("rect", (1.58, -0.95, 1.62), 0.68, 0.6, 0.22, (0, 0.3, 0))],
    nozzles=[dict(loc=(-7.0, 0.95, 1.65), r_front=0.5, r_exit=0.46, length=0.95, petals=12),
             dict(loc=(-7.0, -0.95, 1.65), r_front=0.5, r_exit=0.46, length=0.95, petals=12)],
    gear=[dict(name="nose", x=2.1, y=0.0, top=1.6, radius=0.28, width=0.15, wheels=2, spread=0.24, light=True),
          dict(name="main_L", x=-1.4, y=1.55, top=1.55, radius=0.42, width=0.22, door=(1.1, 0.55), rake=0.1),
          dict(name="main_R", x=-1.4, y=-1.55, top=1.55, radius=0.42, width=0.22, door=(1.1, 0.55), rake=0.1)],
    pylons=[((-0.2, 2.6, WING_Z - 0.08), 1.0, 0.3), ((-0.9, 3.7, WING_Z - 0.08), 0.9, 0.24)],
    stores=[("tank", (-0.3, 2.6, 1.6), 3.8, 0.36), ("missile", (-0.9, 3.7, 1.86), 3.6, 0.1)],
    nose_probe=((8.0, 0.0, 2.2), 0.66),
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(1.5, 0.42, 1.3, 2.0, 1.62, 3.5), (-0.6, 0.48, 1.2, 2.1, 1.64, 3.5),
                                (-4.0, 0.5, 1.15, 2.15, 1.65, 2.6), (-7.0, 0.5, 1.15, 2.15, 1.65)],
               m["paint"], hull, seg=18, loc=(0, side * 0.95, 0))
    A.sensor_ball("irst", (4.95, 0.25, 2.92), 0.14, m, hull)
    box("antiglare", (0.9, 0.55, 0.03), (5.6, 0, 2.8), m["dark"], hull, rot=(0, 0.12, 0), lods=MID)
    A.blade_antenna("antenna_spine", (-1.0, 0, 3.3), 0.22, m, hull)
    # The VKS's red star on the fins (both faces) and under the wings, the
    # photo's blue 777 on the intakes.
    fin_x, fin_z = -3.6 - 1.9 * 0.6 - 0.9, 2.75 + 1.99 * 0.6
    rows = [("insignia", dict(kind="ru_star", centre=(fin_x, y, fin_z), normal=(0, n, 0), up=(0, 0, 1), size=0.3,
                              onto=("tail_fin_",), mirror=False))
            for y, n in ((1.6, 1), (1.0, -1), (-1.6, -1), (-1.0, 1))]
    rows += [("insignia", dict(kind="ru_star", centre=(-1.2, y, WING_Z - 0.3), normal=(0, 0, -1), up=(1, 0, 0),
                               size=0.6, onto=("wing_",), mirror=False)) for y in (3.6, -3.6)]
    rows.append(("text", dict(text="777", height=0.42, centre=(0.4, 1.6, 1.75), normal=(0, 1, 0), up=(0, 0, 1),
                              onto=("nacelle_",), colour="blue")))
    A.markings(v, rows)


def wreck(variant, v):
    A.crash(v, tail_x=-4.0, wing_y=3.4, wing_side=-1, tail_yaw=-0.3, seed=29)


run_disabled("mig29", CARDS, "russian_air_grey", build, wreck)
