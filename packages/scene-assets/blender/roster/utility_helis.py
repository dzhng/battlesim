"""NH90 TTH, AW101 Merlin HC4 and AW159 Wildcat AH1, from
assets/references/nh90/, assets/references/aw101_merlin/ and
assets/references/aw159_wildcat/. Disabled cards.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/utility_helis.py -- [--variant=<card>] [--wreck]

Three European utility rotorcraft drawn by one script (each is its own
variant; they share only the parts library). What the photos settle:

- NH90 TTH: the deep cabin with its big sliding door, the rear ramp under
  the up-swept tail, the roof fairing over two engines, sponsons carrying the
  main wheels, the twin nose wheel, four main blades, the tail rotor on the
  fin's left.
- Merlin HC4: a long cabin and rear ramp, three engines on the roof (two
  forward, one behind the gearbox), sponsons with the main wheels, the twin
  nose wheel, five main blades, the tall canted tail pylon and its rotor on
  the left with a stabiliser on the right; folded-blade navy heritage, the
  Commando's dark scheme.
- Wildcat AH1: the Lynx's compact cabin and short nose with the roof sight,
  the long tail boom and swept tail pylon with a half-span stabiliser on top,
  the four BERP-tipped blades, the main gear legs splayed out; Army grey.

Dimensions stated from published figures (rotorcraft rule: fuselage length
without blades, width over the widest fixed part, height to the top of the
rotor head): NH90 16.13 x 3.6 x 4.33 m; Merlin 19.53 x 4.52 x 4.95 m;
Wildcat 13.0 x 3.0 x 3.73 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID  # noqa: E402

NH90, MERLIN, WILDCAT = "europe_nh90_tth", "europe_aw101_merlin_hc4", "europe_aw159_wildcat_ah1"
CARDS = {NH90: (16.13, 3.6, 4.33), MERLIN: (19.53, 4.52, 4.95), WILDCAT: (13.0, 3.0, 3.73)}

SPECS = {
    NH90: dict(
        fuselage=[(8.07, 0.0, 1.3, 1.3), (7.7, 0.65, 0.75, 1.86, 1.3, 2.2), (6.6, 0.95, 0.55, 2.4, 1.4, 3.0),
                  (4.8, 1.05, 0.5, 2.6, 1.5, 3.5), (0.0, 1.05, 0.5, 2.6, 1.5, 3.5), (-1.8, 0.9, 0.85, 2.56, 1.7, 3.0),
                  (-3.2, 0.45, 1.55, 2.46, 2.0, 2.4), (-7.0, 0.25, 1.85, 2.4, 2.1), (-8.06, 0.22, 1.9, 2.9, 2.2)],
        bodies=[("fuselage_roof", [(3.6, 0.0, 2.6, 2.6), (3.0, 0.8, 2.5, 3.25, 2.75, 3.0), (-1.6, 0.8, 2.5, 3.25, 2.75, 3.0),
                                   (-2.4, 0.0, 2.65, 2.65)])],
        canopy=dict(x_front=7.9, x_back=6.2, sill=1.3, top=2.4, half_width=1.0, peak=0.8, bows=(7.1,)),
        stab=[(-7.0, 0.2, 2.2, 0.8, 0.08), (-7.1, 1.4, 2.2, 0.6, 0.06)],
        fins=[dict(root_x=-7.0, root_z=2.3, height=1.4, root_chord=1.3, tip_chord=0.9, sweep_m=0.6, thick=0.16)],
        gear=[dict(name="nose", x=5.6, y=0.0, top=0.7, radius=0.3, width=0.16, wheels=2, spread=0.32),
              dict(name="main_L", x=-0.3, y=1.4, top=0.9, radius=0.38, width=0.22),
              dict(name="main_R", x=-0.3, y=-1.4, top=0.9, radius=0.38, width=0.22)],
    ),
    MERLIN: dict(
        fuselage=[(9.77, 0.0, 1.4, 1.4), (9.4, 0.75, 0.8, 2.0, 1.4, 2.2), (8.2, 1.1, 0.6, 2.6, 1.5, 3.0),
                  (6.4, 1.2, 0.55, 2.8, 1.6, 3.5), (-2.0, 1.2, 0.55, 2.8, 1.6, 3.5), (-4.2, 1.0, 1.0, 2.8, 1.8, 3.0),
                  (-5.5, 0.5, 1.9, 2.76, 2.25, 2.4), (-8.8, 0.3, 2.15, 2.7, 2.4), (-9.76, 0.25, 2.2, 3.3, 2.5)],
        bodies=[("fuselage_roof", [(4.6, 0.0, 2.8, 2.8), (4.0, 0.95, 2.7, 3.6, 3.0, 3.0), (-2.4, 0.95, 2.7, 3.6, 3.0, 3.0),
                                   (-3.2, 0.0, 2.85, 2.85)])],
        canopy=dict(x_front=9.6, x_back=7.6, sill=1.35, top=2.62, half_width=1.1, peak=0.8, bows=(8.6,)),
        stab=[(-8.9, -0.3, 3.3, 0.9, 0.08), (-9.0, -1.6, 3.3, 0.7, 0.06)],
        fins=[dict(root_x=-8.6, root_z=2.7, height=1.8, root_chord=1.5, tip_chord=1.0, sweep_m=0.5, thick=0.16,
                   y=0.05, cant=0.1)],
        gear=[dict(name="nose", x=7.0, y=0.0, top=0.7, radius=0.32, width=0.18, wheels=2, spread=0.36),
              dict(name="main_L", x=-0.6, y=1.8, top=1.0, radius=0.4, width=0.22, wheels=2, spread=0.3),
              dict(name="main_R", x=-0.6, y=-1.8, top=1.0, radius=0.4, width=0.22, wheels=2, spread=0.3)],
    ),
    WILDCAT: dict(
        fuselage=[(6.5, 0.0, 1.05, 1.05), (6.2, 0.5, 0.7, 1.46, 1.05, 2.2), (5.2, 0.8, 0.5, 1.96, 1.15, 3.0),
                  (3.6, 0.9, 0.45, 2.16, 1.25, 3.5), (0.2, 0.9, 0.5, 2.16, 1.3, 3.5), (-1.2, 0.6, 0.95, 2.02, 1.45, 3.0),
                  (-5.4, 0.25, 1.25, 1.86, 1.55), (-6.5, 0.22, 1.3, 2.7, 1.8)],
        bodies=[("fuselage_roof", [(2.6, 0.0, 2.1, 2.1), (2.1, 0.65, 2.05, 2.66, 2.25, 3.0), (-1.2, 0.65, 2.05, 2.66, 2.25, 3.0),
                                   (-1.8, 0.0, 2.15, 2.15)])],
        canopy=dict(x_front=6.3, x_back=4.6, sill=1.1, top=1.96, half_width=0.85, peak=0.8, bows=(5.5,)),
        fins=[dict(root_x=-5.2, root_z=1.8, height=1.25, root_chord=1.0, tip_chord=0.7, sweep_m=0.6, thick=0.16)],
        gear=[dict(name="nose", x=4.4, y=0.0, top=0.6, radius=0.23, width=0.14, wheels=2, spread=0.26),
              dict(name="main_L", x=-0.4, y=1.38, top=0.8, radius=0.32, width=0.2),
              dict(name="main_R", x=-0.4, y=-1.38, top=0.8, radius=0.32, width=0.2)],
    ),
}
ROTOR = {NH90: (8.15, 4, 0.6, 4.05), MERLIN: (9.3, 5, 0.65, 4.67), WILDCAT: (6.4, 4, 0.5, 3.45)}
TAIL_ROTOR = {NH90: ((-7.7, 0.3, 3.3), 1.6), MERLIN: ((-9.4, 0.35, 3.9), 2.0), WILDCAT: ((-6.2, 0.25, 2.6), 1.1)}


def build(variant, v):
    vid = variant["id"]
    m = A.jet(v, SPECS[vid])
    hull = v.hull
    length = CARDS[vid][0]
    nose = length / 2
    if vid != WILDCAT:
        # Sponsons carrying the main wheels.
        sy = 1.35 if vid == NH90 else 1.75
        for side, k in ((1, "L"), (-1, "R")):
            A.body(f"fuselage_sponson_{k}", [(1.2, 0.0, 0.8, 0.8), (0.8, 0.45, 0.45, 1.15, 0.8, 3.0),
                                              (-1.4, 0.45, 0.45, 1.15, 0.8, 3.0), (-1.9, 0.0, 0.85, 0.85)],
                   m["paint"], hull, seg=14, loc=(0, side * sy, 0))
        box("ramp", (0.08, 1.8, 1.6), (-2.0 if vid == NH90 else -4.4, 0, 1.55), m["dark"], hull,
            rot=(0, -0.7, 0), lods=MID)
    for side, k in ((1, "L"), (-1, "R")):
        half = SPECS[vid]["fuselage"][4][1]
        box(f"cabin_door_{k}", (1.6, 0.03, 1.3), (1.6, side * half, 1.4), m["dark"], hull, lods=MID)
        box(f"cabin_window_{k}", (0.5, 0.03, 0.45), (1.9, side * (half + 0.01), 1.75), m["glass"], hull, lods=MID)
        box(f"cockpit_window_{k}", (0.8, 0.03, 0.5), (nose - 2.0, side * (half - 0.08), 1.55), m["glass"], hull,
            lods=MID)
        cyl(f"engine_intake_{k}", 0.24, 0.06, (2.9 if vid != WILDCAT else 2.0, side * 0.45,
                                               (3.0 if vid == NH90 else 3.25 if vid == MERLIN else 2.45)), "X",
            m["black"], hull, seg=12, lods=MID)
    if vid == MERLIN:
        A.body("nacelle_rear", [(-0.4, 0.0, 3.55, 3.55), (-0.8, 0.36, 3.2, 3.9, 3.55), (-2.4, 0.36, 3.2, 3.9, 3.55),
                                (-2.8, 0.2, 3.35, 3.75, 3.55)], m["paint"], hull, seg=14)
    if vid == WILDCAT:
        A.sensor_ball("roof_sight", (5.55, 0, 2.05), 0.22, m, hull)
        A.surface("tail_stab", [(-5.95, 0.1, 3.05, 0.6, 0.06), (-6.05, 1.0, 3.05, 0.45, 0.05)], m["paint"], hull)
        A.mirrored(lambda side, k: box(f"main_leg_{k}", (0.12, 0.8, 0.08), (-0.4, side * 1.0, 0.75), m["gear"], hull,
                                       rot=(side * -0.3, 0, 0), lods=MID))
    radius, blades, chord, hub_z = ROTOR[vid]
    cyl("mast", 0.18, 0.8, (0.3, 0, hub_z - 0.45), "Z", m["dark"], hull, seg=12)
    A.rotor("main", (0.3, 0, hub_z), radius, blades, chord, m, hull, hub=0.42, mast=0.0, droop=0.025, phase=0.2,
            tip_chord=chord * (1.2 if vid == WILDCAT else 1.0))
    at, r = TAIL_ROTOR[vid]
    A.rotor("tail", at, r, 4, 0.26, m, hull, hub=0.15, mast=0.0, droop=0.0, rot=(-math.pi / 2, 0, 0), thick=0.12)
    A.blade_antenna("antenna_belly", (-0.8, 0, 0.5), 0.25, m, hull, down=True)

    # The Heer's cross and serial on the NH90; the Royal Navy's roundel and
    # title on the Merlin; the Army Air Corps' on the Wildcat.
    if vid == NH90:
        A.markings(v, [
            ("insignia", dict(kind="de", centre=(-4.5, 1.2, 2.12), normal=(0, 1, 0), up=(0, 0, 1), size=0.25, onto=("fuselage",))),
            ("text", dict(text="79+24", height=0.24, centre=(-2.4, 1.2, 1.9), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
        ])
    elif vid == MERLIN:
        A.markings(v, [
            ("insignia", dict(kind="uk", centre=(-6.5, 1.2, 2.42), normal=(0, 1, 0), up=(0, 0, 1), size=0.24, onto=("fuselage",))),
            ("text", dict(text="ROYAL NAVY", height=0.3, centre=(-1.0, 1.2, 2.2), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
            ("text", dict(text="ZJ123", height=0.22, centre=(-4.6, 1.2, 2.2), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
        ])
    else:
        A.markings(v, [
            ("insignia", dict(kind="uk_lowvis", centre=(-3.0, 1.2, 1.62), normal=(0, 1, 0), up=(0, 0, 1), size=0.22, onto=("fuselage",))),
            ("text", dict(text="ARMY", height=0.22, centre=(-4.6, 1.2, 1.55), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
            ("text", dict(text="ZZ387", height=0.18, centre=(-1.0, 1.2, 1.4), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
        ])

def wreck(variant, v):
    A.crash(v, tail_x=-3.6 if variant["id"] != WILDCAT else -2.2, tail_yaw=0.4, tail_drop=0.08,
            blades_broken=(("main", 1),), seed=90)


# The Army's Wildcats fly in grey, as their photos show; NH90 and Merlin in green.
SCHEME = {NH90: "nato_helicopter_green", MERLIN: "nato_helicopter_green", WILDCAT: "nato_air_grey"}

run_disabled("utility_helis", CARDS, SCHEME, build, wreck, skip=("dressing_", "blade_"))
