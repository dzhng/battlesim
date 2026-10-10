"""AH-1Z Viper and UH-1Y Venom, from assets/references/ah_1z_viper/ and
assets/references/uh_1y_venom/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/h1.py -- [--variant=<card>] [--wreck]

What the photos settle: the H-1 family's shared four-blade rotor, tail and
skids. The AH-1Z: a very narrow fuselage, gunner forward and pilot raised
behind, the targeting sight's turret on the nose and the 20 mm turret under
the chin, engines in pods either side of the mast, stub wings with rocket
pods, Hellfire racks and wing-tip Sidewinders. The UH-1Y: the Huey's cabin
with big sliding doors, the windscreen and roof windows, engines on the
roof, the FLIR ball under the nose, door guns. Both on skids, the tail
rotor on the fin's left. Marine Corps grey.

Built to the catalog frame, from published figures (rotorcraft rule:
fuselage length without blades, width over the widest fixed part, height to
the top of the rotor head): AH-1Z 13.67 x 4.06 x 3.76 m; UH-1Y 13.4 x 2.9 x
3.9 m (the sources are in each library's gaps).

The AH-1Z's 20 mm gun is the `chin` mount's rig, within the M197 turret's
elevation; its Hellfires are fixed in the hull (the `missiles` mount's
muzzle is the right rack's upper rail). The UH-1Y's left door gun is the
`door` mount's HMG rig; the right one stays stowed, the hull's.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import rig, run  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

AH1Z, UH1Y = "us_ah_1z_viper_attack_helicopter", "us_uh_1y_venom_utility_transport"
# The M197's elevation in its turret, degrees, and its short stroke.
GUN_PITCH_DEG = (-50.0, 18.0)
GUN_STROKE_M = 0.06
# A door gun's elevation on its pintle, degrees.
DOOR_PITCH_DEG = (-60.0, 20.0)
# The published length, width and height to the top of the rotor head,
# metres, that the art is drawn to (the hull's box is the airframe's, under
# the rotor).
PUBLISHED_M = {AH1Z: (13.67, 4.06, 3.76), UH1Y: (13.4, 2.9, 3.9)}
# How far forward the airframe's parts (placed in their drawing's frame)
# move to centre on the hull box.
AFT_M = {AH1Z: -0.021, UH1Y: 0.0}


def spec(variant):
    if variant["id"] == AH1Z:
        return dict(
            # The side photo: the tandem glass standing high out of a narrow
            # body, a tall engine and transmission deck over the wing.
            fuselage=[(6.83, 0.0, 1.35, 1.35), (6.32, 0.42, 1.0, 1.72, 1.35, 2.2), (5.21, 0.48, 0.9, 1.92, 1.4, 3.0),
                      (3.72, 0.5, 0.9, 2.28, 1.55, 3.0), (2.23, 0.55, 0.92, 2.62, 1.6, 3.2),
                      (0.47, 0.55, 0.95, 2.52, 1.6, 3.2), (-1.12, 0.4, 1.2, 2.32, 1.7, 3.0),
                      (-4.19, 0.2, 1.45, 1.96, 1.7, 2.4), (-6.32, 0.16, 1.5, 1.96, 1.72), (-6.83, 0.15, 1.52, 2.1, 1.75)],
            wing=[(1.3, 0.5, 1.65, 1.12, 0.16), (1.21, 1.92, 1.6, 0.93, 0.12)],
            stab=[(-4.09, 0.2, 1.78, 0.74, 0.08), (-4.19, 1.2, 1.78, 0.56, 0.06)],
            fins=[dict(root_x=-5.35, root_z=1.95, height=1.5, root_chord=1.3, tip_chord=0.84, sweep_m=0.47, thick=0.16)],
            pylons=[((1.12, 0.9, 1.6), 0.74, 0.24), ((1.07, 1.5, 1.58), 0.74, 0.24)],
            stores=[("pod", (1.02, 0.9, 1.12), 1.58, 0.27), ("missile", (0.93, 1.95, 1.6), 2.9, 0.065)],
        )
    return dict(
        # The side photo: a cabin 2 m deep under a tall engine and
        # transmission hump, the boom stepping down behind it.
        fuselage=[(6.7, 0.0, 1.15, 1.15), (6.4, 0.5, 0.75, 1.62, 1.15, 2.4), (5.4, 0.78, 0.58, 2.4, 1.3, 3.0),
                  (3.5, 0.92, 0.55, 2.58, 1.45, 3.5), (0.8, 0.92, 0.55, 2.58, 1.45, 3.5), (-0.5, 0.72, 0.9, 2.42, 1.6, 3.0),
                  (-2.0, 0.4, 1.4, 2.06, 1.7, 2.4), (-5.8, 0.2, 1.55, 1.96, 1.75), (-6.7, 0.18, 1.6, 2.1, 1.8)],
        bodies=[("fuselage_roof", [(2.8, 0.0, 2.5, 2.5), (2.2, 0.62, 2.5, 3.22, 2.75, 3.0),
                                   (-1.4, 0.62, 2.5, 3.22, 2.75, 3.0), (-2.4, 0.0, 2.6, 2.6)])],
        canopy=dict(x_front=6.6, x_back=4.9, sill=1.22, top=2.5, half_width=0.82, peak=0.85, bows=(5.8,), tail=0.9),
        stab=[(-4.0, 0.2, 1.8, 0.8, 0.08), (-4.1, 1.2, 1.8, 0.6, 0.06)],
        fins=[dict(root_x=-5.3, root_z=1.95, height=1.6, root_chord=1.3, tip_chord=0.85, sweep_m=0.5, thick=0.16)],
    )


def build(variant, v):
    ah = variant["id"] == AH1Z
    m = A.jet(v, spec(variant))
    hull = v.hull
    length, _, height = PUBLISHED_M[variant["id"]]
    tail = -length / 2
    rigged = rig(v.frame, v.root)
    if ah:
        A.canopy("canopy", 5.95, 4.6, 1.72, 2.5, 0.48, m, hull, bows=(5.8, 5.21), peak=0.6, tail=0.75)
        A.canopy("canopy_glass_rear", 4.6, 3.1, 2.02, 2.95, 0.5, m, hull, bows=(4.45, 3.85), peak=0.5, tail=0.8)
        A.body("fuselage_deck", [(3.2, 0.0, 2.3, 2.3), (2.6, 0.5, 2.25, 2.95, 2.5, 3.0),
                                 (-1.2, 0.5, 2.25, 2.95, 2.5, 3.0), (-2.4, 0.0, 2.2, 2.2)], m["paint"], hull, seg=16)
        A.sensor_ball("tss", (6.53, 0, 1.22), 0.34, m, hull)
        A.chin_gun(v, rigged, m, GUN_PITCH_DEG, GUN_STROKE_M, drum=0.24)
        for side, k in ((1, "L"), (-1, "R")):
            A.body(f"nacelle_{k}", [(1.67, 0.0, 2.7, 2.7), (1.4, 0.36, 2.34, 3.06, 2.7), (-0.74, 0.36, 2.34, 3.06, 2.7),
                                    (-1.21, 0.2, 2.5, 2.9, 2.7)], m["paint"], hull, seg=14, loc=(0, side * 0.56, 0))
            for j, dz in enumerate((0.16, -0.14)):
                A.store(f"hellfire_{k}_{j}", "missile", (0.98, side * 1.5, 1.3 + dz), 1.63, 0.09, m, hull)
            box(f"hellfire_rack_{k}", (0.9, 0.1, 0.42), (0.98, side * 1.5, 1.35), m["dark"], hull, lods=MID)
        cyl("mast", 0.14, 0.5, (0.3, 0, 3.2), "Z", m["dark"], hull, seg=12)
        hub_z = height - 0.27
    else:
        A.sensor_ball("flir", (5.9, 0, 0.42), 0.22, m, hull)
        for side, k in ((1, "L"), (-1, "R")):
            box(f"cabin_door_{k}", (2.2, 0.03, 1.3), (2.2, side * 0.9, 1.35), m["dark"], hull, lods=MID)
            box(f"cabin_window_{k}", (0.6, 0.03, 0.45), (2.6, side * 0.915, 1.7), m["glass"], hull, lods=MID)
            box(f"cockpit_window_{k}", (0.9, 0.03, 0.5), (4.6, side * 0.88, 1.6), m["glass"], hull, lods=MID)
            if side < 0:
                # The right door gun stowed on its mount, swung out.
                cyl(f"door_gun_{k}", 0.05, 1.1, (2.9, side * 1.4, 1.45), "X", m["steel"], hull, seg=8, lods=MID)
                box(f"door_gun_mount_{k}", (0.2, 0.55, 0.1), (2.4, side * 1.15, 1.4), m["dark"], hull, lods=NEAR)
        A.door_gun(v, rigged, m, DOOR_PITCH_DEG, inboard=0.5)
        A.blade_antenna("antenna_roof", (-0.5, 0, 2.3), 0.3, m, hull)
        cyl("mast", 0.14, 0.7, (0.4, 0, 3.25), "Z", m["dark"], hull, seg=12)
        hub_z = height - 0.27
    A.rotor("main", (0.35, 0, hub_z), 7.32 if ah else 7.44, 4, 0.6, m, hull, hub=0.38, mast=0.0, droop=0.025,
            phase=0.2)
    A.rotor("tail", (tail + 0.45, 0.28, 3.05), 1.17, 4, 0.24, m, hull, hub=0.13, mast=0.0, droop=0.0,
            rot=(-math.pi / 2, 0, 0), thick=0.12, phase=0.4)
    A.skids(2.79 if ah else 3.6, -1.21 if ah else -1.0, 1.05 if ah else 1.25, (2.14, -0.47) if ah else (2.7, -0.2),
            0.95, m, hull)

    # The Marines' low-visibility markings: the star on the boom, MARINES
    # behind it, the serial on the fin.
    ah = variant["id"] == AH1Z
    if ah:
        A.markings(v, [
            ("insignia", dict(kind="us_lowvis", centre=(-2.4, 1.2, 1.9), normal=(0, 1, 0), up=(0, 0, 1), size=0.3, onto=("fuselage",))),
            ("text", dict(text="MARINES", height=0.18, centre=(-4.4, 1.2, 1.72), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="lowvis_dark")),
            ("text", dict(text="168012", height=0.12, centre=(-5.9, 1.2, 2.55), normal=(0, 1, 0), up=(0, 0, 1), onto=("tail_fin_0",), colour="lowvis_dark")),
        ])
    else:
        A.markings(v, [
            ("insignia", dict(kind="us_lowvis", centre=(-3.0, 1.2, 1.82), normal=(0, 1, 0), up=(0, 0, 1), size=0.28, onto=("fuselage",))),
            ("text", dict(text="MARINES", height=0.18, centre=(-4.6, 1.2, 1.76), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="lowvis_dark")),
            ("text", dict(text="168781", height=0.12, centre=(-5.85, 1.2, 2.6), normal=(0, 1, 0), up=(0, 0, 1), onto=("tail_fin_0",), colour="lowvis_dark")),
        ])
    # The airframe as drawn runs off the frame's middle; the frame (and the
    # mounts' pivots in it) is centred on the hull box.
    v.hull.location.x += AFT_M[variant["id"]]


def wreck(variant, v):
    A.rotorcraft_crash(v, tail_x=-2.2, tail_yaw=-0.45, tail_drop=0.1, seed=11)


run("h1", "us_compass_grey", build, wreck,
    references=["assets/references/ah_1z_viper/references.json", "assets/references/uh_1y_venom/references.json"])
