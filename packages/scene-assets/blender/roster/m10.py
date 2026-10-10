"""M10 Booker light tank, from assets/references/m10/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/m10.py -- [--wreck]

What the photos settle (unveiling, C-17 loading, Yuma and Arctic testing):
six road wheels a side, the drive sprocket raised at the front and the idler
at the rear, side skirts in bolted panels over the top run; a tall blunt nose
of one big bolted plate with two hinged towing lugs, headlights in boxes on
the front corners, a short upper glacis to a long flat deck; the turret
amidships, tall and blunt: upright slab sides nearly the hull's width, a
broad front of two flat cheeks with chamfered corners coming to a prow, a
long chamfer from it back to the high flat roof, a long bustle box over the
rear deck, two four-tube smoke clusters on the cheeks,
the commander's panoramic sight dome on a short collar at the right, the M2
on a pintle ahead of the commander's hatch on the left, a mast behind; the
105 mm with a bore evacuator mid-barrel; a spare road wheel hung on the
hull's right rear. US desert tan (the Yuma photos; the unveiling vehicles
are green).

Built to the catalog frame (hull 7.0 x 3.4 x 2.98 m, gun axis 2.10 m up,
turret pivot 0.3 m behind the hull's middle):
the hull's length over its towing lugs (the published 6.85 m hull is the
plates), its width over the skirts and headlight boxes, and its height to
the top of the commander's sight, the highest fixed part (the M2 aside). The
published figures disagree (references gaps); these are the middle of them,
and everything along and up the vehicle (road-wheel pitch and size, deck,
turret, gun axis, sight) is laid out in proportion to that length off the
left-side photo.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

TRUNNION = 1.00
TRACK_Y = 1.32
TRACK_W = 0.50
# Six road wheels a side, 0.60 m across at a 0.82 m pitch (the side photo:
# 0.12 and 0.085 of the hull's length).
ROAD_R = 0.30
ROAD_Z = 0.38
ROAD_X = [1.82 - 0.82 * k for k in range(6)]
SPROCKET = (2.90, 0.66, 0.26)  # front drive
IDLER = (-2.95, 0.62, 0.26)
RETURNS = [(1.2, 0.84, 0.08), (-0.2, 0.85, 0.08), (-1.6, 0.84, 0.08)]
DECK = 1.62
# The nose: one tall blunt plate to 1.42 m, then a short glacis to the deck.
NOSE_TOP = 1.42
GLACIS = 0.55
SKIRT_Y = 1.60
SKIRT_TOP, SKIRT_FOOT = 1.24, 0.66
ROOF = 0.82  # the turret roof above the pivot, 2.60 m up: high and flat, as the side photos show
CUPOLA = (-0.70, 0.62)  # the commander's hatch, behind the M2 (turret frame)


def hull(v):
    m, h = v.mats, v.hull
    half = v.length / 2 - 0.165  # the length counts the towing lugs and hooks
    # The tall blunt nose, a short upper glacis, then the long flat deck.
    prism("hull_upper", [(-half, 0.92), (half - 0.10, 0.92), (half, 1.06), (half, NOSE_TOP), (half - GLACIS, DECK),
                         (-half + 0.08, DECK), (-half, DECK - 0.12)], 3.10, mat=m["paint"], parent=h, bevel=0.06)
    prism("hull_lower", [(-half + 0.40, 0.46), (half - 0.55, 0.46), (half - 0.08, 0.94), (-half, 0.94),
                         (-half, 0.72)], 2.12, mat=m["paint"], parent=h, bevel=0.04)
    # The nose's one big bolted plate, its two towing lugs.
    VP.bolted_panel("nose_plate", (half + 0.005, 0, 1.10), (0.64, 2.70, 0.06), m, h, bolts=(3, 6),
                    rot=(0, math.pi / 2, 0), bevel=0.02, lods=VP.ALL)
    for side, s in ((1, "L"), (-1, "R")):
        box(f"nose_lug_{s}", (0.16, 0.14, 0.30), (half + 0.12, side * 0.62, 1.00), m["paint"], h, bevel=0.03,
            lods=MID)
        VP.shackle(f"front_shackle_{s}", (half + 0.12, side * 0.62, 0.82), m, h, size=0.13, rot=(0, 0, math.pi / 2))
        # Headlights in armoured boxes on the front corners.
        box(f"light_box_{s}", (0.28, 0.42, 0.24), (half - 0.20, side * 1.48, NOSE_TOP + 0.04), m["paint"], h, bevel=0.03)
        VP.light_with_guard(f"headlight_{s}", (half - 0.05, side * 1.42, NOSE_TOP + 0.04), 0.06, m, h)
        VP.light_with_guard(f"tail_light_{s}", (-half - 0.01, side * 1.40, 1.45), 0.05, dict(m, lamp=m["tail"]), h,
                            rot=(0, 0, math.pi))
        VP.tow_hook(f"rear_tow_{s}", (-half + 0.02, side * 0.85, 0.82), m, h, size=0.13, rot=(0, 0, math.pi))
        VP.cable(f"tow_cable_{s}", [(-half + 0.3, side * 1.45, DECK + 0.02), (-1.0, side * 1.48, DECK + 0.02),
                                    (0.5, side * 1.45, DECK + 0.02)], m, h, radius=0.022)
        # Deck bins at the rear corners, clear of the turret's bustle bins
        # as it traverses.
        VP.stowage_box(f"deck_bin_{s}", (-3.03, side * 1.30, DECK), (0.60, 0.36, 0.30), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
    slope = math.atan((DECK - NOSE_TOP) / GLACIS)
    VP.hatch("driver_hatch", (half - 0.85, 0.0, DECK), m, h, radius=0.28)
    for k, y in enumerate((-0.22, 0.0, 0.22)):
        VP.periscope(f"driver_periscope_{k}", (half - 0.40, y, NOSE_TOP + 0.40 * math.tan(slope) - 0.01), m, h,
                     size=(0.12, 0.16, 0.08), rot=(0, slope, 0))
    VP.grille("engine_grille_L", (-2.85, 0.55, DECK), (0.80, 0.80), m, h, slats=7)
    VP.grille("engine_grille_R", (-2.85, -0.55, DECK), (0.80, 0.80), m, h, slats=7)
    VP.bolted_panel("rear_plate", (-half - 0.005, 0, 1.22), (0.55, 2.20, 0.05), m, h, bolts=(2, 4),
                    rot=(0, -math.pi / 2, 0), bevel=0.02, lods=VP.ALL)
    VP.exhaust("exhaust", (-half - 0.02, -1.10, 1.30), 0.09, 0.22, m, h, rot=(0, 0, math.pi))
    # The spare road wheel hung on the hull's right rear.
    spare = empty("dressing_spare_wheel", parent=h)
    cyl("spare_tyre", ROAD_R, 0.18, (-3.00, -1.58, 1.40), "Y", m["rubber"], spare, seg=24)
    cyl("spare_disc", ROAD_R * 0.75, 0.20, (-3.00, -1.58, 1.40), "Y", m["paint"], spare, seg=20, lods=MID)


def skirts(v):
    """Side skirts in five bolted panels over the top run, the front one a
    wedge round the raised sprocket."""
    m, h = v.mats, v.hull
    panels = [[(1.95, SKIRT_FOOT), (2.70, SKIRT_FOOT + 0.05), (3.15, 1.02), (3.15, SKIRT_TOP), (1.95, SKIRT_TOP)]]
    for k in range(4):
        front = 1.93 - 1.17 * k
        panels.append([(front - 1.15, SKIRT_FOOT), (front, SKIRT_FOOT), (front, SKIRT_TOP), (front - 1.15, SKIRT_TOP)])
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
        stencil(f"skirt_mark_{s}", "M10", 0.22, (2.45, face + side * 0.003, 1.02),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], h)


# The turret's shape (turret frame, from the photos): upright slab sides
# nearly the hull's width, a broad blunt front of two flat cheeks either side
# of the mantlet's slot with chamfered outer corners, coming to a prow a
# well above the gun axis; under it the front falls back to the ring, over
# it a short chamfer runs back to the high flat roof. The front stands near
# upright and the bustle is short, so the turret reads as a tall box and not
# the Abrams' long low wedge.
TURRET_SIDE = 1.48
PROW = (1.30, 0.36)  # (x, z): the cheeks' most forward line
UPPER_FRONT = 1.02  # where the upper chamfer meets the roof (x)
BACK = -2.00  # the bustle's back (x)


def turret_plan(front, side):
    """One ring of the turret: the mantlet's slot, the flat cheeks, their
    chamfered corners, the upright sides and the bustle's back."""
    left = [(front - 0.25, 0.38), (front, 0.50), (front, side - 0.30), (front - 0.26, side), (-1.30, side),
            (BACK, side - 0.10)]
    return left + [(x, -y) for x, y in reversed(left)]


