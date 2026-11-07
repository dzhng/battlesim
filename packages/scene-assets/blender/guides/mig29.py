"""MiG-29SMT, from assets/references/mig_29_fulcrum/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/guides/mig29.py -- --variant=<id> --no-lint --out=<abs path>

The measured guide its skin is made from (`aircraft_skin`): exported with
`--out`, never onto the shipped model.

What the photos settle: the short drooping radome in its darker grey and
the long probe, the bubble canopy on the SMT's humped spine (its extra
fuel), the wide flat centre body with the engine nacelles slung apart under
it, a tunnel between them, their raked box intakes under the long curved
leading-edge extensions, the nacelles toeing in to nozzles that all but
touch, a swept wing, twin fins on tail booms outboard of the nozzles canted
a little outward, stabilators on the booms; twin nose wheels. Two greys.

The references' dimensions (MiG figures): length 17.32 m, span
11.36 m, height 4.73 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

WING_Z = 1.95
# The radome's tip, the long pitot probe ahead of it to 8.66 m.
NOSE_X, NOSE_Z = 7.6, 1.84
# The fins stand on the tail booms, flat shelves outboard of the nacelles,
# above the main wheels in the front photo, canted out 6 degrees, thin,
# their tips 4.72 m up.
FIN = dict(root_x=-3.5, root_z=2.04, height=2.69, root_chord=3.4, tip_chord=1.3, sweep_m=2.4, cant=0.11, thick=0.06)
FIN_Y, BOOM_Y = 1.72, 1.6
# The engines' centres, either side of the tunnel under the centre body.
NACELLE_Y = 1.05
# The nacelles toe in from here aft, to nozzles this far out and this big.
TOE_X, NOZZLE_X, NOZZLE_Y, NOZZLE_R = -1.0, -6.0, 0.74, 0.55
# Each intake's mouth: raked so its top lip, under the LEX, leads the
# drooped lower one (x, z of each lip); the front photo puts the lower lip
# level with the main wheels' tops.
LIP_TOP, LIP_LOW = (2.5, 1.7), (1.9, 0.82)
INTAKE_TOP, NACELLE_LOW, ENGINE_Z = 1.89, 0.86, 1.36
RADOME_BASE = (5.85, 0.49, 1.585, 2.6, 2.05, 2.08)
BOOM = [(-1.9, 0.18, 1.8, 2.02, 1.9, 3.0), (-2.6, 0.32, 1.76, 2.06, 1.9, 4.0), (-6.8, 0.3, 1.74, 2.06, 1.9, 4.0),
        (-7.9, 0.1, 1.82, 2.0, 1.9)]
SPEC = dict(
    # The photos: a short radome, its belly nearly level and its top falling
    # to a drooped tip, the canopy's top a metre above the nose's axis, the
    # LEX and wings low on the nose's chine, then the wide flat centre body
    # roofing the tunnel between the nacelles; behind the wing it narrows to
    # the spine, the nacelles' backs showing beside it, to a beaver tail
    # between the nozzles.
    fuselage=[(NOSE_X, 0.0, NOSE_Z, NOSE_Z), (7.1, 0.27, 1.68, 2.2, 1.92), (6.4, 0.41, 1.61, 2.44, 2.0),
              (5.7, 0.5, 1.58, 2.62, 2.06, 2.1), (4.6, 0.6, 1.56, 2.72, 2.08, 2.3), (3.4, 0.76, 1.56, 2.74, 2.0, 2.6),
              (2.0, 1.15, 1.58, 2.58, 1.92, 3.2), (0.0, 1.5, 1.6, 2.3, 1.9, 4.0), (-1.5, 1.42, 1.6, 2.28, 1.9, 4.0),
              (-3.5, 0.78, 1.62, 2.24, 1.9, 3.0), (-5.5, 0.5, 1.66, 2.16, 1.9, 2.6), (-7.4, 0.3, 1.62, 1.98, 1.8)],
    canopy=dict(x_front=4.55, x_back=2.3, sill=2.7, top=3.27, half_width=0.5, bows=(4.0,), peak=0.42, tail=0.75),
    # The SMT's fuel hump: a dome rising from the centre body behind the
    # canopy, as tall as the canopy's tail, its top falling steadily (the
    # side photo, less the take-off's pitch) to the beaver tail.
    bodies=[("fuselage_spine", [(2.45, 0.52, 2.4, 3.12, 2.45, 1.8), (1.2, 0.68, 2.2, 3.0, 2.25, 1.65),
                                (-1.0, 0.72, 2.06, 2.74, 2.1, 1.6), (-3.5, 0.6, 2.02, 2.42, 2.06, 1.8),
                                (-5.8, 0.32, 2.0, 2.2, 2.02), (-7.0, 0.0, 2.0, 2.0)]),
            # The tail booms outboard of the nacelles, carrying fins and stabilators.
            ("boom_L", BOOM, BOOM_Y), ("boom_R", BOOM, -BOOM_Y)],
    wing_hinge=((0.8, 0, 1), 0.1),
    nav=dict(left=(-2.6, 5.55, WING_Z), right=(-2.6, -5.55, WING_Z)),
    # The top photo: the long LEX curving out from the radome's base over the
    # intakes to the wing's leading edge.
    strake=[(5.7, 0.47, 2.06, 4.6, 0.1), (4.0, 0.95, 1.98, 3.0, 0.08), (2.5, 1.5, 1.94, 1.6, 0.06),
            (1.25, 2.0, 1.94, 0.4, 0.04)],
    # The PD three-view: the wing 1.1 m further aft, its root chord a little
    # longer; the stabilators reach further aft and wider.
    wing=[(1.1, 1.0, WING_Z, 4.9, 0.28), (-3.5, 5.66, WING_Z - 0.06, 1.1, 0.05)],
    stab=[(-6.0, 1.85, 1.8, 2.9, 0.12), (-8.2, 4.05, 1.78, 1.1, 0.04)],
    fins=[dict(FIN, y=FIN_Y), dict(FIN, y=-FIN_Y)],
    nozzles=[dict(loc=(NOZZLE_X, y, ENGINE_Z), r_front=NOZZLE_R, r_exit=NOZZLE_R - 0.04, length=0.95, petals=12)
             for y in (NOZZLE_Y, -NOZZLE_Y)],
    gear=[dict(name="nose", x=2.1, y=0.0, top=1.58, radius=0.28, width=0.15, wheels=2, spread=0.24, light=True),
          dict(name="main_L", x=-1.4, y=1.55, top=1.25, radius=0.42, width=0.22, door=(1.1, 0.5), rake=0.1),
          dict(name="main_R", x=-1.4, y=-1.55, top=1.25, radius=0.42, width=0.22, door=(1.1, 0.5), rake=0.1)],
    # Missiles on the inner and middle wing pylons, the nacelles clear in
    # the side photo.
    pylons=[((-0.9, 3.0, WING_Z - 0.08), 1.4, 0.28), ((-1.6, 4.1, WING_Z - 0.08), 1.2, 0.24)],
    stores=[("missile", (-0.9, 3.0, 1.46), 3.6, 0.11), ("missile", (-1.6, 4.1, 1.51), 2.9, 0.09)],
    nose_probe=((NOSE_X, 0.0, NOSE_Z), 8.66 - NOSE_X),
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The radome in its darker grey, a skin over the fuselage's nose.
    A.body("fuselage_radome", [(x, w * 1.012, lo - 0.006, hi + 0.006, *rest) for x, w, lo, hi, *rest in
                               [(NOSE_X + 0.005, 0.0, NOSE_Z, NOSE_Z), *SPEC["fuselage"][1:3], RADOME_BASE]], m["dark"], hull,
           seg=28)
    (xt, zt), (xl, zl) = LIP_TOP, LIP_LOW
    rake = math.atan2(xt - xl, zt - zl)
    for side, k in ((1, "L"), (-1, "R")):
        y = side * NACELLE_Y
        # The intake's box duct under the LEX, its mouth raked back from the
        # top lip to the lower one, then the nacelle rounding to the nozzle.
        top, low, mid = INTAKE_TOP, NACELLE_LOW, ENGINE_Z
        A.body(f"nacelle_intake_{k}", [(xt, 0.44, zt, top, (zt + top) / 2, 6.0), (xl, 0.44, zl, top, mid, 6.0),
                                       (0.6, 0.46, low, top, mid, 6.0)],
               m["paint"], hull, seg=20, loc=(0, y, 0), steps=(1, 1, 1, 1))
        A.body(f"nacelle_{k}", [(0.7, 0.46, low, top, mid, 6.0), (TOE_X, 0.5, low, top - 0.01, mid, 4.5)],
               m["paint"], hull, seg=20, loc=(0, y, 0))
        # Behind the main gear the engines toe in, so the nozzles all but
        # touch (the rear and top photos).
        toe = math.asin((NACELLE_Y - NOZZLE_Y) / (TOE_X - NOZZLE_X))
        A.body(f"nacelle_aft_{k}", [(0.0, 0.5, low, top - 0.01, mid, 4.5), (-2.5, 0.54, low, mid + 0.52, mid, 3.0),
                                    (NOZZLE_X - TOE_X, NOZZLE_R, mid - NOZZLE_R, mid + NOZZLE_R, mid)],
               m["paint"], hull, seg=20, loc=(TOE_X, y, 0), rot=(0, 0, side * toe))
        face = math.hypot(xt - xl, zt - zl)
        box(f"intake_{k}_mouth", (0.02, 0.72, face - 0.08),
            ((xt + xl) / 2 + 0.012 * math.cos(rake), y, (zt + zl) / 2 - 0.012 * math.sin(rake)), m["black"], hull,
            rot=(0, rake, 0), lods=MID)
    # The fins' leading edges run forward and down onto the booms.
    rise = 0.42
    for y in (FIN_Y, -FIN_Y):
        lean = math.copysign(rise * math.tan(FIN["cant"]), y)
        A.surface(f"tail_fin_fillet_{'L' if y > 0 else 'R'}",
                  [(-2.3, y, FIN["root_z"] - 0.02, 1.3, 0.08),
                   (FIN["root_x"] - FIN["sweep_m"] * rise / FIN["height"], y + lean, FIN["root_z"] + rise, 0.3, 0.05)],
                  m["paint"], hull)
    A.sensor_ball("irst", (4.95, -0.25, 2.76), 0.14, m, hull)
    box("antiglare", (0.9, 0.55, 0.03), (5.3, 0, 2.68), m["dark"], hull, rot=(0, 0.12, 0), lods=MID)
    A.blade_antenna("antenna_spine", (-1.0, 0, 2.71), 0.22, m, hull)
    # The VKS's red star on the fins (both faces) and under the wings, the
    # photo's blue 777 on the intakes.
    h = FIN["height"] * 0.6
    fin_x = FIN["root_x"] - FIN["sweep_m"] * 0.6 - 0.95
    fin_y, fin_z = FIN_Y + h * math.sin(FIN["cant"]), FIN["root_z"] + h * math.cos(FIN["cant"])
    rows = [("insignia", dict(kind="ru_star", centre=(fin_x, y, fin_z), normal=(0, n, 0), up=(0, 0, 1), size=0.3,
                              onto=("tail_fin_",), mirror=False))
            for y, n in ((fin_y + 0.2, 1), (fin_y - 0.4, -1), (-fin_y - 0.2, -1), (-fin_y + 0.4, 1))]
    rows += [("insignia", dict(kind="ru_star", centre=(-1.2, y, WING_Z - 0.3), normal=(0, 0, -1), up=(1, 0, 0),
                               size=0.6, onto=("wing_",), mirror=False)) for y in (3.6, -3.6)]
    rows.append(("text", dict(text="777", height=0.42, centre=(0.4, 1.6, 1.4), normal=(0, 1, 0), up=(0, 0, 1),
                              onto=("nacelle_",), colour="blue")))
    A.markings(v, rows)


def wreck(variant, v):
    A.crash(v, tail_x=-4.0, wing_y=3.4, wing_side=-1, tail_yaw=-0.3, seed=29)


run("mig_29_fulcrum", "russian_air_grey", build, wreck, script="mig29.py")
