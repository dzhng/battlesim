"""Tracked 155/152 mm self-propelled howitzers (disabled cards): M109A7 Paladin,
PzH 2000, 2S19M2 Msta-S and PLZ-05, from assets/references/<family>/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/sp_howitzers.py -- [--variant=<card id>] [--wreck]

What the photos settle, shared: a tracked hull with the engine and driver at
the front (the 2S19's T-80 hull has its engine at the rear), a big slab-sided
turret over the rear, its long gun travelling level over the glacis on a
folding travel lock, a cupola with a machine gun, smoke banks, stowage boxes
on the turret's flanks and a rear door into the turret.

- M109A7 (desert tan, Bull Battery and PIM photos): six road wheels (the
  Bradley's), the boxy turret almost the hull's width, the M284's bore
  evacuator and double-baffle brake, the M2 on the commander's cupola.
- PzH 2000 (German three-tone, Hungarian, Croatian and Munster photos):
  seven road wheels (the Leopard's), the turret's angular front and flat
  sides, the 52-calibre gun's pepperpot brake, no evacuator.
- 2S19M2 (Russian green, Oboronexpo 2014): six road wheels, the T-80 hull
  with its engine at the rear, the tall turret centred, the 2A64's fume
  extractor and brake, the remote MG over the commander's hatch, the
  dozer blade under the nose.
- PLZ-05 (PLA digital, Beijing museum and parade): seven road wheels, the
  turret long and low at the rear, the 52-calibre gun with fume extractor
  and multi-baffle brake.

Each card's frame is the hull's length,
width and height stated from its published dimensions (the gun excluded),
recorded in its receipt as `references`. With no mount in the frame the gun
is drawn at rest and does not articulate until the card's mechanics land.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

# Each card: hull dimensions (length, width, height, from references), its
# running gear, deck, turret plan and the gun.
SPECS = {
    "us_m109_paladin_m109a7": dict(
        family="m109_paladin", scheme="us_desert_tan", dims=(6.80, 3.90, 3.25), wheels=6, road_r=0.33,
        front_engine=True, deck=1.55, turret=(-3.15, 0.55, 3.10, 1.30), gun=(0.62, 2.30, 6.05, 0.078),
        brake="baffle", evacuator=0.45, cupola=(-1.2, -0.85), mg="m2", label="B12"),
    "europe_pzh_2000_tracked_155_mm_howitzer": dict(
        family="pzh_2000", scheme="german_three_tone", dims=(7.87, 3.58, 3.06), wheels=7, road_r=0.35,
        front_engine=True, deck=1.62, turret=(-3.90, 0.35, 3.15, 1.20), gun=(0.40, 2.36, 8.06, 0.080),
        brake="pepperpot", evacuator=None, cupola=(-1.6, -0.85), mg="mg3", label="241"),
    "eastern_2s19_msta_s_2s19m2": dict(
        family="2s19_msta_s", scheme="russian_green", dims=(7.15, 3.58, 2.99), wheels=6, road_r=0.37,
        front_engine=False, deck=1.40, turret=(-2.60, 1.20, 3.25, 1.30), gun=(1.25, 2.20, 7.15, 0.080),
        brake="baffle", evacuator=0.40, cupola=(-1.0, -0.90), mg="kord", label="312"),
    "eastern_plz_05_chinese_tracked_155_mm_howitzer": dict(
        family="plz_05", scheme="chinese_digital", dims=(7.30, 3.40, 3.10), wheels=7, road_r=0.33,
        front_engine=True, deck=1.55, turret=(-3.55, 0.45, 3.05, 1.25), gun=(0.50, 2.32, 8.06, 0.080),
        brake="baffle", evacuator=0.42, cupola=(-1.5, -0.80), mg="qjc88", label="05"),
}
TRACK_W = 0.50


def spec(v):
    return SPECS[v.variant["id"]]


def hull(v):
    s = spec(v)
    m, h = v.mats, v.hull
    half = v.length / 2
    deck = s["deck"]
    nose = deck - 0.45
    prism("hull_upper", [(-half, 0.88), (half - 0.35, 0.88), (half, nose), (half - 1.15, deck), (-half + 0.06, deck),
                         (-half, deck - 0.10)], v.width - 0.50, mat=m["paint"], parent=h, bevel=0.05)
    prism("hull_lower", [(-half + 0.35, 0.44), (half - 0.75, 0.44), (half - 0.10, 0.90), (-half, 0.90),
                         (-half, 0.70)], v.width - 1.40, mat=m["paint"], parent=h, bevel=0.04)
    slope = math.atan((deck - nose) / 1.15)
    driver_y = 0.75 if s["front_engine"] else 0.0
    VP.hatch("driver_hatch", (half - 1.45 if s["front_engine"] else half - 1.30, driver_y, deck), m, h, radius=0.26)
    VP.periscope("driver_periscope", (half - 1.12, driver_y, deck), m, h, size=(0.12, 0.26, 0.09))
    if s["front_engine"]:
        VP.grille("engine_grille", (half - 1.80, -0.55, deck), (1.10, 1.30), m, h, slats=10)
        VP.exhaust("exhaust", (half - 1.60, -v.width / 2 + 0.22, deck - 0.25), 0.09, 0.28, m, h,
                   rot=(0, 0, -math.pi / 2))
    else:
        VP.grille("engine_grille", (-half + 0.90, 0, deck), (1.20, v.width - 1.0), m, h, slats=12)
        VP.bolted_panel("dozer_blade", (half + 0.05, 0, 0.50), (0.10, v.width - 1.0, 0.45), m, h, bolts=(1, 4),
                        bevel=0.02, lods=VP.ALL)
    # The travel lock folded up on the glacis, its cradle under the barrel.
    lock_x = half - 0.55
    for side in (-1, 1):
        box(f"travel_lock_leg_{side}", (0.08, 0.06, 0.70), (lock_x, side * 0.16, nose + 0.35), m["dark"], h,
            rot=(0, -0.25, 0), lods=MID)
    box("travel_lock_cradle", (0.12, 0.42, 0.10), (lock_x - 0.08, 0, nose + 0.70), m["dark"], h, bevel=0.02)
    for side, sd in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{sd}", (half - 0.70, side * (v.width / 2 - 0.45), nose + 0.18), 0.07, m, h,
                            rot=(0, slope, 0))
        VP.tow_hook(f"front_tow_{sd}", (half - 0.02, side * 0.70, 0.80), m, h, size=0.13)
        VP.tow_hook(f"rear_tow_{sd}", (-half + 0.02, side * 0.80, 0.80), m, h, size=0.13, rot=(0, 0, math.pi))
        skirts = s["wheels"]
        for k in range(skirts):
            length = (v.length - 0.5) / skirts
            x = half - 0.30 - length * (k + 0.5)
            VP.bolted_panel(f"skirt_{sd}_{k}", (x, side * (v.width / 2 - 0.05), 0.82), (length - 0.03, 0.06, 0.55), m,
                            h, bolts=(2, 1), bevel=0.02, lods=VP.ALL)
        VP.cable(f"tow_cable_{sd}", [(-half + 0.4, side * (v.width / 2 - 0.18), deck + 0.02),
                                     (0.5, side * (v.width / 2 - 0.18), deck + 0.02)], m, h, radius=0.02)


def running_gear(v):
    s = spec(v)
    half = v.length / 2
    n = s["wheels"]
    r = s["road_r"]
    span = v.length - 1.9
    road_x = [half - 1.05 - span * k / (n - 1) for k in range(n)]
    front = (half - 0.40, r + 0.28, r * 0.85)
    rear = (-half + 0.40, r + 0.26, r * 0.80)
    sprocket, idler = (front, rear) if s["front_engine"] else (rear, front)
    returns = [(road_x[1] - 0.2, 2 * r + 0.18, 0.09), (road_x[n // 2], 2 * r + 0.19, 0.09),
               (road_x[-2] + 0.2, 2 * r + 0.18, 0.09)]
    VP.tracked_running_gear(v.mats, v.hull, v.width / 2 - 0.40, TRACK_W, road_x, r + 0.08, r, 0.20, sprocket, idler,
                            returns, bolts=6, teeth=12, arm=(0.40, -0.30), pitch=0.15, dual=True)


def turret(v):
    """The turret, gun, cupola and stowage: fixed parts of the hull (no frame
    mount yet)."""
    s = spec(v)
    m, h = v.mats, v.hull
    rear, front, width, height = s["turret"]
    deck = s["deck"]
    top = deck + height
    half_w = width / 2
    foot = [(front, half_w - 0.45), (front - 0.40, half_w), (rear, half_w), (rear, -half_w), (front - 0.40, -half_w),
            (front, -half_w + 0.45)]
    crown = [(front - 0.40, half_w - 0.55), (front - 0.75, half_w - 0.08), (rear + 0.05, half_w - 0.08),
             (rear + 0.05, -half_w + 0.08), (front - 0.75, -half_w + 0.08), (front - 0.40, -half_w + 0.55)]
    loft("turret_shell", [(deck, foot), (deck + height * 0.55, foot), (top, crown)], mat=m["paint"], parent=h,
         bevel=0.05)
    gun_x, gun_z, length, bore_r = s["gun"]
    # Mantlet, barrel, evacuator, muzzle brake.
    box("gun_mantlet", (0.55, 0.70, 0.55), (front + 0.10, 0, gun_z), m["paint"], h, bevel=0.06)
    start = front + 0.35
    end = start + length
    cyl("gun_cradle", bore_r * 2.3, 0.90, (start + 0.45, 0, gun_z), "X", m["paint"], h, seg=24)
    cyl("gun_barrel", bore_r * 1.25, length - 0.9, ((start + 0.9 + end) / 2, 0, gun_z), "X", m["paint"], h, seg=22)
    if s["evacuator"]:
        cyl("fume_extractor", bore_r * 2.0, s["evacuator"], (start + length * 0.45, 0, gun_z), "X", m["paint"], h,
            seg=22, bevel=0.02)
    if s["brake"] == "pepperpot":
        cyl("muzzle_brake", bore_r * 1.9, 0.55, (end - 0.27, 0, gun_z), "X", m["dark"], h, seg=20, bevel=0.015)
        for k in range(5):
            for side in (-1, 1):
                cyl(f"brake_port_{k}_{side}", 0.030, 0.02, (end - 0.48 + k * 0.10, side * bore_r * 1.9, gun_z), "Y",
                    m["black"], h, seg=8, lods=FINE)
    else:
        for k in range(2):
            box(f"brake_baffle_{k}", (0.12, bore_r * 4.6, bore_r * 3.0), (end - 0.08 - k * 0.24, 0, gun_z), m["dark"], h,
                bevel=0.015)
        cyl("muzzle_brake_core", bore_r * 1.5, 0.45, (end - 0.20, 0, gun_z), "X", m["dark"], h, seg=18)
    cyl("muzzle_bore", bore_r, 0.012, (end + 0.005, 0, gun_z), "X", m["black"], h, seg=16, lods=MID)
    # Cupola with its machine gun, a hatch, sights, smoke banks, boxes.
    cx, cy = s["cupola"]
    VP.cupola("commander_cupola", (cx, cy, top), m, h, radius=0.32, periscopes=5, lid_open=False)
    mg_z = top + 0.30
    box("mg_cradle", (0.30, 0.14, 0.12), (cx + 0.15, cy, mg_z - 0.08), m["dark"], h, lods=MID)
    if s["mg"] == "m2":
        mg = empty("dressing_cupola_mg", loc=(cx + 0.10, cy, mg_z), parent=h)
        VP.browning_m2(mg, 1.35, m)
    else:
        box("mg_receiver", (0.55, 0.10, 0.13), (cx + 0.25, cy, mg_z), m["dark"], h, bevel=0.01)
        cyl("mg_barrel", 0.022, 0.75, (cx + 0.85, cy, mg_z), "X", m["dark"], h, seg=8, lods=NEAR)
    VP.hatch("gunner_hatch", (cx, -cy, top), m, h, radius=0.27)
    VP.sight_housing("gunner_sight", (front - 0.70, half_w - 0.55, top - 0.02), m, h, size=(0.36, 0.26, 0.26))
    for side, sd in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"smoke_{sd}", (front - 0.55, side * (half_w - 0.05), deck + height * 0.70), m, h,
                                 count=4, tube_radius=0.045, tube_length=0.20, elevation=0.4, spread=0.4,
                                 rot=(0, 0, side * 0.9))
        for k in range(2):
            VP.stowage_box(f"turret_box_{sd}_{k}", (rear + 0.9 + k * 1.05, side * (half_w + 0.12), deck + 0.25),
                           (0.95, 0.22, 0.55), m, h, rot=(0, 0, 0 if side > 0 else math.pi))
        whip = empty(f"dressing_antenna_{sd}", parent=h)
        VP.antenna(f"antenna_{sd}", (rear + 0.35, side * (half_w - 0.30), top), m, whip, height=2.2)
        stencil(f"turret_number_{sd}", s["label"], 0.24, (rear + 1.6, side * (half_w + 0.004), deck + height * 0.75),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], h)
    VP.bolted_panel("turret_rear_door", (rear - 0.02, 0, deck + 0.35), (0.05, 0.95, 0.85), m, h, bolts=(1, 3),
                    bevel=0.015, lods=VP.ALL)
    VP.tarp_roll("turret_roll", (rear + 0.45, 0, top + 0.12), 1.5, 0.13, m, h, straps=3)
    turret_fittings(v, rear, front, half_w, deck, top, gun_z, start, bore_r)


def turret_fittings(v, rear, front, half_w, deck, top, gun_z, start, bore_r):
    """What every one of these turrets carries beyond its shape, as the
    photos show: the mantlet's bolts and the cradle's recoil cylinders over
    and under the barrel's root, the roof's ventilator domes and access
    plates, a side periscope and the ammunition loading hatch low on each
    flank toward the rear, grab rails along the roof's edges, the rear
    door's hinges and handle, and the climbing rungs beside it."""
    m, h = v.mats, v.hull
    for k in range(4):
        for side in (-1, 1):
            cyl(f"mantlet_bolt_{side}_{k}", 0.025, 0.03, (front + 0.385, side * 0.28, gun_z - 0.20 + k * 0.13), "X",
                m["steel"], h, seg=6, lods=FINE)
    for k, dz in enumerate((bore_r * 2.6, -bore_r * 2.6)):
        cyl(f"recoil_cylinder_{k}", bore_r * 0.85, 0.80, (start + 0.40, 0, gun_z + dz), "X", m["dark"], h, seg=14,
            lods=MID)
    for k, (x, y) in enumerate(((rear + 0.55, -0.30), (rear + 0.95, 0.35))):
        cyl(f"ventilator_{k}", 0.16, 0.10, (x, y, top + 0.05), "Z", m["paint"], h, seg=16, r2=0.12, bevel=0.02,
            lods=MID)
        cyl(f"ventilator_cap_{k}", 0.10, 0.03, (x, y, top + 0.115), "Z", m["dark"], h, seg=14, lods=NEAR)
    box("roof_access_plate", (0.70, 0.60, 0.03), ((rear + front) / 2 - 0.2, 0.0, top + 0.015), m["paint"], h,
        bevel=0.01, lods=MID)
    for side, sd in ((1, "L"), (-1, "R")):
        VP.periscope(f"side_periscope_{sd}", (front - 1.10, side * (half_w - 0.20), top), m, h,
                     rot=(0, 0, side * math.pi / 2))
        VP.bolted_panel(f"loading_hatch_{sd}", (rear + 0.55, side * (half_w + 0.004), deck + 0.50),
                        (0.55, 0.45, 0.03), m, h, bolts=(2, 2), rot=(-side * math.pi / 2, 0, 0), bevel=0.01)
        box(f"grab_rail_{sd}", (1.6, 0.03, 0.03), ((rear + front) / 2 - 0.3, side * (half_w - 0.12), top + 0.07),
            m["steel"], h, lods=NEAR)
        for k, x in enumerate(((rear + front) / 2 - 1.0, (rear + front) / 2 + 0.4)):
            box(f"grab_post_{sd}_{k}", (0.03, 0.03, 0.07), (x, side * (half_w - 0.12), top + 0.035), m["steel"], h,
                lods=FINE)
        box(f"door_hinge_{sd}", (0.06, 0.06, 0.16), (rear - 0.05, side * 0.50, deck + 0.55), m["steel"], h, lods=FINE)
        for k in range(3):
            box(f"rear_rung_{sd}_{k}", (0.04, 0.30, 0.03), (rear - 0.06, side * (half_w - 0.35),
                                                            deck + 0.30 + k * 0.30), m["steel"], h, lods=FINE)
    box("door_handle", (0.05, 0.16, 0.04), (rear - 0.06, -0.30, deck + 0.70), m["steel"], h, lods=FINE)


