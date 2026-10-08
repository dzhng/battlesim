"""BRM-3K Rys reconnaissance vehicle (disabled card), from assets/references/brm/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/brm.py -- [--wreck]

What the photos settle (Russia Arms Expo 2013, the Rubtsovsk plant's
monument): the BMP-3 hull. Six road wheels a side, the drive sprocket at the
rear and the idler at the front, return rollers over them; the long low
glacis with the folded trim vane across the nose and headlights in its
corners; the flat roof stepping up to the engine deck at the rear, two rear
doors with stowage beside them; a rubber skirt over the track's top run. The
two-man turret carries the 30 mm 2A72 and coax on the centre line, a sight
box each side of the gun, smoke tubes on the cheeks and the 1RL-133 radar's
folded housing on the turret's rear. The model wears Russian green (the 2013
car's desert scheme was an exhibition finish).

Frame 7.14 x 3.2 x 2.4 m, gun pivot 1.56 m (`DIMENSIONS`, `MOUNTS`).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism  # noqa: E402
from vehicle_export import rig, run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

CARD = "eastern_brm_brm_3k"
# The frame its references give (a disabled card has no unit type): the
# hull box, and the mounts its turret and guns are rigged on for the art.
DIMENSIONS = (7.14, 3.2, 2.4)
MOUNTS = [
    dict(name="autocannon", role="gun", on=None, pivot_m=[0.0, 0.0, 1.56], muzzle_m=[2.999, 0.0, 0.432]),
]
TRACK_Y = 1.30
TRACK_W = 0.38
ROAD_R = 0.34
ROAD_Z = 0.42
ROOF = 1.56
DECK = 1.70


def hull(v):
    m, h = v.mats, v.hull
    half = v.length / 2
    nose = 0.95
    prism("hull_upper", [(-half, 0.80), (half - 0.25, 0.80), (half, nose), (half - 1.90, ROOF), (-1.10, ROOF),
                         (-1.00, DECK), (-half + 0.05, DECK), (-half, DECK - 0.12)], 3.0, mat=m["paint"], parent=h,
          bevel=0.05)
    prism("hull_lower", [(-half + 0.30, 0.42), (half - 0.65, 0.42), (half - 0.05, 0.82), (-half, 0.82),
                         (-half, 0.65)], 2.30, mat=m["paint"], parent=h, bevel=0.04)
    slope = math.atan((ROOF - nose) / 1.90)
    # The folded trim vane across the nose, its ribbed face up.
    VP.bolted_panel("trim_vane", (half - 0.45, 0, nose + (ROOF - nose) * 0.2), (0.60, 2.70, 0.06), m, h, bolts=(2, 6),
                    bevel=0.02, rot=(0, slope, 0), lods=VP.ALL)
    for k in range(5):
        box(f"vane_rib_{k}", (0.55, 0.04, 0.05), (half - 0.45, -1.1 + k * 0.55, nose + (ROOF - nose) * 0.2 + 0.08),
            m["paint"], h, rot=(0, slope, 0), lods=NEAR)
    VP.hatch("driver_hatch", (half - 2.10, 0, ROOF), m, h, radius=0.25)
    for k in range(3):
        VP.periscope(f"driver_periscope_{k}", (half - 1.80, -0.25 + k * 0.25, ROOF - 0.04), m, h,
                     size=(0.11, 0.16, 0.08), rot=(0, slope, 0))
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{s}", (half - 1.30, side * 1.25, nose + 0.45), 0.06, m, h)
        VP.hatch(f"bow_gunner_hatch_{s}", (half - 2.10, side * 0.85, ROOF), m, h, radius=0.22)
        VP.tow_hook(f"front_tow_{s}", (half - 0.02, side * 0.65, 0.75), m, h, size=0.12)
        # The rubber skirt over the track's top run, in sections.
        for k in range(5):
            x = half - 0.80 - k * 1.15
            box(f"rubber_skirt_{s}_{k}", (1.10, 0.03, 0.30), (x, side * 1.52, 0.86), m["rubber"], h, lods=MID)
        box(f"fender_{s}", (v.length - 0.5, 0.40, 0.04), (-0.1, side * 1.38, 0.99), m["paint"], h, bevel=0.01,
            lods=MID)
        VP.stowage_box(f"rear_box_{s}", (-half + 0.35, side * 1.15, DECK), (0.50, 0.40, 0.30), m, h)
        VP.cable(f"tow_cable_{s}", [(-half + 0.8, side * 1.45, 1.05), (-1.0, side * 1.47, 1.05),
                                    (0.8, side * 1.45, 1.05)], m, h, radius=0.018)
    VP.grille("engine_grille", (-2.30, 0.45, DECK), (1.0, 0.75), m, h, slats=8)
    VP.grille("radiator_grille", (-2.30, -0.45, DECK), (1.0, 0.75), m, h, slats=8)
    for side in (-1, 1):
        VP.bolted_panel(f"rear_door_{side}", (-half - 0.03, side * 0.40, 0.95), (0.05, 0.62, 0.70), m, h,
                        bolts=(1, 2), bevel=0.015, lods=VP.ALL)
    for k, y in enumerate((0.55, -0.55)):
        VP.hatch(f"roof_hatch_{k}", (-1.65, y, DECK), m, h, size=(0.70, 0.55))
    # What the photos show carried outside: stowage bins along both fenders,
    # spare track links across the nose, the unditching log on the rear plate,
    # an entrenching tool and crowbar clipped on the right fender.
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate((0.55, -0.55, -1.65)):
            VP.stowage_box(f"fender_bin_{s}_{k}", (x, side * 1.38, 1.01), (0.95, 0.34, 0.30), m, h,
                           rot=(0, 0, 0 if side > 0 else math.pi))
    for k in range(4):
        cyl(f"spare_link_{k}", 0.045, 0.52, (half - 0.12, -0.85 + k * 0.57, 0.86), "Y", m["track"], h, seg=8,
            lods=NEAR)
    cyl("unditching_log", 0.11, 2.30, (-half - 0.12, 0, DECK - 0.18), "Y", m["canvas"], h,
        seg=12, bevel=0.02)
    for side in (-1, 1):
        box(f"log_strap_{side}", (0.04, 0.05, 0.26), (-half - 0.12, side * 0.80, DECK - 0.18), m["dark"], h,
            lods=NEAR)
    box("shovel_blade", (0.24, 0.02, 0.18), (1.80, -1.53, 1.10), m["steel"], h, lods=NEAR)
    box("shovel_handle", (0.60, 0.025, 0.035), (1.40, -1.53, 1.10), m["canvas"], h, lods=NEAR)
    box("crowbar", (1.10, 0.03, 0.03), (1.20, -1.53, 1.20), m["steel"], h, lods=NEAR)
    # The rear: its tail lights in their guards and tow hooks.
    for side, sd in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"tail_light_{sd}", (-v.length / 2 - 0.02, side * (v.width / 2 - 0.45), ROOF - 0.25), 0.05,
                            dict(v.mats, lamp=v.mats["tail"]), v.hull, rot=(0, 0, math.pi))
        VP.tow_hook(f"rear_tow_{sd}", (-v.length / 2 + 0.02, side * 0.75, 0.85), v.mats, v.hull, size=0.13,
                    rot=(0, 0, math.pi))

def running_gear(v):
    road_x = [2.30 - 0.90 * k for k in range(6)]
    returns = [(1.85, 0.86, 0.09), (0.55, 0.88, 0.09), (-0.75, 0.88, 0.09), (-2.05, 0.86, 0.09)]
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, road_x, ROAD_Z, ROAD_R, 0.16, (-3.20, 0.62, 0.27),
                            (3.10, 0.60, 0.26), returns, bolts=6, teeth=12, arm=(0.38, -0.30), pitch=0.14,
                            dual=True)


def turret(v, turret, gun):
    m = v.mats
    base = ROOF - v.frame["mounts"][0]["pivot_m"][2]
    top = base + 0.62
    foot = [(1.05, 0.40), (0.85, 0.98), (-0.90, 1.02), (-1.10, 0.75), (-1.10, -0.75), (-0.90, -1.02), (0.85, -0.98),
            (1.05, -0.40)]
    crown = [(0.75, 0.36), (0.55, 0.85), (-0.80, 0.90), (-1.00, 0.66), (-1.00, -0.66), (-0.80, -0.90), (0.55, -0.85),
             (0.75, -0.36)]
    cyl("turret_ring_guard", 0.95, 0.08, (0, 0, base + 0.02), "Z", m["dark"], turret, seg=36, lods=MID)
    loft("turret_shell", [(base + 0.04, foot), (top, crown)], mat=m["paint"], parent=turret, bevel=0.035)
    VP.roof_fittings("roof", crown, top, m, turret, periscopes=((0.30, 0.45, 0.3), (0.30, -0.45, -0.3)))
    for side, s in ((1, "L"), (-1, "R")):
        VP.sight_housing(f"sight_{s}", (0.45, side * 0.62, top - 0.02), m, turret, size=(0.38, 0.28, 0.30))
        VP.smoke_discharger_bank(f"smoke_{s}", (0.55, side * 0.90, base + 0.30), m, turret, count=3,
                                 tube_radius=0.045, tube_length=0.20, elevation=0.4, spread=0.3,
                                 rot=(0, 0, side * 0.9))
        VP.hatch(f"turret_hatch_{s}", (-0.30, side * 0.45, top), m, turret, radius=0.24)
        VP.stowage_box(f"turret_bin_{s}", (-0.55, side * 1.02, base + 0.12), (0.60, 0.22, 0.36), m, turret,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-0.85, side * 0.62, top), m, whip, height=2.2)
    # The IR searchlight in its guard on the turret's left front.
    VP.light_with_guard("ir_searchlight", (0.75, 0.82, top + 0.10), 0.11, dict(m, lamp=m["glass"]), turret)
    # The 1RL-133 radar folded in its housing on the turret rear.
    radar = empty("dressing_radar", parent=turret)
    box("radar_housing", (0.45, 1.00, 0.30), (-1.10, 0, top - 0.05), m["paint"], radar, bevel=0.04)
    box("radar_face", (0.02, 0.90, 0.22), (-1.33, 0, top - 0.05), m["dark"], radar, lods=MID)
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("gun_mantlet", (0.50, 0.40, 0.32), (0.55, 0, 0), m["paint"], gun, bevel=0.04)
    cyl("cannon_sleeve", 0.07, 0.55, (1.05, 0, 0), "X", m["dark"], gun, seg=16)
    cyl("cannon_barrel", 0.035, reach - 1.30, ((reach + 1.30) / 2, 0, 0), "X", m["steel"], gun, seg=14)
    cyl("cannon_muzzle", 0.050, 0.18, (reach - 0.09, 0, 0), "X", m["dark"], gun, seg=14)
    cyl("cannon_bore", 0.022, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=NEAR)
    cyl("coax_barrel", 0.018, 0.40, (0.95, 0.18, 0.0), "X", m["steel"], gun, seg=8, lods=NEAR)


def build(variant, v):
    hull(v)
    running_gear(v)
    t, gun, _, _ = rig(v.frame, v.root)["autocannon"]
    turret(v, t, gun)


def wreck(variant, v):
    """The BRM after its fire: the left track off with its front road wheel
    gone, the trim vane blown loose and hanging, rubber skirts burnt away, the
    rear doors open, the roof hatches gone; the turret is thrown."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, remove, warp
    m = v.mats
    remove("track_L_band", "wheel_L_1_", "rubber_skirt_", "roof_hatch_", "rear_box_L")
    thrown = solid("thrown_track", (3.0, TRACK_W, 0.05), (1.0, 1.95, 0.03), m["track"], v.hull, rot=(0, 0, -0.1))
    densify(thrown)
    warp(thrown, heat(0.03, 0.6, seed=3.0))
    bend(parts("trim_vane", "vane_rib_"), (v.length / 2 - 0.2, 0, 1.0), (0, 1, 0), (1, 0, 0), -0.5)
    bend(parts("rear_door_1"), (-v.length / 2, 0.70, 0.95), (0, 0, 1), (1, 0, 0), -1.1)
    shell = parts("hull_upper", "hull_lower", "turret_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.02, 0.8, seed=31.0), dent((1.2, 1.4, 1.1), 0.45, 0.10, (0, -1, -0.2)))
    rest_on_ground(0.004)


if __name__ == "__main__":
    run_disabled("brm", {CARD: DIMENSIONS}, "russian_green", build, wreck, mounts={CARD: MOUNTS},
                 skip=("dressing_", "gun", "hmg"), chip=1.0)
