"""CH-47F Chinook and Chinook HC6, from assets/references/ch_47_chinook/. Disabled cards.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/chinook.py -- [--variant=<card>] [--wreck]

What the photos settle: the long box fuselage with its rounded cockpit nose,
the forward rotor pylon over the cockpit and the tall aft pylon carrying
the rear rotor and the two engines on its flanks, the fuel sponsons along
the lower sides, the loading ramp under the tail, two three-blade rotors
turning in tandem, four wheels (twin forward, single aft). The CH-47F in
the Army's tan, as its photos show; the RAF's HC6 in its dark green.

Dimensions stated from published figures (rotorcraft rule: fuselage length
without blades, width over the sponsons, height to the top of the rear rotor
head): 15.87 x 3.78 x 5.68 m, both.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

F, HC6 = "us_ch_47_chinook_ch_47f", "europe_ch_47_chinook_hc6"
CARDS = {F: (15.87, 3.78, 5.68), HC6: (15.87, 3.78, 5.68)}
SCHEME = {F: "us_desert_tan", HC6: "nato_helicopter_green"}
SPEC = dict(
    fuselage=[(7.93, 0.0, 1.4, 1.4), (7.65, 0.8, 0.75, 2.3, 1.5, 2.4), (6.7, 1.15, 0.6, 2.9, 1.6, 3.0),
              # The side photos: a box cabin, its belly flat from the cockpit aft.
              (5.0, 1.2, 0.55, 3.0, 1.6, 4.0), (-5.5, 1.2, 0.6, 3.0, 1.6, 4.0), (-7.0, 1.1, 1.05, 3.0, 1.85, 3.5),
              (-7.93, 1.0, 1.6, 3.0, 2.15, 3.0)],
    bodies=[("fuselage_pylon_front", [(6.6, 0.0, 3.0, 3.0), (6.2, 0.55, 2.9, 3.7, 3.1, 3.0),
                                      (4.2, 0.55, 2.9, 3.6, 3.1, 3.0), (3.4, 0.0, 3.0, 3.0)]),
            ("fuselage_pylon_rear", [(-3.6, 0.0, 3.0, 3.0), (-4.4, 0.7, 2.9, 4.3, 3.2, 3.5),
                                     (-6.6, 0.7, 2.9, 5.0, 3.3, 3.5), (-7.93, 0.55, 2.9, 4.6, 3.3, 3.0)])],
    canopy=dict(x_front=7.75, x_back=6.2, sill=1.6, top=2.75, half_width=1.08, peak=0.8, bows=(7.1,)),
    gear=[dict(name="front_L", x=4.0, y=1.55, top=0.95, radius=0.44, width=0.22, wheels=2, spread=0.5),
          dict(name="front_R", x=4.0, y=-1.55, top=0.95, radius=0.44, width=0.22, wheels=2, spread=0.5),
          dict(name="rear_L", x=-3.8, y=1.55, top=0.95, radius=0.44, width=0.24),
          dict(name="rear_R", x=-3.8, y=-1.55, top=0.95, radius=0.44, width=0.24)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    for side, k in ((1, "L"), (-1, "R")):
        # Fuel sponson along the lower side, the engine on the aft pylon.
        A.body(f"fuselage_sponson_{k}", [(4.6, 0.0, 0.95, 0.95), (4.0, 0.36, 0.55, 1.35, 0.95, 3.0),
                                          (-4.4, 0.36, 0.55, 1.35, 0.95, 3.0), (-5.0, 0.0, 0.95, 0.95)],
               m["paint"], hull, seg=14, loc=(0, side * 1.53, 0))
        A.body(f"nacelle_{k}", [(-3.7, 0.0, 3.55, 3.55), (-4.0, 0.42, 3.13, 3.97, 3.55), (-6.4, 0.42, 3.13, 3.97, 3.55),
                                (-6.9, 0.3, 3.25, 3.85, 3.55)], m["paint"], hull, seg=14, loc=(0, side * 1.12, 0))
        cyl(f"nacelle_{k}_intake", 0.34, 0.04, (-3.72, side * 1.12, 3.55), "X", m["black"], hull, seg=14, lods=MID)
        cyl(f"exhaust_{k}", 0.3, 0.2, (-7.0, side * 1.12, 3.55), "X", m["nozzle"], hull, seg=12, lods=MID)
        for j in range(6):
            # The round cabin windows of the side photos.
            cyl(f"window_{k}_{j}", 0.2, 0.03, (4.4 - j * 1.6, side * 1.215, 2.1), "Y", m["glass"], hull, seg=14,
                lods=MID)
        box(f"cabin_door_{k}", (0.9, 0.03, 1.6), (5.4, side * 1.21, 1.6), m["dark"], hull, lods=MID)
    # The loading ramp under the tail, closed.
    box("ramp", (0.08, 2.1, 1.6), (-7.96, 0, 1.75), m["dark"], hull, rot=(0, -0.45, 0), lods=MID)
    cyl("mast_front", 0.2, 0.6, (5.6, 0, 3.9), "Z", m["dark"], hull, seg=12)
    cyl("mast_rear", 0.2, 0.55, (-6.0, 0, 5.15), "Z", m["dark"], hull, seg=12)
    A.rotor("front", (5.6, 0, 4.2), 9.14, 3, 0.8, m, hull, hub=0.44, mast=0.0, droop=0.03, phase=0.3)
    A.rotor("rear", (-6.0, 0, 5.43), 9.14, 3, 0.8, m, hull, hub=0.44, mast=0.0, droop=0.03, phase=0.3 + 1.047)
    A.mirrored(lambda side, k: box(f"step_{k}", (0.4, 0.2, 0.04), (6.2, side * 1.25, 0.7), m["dark"], hull,
                                   lods=NEAR))

    # The Army's black lettering on the F; the RAF's roundel, title and
    # serial on the HC6.
    if variant["id"] == F:
        A.markings(v, [
            ("text", dict(text="U.S. ARMY", height=0.35, centre=(-3.0, 1.2, 2.2), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
            ("text", dict(text="08-08034", height=0.14, centre=(-6.6, 1.2, 2.6), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
        ])
    else:
        A.markings(v, [
            ("insignia", dict(kind="uk_lowvis", centre=(-1.5, 1.2, 1.9), normal=(0, 1, 0), up=(0, 0, 1), size=0.4, onto=("fuselage",))),
            ("text", dict(text="ROYAL AIR FORCE", height=0.22, centre=(2.0, 1.2, 2.5), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
            ("text", dict(text="ZK558", height=0.3, centre=(-6.0, 1.2, 2.0), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
        ])

def wreck(variant, v):
    A.crash(v, tail_x=-4.6, tail_yaw=0.12, tail_drop=0.05, blades_broken=(("front", 0), ("rear", 1)), seed=47)


run_disabled("chinook", CARDS, SCHEME, build, wreck, skip=("dressing_", "blade_"))
