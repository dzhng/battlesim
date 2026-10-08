"""Tiger HAD and UHT, from assets/references/ec665_tiger/. Disabled cards.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/tiger.py -- [--variant=<card>] [--wreck]

What the photos settle: a slim tandem two-seater, pilot forward and gunner
behind and above under separate canopies, the engines either side of the
rotor mast with their exhausts turned up and out, stub wings with two
stations a side, a horizontal tailplane with end-plate fins, the fin with
the three-blade tail rotor on its right, four main blades, fixed main wheels
and a tail wheel. The HAD (Spain, France) has the roof sight over the
cockpit and the 30 mm chin turret; the UHT (Germany) has the mast-mounted
sight on the rotor head and no turret. Army green-grey.

Dimensions stated from published figures (rotorcraft rule: fuselage
length without blades, width over the stub wings' stores, height to the top
of the rotor head or what stands on it): HAD 14.08 x 4.32 x 3.83 m; UHT
14.08 x 4.32 x 4.32 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

HAD, UHT = "europe_ec665_tiger_had", "europe_ec665_tiger_uht"
CARDS = {HAD: (14.08, 4.32, 3.83), UHT: (14.08, 4.32, 4.32)}
SPEC = dict(
    fuselage=[(7.04, 0.0, 1.25, 1.25), (6.7, 0.38, 0.95, 1.56, 1.25, 2.2), (5.8, 0.5, 0.78, 1.96, 1.3, 3.0),
              (4.3, 0.55, 0.78, 2.36, 1.4, 3.0), (2.6, 0.62, 0.8, 2.32, 1.45, 3.2), (0.5, 0.6, 0.9, 2.22, 1.5, 3.2),
              (-1.0, 0.45, 1.1, 2.06, 1.6, 3.0), (-4.0, 0.22, 1.35, 1.86, 1.6, 2.4), (-6.4, 0.18, 1.4, 1.9, 1.65),
              (-7.04, 0.16, 1.45, 2.0, 1.7)],
    wing=[(1.6, 0.55, 1.5, 1.2, 0.16), (1.4, 2.05, 1.42, 1.0, 0.12)],
    stab=[(-5.5, 0.2, 1.78, 0.9, 0.1), (-5.6, 1.3, 1.78, 0.8, 0.08)],
    fins=[dict(root_x=-5.6, root_z=1.95, height=1.45, root_chord=1.4, tip_chord=0.9, sweep_m=0.5, thick=0.16),
          dict(root_x=-5.5, root_z=1.5, height=0.65, root_chord=0.8, tip_chord=0.6, sweep_m=0.2, y=1.32,
               rudder=False),
          dict(root_x=-5.5, root_z=1.5, height=0.65, root_chord=0.8, tip_chord=0.6, sweep_m=0.2, y=-1.32,
               rudder=False)],
    gear=[dict(name="main_L", x=2.6, y=1.0, top=0.95, radius=0.3, width=0.18),
          dict(name="main_R", x=2.6, y=-1.0, top=0.95, radius=0.3, width=0.18),
          dict(name="tail", x=-5.4, y=0.0, top=1.45, radius=0.15, width=0.1)],
    pylons=[((1.3, 1.05, 1.42), 0.8, 0.22), ((1.25, 2.0, 1.38), 0.8, 0.22)],
)


def build(variant, v):
    had = variant["id"] == HAD
    s = dict(SPEC)
    s["stores"] = ([("pod", (1.2, 1.05, 1.0), 1.6, 0.22), ("missile", (1.1, 2.0, 1.0), 1.63, 0.09)] if had else
                   [("pod", (1.2, 1.05, 1.0), 1.6, 0.22), ("missile", (1.1, 2.0, 1.0), 1.5, 0.13)])
    m = A.jet(v, s)
    hull = v.hull
    A.canopy("canopy", 6.4, 5.0, 1.75, 2.18, 0.47, m, hull, bows=(5.6,), peak=0.6)
    A.canopy("canopy_glass_rear", 4.95, 3.4, 2.1, 2.62, 0.5, m, hull, bows=(4.2,), peak=0.5)
    A.sensor_ball("nose_flir", (6.8, 0, 1.05), 0.22, m, hull)
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(1.9, 0.0, 2.25, 2.25), (1.6, 0.3, 1.98, 2.52, 2.25), (-0.9, 0.3, 1.98, 2.5, 2.25),
                                (-1.4, 0.15, 2.1, 2.4, 2.25)], m["paint"], hull, seg=14, loc=(0, side * 0.6, 0))
        box(f"exhaust_{k}", (0.6, 0.22, 0.3), (-1.2, side * 0.84, 2.42), m["nozzle"], hull,
            rot=(side * -0.5, -0.4, 0), lods=MID)
        # Missile rails under the outer station.
        A.store(f"rail_missile_{k}", "missile", (1.1, side * 2.0, 1.22), 1.6, 0.09, m, hull)
    A.body("mast_fairing", [(1.3, 0.0, 2.4, 2.4), (0.9, 0.42, 2.3, 2.75, 2.45), (-0.6, 0.42, 2.3, 2.7, 2.45),
                            (-1.1, 0.0, 2.45, 2.45)], m["paint"], hull, seg=14)
    cyl("mast", 0.14, 0.9, (0.3, 0, 3.05), "Z", m["dark"], hull, seg=12)
    A.rotor("main", (0.3, 0, 3.52), 6.5, 4, 0.5, m, hull, hub=0.36, mast=0.0, droop=0.025, phase=0.5)
    A.rotor("tail", (-6.55, -0.24, 2.95), 1.35, 3, 0.24, m, hull, hub=0.13, mast=0.0, droop=0.0,
            rot=(math.pi / 2, 0, 0), thick=0.12)
    if had:
        # The roof sight over the cockpit and the 30 mm turret under the chin.
        A.sensor_ball("roof_sight", (4.6, 0, 2.86), 0.24, m, hull)
        cyl("gun_turret", 0.2, 0.3, (5.6, 0, 0.62), "Z", m["dark"], hull, seg=12)
        cyl("gun_barrel", 0.05, 1.3, (6.3, 0, 0.55), "X", m["steel"], hull, seg=8, lods=MID)
    else:
        # The mast-mounted sight on the rotor head.
        cyl("mast_sight_post", 0.09, 0.3, (0.3, 0, 3.85), "Z", m["dark"], hull, seg=10)
        A.sensor_ball("mast_sight", (0.3, 0, 4.1), 0.22, m, hull)


def wreck(variant, v):
    A.crash(v, tail_x=-2.4, tail_yaw=0.42, tail_drop=0.1, blades_broken=(("main", 1),), seed=66)


run_disabled("tiger", CARDS, "nato_helicopter_green", build, wreck, skip=("dressing_", "blade_"))
