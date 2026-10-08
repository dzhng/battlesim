"""ZTQ-15 (Type 15) light tank (disabled card), from assets/references/type15/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/type15.py -- [--wreck]

What the photos settle (display and VT-5 export photos): six road wheels a
side on hydropneumatic arms, the sprocket at the rear and the idler at the
front, no return rollers showing under the skirts; a short sharp glacis to a
flat deck, the driver's hatch on the left of it; side skirts in sections. The
welded turret is boxed in modular armour, a wedge of blocks on each cheek
and a deep bustle box behind, a slatted rack round its rear; the 105 mm gun
has a thermal sleeve and a fume extractor; the commander's panoramic sight
on the right rear roof, the remote heavy machine gun station beside it, four
smoke tubes a side. Its paint is the PLA's digital desert pattern.

Frame 7.3 x 3.35 x 2.5 m, gun pivot 1.68 m (`DIMENSIONS`, `MOUNTS`).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism  # noqa: E402
from vehicle_export import rig, run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

CARD = "eastern_type_15_ztq_15_light_tank"
# The frame its references give (a disabled card has no unit type): the
# hull box, and the mounts its turret and guns are rigged on for the art.
DIMENSIONS = (7.3, 3.35, 2.5)
MOUNTS = [
    dict(name="cannon", role="gun", on=None, pivot_m=[0.0, 0.0, 1.68], muzzle_m=[5.36, 0.0, 0.504]),
    dict(name="HMG", role="hmg", on="cannon", pivot_m=[-0.25, -0.58, 2.632], muzzle_m=[1.43, 0.0, 0.32]),
]
TRACK_Y = 1.30
TRACK_W = 0.48
ROAD_R = 0.33
ROAD_Z = 0.40
DECK = 1.56
TRUNNION = 0.85


def hull(v):
    m, h = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 0.86), (half - 0.30, 0.86), (half, 1.06), (half - 1.10, DECK), (-half + 0.08, DECK),
                         (-half, DECK - 0.10)], 3.05, mat=m["paint"], parent=h, bevel=0.05)
    prism("hull_lower", [(-half + 0.35, 0.40), (half - 0.70, 0.40), (half - 0.05, 0.88), (-half, 0.88),
                         (-half, 0.70)], 2.10, mat=m["paint"], parent=h, bevel=0.04)
    slope = math.atan((DECK - 1.06) / 1.10)
    VP.armour_tiles("glacis_armour", (half - 0.55, 0, 1.06 + (DECK - 1.06) * 0.5), (0.95, 2.60), (2, 5), 0.06, m, h,
                    rot=(0, slope, 0))
    VP.hatch("driver_hatch", (half - 1.45, 0.55, DECK), m, h, radius=0.27)
    for k in range(3):
        VP.periscope(f"driver_periscope_{k}", (half - 1.12, 0.35 + k * 0.20, DECK), m, h, size=(0.11, 0.16, 0.08))
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{s}", (half - 0.95, side * 1.30, DECK + 0.08), 0.06, m, h)
        VP.tow_hook(f"front_tow_{s}", (half - 0.02, side * 0.65, 0.82), m, h, size=0.12)
        VP.tow_hook(f"rear_tow_{s}", (-half + 0.02, side * 0.75, 0.80), m, h, size=0.12, rot=(0, 0, math.pi))
        for k in range(6):
            length = (v.length - 0.6) / 6
            x = half - 0.35 - length * (k + 0.5)
            VP.bolted_panel(f"skirt_{s}_{k}", (x, side * 1.58, 0.84), (length - 0.03, 0.08, 0.62), m, h, bolts=(2, 1),
                            bevel=0.02, lods=VP.ALL)
        VP.stowage_box(f"fender_box_{s}", (-1.6, side * 1.40, DECK), (0.90, 0.32, 0.30), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        VP.cable(f"tow_cable_{s}", [(-half + 0.3, side * 1.42, DECK + 0.02), (-0.6, side * 1.45, DECK + 0.02),
                                    (0.8, side * 1.42, DECK + 0.02)], m, h, radius=0.02)
    VP.grille("engine_grille", (-2.65, 0, DECK), (1.10, 1.90), m, h, slats=10)
    VP.exhaust("exhaust", (-half - 0.02, -1.05, 1.25), 0.09, 0.25, m, h, rot=(0, 0, math.pi))
    # The rear: its tail lights in their guards.
    for side, sd in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"tail_light_{sd}", (-v.length / 2 - 0.02, side * (v.width / 2 - 0.5), DECK - 0.20), 0.05,
                            dict(v.mats, lamp=v.mats["tail"]), v.hull, rot=(0, 0, math.pi))

def running_gear(v):
    road_x = [2.25 - 0.86 * k for k in range(6)]
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, road_x, ROAD_Z, ROAD_R, 0.20, (-3.20, 0.58, 0.28),
                            (3.15, 0.56, 0.26), [(1.0, 0.82, 0.08), (-1.0, 0.82, 0.08)], bolts=6, teeth=12,
                            arm=(0.40, -0.30), pitch=0.15, dual=True)


def turret(v, turret, gun, hmg, hmg_gun):
    m = v.mats
    base = DECK - v.frame["mounts"][0]["pivot_m"][2]
    top = base + 0.72
    foot = [(1.30, 0.45), (1.05, 1.15), (-1.30, 1.20), (-1.55, 0.90), (-1.55, -0.90), (-1.30, -1.20), (1.05, -1.15),
            (1.30, -0.45)]
    crown = [(1.00, 0.42), (0.80, 1.05), (-1.25, 1.10), (-1.45, 0.85), (-1.45, -0.85), (-1.25, -1.10), (0.80, -1.05),
             (1.00, -0.42)]
    cyl("turret_ring_guard", 0.98, 0.08, (0, 0, base + 0.02), "Z", m["dark"], turret, seg=36, lods=MID)
    loft("turret_shell", [(base + 0.04, foot), (top, crown)], mat=m["paint"], parent=turret, bevel=0.035)
    VP.roof_fittings("roof", crown, top, m, turret, periscopes=((-0.10, 0.80, 0.6),))
    # A wedge of armour blocks on each cheek, the bustle box behind.
    for side, s in ((1, "L"), (-1, "R")):
        for k in range(3):
            box(f"cheek_block_{s}_{k}", (0.70 - k * 0.12, 0.32, 0.40), (1.15 - k * 0.20, side * (0.55 + k * 0.30),
                                                                        base + 0.42), m["paint"], turret, bevel=0.04,
                rot=(0, 0.25, side * (0.35 + k * 0.10)))
        # Tiles on the turret's leaning flank: the shell's side at x -0.30,
        # from its foot ring to its crown ring.
        loc, rot = VP.on_side(-0.30, base + 0.38, side, (1.179, base + 0.04), (1.077, top))
        VP.armour_tiles(f"side_tiles_{s}", loc, (1.60, 0.50), (4, 1), 0.08, m, turret, rot=rot)
        VP.smoke_discharger_bank(f"smoke_{s}", (0.10, side * 1.10, top - 0.05), m, turret, count=4, tube_radius=0.045,
                                 tube_length=0.20, elevation=0.4, spread=0.4, rot=(0, 0, side * 1.1))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.35, side * 0.80, top), m, whip, height=2.0)
    VP.stowage_box("bustle_box", (-1.85, 0, base + 0.10), (0.50, 2.00, 0.52), m, turret)
    VP.slat_armour("bustle_rack", (-2.12, 0, base + 0.10), (1.90, 0.55), m, turret, rot=(0, 0, math.pi / 2))
    VP.sight_housing("gunner_sight", (0.45, 0.55, top - 0.02), m, turret, size=(0.42, 0.30, 0.28))
    mast = empty("dressing_commander_sight", parent=turret)
    cyl("panorama_post", 0.09, 0.22, (-0.55, -0.55, top + 0.11), "Z", m["dark"], mast, seg=12)
    VP.sight_housing("panorama_head", (-0.55, -0.55, top + 0.20), m, mast, size=(0.32, 0.30, 0.28))
    VP.hatch("loader_hatch", (-0.45, 0.50, top), m, turret, radius=0.26)
    # The 105 mm: mantlet, sleeve, fume extractor, muzzle.
    reach = v.frame["mounts"][0]["muzzle_m"][0] - TRUNNION
    box("mantlet", (0.55, 0.55, 0.42), (0.20, 0, 0), m["paint"], gun, bevel=0.05)
    cyl("barrel_root", 0.12, 0.50, (0.70, 0, 0), "X", m["paint"], gun, seg=24)
    evac = 2.2
    for k, (a, b) in enumerate([(0.95, evac - 0.30), (evac + 0.30, reach - 0.15)]):
        cyl(f"thermal_sleeve_{k}", 0.080, b - a, ((a + b) / 2, 0, 0), "X", m["paint"], gun, seg=22)
    cyl("fume_extractor", 0.125, 0.50, (evac, 0, 0), "X", m["paint"], gun, seg=24, bevel=0.02)
    cyl("muzzle_end", 0.080, 0.16, (reach - 0.08, 0, 0), "X", m["steel"], gun, seg=22)
    cyl("muzzle_bore", 0.055, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=18, lods=MID)
    # The remote heavy machine gun station.
    lift = v.frame["mounts"][1]["muzzle_m"][2]
    cyl("rws_bearing", 0.20, 0.08, (0, 0, -0.04), "Z", m["dark"], hmg, seg=20)
    box("rws_body", (0.42, 0.32, lift), (-0.05, 0, lift / 2), m["paint"], hmg, bevel=0.03)
    VP.sight_housing("rws_sight", (0.05, -0.24, -0.10), m, hmg_gun, size=(0.24, 0.14, 0.20))
    reach = v.frame["mounts"][1]["muzzle_m"][0]
    box("qjc88_receiver", (0.60, 0.11, 0.15), (0.02, 0, 0), m["dark"], hmg_gun, bevel=0.012)
    cyl("qjc88_barrel", 0.030, reach - 0.40, ((reach + 0.30) / 2, 0, 0), "X", m["dark"], hmg_gun, seg=10)
    cyl("qjc88_brake", 0.042, 0.10, (reach - 0.05, 0, 0), "X", m["steel"], hmg_gun, seg=10, lods=NEAR)


def build(variant, v):
    hull(v)
    running_gear(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    t, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret(v, t, gun, hmg, hmg_gun)


def wreck(variant, v):
    """The Type 15 after its fire: the right track off with two road
    wheels gone, skirt sections blown away, the bustle box burst open and its
    rack bent, plates warped; the turret is thrown (`wreckage.burn`)."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, remove, warp
    m = v.mats
    remove("track_R_band", "wheel_R_2_", "wheel_R_3_", "skirt_R_1_", "skirt_R_2_", "fender_box_R", "loader_hatch",
           "bustle_box_lid")
    thrown = solid("thrown_track", (3.4, TRACK_W, 0.05), (0.2, -2.05, 0.03), m["track"], v.hull, rot=(0, 0, 0.08))
    densify(thrown)
    warp(thrown, heat(0.03, 0.6, seed=2.0))
    bend(parts("bustle_rack"), (-2.12, 0, 1.0), (0, 1, 0), (1, 0, 0), 0.4)
    shell = parts("hull_upper", "hull_lower", "skirt_", "turret_shell", "bustle_box")
    densify(shell, scale=2.0)
    warp(shell, heat(0.02, 0.8, seed=21.0), dent((2.9, -0.5, 1.2), 0.45, 0.10, (-0.6, 0, -1)))
    rest_on_ground(0.004)


if __name__ == "__main__":
    run_disabled("type15", {CARD: DIMENSIONS}, "chinese_digital", build, wreck, mounts={CARD: MOUNTS},
                 skip=("dressing_", "gun", "hmg"), chip=1.0)
