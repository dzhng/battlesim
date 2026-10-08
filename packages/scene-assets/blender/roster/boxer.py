"""Boxer APC and RCT30 (Schakal), from assets/references/boxer/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/boxer.py -- [--variant=<id>] [--wreck]

What the photos settle: eight big black road-tread tyres on four evenly
spaced axles, coil springs behind them; the narrow lower hull, and over it a
full-width upper hull on a fender shelf just above the tyres, its side plates
dropping in a V between each pair of wheels; a short upright nose with its
lights and tow shackles, then the long glacis tiled with bolted applique up
to the flat roof; the driver's raised hood with vision blocks at the top of
the glacis on the right; mirrors on arms at the nose corners; bolted plates
along the mission module, its seam behind the drive module; the rear ramp
with its door. The APC carries the FLW 200 remote station with its heavy
machine gun and two roof hatches behind it; the RCT30 the low unmanned
turret with its 30 mm, the Spike launcher on its left cheek and the
commander's panoramic sight on a mast.

Built to the catalog frames (hull 7.93 x 2.99 m; 2.38 m for the APC, 3.63 m
for the RCT30 with its turret): nothing here moves them. `drive` is the one
Boxer hull: the Skyranger 30 (`air_defence.py`) stands its turret on it, so
this script exports only when run as one.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

WHEEL_X = [2.42, 0.82, -0.78, -2.38]
WHEEL_R = 0.62
WHEEL_W = 0.38
WHEEL_Y = 1.28
SHELF = 1.30
ROOF = 2.30  # under the hatches, which reach the frame's 2.38 m top
NOSE = 3.965
GLACIS_TOP = 1.55


def glacis_z(x):
    return SHELF + 0.08 + (ROOF - SHELF - 0.08) * (NOSE - 0.08 - x) / (NOSE - 0.08 - GLACIS_TOP)


def drive(v):
    """The Boxer up to its roof (`ROOF`): the narrow lower hull between the
    wheels, the nose plate, the full-width upper hull on its fender shelf,
    the running gear and the hull's fittings; no roof hatches, which differ
    by mission module."""
    m, hull = v.mats, v.hull
    loft("boxer_lower", [(0.55, VP.hull_plan(-3.60, 3.30, 0.92, 0.25)),
                         (SHELF - 0.02, VP.hull_plan(-3.90, NOSE, 0.95, 0.30))], mat=m["paint"], parent=hull, bevel=0.04)
    loft("boxer_upper", [(SHELF - 0.02, VP.hull_plan(-3.94, NOSE, 1.48, 0.45)),
                         (SHELF + 0.08, VP.hull_plan(-3.94, NOSE - 0.04, 1.48, 0.45)),
                         (ROOF, VP.hull_plan(-3.90, GLACIS_TOP, 1.42, 0.35))], mat=m["paint"], parent=hull, bevel=0.05)
    wheels(v)
    fittings(v)


def build(variant, v):
    drive(v)
    mounts = rig(v.frame, v.root)
    if "rct30" in variant["id"]:
        turret, gun, _, _ = mounts["autocannon"]
        rct30(v, turret, gun)
    else:
        station, pitch, _, pivot = mounts["HMG"]
        flw200(v, station, pitch, pivot)


def wheels(v):
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(WHEEL_X):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, m, hull,
                          rim_radius=0.34, ctis=True, hub_bolts=10)
            box(f"suspension_arm_{s}_{k}", (0.20, 0.40, 0.12), (x, side * (WHEEL_Y - 0.36), WHEEL_R), m["dark"],
                hull, lods=NEAR)
            cyl(f"spring_{s}_{k}", 0.09, 0.40, (x - 0.32, side * 1.02, WHEEL_R + 0.25), "Z", m["dark"], hull, seg=10,
                lods=FINE)
        # The side plate drops in a V between each pair of wheels and ahead
        # of and behind the run.
        gaps = [(WHEEL_X[k] + WHEEL_X[k + 1]) / 2 for k in range(3)]
        for k, x in enumerate(gaps):
            prism(f"skirt_{s}_{k}", [(x - 0.42, SHELF), (x + 0.42, SHELF), (x + 0.10, SHELF - 0.42),
                                     (x - 0.10, SHELF - 0.42)], 0.06, loc=(0, side * 1.44, 0), mat=m["paint"],
                  parent=hull, bevel=0.015, lods=MID)
        VP.mudflap(f"mudflap_{s}", (-3.25, side * WHEEL_Y, SHELF - 0.02), (0.40, 0.55), m, hull)


def fittings(v):
    m, hull = v.mats, v.hull
    slope = math.atan((ROOF - SHELF - 0.08) / (NOSE - 0.08 - GLACIS_TOP))
    # The glacis tiled with bolted applique, the driver's raised hood on the
    # right at its top.
    for i in range(4):
        for j in range(3):
            x = 3.38 - i * 0.56
            if i == 3 and j == 0:
                continue  # the driver's hood stands there
            y = (j - 1) * 0.84
            VP.bolted_panel(f"glacis_tile_{i}_{j}", (x, y, glacis_z(x)), (0.50, 0.78, 0.035), m, hull, bolts=(2, 3),
                            rot=(0, slope, 0), bevel=0.012)
    hx, hy = 1.85, -0.84
    box("driver_hood", (0.62, 0.70, 0.30), (hx, hy, glacis_z(hx) + 0.10), m["paint"], hull, rot=(0, slope * 0.4, 0),
        bevel=0.04)
    for k, y in enumerate((-0.22, 0.0, 0.22)):
        VP.periscope(f"driver_vision_{k}", (hx + 0.30, hy + y, glacis_z(hx) + 0.16), m, hull, size=(0.10, 0.18, 0.10),
                     rot=(0, 0, 0))
    # The nose: lights in their recesses, tow shackles, a grille slot.
    for side, s in ((1, "L"), (-1, "R")):
        box(f"light_recess_{s}", (0.04, 0.36, 0.12), (NOSE - 0.01, side * 1.05, SHELF + 0.02), m["black"], hull,
            lods=MID)
        for k, y in enumerate((0.95, 1.13)):
            cyl(f"headlight_{s}_{k}", 0.05, 0.03, (NOSE, side * y, SHELF + 0.02), "X", m["lamp"], hull, seg=12,
                lods=MID)
        VP.shackle(f"nose_shackle_{s}", (NOSE + 0.04, side * 0.60, SHELF - 0.08), m, hull, size=0.13,
                   rot=(0, 0, math.pi / 2))
        mirror = empty(f"dressing_mirror_{s}", parent=hull)
        box(f"mirror_arm_{s}", (0.04, 0.04, 0.42), (NOSE - 0.40, side * 1.47, ROOF - 0.55), m["dark"], mirror,
            lods=NEAR)
        box(f"mirror_{s}", (0.05, 0.16, 0.30), (NOSE - 0.40, side * 1.47, ROOF - 0.20), m["dark"], mirror,
            bevel=0.012, lods=MID)
        VP.light_with_guard(f"tail_light_{s}", (-3.95, side * 1.20, SHELF + 0.30), 0.05, dict(m, lamp=m["tail"]),
                            hull, rot=(0, 0, math.pi))
        VP.tow_hook(f"rear_tow_{s}", (-3.92, side * 0.65, SHELF - 0.25), m, hull, size=0.10, rot=(0, 0, math.pi))
        # Bolted plates along the mission module and its seam to the drive module.
        face = side * 1.47
        for k in range(4):
            x = 0.55 - k * 1.05
            VP.bolted_panel(f"side_plate_{s}_{k}", (x, face, 1.86), (0.98, 0.86, 0.03), m, hull, bolts=(4, 3),
                            rot=(-side * math.pi / 2, 0, 0), bevel=0.012)
        VP.weld_line(f"module_seam_{s}", [(1.12, face, SHELF + 0.10), (1.12, face, ROOF - 0.05)], m, hull,
                     radius=0.015)
        for k, z in enumerate((1.55, 1.85, 2.15)):
            cyl(f"seam_bolt_{s}_{k}", 0.03, 0.04, (1.12, face + side * 0.02, z), "Y", m["steel"], hull, seg=6,
                lods=FINE)
        # Smoke dischargers on the upper hull's front cheeks.
        VP.smoke_discharger_bank(f"smoke_{s}", (GLACIS_TOP + 0.10, side * 1.30, ROOF - 0.32), m, hull, count=4,
                                 tube_radius=0.045, tube_length=0.22, elevation=0.4, spread=0.4,
                                 rot=(0, 0, side * 1.1))
    # Exhausts on the right of the drive module.
    VP.grille("engine_grille", (2.40, 0.50, glacis_z(2.40) + 0.04), (0.60, 0.50), m, hull, slats=6,
              rot=(0, slope, 0))
    VP.exhaust("exhaust", (1.40, -1.50, SHELF + 0.20), 0.08, 0.40, m, hull, rot=(0, 0, -math.pi / 2))
    # The rear ramp with its door, tail stowage.
    box("rear_ramp", (0.06, 1.70, ROOF - 0.70), (-3.93, 0, (ROOF + 0.70) / 2), m["paint"], hull, bevel=0.015)
    box("ramp_door", (0.03, 0.62, 1.05), (-3.97, -0.40, SHELF + 0.15), m["paint"], hull, bevel=0.012, lods=NEAR)
    box("ramp_door_glass", (0.02, 0.16, 0.08), (-3.985, -0.40, SHELF + 0.50), m["glass"], hull, lods=NEAR)
    cyl("ramp_hinge", 0.045, 1.60, (-3.95, 0, 0.72), "Y", m["steel"], hull, seg=10, lods=NEAR)
    cans = empty("dressing_rear_jerrycans", parent=hull)
    for side in (-1, 1):
        VP.jerrycan(f"rear_jerrycan_{side}", (-4.02, side * 1.15, SHELF + 0.10), dict(m, paint=m["dark"]), cans,
                    size=(0.165, 0.345, 0.47))
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (-3.55, side * 1.20, ROOF), m, whip, height=2.8)
    tarp = empty("dressing_roof_stowage", parent=hull)
    VP.tarp_roll("roof_tarp", (-3.45, 0, ROOF + 0.14), 1.80, 0.14, m, tarp, straps=3)
    VP.stowage_box("roof_box", (-2.95, 0.70, ROOF), (0.60, 0.45, 0.25), dict(m, paint=m["dark"]), tarp)


def flw200(v, station, pitch, pivot):
    """The APC's FLW 200 remote station on its roof plate, its heavy machine
    gun in the cradle, the sight head on its left and the ammunition box on
    its right; the two roof hatches behind it."""
    m = v.mats
    hull = v.hull
    for k, (x, y) in enumerate(((-1.55, 0.62), (-1.55, -0.62))):
        VP.hatch(f"roof_hatch_{k}", (x, y, ROOF), m, hull, size=(0.85, 0.66))
    VP.hatch("commander_hatch", (0.75, 0.60, ROOF), m, hull, radius=0.30)
    for k in range(4):
        a = -0.6 + k * 0.4
        VP.periscope(f"commander_vision_{k}", (0.75 + 0.36 * math.cos(a), 0.60 + 0.36 * math.sin(a), ROOF), m, hull,
                     size=(0.10, 0.13, 0.07), rot=(0, 0, a))
    base = ROOF - pivot.z
    cyl("station_ring", 0.42, 0.06, (0, 0, base + 0.03), "Z", m["dark"], station, seg=24, lods=MID)
    box("station_base", (0.70, 0.62, 0.18), (-0.05, 0, base + 0.15), m["paint"], station, bevel=0.03)
    for side in (-1, 1):
        box(f"station_cradle_{side}", (0.40, 0.06, 0.36), (0.02, side * 0.20, 0.0), m["paint"], station,
            bevel=0.015)
    VP.sight_housing("station_sight", (0.10, 0.36, -0.12), m, pitch, size=(0.34, 0.22, 0.26))
    box("station_ammo", (0.36, 0.18, 0.28), (-0.05, -0.32, -0.08), m["paint"], pitch, bevel=0.015, lods=MID)
    VP.browning_m2(pitch, v.frame["mounts"][0]["muzzle_m"][0], m)


def rct30(v, turret, gun):
    """The RCT30: a low, wide unmanned turret with chamfered cheeks, the
    30 mm in its cradle on top, the Spike launcher on its left cheek, smoke
    banks either side and the commander's panoramic sight on its mast."""
    m = v.mats
    hull = v.hull
    for k, (x, y) in enumerate(((-1.75, 0.62), (-1.75, -0.62))):
        VP.hatch(f"roof_hatch_{k}", (x, y, ROOF), m, hull, size=(0.85, 0.66))
    base = ROOF - v.frame["mounts"][0]["pivot_m"][2]
    cyl("turret_ring", 1.00, base * -1 + 0.06, (0, 0, base / 2 + 0.03), "Z", m["dark"], turret, seg=32, lods=MID)
    foot = [(1.05, 0.40), (0.75, 0.95), (-0.95, 0.95), (-1.15, 0.70), (-1.15, -0.70), (-0.95, -0.95), (0.75, -0.95),
            (1.05, -0.40)]
    crown = [(x * 0.9 - 0.05, y * 0.88) for x, y in foot]
    loft("turret_shell", [(0.05, foot), (0.40, foot), (0.55, crown)], mat=m["paint"], parent=turret, bevel=0.045)
    for i in range(3):
        for side in (-1, 1):
            VP.bolted_panel(f"turret_tile_{side}_{i}", (0.45 - i * 0.55, side * 0.95, 0.25), (0.50, 0.32, 0.03), m,
                            turret, bolts=(2, 2), rot=(-side * math.pi / 2, 0, 0), bevel=0.01)
    # The gun's cradle on top, its shroud running forward.
    box("gun_cradle", (1.10, 0.46, 0.30), (0.20, 0.0, 0.62), m["paint"], turret, bevel=0.035)
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("gun_shroud", (0.70, 0.30, 0.24), (1.05, 0, 0.0), m["paint"], gun, bevel=0.03)
    cyl("gun_sleeve", 0.07, 0.40, (1.55, 0, 0), "X", m["dark"], gun, seg=16)
    cyl("gun_barrel", 0.042, reach - 1.75, ((reach + 1.75) / 2, 0, 0), "X", m["steel"], gun, seg=14)
    cyl("gun_muzzle", 0.07, 0.22, (reach - 0.11, 0, 0), "X", m["dark"], gun, seg=14)
    cyl("gun_bore", 0.03, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=NEAR)
    # Spike launcher on the left cheek.
    box("spike_arm", (0.30, 0.20, 0.30), (0.0, 1.02, 0.45), m["dark"], turret, bevel=0.02, lods=MID)
    box("spike_box", (1.05, 0.34, 0.36), (0.15, 1.18, 0.62), m["paint"], turret, bevel=0.03)
    for k, z in enumerate((0.55, 0.70)):
        cyl(f"spike_mouth_{k}", 0.06, 0.02, (0.68, 1.18, z), "X", m["black"], turret, seg=12, lods=MID)
    # The gunner's sight beside the gun, the commander's panoramic sight on its mast.
    VP.sight_housing("gunner_sight", (0.55, -0.55, 0.55), m, turret, size=(0.38, 0.30, 0.28))
    cyl("peri_mast", 0.10, 0.45, (-0.45, -0.40, 0.75), "Z", m["dark"], turret, seg=12)
    VP.sight_housing("peri_head", (-0.45, -0.40, 0.97), m, turret, size=(0.34, 0.34, 0.24))
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"turret_smoke_{s}", (0.70, side * 0.90, 0.40), m, turret, count=4,
                                 tube_radius=0.045, tube_length=0.22, elevation=0.4, spread=0.3,
                                 rot=(0, 0, side * 0.8))
    box("bustle_rack", (0.35, 1.60, 0.30), (-1.30, 0, 0.30), m["steel"], turret, lods=MID)
    VP.tarp_roll("bustle_roll", (-1.30, 0, 0.36), 1.40, 0.14, m, turret, straps=2)