def turret_body(v, turret):
    m = v.mats
    base = DECK - v.frame["mounts"][0]["pivot_m"][2]
    cyl("turret_ring_guard", 1.00, 0.08, (0, 0, base + 0.02), "Z", m["dark"], turret, seg=36, lods=MID)
    loft("turret_shell", [(base + 0.03, turret_plan(PROW[0] - 0.22, TURRET_SIDE - 0.02)),
                          (PROW[1], turret_plan(PROW[0], TURRET_SIDE)),
                          (ROOF, turret_plan(UPPER_FRONT, TURRET_SIDE - 0.03))],
         mat=m["paint"], parent=turret, bevel=0.05)
    side_face = ((TURRET_SIDE, PROW[1]), (TURRET_SIDE - 0.03, ROOF))
    for side, s in ((1, "L"), (-1, "R")):
        # A four-tube smoke cluster on each cheek, firing forward over the
        # prow.
        VP.smoke_discharger_bank(f"smoke_{s}", (PROW[0] - 0.14, side * 0.88, PROW[1] + 0.06), m, turret, count=4,
                                 tube_radius=0.055, tube_length=0.22, elevation=0.25, spread=0.25,
                                 rot=(0, 0, side * 0.2))
        # The cheek module's bolted side plate, on the turret's flank.
        loc, rot = VP.on_side(0.30, (PROW[1] + ROOF) / 2, side, *side_face)
        VP.bolted_panel(f"cheek_plate_{s}", loc, (1.30, ROOF - PROW[1] - 0.12, 0.05), m, turret, bolts=(4, 2),
                        bevel=0.02, lods=VP.ALL, rot=rot)
        VP.stowage_box(f"bustle_bin_{s}", (-1.60, side * (TURRET_SIDE - 0.08), base + 0.28), (0.85, 0.24, 0.40), m,
                       turret, rot=(0, 0, 0 if side > 0 else math.pi))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (BACK + 0.22, side * 0.95, ROOF), m, whip, height=2.2)
    # Roof: the gunner's sight behind the chamfer on the right, the
    # commander's dome on a short collar at the right, top of the frame
    # (fixed armour, not dressing), the loader's hatch and the mast.
    VP.sight_housing("gunner_sight", (UPPER_FRONT - 0.10, -0.62, ROOF - 0.04), m, turret, size=(0.46, 0.38, 0.26))
    dome = empty("commander_sight", parent=turret)
    cyl("dome_post", 0.14, 0.08, (-0.25, -0.70, ROOF + 0.04), "Z", m["dark"], dome, seg=16, lods=MID)
    cyl("dome_head", 0.24, 0.30, (-0.25, -0.70, ROOF + 0.23), "Z", m["paint"], dome, seg=24, bevel=0.07)
    box("dome_window", (0.03, 0.22, 0.12), (-0.01, -0.70, ROOF + 0.23), m["glass"], dome, lods=MID)
    VP.cupola("commander_cupola", (*CUPOLA, ROOF), m, turret, radius=0.34, periscopes=5)
    VP.hatch("loader_hatch", (-0.95, -0.15, ROOF), m, turret, radius=0.26)
    mast = empty("dressing_mast", parent=turret)
    cyl("mast_pole", 0.03, 0.90, (BACK + 0.25, 0.30, ROOF + 0.45), "Z", m["dark"], mast, seg=8, lods=NEAR)
    box("mast_head", (0.14, 0.14, 0.12), (BACK + 0.25, 0.30, ROOF + 0.92), m["dark"], mast, bevel=0.02, lods=NEAR)
    VP.stowage_box("bustle_box", (BACK + 0.26, 0, base + 0.25), (0.50, 2.10, 0.55), m, turret)


