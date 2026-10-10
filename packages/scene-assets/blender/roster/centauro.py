"""Centauro II wheeled tank destroyer, from assets/references/centauro/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/centauro.py -- [--wreck]

What the photos settle (Italian Army, 2024-2025): eight big wheels on black
treaded tyres, the second and third axles further apart than the others; a
boat-shaped nose with a trim vane across it and the driver's hatch on its
left; the upper hull flaring out over the wheels in sloped sponsons, the
engine at the front right under a grilled deck; a long, low, angular
turret set back over the rear half, faceted cheeks either side of the gun
shield, flat sides with stowage boxes and a deep bustle; the 120/45 gun
with a thermal sleeve and a fume extractor; the commander's panoramic sight
on its mast at the right rear of the roof, the remote weapon station on the
left, smoke banks on the cheeks, antennas behind. Italian vegetata
camouflage.

Built to the catalog frame: the published hull length, the width over the
wheels and the height over the turret roof's fittings (references gaps). The
turret turns on the hull's middle, as every carried mount's carrier does in
the simulation (its remote weapon station swings about the hull origin), so
it sits a little further forward than the photos' turret.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

NEAR, MID = VP.NEAR, VP.MID

TRUNNION = 1.05
WHEEL_X = [2.70, 1.25, -0.65, -2.10]
WHEEL_R = 0.62
WHEEL_W = 0.40
WHEEL_Y = 1.20
SKIRT = 1.28  # the sponsons' underside
DECK = 1.92
ROOF = 0.72  # the turret roof above the pivot


def hull(v):
    m, h = v.mats, v.hull
    half = v.length / 2 - 0.06  # the length counts the rear tow hooks and lights
    dark = dict(m, paint=m["dark"])
    # The narrow lower hull between the wheels, its boat nose rising forward.
    loft("hull_lower", [(0.50, VP.hull_plan(-half + 0.40, half - 0.70, 0.78, 0.30)),
                        (SKIRT, VP.hull_plan(-half + 0.05, half - 0.05, 0.92, 0.45))],
         mat=m["dark"], parent=h, bevel=0.04)
    # The upper hull flaring out over the wheels, the nose sloping to the deck.
    loft("hull_upper", [(SKIRT, VP.hull_plan(-half, half, 1.45, 0.55)),
                        (SKIRT + 0.30, VP.hull_plan(-half, half - 0.15, 1.56, 0.62)),
                        (DECK, VP.hull_plan(-half + 0.04, half - 1.25, 1.42, 0.40))],
         mat=m["paint"], parent=h, bevel=0.05)
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(WHEEL_X):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, dark, h,
                          rim_radius=0.34, hub_bolts=10, ctis=True)
            box(f"suspension_arm_{s}_{k}", (0.26, 0.38, 0.12), (x, side * (WHEEL_Y - 0.36), WHEEL_R), m["dark"], h,
                lods=NEAR)
        VP.mudflap(f"mudflap_{s}", (-half + 0.35, side * WHEEL_Y, SKIRT - 0.02), (0.40, 0.55), m, h)
        # Sponson stowage boxes over the rear wheels, lights and tow points.
        VP.stowage_box(f"sponson_box_{s}", (-1.40, side * 1.30, DECK), (1.60, 0.28, 0.26), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        VP.light_with_guard(f"headlight_{s}", (half - 0.42, side * 1.25, SKIRT + 0.40), 0.065, m, h)
        VP.light_with_guard(f"tail_light_{s}", (-half - 0.01, side * 1.30, SKIRT + 0.30), 0.05,
                            dict(m, lamp=m["tail"]), h, rot=(0, 0, math.pi))
        VP.tow_hook(f"front_tow_{s}", (half - 0.30, side * 0.55, 0.90), m, h, size=0.13)
        VP.tow_hook(f"rear_tow_{s}", (-half + 0.02, side * 0.70, 0.95), m, h, size=0.13, rot=(0, 0, math.pi))
    # The trim vane folded across the nose, the driver's hatch on the left.
    slope = math.atan((DECK - SKIRT - 0.30) / 1.10)
    VP.bolted_panel("trim_vane", (half - 0.45, 0, SKIRT + 0.52), (0.55, 2.40, 0.05), m, h, bolts=(2, 5),
                    rot=(0, slope, 0), bevel=0.02, lods=VP.ALL)
    VP.hatch("driver_hatch", (half - 1.45, 0.62, DECK), m, h, radius=0.27)
    for k in range(3):
        VP.periscope(f"driver_periscope_{k}", (half - 1.12, 0.42 + k * 0.20, DECK), m, h, size=(0.11, 0.16, 0.08))
    # The engine at the front right under its grilles; exhaust out the side.
    VP.grille("engine_grille", (half - 1.75, -0.65, DECK), (1.10, 1.00), m, h, slats=9)
    VP.exhaust("exhaust", (half - 1.70, -1.57, SKIRT + 0.35), 0.08, 0.20, m, h, rot=(0, 0, -math.pi / 2))
    VP.bolted_panel("rear_door", (-half - 0.005, 0, 1.25), (0.85, 1.00, 0.05), m, h, bolts=(2, 3),
                    rot=(0, -math.pi / 2, 0), bevel=0.02, lods=VP.ALL)
    VP.cable("tow_cable", [(-half + 0.4, 1.05, DECK + 0.02), (-0.2, 1.10, DECK + 0.02), (1.2, 1.05, DECK + 0.02)],
             m, h, radius=0.02)
    # The photos' add-on armour: bolted plates along the upper hull's band
    # leaning in from the wheel arches' flare to the deck; jerrycans aft.
    band = ((1.56, SKIRT + 0.30), (1.42, DECK))
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate((2.55, 1.30, 0.05, -1.20, -2.45)):
            loc, rot = VP.on_side(x, (SKIRT + 0.30 + DECK) / 2, side, *band)
            VP.bolted_panel(f"band_plate_{s}_{k}", loc, (1.15, 0.30, 0.04), m, h, bolts=(3, 2), rot=rot, bevel=0.012,
                            lods=VP.ALL)
        VP.jerrycan(f"jerrycan_{s}", (-half + 0.30, side * 0.55, DECK), dict(m, paint=m["dark"]), h,
                    rot=(0, 0, math.pi / 2))


def turret_body(v, turret):
    m = v.mats
    base = DECK - v.frame["mounts"][0]["pivot_m"][2]
    foot = [(1.35, 0.40), (1.70, 0.50), (0.85, 1.40), (-1.30, 1.45), (-2.35, 1.25), (-2.35, -1.25), (-1.30, -1.45),
            (0.85, -1.40), (1.70, -0.50), (1.35, -0.40)]
    crown = [(1.25, 0.40), (1.35, 0.48), (0.55, 1.25), (-1.30, 1.32), (-2.30, 1.15), (-2.30, -1.15), (-1.30, -1.32),
             (0.55, -1.25), (1.35, -0.48), (1.25, -0.40)]
    cyl("turret_ring_guard", 1.00, 0.08, (0, 0, base + 0.02), "Z", m["dark"], turret, seg=36, lods=MID)
    loft("turret_shell", [(base + 0.03, foot), (base + 0.20, foot), (ROOF, crown)], mat=m["paint"], parent=turret,
         bevel=0.05)
    VP.roof_fittings("roof", crown, ROOF, m, turret, periscopes=((0.20, -0.95, -0.5), (0.20, 0.95, 0.5)))
    VP.laser_warners("laser_warner", crown, ROOF, m, turret)
    # Armour modules down each flank, a basket round the bustle.
    for side, s in ((1, "L"), (-1, "R")):
        loc, rot = VP.on_side(-0.55, (base + 0.20 + ROOF) / 2, side, (1.45, base + 0.20), (1.32, ROOF))
        VP.armour_tiles(f"flank_armour_{s}", loc, (1.40, (ROOF - base - 0.20) * 0.8), (3, 1), 0.06, m, turret, rot=rot)
    VP.slat_armour("bustle_basket", (-2.45, 0, base + 0.25), (2.10, 0.40), m, turret, spacing=0.10, bar=0.014,
                   rot=(0, 0, math.pi / 2))
    for side, s in ((1, "L"), (-1, "R")):
        VP.weld_line(f"cheek_weld_{s}", [(1.60, side * 0.50, base + 0.20), (0.82, side * 1.38, base + 0.20)], m,
                     turret)
        VP.smoke_discharger_bank(f"smoke_{s}", (0.70, side * 1.22, ROOF - 0.10), m, turret, count=4,
                                 tube_radius=0.045, tube_length=0.20, elevation=0.3, spread=0.3,
                                 rot=(0, 0, side * 0.6))
        VP.stowage_box(f"turret_box_{s}", (-0.85, side * 1.40, base + 0.28), (1.10, 0.20, 0.38), m, turret,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-2.10, side * 0.95, ROOF), m, whip, height=2.2)
    VP.stowage_box("bustle_box", (-2.10, 0, base + 0.22), (0.45, 2.10, 0.45), m, turret)
    VP.sight_housing("gunner_sight", (0.75, 0.62, ROOF - 0.04), m, turret, size=(0.42, 0.34, 0.24))
    mast = empty("dressing_commander_sight", parent=turret)
    cyl("panorama_post", 0.10, 0.16, (-0.80, -0.62, ROOF + 0.08), "Z", m["dark"], mast, seg=14, lods=MID)
    VP.sight_housing("panorama_head", (-0.80, -0.62, ROOF + 0.14), m, mast, size=(0.36, 0.32, 0.30))
    VP.hatch("commander_hatch", (-0.25, -0.55, ROOF), m, turret, radius=0.28)
    VP.hatch("loader_hatch", (-0.10, 0.55, ROOF), m, turret, radius=0.26)


def gun_120(v, gun):
    """The 120/45: a squared gun shield, the barrel in thermal sleeve
    sections, the fume extractor two thirds out and a plain muzzle with its
    reference sensor."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0] - TRUNNION
    box("gun_shield", (0.50, 0.68, 0.48), (0.05, 0, 0.0), m["paint"], gun, bevel=0.05)
    cyl("barrel_root", 0.12, 0.40, (0.45, 0, 0), "X", m["paint"], gun, seg=24)
    evac = 2.9
    for k, (a, b) in enumerate([(0.65, evac - 0.30), (evac + 0.30, reach - 0.20)]):
        cyl(f"thermal_sleeve_{k}", 0.085, b - a, ((a + b) / 2, 0, 0), "X", m["paint"], gun, seg=22)
    for k, x in enumerate((1.4, 2.1, evac + 0.9, evac + 1.7)):
        cyl(f"sleeve_band_{k}", 0.092, 0.04, (x, 0, 0), "X", m["dark"], gun, seg=20, lods=NEAR)
    cyl("fume_extractor", 0.14, 0.56, (evac, 0, 0), "X", m["paint"], gun, seg=24, bevel=0.02)
    cyl("muzzle_end", 0.092, 0.20, (reach - 0.10, 0, 0), "X", m["steel"], gun, seg=22)
    cyl("muzzle_bore", 0.06, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=18, lods=MID)
    box("muzzle_reference", (0.10, 0.08, 0.07), (reach - 0.32, 0, 0.14), m["dark"], gun, lods=NEAR)


