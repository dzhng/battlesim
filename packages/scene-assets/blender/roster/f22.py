"""F-22A Raptor, from assets/references/f_22_raptor/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/f22.py -- [--wreck]

What the photos settle: the chined, flat-sided nose under a frameless bubble
canopy, caret intakes raked back either side of the cockpit, the body widening
into a broad flat blended deck, the trapezoid wing (leading edge swept, the
trailing edge forward-swept inboard), twin fins canted outward, stabilators
level with the wing, two-dimensional vectoring nozzles side by side between
the tail booms, the tall gear. Compass greys.

Dimensions stated from the references (USAF fact sheet figures agree):
length 18.92 m, span 13.56 m, height 5.08 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

CARDS = {"us_f_22_raptor_f_22a": (18.92, 13.56, 5.08)}

WING_Z = 2.24
FIN = dict(root_x=-4.2, root_z=2.55, height=2.87, root_chord=3.8, tip_chord=1.5, sweep_m=2.4, cant=0.49)
SPEC = dict(
    fuselage=[(9.46, 0.0, 2.08, 2.08), (8.6, 0.42, 1.88, 2.36, 2.1, 1.4), (7.2, 0.78, 1.7, 2.62, 2.08, 1.5),
              (5.6, 1.02, 1.58, 2.76, 2.04, 1.7), (3.8, 1.72, 1.5, 2.72, 2.0, 2.5), (0.5, 1.96, 1.46, 2.66, 2.0, 3.4),
              (-3.5, 1.82, 1.5, 2.56, 2.0, 3.4), (-6.5, 1.52, 1.6, 2.46, 2.0, 3.0), (-8.3, 1.32, 1.7, 2.36, 2.0, 3.0)],
    canopy=dict(x_front=7.6, x_back=4.5, sill=2.64, top=3.28, half_width=0.52, peak=0.45),
    wing=[(2.3, 1.8, WING_Z, 6.4, 0.3), (-2.7, 6.78, WING_Z - 0.1, 1.6, 0.06)],
    stab=[(-6.4, 1.4, 2.1, 3.0, 0.12), (-8.5, 4.4, 2.06, 1.0, 0.04)],
    fins=[dict(FIN, y=1.25), dict(FIN, y=-1.25)],
    intakes=[("rect", (4.7, 1.42, 1.98), 0.7, 1.0, 0.24, (0.25, 0.3, -0.2)),
             ("rect", (4.7, -1.42, 1.98), 0.7, 1.0, 0.24, (-0.25, 0.3, 0.2))],
    gear=[dict(name="nose", x=6.3, y=0.0, top=1.6, radius=0.32, width=0.18, door=(1.0, 0.5)),
          dict(name="main_L", x=-0.8, y=1.62, top=1.45, radius=0.48, width=0.24, door=(1.4, 0.6), rake=0.1),
          dict(name="main_R", x=-0.8, y=-1.62, top=1.45, radius=0.48, width=0.24, door=(1.4, 0.6), rake=0.1)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The two-dimensional vectoring nozzles: square-section pipes ending in
    # the upper and lower flaps, dark inside.
    for side, s in ((1, "L"), (-1, "R")):
        A.body(f"nozzle_{s}", [(-8.2, 0.5, 1.6, 2.3, 1.95, 5.0), (-9.1, 0.46, 1.66, 2.24, 1.95, 5.0)], m["nozzle"], hull,
               seg=16, loc=(0, side * 0.62, 0), steps=(1, 1, 1, 1))
        box(f"nozzle_{s}_flap_top", (0.6, 0.9, 0.06), (-9.2, side * 0.62, 2.22), m["nozzle"], hull, rot=(0, 0.12, 0),
            lods=MID)
        box(f"nozzle_{s}_flap_low", (0.6, 0.9, 0.06), (-9.2, side * 0.62, 1.68), m["nozzle"], hull, rot=(0, -0.12, 0),
            lods=MID)
        box(f"nozzle_{s}_bore", (0.04, 0.78, 0.4), (-9.12, side * 0.62, 1.95), m["black"], hull, lods=MID)
        # The weapon bay doors' seams under the intakes and on the belly.
        box(f"side_bay_{s}", (1.8, 0.03, 0.4), (2.4, side * 1.78, 1.7), m["dark"], hull, lods=NEAR)
    box("main_bay", (4.2, 1.6, 0.03), (0.6, 0, 1.45), m["dark"], hull, lods=MID)
    box("antiglare", (1.2, 0.6, 0.03), (8.3, 0, 2.38), m["dark"], hull, rot=(0, 0.16, 0), lods=MID)
    box("gun_door", (0.6, 0.06, 0.12), (3.6, -1.84, 2.4), m["dark"], hull, lods=NEAR)


def wreck(variant, v):
    A.crash(v, tail_x=-5.2, wing_y=4.2, wing_side=1, tail_yaw=0.3, seed=22)


run_disabled("f22", CARDS, "us_compass_grey", build, wreck)