def gun_105(v, gun):
    """The 105 mm: a squared gun shield, the barrel with its bore evacuator
    mid-way and a plain muzzle."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0] - TRUNNION
    box("gun_shield", (0.50, 0.66, 0.50), (0.05, 0, 0.0), m["paint"], gun, bevel=0.05)
    cyl("barrel_root", 0.11, 0.40, (0.45, 0, 0), "X", m["paint"], gun, seg=24)
    evac = 1.70
    a, b = 0.65, reach - 0.18  # one tube under the evacuator, so the barrel reads unbroken
    cyl("barrel", 0.075, b - a, ((a + b) / 2, 0, 0), "X", m["paint"], gun, seg=22)
    cyl("bore_evacuator", 0.13, 0.48, (evac, 0, 0), "X", m["paint"], gun, seg=24, bevel=0.02)
    cyl("muzzle_end", 0.085, 0.18, (reach - 0.09, 0, 0), "X", m["steel"], gun, seg=22)
    cyl("muzzle_bore", 0.053, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=18, lods=MID)


def build(variant, v):
    hull(v)
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.22, SPROCKET, IDLER, RETURNS,
                            bolts=8, teeth=11, arm=(0.40, 0.40), pitch=0.15)
    skirts(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret_body(v, turret)
    gun_105(v, gun)
    cyl("pintle", 0.04, 0.24, (0, 0, -0.10), "Z", v.mats["dark"], hmg, seg=10)
    gun_mount, hmg_mount = v.frame["mounts"]
    VP.browning_m2(hmg_gun, hmg_mount["muzzle_m"][0], v.mats, grips=True)
    v.head_out("commander", turret, gun_mount["pivot_m"][0] + CUPOLA[0], CUPOLA[1], gun_mount["pivot_m"][2] + ROOF)


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
    thrown = solid("thrown_track", (3.2, TRACK_W, 0.05), (-0.3, 1.95, 0.03), m["track"], v.hull, rot=(0, 0, -0.05),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.03, 0.6, seed=4.0))
    bend(parts("skirt_L_3"), (0, SKIRT_Y, SKIRT_TOP - 0.05), (1, 0, 0), (0, 0, -1), 0.5)
    shell = parts("hull_upper", "hull_lower", "skirt_", "turret_shell", "bustle_box", "nose_plate")
    densify(shell, scale=2.0)
    warp(shell, heat(0.022, 0.9, seed=31.0), dent((3.34, -0.6, 1.05), 0.45, 0.10, (-1, 0, -0.2)))
    for k, (loc, rot, size) in enumerate((((1.0, 2.4, 0.03), (0.04, 0.02, 0.5), 0.42),
                                          ((-2.3, -2.1, 0.03), (-0.03, 0.05, 1.7), 0.32),
                                          ((3.9, 0.8, 0.03), (0.0, 0.06, 2.4), 0.28))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.15, seed=141 + k)
    rest_on_ground(0.004)


run("m10", "us_desert_tan", build, wreck, chip=1.0)
