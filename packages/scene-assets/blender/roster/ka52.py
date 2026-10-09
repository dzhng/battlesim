"""Ka-52M Alligator, from assets/references/ka_52_alligator/. A disabled card.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/ka52.py -- [--wreck]

What the photos settle: two three-blade rotors stacked coaxially on one
mast (no tail rotor), the wide side-by-side cockpit under a big glazed
canopy, the sensor ball under the nose and the radar in its nose dome (the
M), the 30 mm cannon on the right flank, engines either side of the
gearbox, broad stub wings with three stations each and wing-tip pods, the
tail with a horizontal stabiliser carrying two end-plate fins and a
central fin; retractable tricycle gear. Dark grey.

Dimensions stated from published figures (rotorcraft rule: fuselage length
without blades, width over the wing-tip pods, height to the top of the upper
rotor head): 13.53 x 7.3 x 4.93 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

CARDS = {"eastern_ka_52_alligator_ka_52m": (13.53, 7.3, 4.93)}
SPEC = dict(
    fuselage=[(6.77, 0.0, 1.45, 1.45), (6.4, 0.55, 1.0, 1.9, 1.45, 2.2), (5.3, 0.9, 0.82, 2.1, 1.5, 3.0),
              (3.6, 1.0, 0.8, 2.55, 1.6, 3.2), (1.2, 1.0, 0.85, 2.6, 1.65, 3.4), (-1.0, 0.82, 1.0, 2.5, 1.75, 3.2),
              (-3.0, 0.4, 1.45, 2.3, 1.85, 2.6), (-5.6, 0.32, 1.6, 2.25, 1.9, 2.4), (-6.76, 0.3, 1.65, 2.35, 1.95)],
    canopy=dict(x_front=6.1, x_back=3.6, sill=1.62, top=2.8, half_width=0.98, peak=0.6, bows=(5.6, 4.9), tail=0.8),
    wing=[(1.4, 0.95, 1.9, 1.7, 0.22), (1.2, 3.5, 1.86, 1.4, 0.14)],
    stab=[(-5.6, 0.3, 2.05, 1.1, 0.12), (-5.7, 1.9, 2.05, 0.9, 0.08)],
    fins=[dict(root_x=-5.2, root_z=2.25, height=1.2, root_chord=1.5, tip_chord=1.0, sweep_m=0.6, thick=0.16),
          dict(root_x=-5.4, root_z=1.6, height=1.0, root_chord=1.0, tip_chord=0.8, sweep_m=0.3, y=1.9, rudder=False),
          dict(root_x=-5.4, root_z=1.6, height=1.0, root_chord=1.0, tip_chord=0.8, sweep_m=0.3, y=-1.9,
               rudder=False)],
    gear=[dict(name="nose", x=5.0, y=0.0, top=0.9, radius=0.28, width=0.16, wheels=2, spread=0.3),
          dict(name="main_L", x=-0.2, y=1.2, top=1.0, radius=0.38, width=0.22),
          dict(name="main_R", x=-0.2, y=-1.2, top=1.0, radius=0.38, width=0.22)],
    pylons=[((1.1, 1.6, 1.84), 0.8, 0.22), ((1.05, 2.4, 1.84), 0.8, 0.22), ((1.0, 3.1, 1.84), 0.8, 0.22)],
    stores=[("pod", (1.0, 1.6, 1.38), 1.7, 0.25), ("missile", (0.95, 2.4, 1.45), 2.0, 0.12),
            ("pod", (0.9, 3.1, 1.38), 1.7, 0.25), ("pod", (0.6, 3.56, 1.86), 1.8, 0.09)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    A.body("nose_radar", [(6.85, 0.0, 1.45, 1.45), (6.6, 0.4, 1.12, 1.78, 1.45), (6.2, 0.48, 1.05, 1.85, 1.45)],
           m["dark"], hull, seg=18)
    A.sensor_ball("sensor", (5.6, 0, 0.7), 0.32, m, hull)
    # The 30 mm cannon on the right flank.
    box("gun_mount", (1.6, 0.3, 0.35), (2.0, -1.05, 1.15), m["paint"], hull, bevel=0.03)
    cyl("gun_barrel", 0.06, 2.0, (3.6, -1.05, 1.15), "X", m["steel"], hull, seg=8, lods=MID)
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(1.9, 0.0, 2.55, 2.55), (1.6, 0.36, 2.2, 2.9, 2.55), (-0.8, 0.36, 2.2, 2.9, 2.55),
                                (-1.3, 0.26, 2.3, 2.8, 2.55)], m["paint"], hull, seg=14, loc=(0, side * 0.82, 0))
        box(f"exhaust_{k}", (0.7, 0.3, 0.4), (-1.2, side * 1.0, 2.65), m["nozzle"], hull, rot=(0, -0.3, side * 0.4),
            lods=MID)
    cyl("mast", 0.2, 2.2, (0.3, 0, 3.75), "Z", m["dark"], hull, seg=12)
    A.rotor("lower", (0.3, 0, 3.5), 7.25, 3, 0.5, m, hull, hub=0.42, mast=0.0, droop=0.02, phase=0.0)
    A.rotor("upper", (0.3, 0, 4.7), 7.25, 3, 0.5, m, hull, hub=0.42, mast=0.0, droop=0.02, phase=1.047)

    # The VVS's red star on the boom, a red bort number on the nose.
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=(-4.5, 1.2, 1.92), normal=(0, 1, 0), up=(0, 0, 1), size=0.28, onto=("fuselage",))),
        ("text", dict(text="12", height=0.42, centre=(4.0, 1.2, 1.55), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="red")),
    ])

def wreck(variant, v):
    A.crash(v, tail_x=-2.8, tail_yaw=0.35, tail_drop=0.1, blades_broken=(("upper", 1), ("lower", 2)), seed=52)


run_disabled("ka52", CARDS, "russian_air_grey", build, wreck, skip=("dressing_", "blade_"))
