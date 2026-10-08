"""F-35A and F-35B Lightning II, from assets/references/f_35_lightning_ii/. Disabled cards.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/f35.py -- [--variant=<card>] [--wreck]

What the photos settle: a short, deep, chined body, the canopy high on it
with one aft frame, caret intakes behind the cockpit, the trapezoid wing and
stabilators, twin fins canted outward, one big engine nozzle; the sensor
window under the nose. The A has the gun's bulge on the left shoulder and a
round nozzle; the B (short take-off, vertical landing) has the lift-fan doors
behind the canopy, the auxiliary inlet doors above the engine and the
swivelling three-bearing nozzle, no gun. Both in the dark grey radar
coating.

Dimensions stated from the references (Lockheed Martin figures agree):
F-35A 15.7 x 10.7 x 4.38 m; F-35B 15.6 x 10.7 x 4.36 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

FA, FB = "f_35a", "us_f_35_lightning_ii_f_35b"
CARDS = {FA: (15.7, 10.7, 4.38), FB: (15.6, 10.7, 4.36)}

WING_Z = 2.0


def spec(variant):
    b = variant["id"] == FB
    tip = 2.36 - (0.02 if b else 0.0)
    fin = dict(root_x=-3.55, root_z=2.45, height=(4.36 if b else 4.38) - 2.45, root_chord=2.9, tip_chord=1.2,
               sweep_m=1.6, cant=0.44)
    fin["height"] /= 0.905
    return dict(
        fuselage=[(7.82, 0.0, 1.95, 1.95), (7.1, 0.4, 1.72, 2.2, 1.95, 1.4), (5.9, 0.74, 1.52, 2.5, 1.92, 1.6),
                  (4.4, 0.98, 1.4, 2.62, 1.9, 2.0), (2.8, 1.56, 1.32, 2.64, 1.88, 2.6), (0.0, 1.76, 1.3, 2.62, 1.88, 3.2),
                  (-3.0, 1.62, 1.4, 2.5, 1.9, 3.0), (-5.3, 1.06, 1.5, tip, 1.9, 2.6), (-6.2, 0.76, 1.56, 2.24, 1.9, 2.2)],
        canopy=dict(x_front=6.05, x_back=3.9 if b else 3.6, sill=2.5, top=3.06, half_width=0.48, bows=(4.15,),
                    peak=0.42),
        wing=[(1.2, 1.62, WING_Z, 5.4, 0.3), (-2.2, 5.35, WING_Z - 0.06, 1.4, 0.06)],
        stab=[(-4.8, 1.0, 1.96, 2.4, 0.1), (-6.4, 3.3, 1.94, 1.0, 0.04)],
        fins=[dict(fin, y=1.05), dict(fin, y=-1.05)],
        intakes=[("rect", (3.65, 1.26, 1.96), 0.55, 0.8, 0.22, (0.22, 0.25, -0.15)),
                 ("rect", (3.65, -1.26, 1.96), 0.55, 0.8, 0.22, (-0.22, 0.25, 0.15))],
        nozzles=[] if b else [dict(loc=(-6.2, 0.0, 1.9), r_front=0.6, r_exit=0.54, length=1.6, petals=14)],
        gear=[dict(name="nose", x=4.9, y=0.0, top=1.42, radius=0.3, width=0.17, door=(0.9, 0.45)),
              dict(name="main_L", x=-1.0, y=1.32, top=1.36, radius=0.45, width=0.22, door=(1.2, 0.5), rake=0.1),
              dict(name="main_R", x=-1.0, y=-1.32, top=1.36, radius=0.45, width=0.22, door=(1.2, 0.5), rake=0.1)],
    )


def build(variant, v):
    m = A.jet(v, spec(variant))
    hull = v.hull
    # The electro-optical targeting window under the chin, the distributed
    # aperture windows round the nose.
    box("eots_window", (0.5, 0.36, 0.14), (6.4, 0, 1.6), m["glass"], hull, rot=(0, 0.3, 0), lods=MID)
    A.mirrored(lambda side, k: box(f"das_window_{k}", (0.18, 0.03, 0.14), (5.2, side * 0.92, 2.2), m["glass"], hull,
                                   lods=NEAR))
    box("antiglare", (1.0, 0.5, 0.03), (6.8, 0, 2.3), m["dark"], hull, rot=(0, 0.2, 0), lods=MID)
    A.mirrored(lambda side, k: box(f"bay_seam_{k}", (3.4, 0.03, 0.03), (0.4, side * 0.5, 1.31), m["dark"], hull,
                                   lods=NEAR))
    if variant["id"] == FB:
        # Lift-fan door behind the canopy, the auxiliary inlet doors over the
        # engine, the roll posts' doors under the wings; the three-bearing
        # swivel nozzle, straight aft.
        box("lift_fan_door", (1.6, 1.0, 0.05), (2.9, 0, 2.66), m["paint"], hull, bevel=0.02, lods=MID)
        A.mirrored(lambda side, k: box(f"aux_inlet_{k}", (0.9, 0.35, 0.04), (0.9, side * 0.42, 2.64), m["dark"], hull,
                                       lods=MID))
        for k, (x, r, l) in enumerate(((-6.2, 0.6, 0.5), (-6.7, 0.57, 0.5), (-7.2, 0.54, 0.5))):
            cyl(f"swivel_duct_{k}", r, l, (x - l / 2, 0, 1.9), "X", m["nozzle"], hull, seg=22, r2=r * 1.03)
            box(f"swivel_seam_{k}", (0.03, r * 2.05, r * 2.05), (x - l, 0, 1.9), m["dark"], hull, lods=NEAR)
        cyl("swivel_bore", 0.48, 0.04, (-7.67, 0, 1.9), "X", m["black"], hull, seg=18, lods=MID)
    else:
        A.body("gun_bulge", [(3.2, 0.0, 2.45, 2.45), (2.8, 0.14, 2.38, 2.6), (0.6, 0.14, 2.38, 2.6),
                             (0.2, 0.0, 2.48, 2.48)], m["paint"], hull, seg=10, loc=(0, 0.95, 0), lods=MID)


def wreck(variant, v):
    A.crash(v, tail_x=-3.8, wing_y=3.2, wing_side=-1, tail_yaw=-0.25, seed=35)


run_disabled("f35", CARDS, "us_gunship_grey", build, wreck)