def wreck(variant, v):
    """The Boxer after its fire: the front left wheel and the one behind it
    blown off and the hull down on that corner, a skirt plate torn away, the
    ramp's door blown out and thrown onto the roof, two glacis tiles gone, roof
    stowage burnt off, plates warped and a dent in the side where it was hit,
    torn plate on the ground. The RCT30 throws its turret."""
    from parts import rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_L_1_", "wheel_L_2_", "skirt_L_0", "ramp_door", "glacis_tile_1_2", "glacis_tile_2_0",
           "roof_hatch_", "rear_jerrycan_")
    plate("door_lying", [(-0.52, -0.31), (0.53, -0.32), (0.52, 0.31), (-0.53, 0.30)], 0.05, (-2.6, -0.45, ROOF + 0.06),
          (0.02, 0.03, 0.15), m["paint"], v.root, seed=91)
    bend(parts("mirror_"), (NOSE - 0.40, 1.47, ROOF - 0.70), (1, 0, 0), (0, 0, 1), -0.8)
    shell = parts("boxer_upper", "boxer_lower", "side_plate_")
    densify(shell, scale=2.0)
    warp(shell, heat(0.02, 0.8, seed=14.0), dent((0.0, 1.48, 1.80), 0.55, 0.13, (0, -1, -0.1)))
    for k, (loc, rot, size) in enumerate((((1.62, 1.45, 0.03), (0.03, 0.04, 0.9), 0.22),
                                          ((0.02, -1.62, 0.03), (-0.04, 0.02, 2.2), 0.22),
                                          ((-1.5, 0.3, ROOF + 0.04), (0.02, 0.04, 0.3), 0.36))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.12,
              seed=101 + k)
    v.root.rotation_euler = (0.0, 0.03, 0)
    v.root.location.z -= 0.07
    rest_on_ground(0.004)


if __name__ == "__main__":
    run("boxer", "german_three_tone", build, wreck, chip=0.6)
