"""Challenger 3, from assets/references/challenger_3/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/challenger_3.py -- [--wreck]

No Challenger 3 photo is under an allowed licence, so the library is the
Challenger 2 TES photos of the hull it keeps and two generated views of the
new turret (labelled as such). What they settle: the Challenger 2's hull,
six road wheels a side with the sprocket at the rear, deep skirts in bolted
appliqué panels, a shallow upper glacis (the driver's hatch at its head)
over a short upright nose plate and a steep lower glacis, the
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
HULL_Y = 1.75  # the hull box's side
SKIRT_T = 0.06
SKIRT_TOP = DECK + 0.03
RISE_X = 2.72  # where the front skirt's lower edge starts to climb
BOX_BACK, BOX_FRONT, BOX_FOOT = 3.30, 4.12, 1.17  # the nose fender boxes
# The upper glacis: from its head, GLACIS_RUN back of the nose, it falls
# GLACIS_DROP to the nose plate, about 11 degrees side-on.
GLACIS_RUN, GLACIS_DROP = 1.70, 0.33


def glacis_z(x, half):
    """The hull top's height at x: the deck, then the glacis falling to the
    nose."""
    return DECK - max(0.0, x - (half - GLACIS_RUN)) / GLACIS_RUN * GLACIS_DROP


def skirt_top(x, half):
    """The skirts' and fender boxes' top at x: level on the deck's line, then
    falling with the glacis toward the nose."""
    return min(SKIRT_TOP, glacis_z(x, half) + 0.05)


def box_foot(x):
    """The fender boxes' underside: level over the track's top run, then
    stepping down ahead of the idler round the track's front."""
    return BOX_FOOT - max(0.0, min(1.0, (x - 3.66) / 0.32)) * (BOX_FOOT - 0.95)


def skirt_foot(x):
    """The front skirt's lower edge: level to RISE_X, then climbing with the
    track toward the idler."""
    return 0.70 + max(0.0, x - RISE_X) * (1.10 - 0.70) / (3.95 - RISE_X)


NOSE_TOP, NOSE_FOOT = DECK - GLACIS_DROP, 1.04  # the nose plate's short upright face
LOWER_FOOT_X = 0.80  # how far back of the nose the lower glacis meets the belly


def lower_glacis_x(z, half):
    """The steep lower glacis's x at height `z`: from the belly at 0.44 m,
    LOWER_FOOT_X back of the nose, up and forward to the nose plate."""
    return half - LOWER_FOOT_X + (z - 0.44) * LOWER_FOOT_X / (NOSE_FOOT - 0.44)


