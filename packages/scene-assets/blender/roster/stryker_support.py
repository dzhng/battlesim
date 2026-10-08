"""Stryker M-SHORAD and M1129 Mortar Carrier (disabled cards), from
assets/references/stryker_m_shorad/ and m1129_stryker_mortar/, on the
Stryker hull and running gear `roster/stryker.py` builds.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/stryker_support.py -- [--variant=<card id>] [--wreck]

What the photos settle:
- M-SHORAD (Inc 1, Grafenwöhr and Oberdachstetten): the Stryker hull with
  the Reconfigurable Integrated-weapons Platform turret on the roof: the
  30 mm XM914 in the middle, a four-round Stinger pod on its left, two
  Hellfire rails on its right, an M240 coax, the sensor head over the gun;
  radar panels on the hull's corners; no squad hatches.
- M1129 (Germany, 2017): the Stryker hull whose rear roof is the mortar bay,
  two long split doors folding out to the sides, the 120 mm tube inside;
  no remote weapon station, a ring mount for the M2 on the commander's
  hatch.

The frames are the Stryker's
published dimensions (6.95 x 2.72 m; 3.2 m to the M-SHORAD turret's top,
2.5 m for the mortar carrier, whose roof carries no remote station),
recorded as `references`. The turret and
the mortar are drawn at rest.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import stryker as S  # noqa: E402
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

SHORAD = "us_stryker_m_shorad_gun_and_missile_air_defense_vehicle"
MORTAR = "us_m1129_stryker_mortar_120_mm_mortar_carrier"


def stryker_hull(v):
    from wreckage import remove
    m, h = v.mats, v.hull
    v.roof = S.ROOF
    loft("stryker_hull", S.hull_rings(S.ROOF), mat=m["paint"], parent=h, bevel=0.035)
    S.wheels(v)
    S.fittings(v, S.ROOF, dragoon=False)
    # Neither carries the squad's roof hatches or the rear tarp.
    remove("squad_hatch_", "roof_tarp")


def rwip_turret(v):
    """The RIwP turret at the roof's middle: its base, the XM914, the
    Stinger pod, the Hellfire rails, the sensor head."""
    m, h = v.mats, v.hull
    x, y, z = -0.40, 0.0, S.ROOF
    cyl("rwip_base", 0.55, 0.20, (x, y, z + 0.10), "Z", m["paint"], h, seg=28, bevel=0.015)
    cyl("rwip_bearing", 0.50, 0.06, (x, y, z + 0.23), "Z", m["dark"], h, seg=28, lods=MID)
    box("rwip_body", (0.90, 0.80, 0.42), (x, y, z + 0.47), m["paint"], h, bevel=0.04)
    box("xm914_receiver", (0.70, 0.22, 0.24), (x + 0.45, y, z + 0.58), m["dark"], h, bevel=0.02)
    cyl("xm914_barrel", 0.035, 1.35, (x + 1.45, y, z + 0.60), "X", m["steel"], h, seg=12)
    cyl("xm914_muzzle", 0.050, 0.14, (x + 2.10, y, z + 0.60), "X", m["dark"], h, seg=12, lods=NEAR)
    cyl("m240_barrel", 0.016, 0.50, (x + 0.85, y - 0.18, z + 0.50), "X", m["dark"], h, seg=8, lods=NEAR)
    VP.sight_housing("rwip_sensor", (x + 0.10, y - 0.10, z + 0.68), m, h, size=(0.36, 0.30, 0.28))
    # The Stinger pod on the left, the Hellfire rails on the right.
    box("stinger_arm", (0.20, 0.30, 0.20), (x, y + 0.55, z + 0.55), m["dark"], h, lods=MID)
    box("stinger_pod", (1.40, 0.42, 0.42), (x + 0.30, y + 0.85, z + 0.62), m["paint"], h, bevel=0.03)
    for i in range(2):
        for j in range(2):
            cyl(f"stinger_cap_{i}_{j}", 0.07, 0.01, (x + 1.01, y + 0.75 + i * 0.20, z + 0.52 + j * 0.20), "X",
                m["dark"], h, seg=12, lods=MID)
    box("hellfire_arm", (0.20, 0.30, 0.20), (x, y - 0.55, z + 0.55), m["dark"], h, lods=MID)
    for k, zz in enumerate((0.50, 0.78)):
        box(f"hellfire_rail_{k}", (1.60, 0.10, 0.06), (x + 0.30, y - 0.82, z + zz), m["dark"], h, lods=MID)
        cyl(f"hellfire_{k}", 0.09, 1.62, (x + 0.35, y - 0.82, z + zz + 0.11), "X", m["paint"], h, seg=14,
            bevel=0.01)
        cyl(f"hellfire_nose_{k}", 0.09, 0.14, (x + 1.23, y - 0.82, z + zz + 0.11), "X", m["dark"], h, seg=14,
            r2=0.03, lods=MID)
    # The turret's own fittings, as the M-SHORAD photos show them: the
    # XM914's ammunition can and feed chute, the gun's cradle cheeks, the
    # sensor's second (thermal) window, the Stinger pod's ribs and hinged
    # rear cover, the Hellfire rails' launcher shoes, smoke banks on the
    # body's rear corners, a cable harness off the base.
    box("xm914_ammo", (0.40, 0.22, 0.30), (x - 0.10, y + 0.30, z + 0.50), m["paint"], h, bevel=0.02, lods=MID)
    VP.cable("xm914_feed", [(x - 0.10, y + 0.30, z + 0.67), (x + 0.10, y + 0.20, z + 0.78), (x + 0.30, y + 0.10,
                                                                                          z + 0.70)], m, h,
             radius=0.03, eyes=False)
    for side in (-1, 1):
        box(f"xm914_cheek_{side}", (0.40, 0.04, 0.30), (x + 0.40, y + side * 0.14, z + 0.56), m["paint"], h,
            bevel=0.01, lods=NEAR)
        VP.smoke_discharger_bank(f"rwip_smoke_{side}", (x - 0.40, y + side * 0.36, z + 0.68), m, h, count=3,
                                 tube_radius=0.04, tube_length=0.18, elevation=0.4, spread=0.3,
                                 rot=(0, 0, math.pi + side * 0.6))
    box("rwip_thermal_window", (0.012, 0.10, 0.10), (x + 0.285, y - 0.18, z + 0.80), m["glass"], h, lods=NEAR)
    for k in range(3):
        box(f"stinger_pod_rib_{k}", (0.05, 0.45, 0.45), (x - 0.25 + k * 0.50, y + 0.85, z + 0.62), m["paint"], h,
            lods=NEAR)
    box("stinger_rear_cover", (0.03, 0.40, 0.40), (x - 0.41, y + 0.85, z + 0.62), m["dark"], h, lods=MID)
    for k, zz in enumerate((0.50, 0.78)):
        for j, dx in enumerate((-0.30, 0.60)):
            box(f"hellfire_shoe_{k}_{j}", (0.12, 0.08, 0.06), (x + dx, y - 0.82, z + zz + 0.05), m["steel"], h,
                lods=FINE)
    VP.cable("rwip_harness", [(x - 0.50, y + 0.10, z + 0.12), (x - 0.75, y + 0.15, z + 0.05),
                              (x - 1.10, y + 0.20, z + 0.03)], m, h, radius=0.022, eyes=False)
    # Radar panels on the hull's four corners.
    for k, (px, py, yaw) in enumerate(((0.95, 1.05, 0.6), (0.95, -1.05, -0.6), (-3.25, 1.05, 2.5),
                                       (-3.25, -1.05, -2.5))):
        box(f"radar_panel_{k}", (0.06, 0.45, 0.40), (px, py, S.ROOF + 0.20), m["dark"], h, bevel=0.01,
            rot=(0, 0, yaw))


def mortar_bay(v):
    """The M1129's mortar bay: the long split roof doors over the rear, the
    120 mm's muzzle just under them, and the M2 ring on the commander's
    hatch instead of the remote station."""
    m, h = v.mats, v.hull
    edge = S.side_face(S.ROOF)[1][0]
    for k, side in enumerate((1, -1)):
        VP.bolted_panel(f"bay_door_{k}", (-1.85, side * edge / 2, S.ROOF), (2.6, edge - 0.06, 0.06), m, h,
                        bolts=(4, 2), bevel=0.02, lods=VP.ALL)
        cyl(f"bay_hinge_{k}", 0.035, 2.5, (-1.85, side * (edge - 0.02), S.ROOF + 0.04), "X", m["steel"], h, seg=10,
            lods=NEAR)
    box("bay_seam", (2.6, 0.04, 0.02), (-1.85, 0, S.ROOF + 0.07), m["black"], h, lods=NEAR)
    # The bay doors' grab handles and stops, the mortar's baseplate ring and
    # bipod under the open seam, and the ammunition racks' lids beside it.
    for k, side in enumerate((1, -1)):
        for j, dx in enumerate((-0.80, 0.80)):
            VP.tow_hook(f"bay_handle_{k}_{j}", (-1.85 + dx, side * edge / 2, S.ROOF + 0.06), m, h, size=0.07,
                        rot=(0, -math.pi / 2, 0))
        box(f"bay_stop_{k}", (0.10, 0.06, 0.10), (-3.10, side * (edge - 0.10), S.ROOF + 0.05), m["dark"], h, lods=NEAR)
    cyl("mortar_ring", 0.16, 0.05, (-1.12, 0, S.ROOF - 0.02), "Z", m["dark"], h, seg=16, lods=NEAR)
    for side in (-1, 1):
        cyl(f"mortar_bipod_{side}", 0.02, 0.40, (-0.92, side * 0.10, S.ROOF - 0.10), "Z", m["dark"], h, seg=8,
            rot=(side * 0.4, 0.3, 0), lods=FINE)
    cyl("mortar_muzzle", 0.09, 0.30, (-1.0, 0, S.ROOF - 0.10), "Z", m["dark"], h, seg=16, rot=(0, -0.45, 0),
        lods=MID)
    ring = empty("dressing_commander_mg", parent=h)
    cyl("mg_ring", 0.42, 0.06, (0.25, S.COMMANDER_Y, S.ROOF + 0.08), "Z", m["dark"], ring, seg=24, lods=MID)
    gun = empty("cupola_m2", loc=(0.45, S.COMMANDER_Y, S.ROOF + 0.45), parent=ring)
    VP.browning_m2(gun, 1.30, m)
    cyl("mg_post", 0.04, 0.35, (0.45, S.COMMANDER_Y, S.ROOF + 0.25), "Z", m["steel"], ring, seg=10, lods=MID)


def build(variant, v):
    stryker_hull(v)
    (rwip_turret if variant["id"] == SHORAD else mortar_bay)(v)


def wreck(variant, v):
    """Stryker's own wreck with this mission load: the front left wheels
    gone and the hull down on that corner, bins torn, the ramp door out;
    the M-SHORAD's missiles gone from its rails and pod, the M1129's bay
    doors blown open."""
    from wreckage import bend, parts, remove
    remove("hellfire_", "stinger_cap_")
    edge = S.side_face(S.ROOF)[1][0]
    for k, side in enumerate((1, -1)):
        if parts(f"bay_door_{k}"):
            bend(parts(f"bay_door_{k}"), (-1.85, side * edge, S.ROOF), (1, 0, 0), (0, 0, 1), side * 1.9)
    S.wreck(variant, v)


if __name__ == "__main__":
    run_disabled("stryker_support", {SHORAD: (6.95, 2.72, 3.20), MORTAR: (6.95, 2.72, 2.50)}, "us_desert_tan", build,
                 wreck, chip=0.6)
