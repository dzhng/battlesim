"""T-14 Armata, from assets/references/t14/, on the Armata platform (`roster/armata.py`).

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/t14.py -- [--wreck]

Engine at the rear. The crew capsule's three hatches sit in a row on the deck
ahead of the turret; the unmanned turret is a long, tall faceted shell, half
the hull's length, under a sloped sensor housing, the gunner's sight box on
its right front, the commander's panoramic sight on a mast over its left, a
remote Kord on the roof, the Afganit launcher racks low on each flank, a
slatted cage round the bustle. The 125 mm 2A82-1M has a thermal sleeve and
no fume extractor.

Built to the catalog frame (hull 8.7 x 3.5 x 3.3 m, gun pivot 1.98 m).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import vehicle_parts as VP  # noqa: E402
from armata import (DECK, armour_kit, engine_deck, hull, running_gear, skirts, turret_kit,  # noqa: E402
                    wreck)
from parts import box, cyl, empty, loft, prism  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID


def build(variant, v):
    m, h = v.mats, v.hull
    half = v.length / 2
    top = hull(v)
    running_gear(v)
    skirts(v)
    armour_kit(v)
    engine_deck(v, -half + 0.4, -2.0)
    # The crew capsule's three hatches across the deck ahead of the turret,
    # each with its periscope block.
    for k, y in enumerate((0.75, 0.0, -0.75)):
        VP.hatch(f"crew_hatch_{k}", (top - 0.55, y, DECK), m, h, radius=0.26)
        VP.periscope(f"crew_periscope_{k}", (top - 0.18, y, DECK), m, h, size=(0.12, 0.30, 0.10))
    mounts = rig(v.frame, v.root, trunnion={"cannon": 1.05})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    base = DECK - v.frame["mounts"][0]["pivot_m"][2]
    t14_turret(v, turret, base)
    gun_2a82(v, gun)
    kord_station(v, hmg, hmg_gun)


def t14_turret(v, turret, base):
    """The unmanned turret: long and tall over the hull, its walls overhanging
    a tucked-in foot where the Afganit racks sit, its sloped front, the sensor
    housing over it, the gunner's sight box, the commander's sight, and the
    bustle cage reaching back over the engine deck."""
    m = v.mats
    z0, z1 = base + 0.04, base + 1.10
    waist = z0 + 0.40  # the walls' widest line, over the tucked-in foot
    wall = [(2.00, 0.62), (1.70, 1.55), (-1.80, 1.60), (-2.45, 1.32), (-2.45, -1.32), (-1.80, -1.60), (1.70, -1.55),
            (2.00, -0.62)]
    foot = [(x - 0.20 if x > 0 else x + 0.10, y - (0.22 if y > 0 else -0.22)) for x, y in wall]
    crown = [(1.65, 0.58), (1.40, 1.40), (-1.75, 1.46), (-2.35, 1.20), (-2.35, -1.20), (-1.75, -1.46), (1.40, -1.40),
             (1.65, -0.58)]
    cyl("turret_ring_guard", 1.15, 0.08, (0, 0, base + 0.02), "Z", m["dark"], turret, seg=40, lods=MID)
    loft("turret_shell", [(z0, foot), (waist, wall), (z1, crown)], mat=m["paint"], parent=turret, bevel=0.04)
    VP.roof_fittings("roof", crown, z1, m, turret)
    turret_kit(v, turret, crown, (1.60, waist), (1.46, z1), -1.75, 1.00, waist + 0.05, z1 - 0.05)
    # The soft-kill launcher's rotating box on the roof's rear.
    cyl("softkill_base", 0.30, 0.04, (-1.40, 0, z1 + 0.02), "Z", m["dark"], turret, seg=20, lods=MID)
    box("softkill_box", (0.45, 0.70, 0.28), (-1.40, 0, z1 + 0.15), m["paint"], turret, bevel=0.03)
    for k in range(4):
        cyl(f"softkill_tube_{k}", 0.05, 0.02, (-1.17, -0.24 + k * 0.16, z1 + 0.15), "X", m["black"], turret, seg=10,
            lods=NEAR)
    # The gun's slot and the gunner's sight box on the right front.
    box("gun_slot", (0.30, 0.36, 0.40), (1.92, 0, base + 0.55), m["black"], turret, lods=MID)
    # The raised sensor brow across the front of the roof.
    prism("sensor_brow", [(1.60, z1 - 0.02), (1.10, z1 + 0.14), (0.10, z1 + 0.14), (0.10, z1 - 0.02)], 2.1,
          mat=m["paint"], parent=turret, bevel=0.03)
    VP.sight_housing("gunner_sight", (0.75, -1.05, z1 - 0.02), m, turret, size=(0.55, 0.40, 0.30))
    # The sensor housing's sloped roof plates and radar panels on the corners.
    for side, s in ((1, "L"), (-1, "R")):
        box(f"radar_panel_{s}", (0.06, 0.36, 0.28), (1.45, side * 1.45, waist + 0.20), m["dark"], turret, bevel=0.01,
            rot=(0, 0, side * 0.75))
        box(f"radar_panel_rear_{s}", (0.36, 0.06, 0.28), (-1.90, side * 1.58, waist + 0.20), m["dark"], turret,
            bevel=0.01)
        # Afganit: a rack of launcher tubes under each flank's overhang,
        # pointing out and up.
        for j in range(5):
            cyl(f"afganit_{s}_{j}", 0.06, 0.38, (-0.15 - j * 0.15, side * 1.40, z0 + 0.14), "Y", m["paint"], turret,
                seg=12, rot=(side * -0.35, 0, 0), lods=MID)
            cyl(f"afganit_bore_{s}_{j}", 0.045, 0.01, (-0.15 - j * 0.15, side * 1.59, z0 + 0.21), "Y", m["black"],
                turret, seg=10, rot=(side * -0.35, 0, 0), lods=FINE)
        VP.smoke_discharger_bank(f"smoke_{s}", (1.00, side * 1.30, z1 - 0.10), m, turret, count=4, tube_radius=0.045,
                                 tube_length=0.20, elevation=0.35, spread=0.4, rot=(0, 0, side * 0.9))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-2.10, side * 0.95, z1), m, whip, height=2.0)
    # The commander's panoramic sight on its mast over the left, standing
    # over the frame's top: dressing.
    sight = empty("dressing_panorama", parent=turret)
    cyl("panorama_mast", 0.11, 0.16, (-0.30, 0.60, z1 + 0.08), "Z", m["dark"], sight, seg=12)
    VP.sight_housing("panorama_head", (-0.30, 0.60, z1 + 0.14), m, sight, size=(0.52, 0.46, 0.40))
    # The bustle cage of slats round the rear.
    VP.slat_armour("bustle_cage", (-2.65, 0, base + 0.10), (2.40, 0.60), m, turret, rot=(0, 0, math.pi / 2))
    VP.tarp_roll("bustle_tarp", (-2.45, 0, base + 0.45), 2.0, 0.14, m, turret, straps=3)
    for side in (-1, 1):
        VP.stowage_box(f"bustle_box_{side}", (-2.25, side * 0.75, base + 0.10), (0.30, 0.55, 0.40), m, turret)
    for side, s in ((1, "L"), (-1, "R")):
        VP.slat_armour(f"bustle_cage_{s}", (-2.20, side * 1.25, base + 0.10), (0.75, 0.60), m, turret)


def gun_2a82(v, gun):
    """The 125 mm 2A82-1M: no fume extractor, a thermal sleeve in sections
    with clamps, the muzzle reference sensor near the end."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0] - 1.05
    box("mantlet", (0.70, 0.46, 0.42), (0.30, 0, 0), m["paint"], gun, bevel=0.06)
    cyl("barrel_root", 0.13, 0.60, (0.85, 0, 0), "X", m["paint"], gun, seg=24, bevel=0.015)
    cyl("thermal_sleeve", 0.095, reach - 1.0, ((reach + 0.8) / 2, 0, 0), "X", m["paint"], gun, seg=24)
    for k in range(5):
        cyl(f"sleeve_clamp_{k}", 0.105, 0.05, (1.1 + k * (reach - 1.6) / 4, 0, 0), "X", m["dark"], gun, seg=20,
            lods=NEAR)
    box("muzzle_sensor", (0.12, 0.08, 0.10), (reach - 0.35, 0, 0.13), m["dark"], gun, lods=NEAR)
    cyl("muzzle_end", 0.085, 0.20, (reach - 0.10, 0, 0), "X", m["steel"], gun, seg=24)
    cyl("muzzle_bore", 0.063, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=20, lods=MID)


def kord_station(v, hmg, hmg_gun):
    """The roof's remote Kord station: its bearing, body, sight and gun."""
    m = v.mats
    lift = v.frame["mounts"][1]["muzzle_m"][2]
    cyl("rws_bearing", 0.22, 0.08, (0, 0, -0.04), "Z", m["dark"], hmg, seg=20, bevel=0.01)
    box("rws_body", (0.46, 0.36, lift), (-0.05, 0, lift / 2), m["paint"], hmg, bevel=0.03)
    VP.sight_housing("rws_sight", (0.05, -0.26, -0.10), m, hmg_gun, size=(0.26, 0.14, 0.20))
    VP.kord(hmg_gun, v.frame["mounts"][1]["muzzle_m"][0], m)


run("t14", "russian_green", build, wreck, chip=1.0)