def hull(v):
    m, h = v.mats, v.hull
    half = v.length / 2

    def g(z):
        return lower_glacis_x(z, half)

    # The deck falls over a shallow upper glacis, then breaks over a short
    # upright nose plate onto the steep lower glacis.
    head = half - GLACIS_RUN
    slope = math.atan(GLACIS_DROP / GLACIS_RUN)
    prism("hull_upper", [(-half, 0.86), (g(0.86), 0.86), (half, NOSE_FOOT), (half, NOSE_TOP), (head, DECK),
                         (-half + 0.06, DECK), (-half, DECK - 0.10)], 3.50, mat=m["paint"], parent=h, bevel=0.05)
    prism("hull_lower", [(-half + 0.35, 0.44), (g(0.44), 0.44), (g(0.90), 0.90), (-half, 0.90),
                         (-half, 0.72)], 2.40, mat=m["paint"], parent=h, bevel=0.04)
    VP.bolted_panel("glacis_plate", (half - 0.70, 0, glacis_z(half - 0.70, half)), (0.80, 3.0, 0.03), m, h,
                    bolts=(2, 5), bevel=0.01, rot=(0, slope, 0), lods=VP.ALL)
    VP.hatch("driver_hatch", (half - 1.55, 0, glacis_z(half - 1.55, half)), m, h, radius=0.28, rot=(0, slope, 0))
    VP.periscope("driver_periscope", (half - 1.25, 0, glacis_z(half - 1.25, half)), m, h, size=(0.12, 0.28, 0.09),
                 rot=(0, slope, 0))
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{s}", (half - 1.10, side * 1.45, glacis_z(half - 1.10, half) + 0.08), 0.07,
                            m, h)
        VP.tow_hook(f"front_tow_{s}", (g(0.85) + 0.02, side * 0.75, 0.85), m, h, size=0.14)
        VP.tow_hook(f"rear_tow_{s}", (-half + 0.02, side * 0.85, 0.85), m, h, size=0.08, rot=(0, 0, math.pi))
        # The skirts: thin bolted panels, their outer face on the fender
        # line, their top flush with the fender plate that carries the deck
        # out to them, running the hull's length. The front section's top
        # falls with the glacis; its lower edge climbs with the track toward
        # the idler and runs on, as a tongue, under the fender box over the
        # track's front.
        length = (v.length - 0.5) / 6
        for k in range(1, 6):
            x = half - 0.30 - length * (k + 0.5)
            VP.bolted_panel(f"skirt_{s}_{k}", (x, side * (SKIRT_Y - SKIRT_T / 2), 0.70),
                            (length - 0.04, SKIRT_T, SKIRT_TOP - 0.70), m, h, bolts=(2, 2), bevel=0.012,
                            lods=VP.ALL)
        back = half - 0.30 - length + 0.02
        y = side * (SKIRT_Y - SKIRT_T / 2)
        knee = head + (DECK + 0.05 - SKIRT_TOP) / GLACIS_DROP * GLACIS_RUN
        tops = [(x, skirt_top(x, half)) for x in [BOX_BACK] + ([knee] if back < knee < BOX_BACK else []) + [back]]
        prism(f"skirt_{s}_0_plate", [(back, 0.70), (RISE_X, 0.70), (BOX_BACK, skirt_foot(BOX_BACK))] + tops,
              SKIRT_T, loc=(0, y, 0), mat=m["paint"], parent=h, bevel=0.012, lods=VP.ALL)
        prism(f"skirt_{s}_0_tongue", [(BOX_BACK - 0.02, skirt_foot(BOX_BACK - 0.02)), (3.80, skirt_foot(3.80)),
                                      (3.80, box_foot(3.80) + 0.01), (3.66, BOX_FOOT + 0.01),
                                      (BOX_BACK - 0.02, BOX_FOOT + 0.01)], SKIRT_T, loc=(0, y, 0),
              mat=m["paint"], parent=h, bevel=0.01, lods=VP.ALL)
        # Bolt rows along each panel's top and foot on its outer face.
        spans = [(half - 0.30 - length * (k + 1) + 0.02, half - 0.30 - length * k - 0.02) for k in range(1, 6)]
        for k, (x0, x1) in enumerate(spans + [(back, BOX_BACK)]):
            n = 4
            for j in range(n):
                bx = x0 + 0.10 + (x1 - x0 - 0.20) * j / (n - 1)
                for r, bz in enumerate((skirt_top(bx, half) - 0.07, skirt_foot(bx) + 0.06)):
                    cyl(f"skirt_{s}_bolt_{k}_{j}_{r}", 0.022, 0.024, (bx, side * SKIRT_Y, bz), "Y", m["steel"], h,
                        seg=6, lods=FINE)
        # The fender plate from the hull side out to the skirts' top.
        inner = HULL_Y - 0.05
        st = skirt_top(BOX_BACK, half)
        prism(f"skirt_top_{s}", [(-half + 0.06, SKIRT_TOP - 0.04), (knee, SKIRT_TOP - 0.04), (BOX_BACK, st - 0.04),
                                 (BOX_BACK, st), (knee, SKIRT_TOP), (-half + 0.06, SKIRT_TOP)], SKIRT_Y - inner,
              loc=(0, side * (SKIRT_Y + inner) / 2, 0), mat=m["paint"], parent=h, bevel=0.01, lods=MID)
        VP.stowage_box(f"fender_box_{s}", (-2.6, side * 1.70, DECK), (1.20, 0.40, 0.35), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        VP.cable(f"tow_cable_{s}", [(-half + 0.4, side * 1.62, DECK + 0.02), (-1.2, side * 1.66, DECK + 0.02),
                                    (0.6, side * 1.62, DECK + 0.02)], m, h, radius=0.022)
        stencil(f"side_marking_{s}", "C3", 0.20, (half - 1.2, side * (SKIRT_Y + 0.004), 1.0),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], h)
    VP.grille("engine_grille", (-2.95, 0, DECK), (1.40, 2.20), m, h, slats=12)
    VP.bolted_panel("engine_door", (-1.75, 0, DECK), (0.80, 2.20, 0.03), m, h, bolts=(2, 3), bevel=0.01, lods=MID)
    VP.slat_armour("rear_bars", (-half + 0.03, 0, 0.75), (3.30, 0.75), m, h, rot=(0, 0, math.pi / 2))
    # The photos' fittings beyond the hull: the fender boxes over the
    # tracks' front, spare track links across the nose, jerrycans in their
    # rack at the rear.
    for side, s in ((1, "L"), (-1, "R")):
        # The fender boxes over the tracks' front, as the photos show them:
        # boxes as wide as track and skirt, their side flush with the skirts,
        # standing on the glacis front corners beside the headlights, their
        # top falling with the glacis, their underside stepping down round
        # the track's front, a rubber flap hanging from the front.
        lo, hi = TRACK_Y - TRACK_W / 2 - 0.01, SKIRT_Y
        prism(f"nose_box_{s}", [(BOX_BACK, BOX_FOOT), (3.66, BOX_FOOT), (3.98, box_foot(3.98)),
                                (BOX_FRONT, box_foot(BOX_FRONT)), (BOX_FRONT, skirt_top(BOX_FRONT, half)),
                                (BOX_BACK, skirt_top(BOX_BACK, half))], hi - lo, loc=(0, side * (lo + hi) / 2, 0),
              mat=m["paint"], parent=h, bevel=0.025, lods=VP.ALL)
        prism(f"nose_cowl_{s}", [(BOX_FRONT - 0.10, 0.80), (half - 0.01, 0.80), (half - 0.01, 0.97),
                                 (BOX_FRONT - 0.10, 0.97)], hi - lo - 0.02,
              loc=(0, side * (lo + hi) / 2, 0), mat=m["black"], parent=h, bevel=0.015, lods=VP.ALL)
        for k in range(2):
            VP.jerrycan(f"jerrycan_{s}_{k}", (-half + 0.40, side * (0.95 + k * 0.38), DECK), dict(m, paint=m["dark"]), h,
                        rot=(0, 0, math.pi / 2))
    for k in range(4):
        cyl(f"spare_link_{k}", 0.05, 0.55, (lower_glacis_x(0.98, half) + 0.045, -0.90 + k * 0.60, 0.98), "Y",
            m["track"], h, seg=8, lods=NEAR)
    # The rear: its tail lights in their guards.
    for side, sd in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"tail_light_{sd}", (-v.length / 2 - 0.02, side * (v.width / 2 - 0.7), DECK - 0.15), 0.05,
                            dict(v.mats, lamp=v.mats["tail"]), v.hull, rot=(0, 0, math.pi))

