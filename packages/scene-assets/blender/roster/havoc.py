"""Mi-28NM, from assets/references/mi_28_havoc/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/havoc.py -- [--wreck]

What the photos settle: the stepped tandem cockpits under flat armoured
glass, the sensor turret in the nose and the 30 mm cannon on its chin
mount, engines in nacelles either side of the gearbox with down-turned
exhaust suppressors, stub wings with anhedral and two stations each plus
wing-tip launchers, the tail with its X-shaped tail rotor on the right and
the stabilator, the NM's radar dome over the rotor head; fixed main wheels
and tail wheel. Dark grey.

Built to the catalog frame, from published figures (rotorcraft rule:
fuselage length without blades, width over the wing tips, height to the top
of the mast radar): 17.01 x 4.88 x 4.7 m.

The 30 mm cannon is the `chin` mount's rig; the missiles are fixed in the
hull (the `missiles` mount's muzzle is the right rack's upper tube).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import rig, run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

# The 2A42's elevation in its chin mount, degrees, and its stroke.
GUN_PITCH_DEG = (-40.0, 13.0)
GUN_STROKE_M = 0.08
SPEC = dict(
    fuselage=[(8.2, 0.0, 1.35, 1.35), (7.9, 0.5, 1.0, 1.8, 1.35, 2.4), (6.6, 0.64, 0.85, 2.05, 1.45, 3.0),
              (4.8, 0.7, 0.85, 2.45, 1.6, 3.0), (2.4, 0.82, 0.9, 2.66, 1.65, 3.4), (0.0, 0.78, 0.95, 2.6, 1.7, 3.4),
              (-1.6, 0.5, 1.25, 2.42, 1.8, 3.0), (-4.5, 0.26, 1.6, 2.2, 1.9, 2.4), (-7.6, 0.22, 1.75, 2.3, 2.0),
              (-8.5, 0.2, 1.8, 2.6, 2.1)],
    wing=[(1.6, 0.75, 2.0, 1.4, 0.18), (1.35, 2.32, 1.72, 1.1, 0.12)],
    stab=[(-7.2, 0.2, 2.0, 0.9, 0.1), (-7.3, 1.4, 2.0, 0.7, 0.06)],
    fins=[dict(root_x=-7.0, root_z=2.2, height=1.7, root_chord=1.3, tip_chord=0.9, sweep_m=0.7, thick=0.16)],
    gear=[dict(name="main_L", x=3.0, y=1.2, top=1.05, radius=0.38, width=0.22),
          dict(name="main_R", x=3.0, y=-1.2, top=1.05, radius=0.38, width=0.22),
          dict(name="tail", x=-6.6, y=0.0, top=1.75, radius=0.2, width=0.12)],
    pylons=[((1.3, 1.25, 1.9), 0.8, 0.2), ((1.25, 1.95, 1.78), 0.8, 0.2)],
    stores=[("pod", (1.2, 1.25, 1.45), 1.7, 0.25), ("missile", (1.1, 2.32, 1.72), 1.8, 0.1)],
)
# How far forward the airframe's parts (placed in their drawing's frame)
# move to centre on the hull box.
AFT_M = 0.064


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # The photo's flat armoured glass stands tall out of the body.
    A.canopy("canopy", 7.8, 6.25, 1.62, 2.45, 0.6, m, hull, bows=(7.3, 6.8), peak=0.6, tail=0.75)
    A.canopy("canopy_glass_rear", 6.25, 4.55, 1.95, 2.98, 0.64, m, hull, bows=(5.7, 5.1), peak=0.5, tail=0.8)
    A.sensor_ball("sensor", (8.15, 0, 1.1), 0.32, m, hull)
    A.chin_gun(v, rig(v.frame, v.root), m, GUN_PITCH_DEG, GUN_STROKE_M, drum=0.28)
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(2.3, 0.0, 2.4, 2.4), (2.0, 0.34, 2.06, 2.74, 2.4), (-0.6, 0.34, 2.06, 2.74, 2.4),
                                (-1.0, 0.24, 2.16, 2.64, 2.4)], m["paint"], hull, seg=14, loc=(0, side * 0.95, 0))
        box(f"exhaust_{k}", (0.8, 0.3, 0.5), (-1.0, side * 1.05, 2.1), m["nozzle"], hull, rot=(0, -0.6, 0), lods=MID)
        for j, dz in enumerate((0.16, -0.14)):
            A.store(f"atgm_{k}_{j}", "missile", (1.15, side * 1.95 + (0.06 if j else -0.06), 1.45 + dz), 1.8, 0.1, m,
                    hull)
    cyl("mast", 0.18, 0.9, (0.3, 0, 3.15), "Z", m["dark"], hull, seg=12)
    A.rotor("main", (0.3, 0, 3.7), 8.6, 5, 0.58, m, hull, hub=0.44, mast=0.0, droop=0.03, phase=0.3)
    A.body("mast_radar", [(0.95, 0.0, 4.35, 4.35), (0.75, 0.48, 4.1, 4.62, 4.36), (-0.15, 0.5, 4.05, 4.7, 4.38),
                          (-0.35, 0.0, 4.38, 4.38)], m["paint"], hull, seg=16)
    cyl("mast_radar_post", 0.12, 0.3, (0.3, 0, 3.95), "Z", m["dark"], hull, seg=10)
    # The X-shaped tail rotor: two two-blade rotors at an angle on one hub.
    A.rotor("tail", (-7.95, -0.3, 3.4), 1.9, 4, 0.28, m, hull, hub=0.16, mast=0.0, droop=0.0,
            rot=(math.pi / 2, 0, 0), thick=0.12, phase=0.4)

    # The VVS's red star on the boom, a red bort number on the nose.
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=(-5.5, 1.2, 1.95), normal=(0, 1, 0), up=(0, 0, 1), size=0.28, onto=("fuselage",))),
        ("text", dict(text="45", height=0.42, centre=(5.8, 1.2, 1.55), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="red")),
    ])
    # The airframe as drawn runs off the frame's middle; the frame (and the
    # mounts' pivots in it) is centred on the hull box.
    v.hull.location.x += AFT_M


def wreck(variant, v):
    A.rotorcraft_crash(v, tail_x=-2.8, tail_yaw=0.42, tail_drop=0.1, seed=28)


run("havoc", "russian_air_grey", build, wreck, references="assets/references/mi_28_havoc/references.json")
