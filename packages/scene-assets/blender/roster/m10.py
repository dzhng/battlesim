"""M10 Booker light tank (disabled card), from assets/references/m10/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/m10.py -- [--wreck]

What the photos settle (unveiling, C-17 loading, Yuma and Arctic testing):
six road wheels a side, the drive sprocket raised at the front and the idler
at the rear, side skirts in bolted panels over the top run; a tall blunt nose
of one big bolted plate with two hinged towing lugs, headlights in boxes on
the front corners, a short upper glacis to a long flat deck; the turret well
forward of centre, slab-sided like the Abrams' with flat swept cheeks and a
long bustle box, two four-tube smoke clusters on the cheeks, the
commander's panoramic sight dome on its post at the right rear, the M2 on a
pintle at the commander's hatch on the left, a mast behind; the 105 mm with a
bore evacuator mid-barrel; a spare road wheel hung on the hull's right rear.
US desert tan (the Yuma photos; the unveiling vehicles are green).

No archived frame exists: the frame is the hull's length, width and height
to the turret roof, measured off the left-side photo with its road wheels
(no published dimensions; references gaps). With no mount in a disabled
frame, the turret and gun articulate on nodes stated here from the photos,
so its wreck throws the turret; nothing in the simulation reads them.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

CARD = "us_m10_booker"
DIMENSIONS = (8.10, 3.45, 2.80)
MOUNTS = [
    dict(name="cannon", role="gun", on=None, pivot_m=[0.55, 0.0, 1.78], muzzle_m=[5.10, 0.0, 0.33]),
    dict(name="HMG", role="hmg", on="cannon", pivot_m=[0.70, 0.62, 2.79], muzzle_m=[1.20, 0.0, 0.22]),
]
TRUNNION = 1.05
TRACK_Y = 1.32
TRACK_W = 0.50
ROAD_R = 0.34
ROAD_Z = 0.42
ROAD_X = [2.10 - 0.86 * k for k in range(6)]
SPROCKET = (3.10, 0.80, 0.30)  # front drive
IDLER = (-3.25, 0.62, 0.30)
RETURNS = [(1.5, 0.98, 0.09), (-0.2, 0.99, 0.09), (-1.9, 0.98, 0.09)]
DECK = 1.62
SKIRT_Y = 1.62
SKIRT_TOP, SKIRT_FOOT = 1.45, 0.70
ROOF = 0.86  # the turret roof above the pivot: low and squat, as the side photos show


def hull(v):
    m, h = v.mats, v.hull
    half = v.length / 2 - 0.15  # the length counts the towing lugs and hooks
    # The tall blunt nose, a short upper glacis, then the long flat deck.
    prism("hull_upper", [(-half, 0.92), (half - 0.10, 0.92), (half, 1.06), (half, 1.30), (half - 0.85, DECK),
                         (-half + 0.08, DECK), (-half, DECK - 0.12)], 3.10, mat=m["paint"], parent=h, bevel=0.06)
    prism("hull_lower", [(-half + 0.40, 0.46), (half - 0.55, 0.46), (half - 0.08, 0.94), (-half, 0.94),
                         (-half, 0.72)], 2.12, mat=m["paint"], parent=h, bevel=0.04)
    # The nose's one big bolted plate, its two towing lugs.
    VP.bolted_panel("nose_plate", (half + 0.005, 0, 1.02), (0.50, 2.70, 0.06), m, h, bolts=(3, 6),
                    rot=(0, math.pi / 2, 0), bevel=0.02, lods=VP.ALL)
    for side, s in ((1, "L"), (-1, "R")):
        box(f"nose_lug_{s}", (0.16, 0.14, 0.30), (half + 0.12, side * 0.62, 1.00), m["paint"], h, bevel=0.03,
            lods=MID)
        VP.shackle(f"front_shackle_{s}", (half + 0.12, side * 0.62, 0.82), m, h, size=0.13, rot=(0, 0, math.pi / 2))
        # Headlights in armoured boxes on the front corners.
        box(f"light_box_{s}", (0.28, 0.42, 0.24), (half - 0.20, side * 1.48, 1.42), m["paint"], h, bevel=0.03)
        VP.light_with_guard(f"headlight_{s}", (half - 0.05, side * 1.42, 1.42), 0.06, m, h)
        VP.light_with_guard(f"tail_light_{s}", (-half - 0.01, side * 1.40, 1.45), 0.05, dict(m, lamp=m["tail"]), h,
                            rot=(0, 0, math.pi))
        VP.tow_hook(f"rear_tow_{s}", (-half + 0.02, side * 0.85, 0.82), m, h, size=0.13, rot=(0, 0, math.pi))
        VP.cable(f"tow_cable_{s}", [(-half + 0.3, side * 1.45, DECK + 0.02), (-1.2, side * 1.48, DECK + 0.02),
                                    (0.6, side * 1.45, DECK + 0.02)], m, h, radius=0.022)
        VP.stowage_box(f"deck_bin_{s}", (-2.20, side * 1.30, DECK), (1.30, 0.36, 0.30), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
    slope = math.atan((DECK - 1.30) / 0.85)
    VP.hatch("driver_hatch", (half - 0.95, 0.0, DECK), m, h, radius=0.28)
    for k, y in enumerate((-0.22, 0.0, 0.22)):
        VP.periscope(f"driver_periscope_{k}", (half - 0.62, y, 1.30 + 0.62 * math.tan(slope) - 0.01), m, h,
                     size=(0.12, 0.16, 0.08), rot=(0, slope, 0))
    VP.grille("engine_grille_L", (-2.65, 0.55, DECK), (1.20, 0.80), m, h, slats=9)
    VP.grille("engine_grille_R", (-2.65, -0.55, DECK), (1.20, 0.80), m, h, slats=9)
    VP.bolted_panel("rear_plate", (-half - 0.005, 0, 1.22), (0.55, 2.20, 0.05), m, h, bolts=(2, 4),
                    rot=(0, -math.pi / 2, 0), bevel=0.02, lods=VP.ALL)
    VP.exhaust("exhaust", (-half - 0.02, -1.10, 1.30), 0.09, 0.22, m, h, rot=(0, 0, math.pi))
    # The spare road wheel hung on the hull's right rear.
    spare = empty("dressing_spare_wheel", parent=h)
    cyl("spare_tyre", ROAD_R, 0.18, (-3.05, -1.62, 1.30), "Y", m["rubber"], spare, seg=24)
    cyl("spare_disc", ROAD_R * 0.75, 0.20, (-3.05, -1.62, 1.30), "Y", m["paint"], spare, seg=20, lods=MID)


def skirts(v):
    """Side skirts in five bolted panels over the top run, the front one a
    wedge round the raised sprocket."""
    m, h = v.mats, v.hull
    panels = [[(2.25, SKIRT_FOOT), (3.05, SKIRT_FOOT + 0.05), (3.55, 1.12), (3.55, SKIRT_TOP), (2.25, SKIRT_TOP)]]
    for front, rear in ((2.23, 0.93), (0.91, -0.39), (-0.41, -1.71), (-1.73, -3.10)):
        panels.append([(rear, SKIRT_FOOT), (front, SKIRT_FOOT), (front, SKIRT_TOP), (rear, SKIRT_TOP)])
    for side, s in ((1, "L"), (-1, "R")):
        face = side * (SKIRT_Y + 0.04)
        for k, outline in enumerate(panels):
            prism(f"skirt_{s}_{k}", outline, 0.08, loc=(0, side * SKIRT_Y, 0), mat=m["paint"], parent=h, bevel=0.025,
                  lods=VP.ALL)
            xs = [p[0] for p in outline]
            for j in range(3):
                bx = min(xs) + (max(xs) - min(xs)) * (j + 0.5) / 3
                for z in (SKIRT_TOP - 0.06, SKIRT_FOOT + 0.08):
                    cyl(f"skirt_bolt_{s}_{k}_{j}_{int(z * 100)}", 0.022, 0.02, (bx, face + side * 0.006, z), "Y",
                        m["steel"], h, seg=6, lods=FINE)
        stencil(f"skirt_mark_{s}", "M10", 0.24, (2.80, face + side * 0.003, 1.22),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], h)


def turret_body(v, turret):
    m = v.mats
    foot = [(1.30, 0.40), (1.60, 0.52), (0.90, 1.42), (-1.40, 1.48), (-2.45, 1.30), (-2.45, -1.30), (-1.40, -1.48),
            (0.90, -1.42), (1.60, -0.52), (1.30, -0.40)]
    crown = [(1.20, 0.40), (1.30, 0.50), (0.60, 1.30), (-1.40, 1.36), (-2.40, 1.22), (-2.40, -1.22), (-1.40, -1.36),
             (0.60, -1.30), (1.30, -0.50), (1.20, -0.40)]
    base = DECK - MOUNTS[0]["pivot_m"][2]
    cyl("turret_ring_guard", 1.00, 0.08, (0, 0, base + 0.02), "Z", m["dark"], turret, seg=36, lods=MID)
    loft("turret_shell", [(base + 0.03, foot), (base + 0.18, foot), (ROOF, crown)], mat=m["paint"], parent=turret,
         bevel=0.05)
    for side, s in ((1, "L"), (-1, "R")):
        # Two four-tube smoke clusters on the cheeks.
        VP.smoke_discharger_bank(f"smoke_{s}", (0.70, side * 1.25, ROOF - 0.32), m, turret, count=4,
                                 tube_radius=0.055, tube_length=0.22, elevation=0.25, spread=0.25,
                                 rot=(0, 0, side * 0.75))
        VP.bolted_panel(f"cheek_plate_{s}", (-0.40, side * 1.44, ROOF - 0.45), (1.40, 0.50, 0.05), m, turret,
                        bolts=(4, 2), bevel=0.02, lods=VP.ALL, rot=(-side * math.pi / 2, 0, 0))
        VP.stowage_box(f"bustle_bin_{s}", (-1.95, side * 1.38, base + 0.30), (0.95, 0.24, 0.42), m, turret,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-2.20, side * 0.95, ROOF), m, whip, height=2.2)
    # Roof: the gunner's sight ahead on the right, the commander's dome on
    # its post at the right rear, the loader's hatch and the mast.
    VP.sight_housing("gunner_sight", (0.75, -0.62, ROOF - 0.04), m, turret, size=(0.46, 0.38, 0.26))
    dome = empty("dressing_commander_sight", parent=turret)
    cyl("dome_post", 0.12, 0.30, (-0.85, -0.70, ROOF + 0.15), "Z", m["dark"], dome, seg=16, lods=MID)
    cyl("dome_head", 0.26, 0.34, (-0.85, -0.70, ROOF + 0.45), "Z", m["paint"], dome, seg=24, bevel=0.08)
    box("dome_window", (0.03, 0.26, 0.14), (-0.60, -0.70, ROOF + 0.45), m["glass"], dome, lods=MID)
    VP.cupola("commander_cupola", (-0.20, 0.62, ROOF), m, turret, radius=0.34, periscopes=5)
    VP.hatch("loader_hatch", (-0.65, -0.05, ROOF), m, turret, radius=0.26)
    mast = empty("dressing_mast", parent=turret)
    cyl("mast_pole", 0.03, 0.90, (-1.70, 0.40, ROOF + 0.45), "Z", m["dark"], mast, seg=8, lods=NEAR)
    box("mast_head", (0.14, 0.14, 0.12), (-1.70, 0.40, ROOF + 0.92), m["dark"], mast, bevel=0.02, lods=NEAR)
    VP.stowage_box("bustle_box", (-2.15, 0, base + 0.25), (0.55, 2.10, 0.55), m, turret)


def gun_105(v, gun):
    """The 105 mm: a squared gun shield, the barrel with its bore evacuator
    mid-way and a plain muzzle."""
    m = v.mats
    reach = MOUNTS[0]["muzzle_m"][0] - TRUNNION
    box("gun_shield", (0.50, 0.66, 0.50), (0.05, 0, 0.0), m["paint"], gun, bevel=0.05)
    cyl("barrel_root", 0.11, 0.40, (0.45, 0, 0), "X", m["paint"], gun, seg=24)
    evac = 1.75
    for k, (a, b) in enumerate([(0.65, evac - 0.28), (evac + 0.28, reach - 0.18)]):
        cyl(f"barrel_{k}", 0.075, b - a, ((a + b) / 2, 0, 0), "X", m["paint"], gun, seg=22)
    cyl("bore_evacuator", 0.13, 0.48, (evac, 0, 0), "X", m["paint"], gun, seg=24, bevel=0.02)
    cyl("muzzle_end", 0.085, 0.18, (reach - 0.09, 0, 0), "X", m["steel"], gun, seg=22)
    cyl("muzzle_bore", 0.053, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=18, lods=MID)


def build(variant, v):
    hull(v)
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.22, SPROCKET, IDLER, RETURNS,
                            bolts=8, teeth=11, arm=(0.44, 0.40), pitch=0.16)
    skirts(v)
    mounts = rig({"mounts": MOUNTS}, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret_body(v, turret)
    gun_105(v, gun)
    cyl("pintle", 0.04, 0.24, (0, 0, -0.10), "Z", v.mats["dark"], hmg, seg=10)
    VP.browning_m2(hmg_gun, MOUNTS[1]["muzzle_m"][0], v.mats, grips=True)
    v.head_out("commander", turret, MOUNTS[0]["pivot_m"][0] - 0.20, 0.62, MOUNTS[0]["pivot_m"][2] + ROOF)


def wreck(variant, v):
    """The Booker after its fire: the left track run off and lying beside it
    with two road wheels gone, two skirt panels blown away and one bent out,
    the deck bins and bustle box burst, plates warped, a dent in the nose
    plate; `wreckage.burn` heaves the turret."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("track_L_band", "wheel_L_2_", "wheel_L_4_", "skirt_L_1", "skirt_L_2", "skirt_bolt_L_1_",
           "skirt_bolt_L_2_", "deck_bin_L", "loader_hatch", "bustle_box_lid", "skirt_mark_L")
    thrown = solid("thrown_track", (3.6, TRACK_W, 0.05), (-0.3, 2.00, 0.03), m["track"], v.hull, rot=(0, 0, -0.05),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.03, 0.6, seed=4.0))
    bend(parts("skirt_L_3"), (0, SKIRT_Y, SKIRT_TOP - 0.05), (1, 0, 0), (0, 0, -1), 0.5)
    shell = parts("hull_upper", "hull_lower", "skirt_", "turret_shell", "bustle_box", "nose_plate")
    densify(shell, scale=2.0)
    warp(shell, heat(0.022, 0.9, seed=31.0), dent((3.9, -0.6, 1.05), 0.45, 0.10, (-1, 0, -0.2)))
    for k, (loc, rot, size) in enumerate((((1.2, 2.5, 0.03), (0.04, 0.02, 0.5), 0.42),
                                          ((-2.6, -2.2, 0.03), (-0.03, 0.05, 1.7), 0.32),
                                          ((4.3, 0.8, 0.03), (0.0, 0.06, 2.4), 0.28))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.15, seed=141 + k)
    rest_on_ground(0.004)


if __name__ == "__main__":
    run_disabled("m10", {CARD: DIMENSIONS}, "us_desert_tan", build, wreck,
                 skip=("dressing_", "gun", "hmg", "muzzle"), chip=1.0)