def running_gear(v):
    road_x = [2.55 - 0.98 * k for k in range(6)]
    # The idler carries the belt's top run level from the return rollers to
    # the nose (its top at their 1.08 m), as the photos show it.
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, road_x, ROAD_Z, ROAD_R, 0.24, (-3.55, 0.70, 0.33),
                            (3.55, 0.77, 0.31), [(1.6, 0.98, 0.10), (0.0, 1.0, 0.10), (-1.6, 0.98, 0.10)], bolts=8,
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
        solid(f"fallen_skirt_{k}", (1.25, SKIRT_T, SKIRT_TOP - 0.70), loc, m["paint"], v.hull, rot=rot, bevel=0.02)
    bend(parts("skirt_R_1_"), (1.6, -(SKIRT_Y - SKIRT_T / 2), 1.13), (1, 0, 0), (0, 0, -1), -0.6)
    bend(parts("bustle_cage"), (v.frame["mounts"][0]["pivot_m"][0] - 2.35, 0, 1.4), (0, 1, 0), (1, 0, 0), 0.3)
    shell = parts("hull_upper", "hull_lower", "skirt_", "turret_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.022, 0.9, seed=51.0), dent((3.7, 0.5, glacis_z(3.7, v.length / 2)), 0.5, 0.10, (-0.3, 0, -1)))
    rest_on_ground(0.004)


run("challenger_3", "british_green", build, wreck, chip=1.0)
