"""UH-60M Black Hawk and Z-20, from assets/references/uh_60_black_hawk/ and
assets/references/z_20/. Disabled cards.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/hawk.py -- [--variant=<card>] [--wreck]

What the photos settle: one layout (the Z-20 is the Chinese medium
helicopter on the Black Hawk's pattern). A low, wide cabin with big sliding
doors, the glazed cockpit with its sloping windscreen, two engines side by
side on the roof either side of the transmission, the main rotor on its
mast; the tail cone rising to a swept tail pylon with the tail rotor canted
on its right side, the stabilator low on the tail; fixed main wheels on
struts under the cabin and a tail wheel. The UH-60M: four wide-chord
blades, door guns, Army green. The Z-20: five blades, a slimmer nose and
fairing, PLA grey.

Dimensions stated from published figures (the rule for every rotorcraft
here: fuselage length nose to tail without blades, width over the widest
fixed part, height to the top of the main rotor head): UH-60M 15.26 x 4.38
x 3.76 m; Z-20 15.4 x 4.4 x 3.9 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

UH60, Z20 = "us_uh_60_black_hawk_uh_60m", "eastern_z_20_utility_transport"
CARDS = {UH60: (15.26, 4.38, 3.76), Z20: (15.4, 4.4, 3.9)}
SCHEME = {UH60: "us_army_aviation", Z20: "chinese_air_grey"}


def spec(variant):
    z20 = variant["id"] == Z20
    nose = CARDS[variant["id"]][0] / 2
    tail = -CARDS[variant["id"]][0] / 2
    return dict(
        # The photos: a short, tall cabin under the engines' hump, the
        # mains forward under the cockpit's back, the tail wheel far aft.
        fuselage=[(nose, 0.0, 1.28, 1.28), (nose - 0.4, 0.55, 0.78, 1.7, 1.18, 2.2),
                  (nose - 1.4, 0.84, 0.56, 2.14, 1.15, 2.4), (nose - 2.8, 0.98, 0.48, 2.36, 1.2, 3.0),
                  (2.0, 1.06, 0.45, 2.4, 1.28, 3.5), (-0.5, 1.0, 0.5, 2.34, 1.32, 3.5), (-1.6, 0.7, 0.9, 2.12, 1.5, 2.6),
                  (-4.0, 0.38, 1.35, 2.05, 1.7, 2.2), (-6.6, 0.28, 1.5, 2.0, 1.75), (tail, 0.24, 1.55, 2.2, 1.85)],
        bodies=[("fuselage_roof", [(3.7, 0.0, 2.35, 2.35), (3.1, 0.8, 2.25, 2.86, 2.45, 3.0),
                                   (-0.8, 0.75, 2.25, 2.9, 2.45, 3.0), (-2.1, 0.0, 2.35, 2.35)])],
        canopy=dict(x_front=nose - 0.3, x_back=nose - 2.1, sill=1.25, top=2.32, half_width=0.96, peak=0.8,
                    bows=(nose - 1.2, nose - 0.7), tail=0.9),
        stab=[(tail + 1.0, 0.25, 1.72, 1.1, 0.12), (tail + 0.8, 2.19, 1.72, 0.9, 0.08)],
        fins=[dict(root_x=tail + 1.3, root_z=1.95, height=1.7 if not z20 else 1.8, root_chord=1.6, tip_chord=1.05,
                   sweep_m=0.8, thick=0.16)],
        gear=[dict(name="main_L", x=nose - 3.3, y=1.38, top=0.95, radius=0.38, width=0.22),
              dict(name="main_R", x=nose - 3.3, y=-1.38, top=0.95, radius=0.38, width=0.22),
              dict(name="tail", x=tail + 1.5, y=0.0, top=1.5, radius=0.2, width=0.12)],
    )


def build(variant, v):
    s = spec(variant)
    z20 = variant["id"] == Z20
    m = A.jet(v, s)
    hull = v.hull
    length = CARDS[variant["id"]][0]
    tail = -length / 2
    height = CARDS[variant["id"]][2]
    # The engines either side of the transmission, their exhausts turned out.
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(2.8, 0.0, 2.6, 2.6), (2.5, 0.34, 2.3, 2.92, 2.6), (0.0, 0.34, 2.28, 2.9, 2.58),
                                (-0.6, 0.24, 2.42, 2.78, 2.6)], m["paint"], hull, seg=14, loc=(0, side * 0.58, 0))
        cyl(f"nacelle_{k}_intake", 0.26, 0.04, (2.82, side * 0.58, 2.6), "X", m["black"], hull, seg=14, lods=MID)
        cyl(f"exhaust_{k}", 0.2, 0.5, (-0.7, side * 0.86, 2.6), "Y", m["nozzle"], hull, seg=12,
            rot=(0, 0, side * 0.4), lods=MID)
        # Cabin door rails and windows, cockpit door windows.
        box(f"cabin_door_{k}", (1.8, 0.03, 1.2), (1.2, side * 1.06, 1.25), m["dark"], hull, lods=MID)
        box(f"cabin_window_{k}", (0.5, 0.03, 0.42), (1.6, side * 1.075, 1.55), m["glass"], hull, lods=MID)
        box(f"cockpit_window_{k}", (0.9, 0.03, 0.55), (length / 2 - 2.3, side * 0.98, 1.55), m["glass"], hull,
            lods=MID)
        if not z20:
            # The door gun on its mount in the cabin window.
            cyl(f"door_gun_{k}", 0.03, 1.1, (2.2, side * 1.2, 1.5), "X", m["steel"], hull, seg=6, lods=NEAR)
    cyl("mast", 0.2, 0.75, (0.3, 0, 2.95), "Z", m["dark"], hull, seg=12)
    A.rotor("main", (0.3, 0, height - 0.22), 8.18 if not z20 else 8.2, 4 if not z20 else 5, 0.53, m, hull,
            hub=0.42, mast=0.0, droop=0.025, phase=0.4)
    A.rotor("tail", (tail + 0.55, -0.32, 3.05), 1.68, 4, 0.25, m, hull, hub=0.16, mast=0.0, droop=0.0,
            rot=(math.pi / 2 - 0.35, 0, 0), thick=0.12)
    A.mirrored(lambda side, k: box(f"step_{k}", (0.4, 0.2, 0.04), (length / 2 - 2.6, side * 1.0, 0.5), m["dark"], hull,
                                   lods=NEAR))
    A.blade_antenna("antenna_belly", (-0.8, 0, 0.6), 0.25, m, hull, down=True)
    if z20:
        A.sensor_ball("nose_sensor", (length / 2 - 0.7, 0, 0.62), 0.18, m, hull)

    # The Army's black lettering on the UH-60; the PLA's star on the Z-20.
    if variant["id"] == UH60:
        A.markings(v, [
            ("text", dict(text="U.S. ARMY", height=0.25, centre=(-3.2, 1.2, 1.72), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
            ("text", dict(text="20-21000", height=0.12, centre=(-1.2, 1.2, 1.3), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
        ])
    else:
        A.markings(v, [
            ("insignia", dict(kind="cn_star", centre=(-3.0, 1.2, 1.72), normal=(0, 1, 0), up=(0, 0, 1), size=0.3, onto=("fuselage",))),
            ("text", dict(text="20", height=0.4, centre=(3.0, 1.2, 1.5), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="red")),
        ])

def wreck(variant, v):
    A.crash(v, tail_x=-3.0, tail_yaw=0.45, tail_drop=0.1, blades_broken=(("main", 1), ("main", 3)), seed=60)


run_disabled("hawk", CARDS, SCHEME, build, wreck, skip=("dressing_", "blade_"))
