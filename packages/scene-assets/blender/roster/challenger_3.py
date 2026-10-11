"""Challenger 3, from assets/references/challenger_3/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/challenger_3.py -- [--wreck]

No Challenger 3 photo is under an allowed licence, so the library is the
Challenger 2 TES photos of the hull it keeps and two generated views of the
new turret (labelled as such). What they settle: the Challenger 2's hull,
six road wheels a side with the sprocket at the rear, deep skirts in bolted
appliqué panels, a short glacis with the driver's hatch at its head, the
engine deck behind the turret; the new turret is angular and slab-sided,
its front a sloped wedge, the 120 mm L55A1 smoothbore without fume
extractor, a Trophy radar panel and launcher on each side, the commander's
panoramic sight on its right, a remote machine gun station, a slatted
bustle cage. British green.

Built to the catalog frame, the Challenger 2's (hull 8.3 x 4.2 x 2.49 m, gun
pivot 0.2 m ahead of the hull's middle and 1.494 m up): the generated views
put the turret roof near 2.5 m too. The turret is built at the frame's pivot.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

TRACK_Y = 1.55
TRACK_W = 0.62
ROAD_R = 0.39
ROAD_Z = 0.47
DECK = 1.47
SKIRT_Y = 2.02
TRUNNION = 1.05


def hull(v):
    m, h = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 0.86), (half - 0.40, 0.86), (half, 1.12), (half - 1.30, DECK), (-half + 0.06, DECK),
                         (-half, DECK - 0.10)], 3.50, mat=m["paint"], parent=h, bevel=0.05)
    prism("hull_lower", [(-half + 0.35, 0.44), (half - 0.85, 0.44), (half - 0.10, 0.90), (-half, 0.90),
                         (-half, 0.72)], 2.40, mat=m["paint"], parent=h, bevel=0.04)
    slope = math.atan((DECK - 1.12) / 1.30)
    VP.bolted_panel("glacis_plate", (half - 0.65, 0, 1.12 + (DECK - 1.12) * 0.5 - 0.02), (1.10, 3.0, 0.06), m, h,
                    bolts=(2, 5), bevel=0.02, rot=(0, slope, 0), lods=VP.ALL)
    VP.hatch("driver_hatch", (half - 1.55, 0, DECK), m, h, radius=0.28)
    VP.periscope("driver_periscope", (half - 1.25, 0, DECK), m, h, size=(0.12, 0.28, 0.09))
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{s}", (half - 1.10, side * 1.45, DECK + 0.08), 0.07, m, h)
        VP.tow_hook(f"front_tow_{s}", (half - 0.18, side * 0.75, 0.85), m, h, size=0.14)
        VP.tow_hook(f"rear_tow_{s}", (-half + 0.02, side * 0.85, 0.85), m, h, size=0.08, rot=(0, 0, math.pi))
        # Deep skirts of bolted appliqué panels.
        for k in range(6):
            length = (v.length - 0.5) / 6
            x = half - 0.30 - length * (k + 0.5)
            VP.bolted_panel(f"skirt_{s}_{k}", (x, side * (SKIRT_Y - 0.07), 0.70), (length - 0.04, 0.14, 0.86), m, h,
                            bolts=(2, 2), bevel=0.03, lods=VP.ALL)
        box(f"skirt_rail_{s}", (v.length - 0.3, 0.10, 0.06), (0, side * (SKIRT_Y - 0.12), DECK - 0.02), m["paint"], h,
            bevel=0.02, lods=MID)
        VP.stowage_box(f"fender_box_{s}", (-2.6, side * 1.70, DECK), (1.20, 0.40, 0.35), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        VP.cable(f"tow_cable_{s}", [(-half + 0.4, side * 1.62, DECK + 0.02), (-1.2, side * 1.66, DECK + 0.02),
                                    (0.6, side * 1.62, DECK + 0.02)], m, h, radius=0.022)
        stencil(f"side_marking_{s}", "C3", 0.20, (half - 0.9, side * (SKIRT_Y + 0.004), 1.0),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], h)
    VP.grille("engine_grille", (-2.95, 0, DECK), (1.40, 2.20), m, h, slats=12)
    VP.bolted_panel("engine_door", (-1.75, 0, DECK), (0.80, 2.20, 0.03), m, h, bolts=(2, 3), bevel=0.01, lods=MID)
    VP.slat_armour("rear_bars", (-half + 0.03, 0, 0.75), (3.30, 0.75), m, h, rot=(0, 0, math.pi / 2))
    # The photos' armour and stowage beyond the hull: the front skirt
    # sections' extra armour blocks, spare track links across the nose,
    # jerrycans in their rack at the rear.
    for side, s in ((1, "L"), (-1, "R")):
        VP.armour_tiles(f"skirt_armour_{s}", (half - 1.75, side * SKIRT_Y, 0.78), (2.70, 0.62), (4, 2), 0.07, m, h,
                        rot=(-side * math.pi / 2, 0, 0))
        for k in range(2):
            VP.jerrycan(f"jerrycan_{s}_{k}", (-half + 0.40, side * (0.95 + k * 0.38), DECK), dict(m, paint=m["dark"]), h,
                        rot=(0, 0, math.pi / 2))
    for k in range(5):
        cyl(f"spare_link_{k}", 0.05, 0.55, (half - 0.20, -1.20 + k * 0.60, 1.02), "Y", m["track"], h, seg=8,
            lods=NEAR)
    # The rear: its tail lights in their guards.
    for side, sd in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"tail_light_{sd}", (-v.length / 2 - 0.02, side * (v.width / 2 - 0.7), DECK - 0.15), 0.05,
                            dict(v.mats, lamp=v.mats["tail"]), v.hull, rot=(0, 0, math.pi))

def running_gear(v):
    road_x = [2.55 - 0.98 * k for k in range(6)]
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, road_x, ROAD_Z, ROAD_R, 0.24, (-3.55, 0.70, 0.33),
                            (3.55, 0.66, 0.31), [(1.6, 0.98, 0.10), (0.0, 1.0, 0.10), (-1.6, 0.98, 0.10)], bolts=8,
                            teeth=12, arm=(0.46, -0.32), pitch=0.16, dual=True)


def turret(v, t, gun, hmg, hmg_gun):
    m = v.mats
    base = DECK - v.frame["mounts"][0]["pivot_m"][2]
    # The tall slab-sided turret: its roof at the hull box's top.
    top = base + 0.98
    foot = [(1.85, 0.55), (1.45, 1.55), (-1.70, 1.60), (-2.10, 1.30), (-2.10, -1.30), (-1.70, -1.60), (1.45, -1.55),
            (1.85, -0.55)]
    # Slab sides near vertical; only the front wedge slopes back.
    crown = [(1.20, 0.50), (1.00, 1.48), (-1.66, 1.54), (-2.06, 1.26), (-2.06, -1.26), (-1.66, -1.54), (1.00, -1.48),
             (1.20, -0.50)]
    cyl("turret_ring_guard", 1.15, 0.08, (0, 0, base + 0.02), "Z", m["dark"], t, seg=40, lods=MID)
    loft("turret_shell", [(base + 0.04, foot), (base + 0.55, [(x * 0.99, y) for x, y in foot]), (top, crown)],
         mat=m["paint"], parent=t, bevel=0.04)
    VP.roof_fittings("roof", crown, top, m, t, periscopes=((0.10, -1.05, -0.4),))
    VP.laser_warners("laser_warner", crown, top, m, t)
    for side, s in ((1, "L"), (-1, "R")):
        # The cheeks' armour packs and the stowage bins down the flank's rear.
        VP.armour_tiles(f"cheek_armour_{s}", (1.05, side * 1.10, base + 0.45), (0.50, 0.70), (1, 2), 0.07, m, t,
                        rot=(0, math.pi / 2 - 0.20, side * -0.45))
        for k in range(2):
            VP.stowage_box(f"turret_bin_{s}_{k}", (-1.25 - k * 0.62, side * 1.56, base + 0.20), (0.58, 0.30, 0.48), m,
                           t, rot=(0, 0, 0 if side > 0 else math.pi))
    for side, s in ((1, "L"), (-1, "R")):
        # Trophy: the radar panel and the launcher housing on each side. Their
        # nodes (`trophy_radar_*`, `trophy_launcher_*`) are the part's.
        radar = empty(f"trophy_radar_{s}", (0.20, side * 1.58, base + 0.55), t, rot=(0, 0, side * -0.25))
        box(f"trophy_panel_{s}", (0.50, 0.08, 0.40), (0, 0, 0), m["dark"], radar, bevel=0.02)
        holder = empty(f"dressing_trophy_{s}", parent=t)
        launcher = empty(f"trophy_launcher_{s}", (-0.70, side * 1.48, top + 0.05), holder)
        box(f"trophy_housing_{s}", (0.40, 0.36, 0.30), (0, 0, 0), m["paint"], launcher, bevel=0.04)
        VP.bolted_panel(f"side_applique_{s}", (-0.60, side * 1.58, base + 0.15), (1.60, 0.10, 0.55), m, t,
                        bolts=(4, 1), bevel=0.02, lods=VP.ALL)
        VP.smoke_discharger_bank(f"smoke_{s}", (0.55, side * 1.30, top - 0.08), m, t, count=5, tube_radius=0.045,
                                 tube_length=0.20, elevation=0.35, spread=0.5, rot=(0, 0, side * 0.9))
        whip = empty(f"dressing_antenna_{s}", parent=t)
        VP.antenna(f"antenna_{s}", (-1.75, side * 0.95, top), m, whip, height=2.2)
    sight = empty("dressing_gunner_sight", parent=t)
    VP.sight_housing("gunner_sight", (0.55, 0.75, top - 0.02), m, sight, size=(0.45, 0.34, 0.24))
    mast = empty("dressing_commander_sight", parent=t)
    cyl("panorama_post", 0.10, 0.04, (-0.35, 0.55, top + 0.02), "Z", m["dark"], mast, seg=12)
    VP.sight_housing("panorama_head", (-0.35, 0.55, top + 0.02), m, mast, size=(0.36, 0.34, 0.26))
    VP.hatch("loader_hatch", (-0.60, -0.55, top), m, t, radius=0.28)
    VP.slat_armour("bustle_cage", (-2.35, 0, base + 0.15), (2.40, 0.62), m, t, rot=(0, 0, math.pi / 2))
    for side, s in ((1, "L"), (-1, "R")):
        VP.slat_armour(f"bustle_cage_{s}", (-2.05, side * 1.20, base + 0.15), (0.60, 0.62), m, t)
    VP.tarp_roll("bustle_roll", (-2.05, 0, top - 0.14), 1.6, 0.16, m, t, straps=3)
    # The L55A1: mantlet, plain sleeve with clamps, no fume extractor.
    reach = v.frame["mounts"][0]["muzzle_m"][0] - TRUNNION
    box("mantlet", (0.75, 0.62, 0.46), (0.35, 0, 0), m["paint"], gun, bevel=0.06)
    cyl("barrel_root", 0.14, 0.50, (0.90, 0, 0), "X", m["paint"], gun, seg=24)
    cyl("thermal_sleeve", 0.095, reach - 1.15, ((reach + 1.05) / 2, 0, 0), "X", m["paint"], gun, seg=24)
    for k in range(5):
        cyl(f"sleeve_clamp_{k}", 0.105, 0.05, (1.3 + k * (reach - 1.8) / 4, 0, 0), "X", m["dark"], gun, seg=20,
            lods=NEAR)
    box("muzzle_sensor", (0.12, 0.08, 0.10), (reach - 0.35, 0, 0.13), m["dark"], gun, lods=NEAR)
    cyl("muzzle_end", 0.088, 0.20, (reach - 0.10, 0, 0), "X", m["steel"], gun, seg=24)
    cyl("muzzle_bore", 0.062, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=20, lods=MID)
    # The remote machine gun station.
    lift = v.frame["mounts"][1]["muzzle_m"][2]
    cyl("rws_bearing", 0.20, 0.08, (0, 0, -0.04), "Z", m["dark"], hmg, seg=20)
    box("rws_body", (0.42, 0.32, lift), (-0.05, 0, lift / 2), m["paint"], hmg, bevel=0.03)
    VP.sight_housing("rws_sight", (0.05, -0.24, -0.10), m, hmg_gun, size=(0.24, 0.14, 0.20))
    VP.browning_m2(hmg_gun, v.frame["mounts"][1]["muzzle_m"][0], m)


def build(variant, v):
    hull(v)
    running_gear(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    t, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret(v, t, gun, hmg, hmg_gun)


def wreck(variant, v):
    """The Challenger 3 after its fire: the right track off and two road
    wheels gone, skirt panels blown away and one bent out, the fuel drums
    gone, the bustle cage crushed, plates warped and the glacis dented; the
    turret is thrown."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, remove, warp
    m = v.mats
    remove("track_R_band", "wheel_R_3_", "wheel_R_4_", "skirt_R_2_", "skirt_R_3_", "fender_box_R",
           "side_marking_R", "bustle_roll", "loader_hatch")
    thrown = solid("thrown_track", (4.0, TRACK_W, 0.05), (-0.3, -2.45, 0.03), m["track"], v.hull, rot=(0, 0, 0.06))
    densify(thrown)
    warp(thrown, heat(0.035, 0.6, seed=4.0))
    for k, (loc, rot) in enumerate((((0.8, -2.95, 0.10), (1.5, 0.1, 0.3)), ((-0.8, -3.1, 0.10), (1.45, -0.2, -0.4)))):
        solid(f"fallen_skirt_{k}", (1.25, 0.14, 0.86), loc, m["paint"], v.hull, rot=rot, bevel=0.02)
    bend(parts("skirt_R_1_"), (1.6, -1.95, 1.13), (1, 0, 0), (0, 0, -1), -0.6)
    bend(parts("bustle_cage"), (v.frame["mounts"][0]["pivot_m"][0] - 2.35, 0, 1.4), (0, 1, 0), (1, 0, 0), 0.3)
    shell = parts("hull_upper", "hull_lower", "skirt_", "turret_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.022, 0.9, seed=51.0), dent((3.6, 0.5, 1.2), 0.5, 0.10, (-0.6, 0, -1)))
    rest_on_ground(0.004)


run("challenger_3", "british_green", build, wreck, chip=1.0)
