"""F-35A and F-35B Lightning II, from assets/references/f_35_lightning_ii/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/f35.py -- [--variant=<card>] [--wreck]

What the photos settle: a short, deep, chined body, the canopy high on it
with one aft frame, caret intakes behind the cockpit, the trapezoid wing and
stabilators, twin fins canted outward, one big engine nozzle; the sensor
window under the nose. The A has the gun's bulge on the left shoulder and a
round nozzle; the B (short take-off, vertical landing) has the lift-fan doors
behind the canopy, the auxiliary inlet doors above the engine and the
swivelling three-bearing nozzle, no gun. Both in the dark grey radar
coating.

The references' dimensions (Lockheed Martin figures agree):
F-35A 15.7 x 10.7 x 4.38 m; F-35B 15.6 x 10.7 x 4.36 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

FA, FB = "f_35a", "us_f_35_lightning_ii_f_35b"

WING_Z = 2.0


def spec(variant):
    b = variant["id"] == FB
    tip = 2.36 - (0.02 if b else 0.0)
    fin = dict(root_x=-3.65, root_z=2.45, height=(4.36 if b else 4.38) - 2.45, root_chord=2.6, tip_chord=1.15,
               sweep_m=2.4, cant=0.44)
    fin["height"] /= 0.905
    return dict(
        # The deep body of the side photo: its spine 3 m up behind the
        # canopy, its belly 1.2 m off the ground, the fins on its flanks.
        fuselage=[(7.82, 0.0, 2.12, 2.12), (7.1, 0.42, 1.84, 2.4, 2.08, 1.4), (5.9, 0.76, 1.56, 2.68, 2.05, 1.6),
                  (4.4, 0.86, 1.24, 2.76, 2.02, 2.0), (2.8, 1.3, 1.06, 2.92, 2.0, 2.6), (0.0, 1.76, 1.04, 2.98, 2.0, 3.2),
                  (-3.0, 1.62, 1.2, 2.8, 2.0, 3.0), (-5.0, 1.12, 1.48, tip + 0.1, 1.95, 2.6),
                  (-5.75, 0.8, 1.54, 2.3, 1.92, 2.2)],
        canopy=dict(x_front=5.8, x_back=3.4 if b else 3.2, sill=2.62, top=3.36, half_width=0.5, bows=(),
                    peak=0.42, tail=0.75),
        bodies=[("fuselage_spine", [(3.35, 0.4, 2.8, 3.22, 2.9, 2.4), (0.6, 0.5, 2.8, 3.08, 2.9, 2.6),
                                    (-3.4, 0.0, 2.7, 2.7)]),
                # The caret intakes' trunks, from their mouths back into the body.
                *[(f"fuselage_intake_{k}", [(3.64, 0.44, 1.46, 2.48, 1.96, 4.0), (2.2, 0.46, 1.4, 2.54, 1.96, 3.5),
                                            (0.2, 0.26, 1.6, 2.4, 1.96, 3.0)], side * 1.22)
                  for side, k in ((1, "L"), (-1, "R"))],
                # The short tail booms beside the nozzle that carry the fins
                # and stabilators aft of the body.
                *[(f"tail_boom_{k}", [(-2.4, 0.3, 1.62, 2.5, 2.0, 2.6), (-5.0, 0.42, 1.64, 2.48, 2.0, 2.6),
                                      (-7.0, 0.34, 1.78, 2.3, 2.0, 2.4), (-7.45, 0.0, 2.0, 2.0)], side * 1.12)
                  for side, k in ((1, "L"), (-1, "R"))]],
        wing_hinge=(0.1, (0.8, 0, 1)),
        nav=dict(left=(-2.9, 5.2, WING_Z - 0.04), right=(-2.9, -5.2, WING_Z - 0.04)),
        wing=[(0.7, 1.7, WING_Z, 5.65, 0.3), (-2.3, 5.35, WING_Z - 0.06, 1.7, 0.06)],
        stab=[(-4.1, 1.0, 1.96, 3.72, 0.12), (-5.85, 3.55, 1.94, 1.1, 0.04)],
        fins=[dict(fin, y=1.05), dict(fin, y=-1.05)],
        intakes=[("rect", (3.64, 1.22, 1.96), 0.76, 0.98, 0.22, (0.22, 0.25, -0.15)),
                 ("rect", (3.64, -1.22, 1.96), 0.76, 0.98, 0.22, (-0.22, 0.25, 0.15))],
        nozzles=[] if b else [dict(loc=(-5.7, 0.0, 1.9), r_front=0.66, r_exit=0.56, length=0.8, petals=14)],
        gear=[dict(name="nose", x=3.8, y=0.0, top=1.2, radius=0.3, width=0.17, door=(0.9, 0.45), light=True),
              dict(name="main_L", x=-2.35, y=1.32, top=1.15, radius=0.4, width=0.22, door=(1.2, 0.5), rake=0.1),
              dict(name="main_R", x=-2.35, y=-1.32, top=1.15, radius=0.4, width=0.22, door=(1.2, 0.5), rake=0.1)],
    )


def build(variant, v):
    m = A.jet(v, spec(variant))
    hull = v.hull
    # Each intake's dark mouth on its raked lip.
    for k, (_, loc, w, h, d, rot) in enumerate(spec(variant)["intakes"]):
        A.intake_mouth(f"intake_{k}", loc, w, h, d, m, hull, rot=rot)
    # The electro-optical targeting window under the chin, the distributed
    # aperture windows round the nose.
    box("eots_window", (0.5, 0.36, 0.14), (6.4, 0, 1.66), m["glass"], hull, rot=(0, 0.3, 0), lods=MID)
    A.mirrored(lambda side, k: box(f"das_window_{k}", (0.18, 0.03, 0.14), (5.2, side * 0.78, 2.2), m["glass"], hull,
                                   lods=NEAR))
    box("antiglare", (0.9, 0.5, 0.03), (6.4, 0, 2.56), m["dark"], hull, rot=(0, 0.2, 0), lods=MID)
    A.mirrored(lambda side, k: box(f"bay_seam_{k}", (3.4, 0.03, 0.03), (0.4, side * 0.5, 1.05), m["dark"], hull,
                                   lods=NEAR))
    # Eielson's AK (the side photo) on the A, the Marines' VK on the B, in
    # the low-visibility grey both wear.
    b = variant["id"] == FB
    fin_n, fin_up = (0, math.cos(0.44), -math.sin(0.44)), (0, math.sin(0.44), math.cos(0.44))
    rows = [
        ("insignia", dict(kind="us_lowvis", centre=(-4.6, 1.6, 2.1), normal=(0, 1, 0), up=(0, 0, 1), size=0.3,
                          onto=("fuselage",))),
        ("insignia", dict(kind="us_lowvis", centre=(-1.2, 3.8, WING_Z + 0.2), normal=(0, 0, 1), up=(1, 0, 0),
                          size=0.45, onto=("wing_L",), mirror=False)),
        ("insignia", dict(kind="us_lowvis", centre=(-1.2, -3.8, WING_Z - 0.2), normal=(0, 0, -1), up=(1, 0, 0),
                          size=0.45, onto=("wing_R",), mirror=False)),
        ("text", dict(text="VK" if b else "AK", height=0.48, centre=(-5.3, 1.05 + 1.0 * fin_up[1] + 0.3,
                                                                     2.45 + 1.0 * fin_up[2]),
                      normal=fin_n, up=fin_up, onto=("tail_fin_",), colour="lowvis_light")),
        ("text", dict(text="169164" if b else "AF 19 5458", height=0.12,
                      centre=(-5.0, 1.05 + 0.45 * fin_up[1] + 0.3, 2.45 + 0.45 * fin_up[2]), normal=fin_n, up=fin_up,
                      onto=("tail_fin_",), colour="lowvis_light")),
    ]
    if b:
        rows.append(("text", dict(text="MARINES", height=0.24, centre=(-2.0, 1.9, 2.0), normal=(0, 1, 0),
                                  up=(0, 0, 1), onto=("fuselage",), colour="lowvis_light")))
    A.markings(v, rows)
    if variant["id"] == FB:
        # Lift-fan door behind the canopy, the auxiliary inlet doors over the
        # engine, the roll posts' doors under the wings; the three-bearing
        # swivel nozzle, straight aft.
        box("lift_fan_door", (1.6, 0.8, 0.05), (2.6, 0, 3.2), m["paint"], hull, bevel=0.02, lods=MID)
        A.mirrored(lambda side, k: box(f"aux_inlet_{k}", (0.9, 0.3, 0.04), (0.9, side * 0.3, 3.12), m["dark"], hull,
                                       lods=MID))
        for k, (x, r, l) in enumerate(((-5.7, 0.62, 0.45), (-6.15, 0.59, 0.45), (-6.6, 0.56, 0.45))):
            cyl(f"swivel_duct_{k}", r, l, (x - l / 2, 0, 1.9), "X", m["nozzle"], hull, seg=22, r2=r * 1.03)
            cyl(f"swivel_seam_{k}", r * 1.05, 0.03, (x - l, 0, 1.9), "X", m["dark"], hull, seg=22, lods=NEAR)
        cyl("swivel_bore", 0.48, 0.04, (-7.03, 0, 1.9), "X", m["black"], hull, seg=18, lods=MID)
    else:
        A.body("gun_bulge", [(3.2, 0.0, 2.88, 2.88), (2.8, 0.14, 2.8, 3.02), (0.6, 0.14, 2.8, 3.02),
                             (0.2, 0.0, 2.9, 2.9)], m["paint"], hull, seg=10, loc=(0, 0.95, 0), lods=MID)


def wreck(variant, v):
    A.crash(v, tail_x=-3.8, wing_y=3.2, wing_side=-1, tail_yaw=-0.25, seed=35)


run("f_35_lightning_ii", "us_gunship_grey", build, wreck, script="f35.py")