def build(variant, v):
    hull(v)
    running_gear(v)
    turret(v)


def wreck(variant, v):
    """A howitzer after its ammunition burned: the turret's roof blown up and
    its rear door gone, the barrel drooping onto the travel lock, the left
    track off with a road wheel gone, side boxes torn away, skirts bent,
    plates warped, the propellant's debris round it."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, remove, warp
    s = spec(v)
    m = v.mats
    half = v.length / 2
    rear, front, width, height = s["turret"]
    remove("track_L_band", "wheel_L_2_", "turret_box_L_", "turret_rear_door", "turret_roll", "gunner_hatch",
           "skirt_L_1_", "skirt_L_2_", "turret_number_L")
    gun_x, gun_z, length, _ = s["gun"]
    bend(parts("gun_", "fume_extractor", "muzzle_", "brake_"), (front + 0.4, 0, gun_z), (0, 1, 0), (0, 0, 1),
         -0.07)
    bend(parts("skirt_L_3_"), (0, v.width / 2, 1.1), (1, 0, 0), (0, 0, -1), 0.5)
    thrown = solid("thrown_track", (3.6, TRACK_W, 0.05), (0.2, v.width / 2 + 0.55, 0.03), m["track"], v.hull,
                   rot=(0, 0, -0.06))
    densify(thrown)
    warp(thrown, heat(0.03, 0.6, seed=5.0))
    shell = parts("hull_upper", "hull_lower", "turret_shell", "skirt_")
    densify(shell, scale=2.0)
    top = s["deck"] + height
    warp(shell, heat(0.025, 0.9, seed=61.0), dent(((rear + front) / 2, 0, top + 0.3), 1.2, -0.18, (0, 0, 1)))
    rest_on_ground(0.004)


if __name__ == "__main__":
    run_disabled("sp_howitzers", {c: s["dims"] for c, s in SPECS.items()},
                 {c: s["scheme"] for c, s in SPECS.items()}, build, wreck, skip=("dressing_", "gun_", "fume_", "muzzle_", "brake_"), chip=1.0)
