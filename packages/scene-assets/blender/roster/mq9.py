"""MQ-9A Reaper, from assets/references/mq_9_reaper/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/mq9.py -- [--wreck]

What the photos settle: a slim body under the bulged satellite-antenna nose,
the sensor ball under the chin, a long thin straight wing, the V-tail over a
ventral fin, the four-blade pusher propeller at the tail, the tall splayed
main gear and the nose gear; Hellfires and GBU-12s on the wing pylons. Light
grey.

Dimensions stated from the references (USAF fact sheet figures agree):
length 11.0 m, span 20.1 m, height 3.81 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"us_mq_9_reaper_mq_9a": (11.0, 20.1, 3.81)}

WING_Z = 1.5
V_TAIL = dict(root_x=-3.7, root_z=1.42, height=3.08, root_chord=1.15, tip_chord=0.55, sweep_m=0.9, cant=0.7,
              rudder=False)
SPEC = dict(
    fuselage=[(5.5, 0.0, 1.28, 1.28), (5.2, 0.36, 0.96, 1.62, 1.3), (4.4, 0.5, 0.88, 1.74, 1.3),
              (3.0, 0.45, 0.9, 1.56, 1.25), (0.0, 0.42, 0.95, 1.52, 1.22), (-3.0, 0.32, 1.06, 1.46, 1.25),
              (-4.7, 0.18, 1.16, 1.4, 1.28)],
    wing=[(0.62, 0.4, WING_Z, 1.35, 0.2), (0.2, 10.05, WING_Z + 0.25, 0.55, 0.06)],
    fins=[dict(V_TAIL, y=0.12), dict(V_TAIL, y=-0.12),
          dict(root_x=-3.9, root_z=1.08, height=0.62, root_chord=1.0, tip_chord=0.6, sweep_m=0.35, cant=math.pi,
               rudder=False)],
    gear=[dict(name="nose", x=3.9, y=0.0, top=0.95, radius=0.2, width=0.12),
          dict(name="main_L", x=0.5, y=1.25, top=1.0, radius=0.28, width=0.16),
          dict(name="main_R", x=0.5, y=-1.25, top=1.0, radius=0.28, width=0.16)],
    pylons=[((0.3, 2.6, WING_Z + 0.02), 0.9, 0.2), ((0.25, 4.2, WING_Z + 0.06), 0.8, 0.18),
            ((0.2, 5.6, WING_Z + 0.1), 0.7, 0.16)],
    stores=[("bomb", (0.2, 2.6, 1.1), 3.3, 0.19), ("missile", (0.25, 4.2, 1.2), 1.63, 0.09),
            ("missile", (0.2, 5.6, 1.3), 1.63, 0.09)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    A.sensor_ball("sensor", (3.7, 0, 0.72), 0.3, m, hull)
    box("sensor_mount", (0.25, 0.2, 0.25), (3.7, 0, 0.95), m["paint"], hull, lods=MID)
    # The main gear legs splay out from the belly to the wheels.
    A.mirrored(lambda side, k: box(f"gear_leg_{k}", (0.12, 1.15, 0.08), (0.5, side * 0.68, 0.62), m["gear"], hull,
                                   rot=(side * -0.62, 0, 0), lods=(0, 1, 2)))
    # The pusher propeller and its spinner, behind the engine's exhaust.
    A.rotor("prop", (-5.2, 0, 1.28), 1.15, 4, 0.22, m, hull, hub=0.14, mast=0.0, droop=0.0, rot=(0, math.pi / 2, 0),
            thick=0.12, tip_chord=0.12)
    cyl("prop_spinner", 0.16, 0.3, (-5.4, 0, 1.28), "X", m["paint"], hull, seg=12, r2=0.04)
    A.mirrored(lambda side, k: cyl(f"exhaust_{k}", 0.06, 0.25, (-4.3, side * 0.24, 1.15), "Y", m["nozzle"], hull,
                                   seg=8, lods=MID))
    box("satcom_seam", (0.03, 0.7, 0.5), (4.6, 0, 1.45), m["dark"], hull, lods=MID)


def wreck(variant, v):
    A.crash(v, tail_x=-2.8, wing_y=4.5, wing_side=1, tail_yaw=0.4, seed=9)


run_disabled("mq9", CARDS, "us_compass_grey", build, wreck)