def remote_station(v, rws, rws_gun):
    m = v.mats
    cyl("rws_bearing", 0.20, 0.08, (0, 0, -0.04), "Z", m["dark"], rws, seg=20)
    box("rws_body", (0.42, 0.34, 0.22), (-0.06, 0, 0.08), m["paint"], rws, bevel=0.03)
    VP.sight_housing("rws_sight", (0.06, -0.24, -0.10), m, rws_gun, size=(0.26, 0.14, 0.20))
    box("rws_ammo", (0.30, 0.14, 0.24), (-0.08, 0.26, -0.08), m["paint"], rws_gun, bevel=0.02, lods=MID)
    VP.browning_m2(rws_gun, v.frame["mounts"][1]["muzzle_m"][0], m)


def build(variant, v):
    hull(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    rws, rws_gun, _, _ = mounts["RWS"]
    turret_body(v, turret)
    gun_120(v, gun)
    remote_station(v, rws, rws_gun)
    pivot = v.frame["mounts"][0]["pivot_m"]
    v.head_out("commander", turret, pivot[0] - 0.25, -0.55, pivot[2] + ROOF)


def wreck(variant, v):
    """The Centauro after its fire: the front left wheel and the third right
    torn off, the hull settled onto that corner, a sponson box and the rear
    door blown off, the trim vane bent down, plates warped and dented;
    `wreckage.burn` heaves the turret."""
    from parts import rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    import bpy
    m = v.mats
    remove("wheel_L_1_", "wheel_R_3_", "suspension_arm_L_0", "suspension_arm_R_2", "sponson_box_L", "rear_door")
    for name in ("wheel_L_1", "wheel_R_3"):
        node = bpy.data.objects.get(name)
        if node is not None:
            bpy.data.objects.remove(node, do_unlink=True)
    bend(parts("trim_vane"), (v.length / 2 - 0.45, 0, 1.6), (0, 1, 0), (1, 0, 0), 0.45)
    shell = parts("hull_upper", "hull_lower", "turret_shell", "bustle_box")
    densify(shell, scale=2.0)
    warp(shell, heat(0.022, 0.9, seed=41.0), dent((3.4, 0.6, 1.5), 0.45, 0.10, (-0.6, 0, -1)))
    # The hull settles onto the missing front-left wheel.
    v.root.rotation_euler = (-0.04, 0.035, 0)
    plate("door_fallen", [(-0.45, -0.4), (0.45, -0.42), (0.47, 0.4), (-0.45, 0.42)], 0.04, (-4.6, 0.4, 0.05),
          (0.0, 1.2, 0.2), m["paint"], v.hull, seed=51)
    rest_on_ground(0.004)


# Edge wear 0.6, as every live wheeled family has.
run("centauro", "italian_vegetata", build, wreck, chip=0.6)
