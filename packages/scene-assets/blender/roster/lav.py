"""LAV-25A2 and LAV-AT, from assets/references/lav/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/lav.py -- [--variant=<id>] [--wreck]

What the photos and Mick Bell's 1987 elevations settle: eight black road-tread
tyres on 0.49 m wheels, axles in two pairs (1.0 m apart) with the wide gap
amidships, coil springs over each; the boat hull: a tucked-in belly, a sharp
chine at 1.2 m where the pointed bow's upper and lower glacis meet, upper
sides leaning hard in to a narrow roof; the driver's hatch and periscopes on
the left of the glacis, the engine deck's grille on its right, lights in
guards at the glacis corners and bolted A2 applique on it; the big exhaust
silencer on the right side; jerrycans racked on the leaning rear sides; two
troop hatches in the rear roof and twin rear doors. The LAV-25's two-man
turret: faceted, a mesh bustle basket, the M242 with its coax, a bank of four
smoke tubes on each cheek, sight heads on the roof, the commander up in his
hatch (photos: side, three-quarter front). The LAV-AT's Emerson turret: a
low cupola with the TOW hammerhead raised on its arm, two tube mouths beside
the sight.

Built to the catalog frames (hull 6.39 x 2.5 m; 2.69 m for the LAV-25 with
its turret, 2.0 m for the LAV-AT): nothing here moves them. The LAV-25's
frame puts the turret ring amidships (x 0) and the muzzle 3.65 m ahead of
it; the drawings put the ring about 0.85 m further back and the muzzle 1.5 m
behind the bow. The turret is drawn on the frame's pivot (choices.md, slice
16).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

WHEEL_X = [1.49, 0.50, -0.95, -1.93]
WHEEL_R = 0.49
WHEEL_W = 0.30
WHEEL_Y = 1.03
BELLY = 0.52
CHINE = 1.20
ROOF = 1.94
BOW = 3.195
GLACIS_TOP = 1.74  # x where the upper glacis meets the roof


def plan(rear, front, half, chamfer):
    """A ring of the hull in plan: square at the rear, chamfered at the bow."""
    return [(rear, -half), (front - chamfer, -half), (front, -half + chamfer), (front, half - chamfer),
            (front - chamfer, half), (rear, half)]


def glacis_z(x):
    """Height of the upper glacis at x."""
    return CHINE + 0.05 + (ROOF - CHINE - 0.05) * (BOW - 0.05 - x) / (BOW - 0.05 - GLACIS_TOP)


def build(variant, v):
    m, hull = v.mats, v.hull
    loft("lav_hull", [(BELLY, plan(-2.95, 2.30, 0.92, 0.30)),
                      (CHINE, plan(-3.195, BOW, 1.22, 0.55)),
                      (CHINE + 0.05, plan(-3.18, BOW - 0.05, 1.21, 0.55)),
                      (ROOF, plan(-2.98, GLACIS_TOP, 0.93, 0.30))], mat=m["paint"], parent=hull, bevel=0.05)
    wheels(v)
    fittings(v)
    mounts = rig(v.frame, v.root)
    if "lav_at" in variant["id"]:
        launcher, pitch, _, pivot = mounts["launcher"]
        emerson(v, launcher, pitch, pivot)
    else:
        turret, gun, _, _ = mounts["autocannon"]
        lav_turret(v, turret, gun)


def wheels(v):
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(WHEEL_X):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, m, hull,
                          rim_radius=0.26, hub_bolts=8)
            box(f"suspension_arm_{s}_{k}", (0.18, 0.40, 0.10), (x, side * (WHEEL_Y - 0.36), WHEEL_R + 0.02),
                m["dark"], hull, lods=NEAR)
            cyl(f"spring_{s}_{k}", 0.08, 0.42, (x, side * (WHEEL_Y - 0.40), WHEEL_R + 0.36), "Z", m["dark"], hull,
                seg=10, lods=FINE)
        VP.mudflap(f"mudflap_{s}", (-2.55, side * WHEEL_Y, CHINE - 0.05), (0.34, 0.45), m, hull)


def fittings(v):
    m, hull = v.mats, v.hull
    slope = math.atan((ROOF - CHINE - 0.05) / (BOW - 0.05 - GLACIS_TOP))
    lean = math.atan((1.22 - 0.93) / (ROOF - CHINE))  # the upper sides' lean inward
    # The glacis: the engine's grille on the right, the driver's hatch and
    # periscopes on the left, the A2's bolted applique, lights in guards at
    # its corners and tow eyes on the bow.
    VP.grille("engine_grille", (2.05, -0.42, glacis_z(2.05) + 0.01), (0.75, 0.70), m, hull, slats=7,
              rot=(0, slope, 0))
    VP.hatch("driver_hatch", (GLACIS_TOP - 0.32, 0.48, ROOF), m, hull, radius=0.28)
    for k, y in enumerate((0.26, 0.48, 0.70)):
        x = GLACIS_TOP + 0.10
        VP.periscope(f"driver_periscope_{k}", (x, y, glacis_z(x)), m, hull, size=(0.12, 0.16, 0.08),
                     rot=(0, slope, 0))
    for k, y in enumerate((0.42, -0.42)):
        x = 2.75
        VP.bolted_panel(f"glacis_applique_{k}", (x, y, glacis_z(x) - 0.01), (0.55, 0.62, 0.035), m, hull,
                        bolts=(3, 3), rot=(0, slope, 0))
    for side, s in ((1, "L"), (-1, "R")):
        x = 2.30
        VP.light_with_guard(f"headlight_{s}", (x, side * 0.80, glacis_z(x) + 0.10), 0.075, m, hull,
                            rot=(0, 0, side * 0.12))
        VP.light_with_guard(f"marker_{s}", (x - 0.05, side * 0.62, glacis_z(x - 0.05) + 0.08), 0.05, m, hull)
        VP.shackle(f"bow_shackle_{s}", (3.08, side * 0.55, CHINE - 0.05), m, hull, size=0.12,
                   rot=(0, 0, math.pi / 2))
        VP.light_with_guard(f"tail_light_{s}", (-3.10, side * 0.92, CHINE + 0.42), 0.05, dict(m, lamp=m["tail"]),
                            hull, rot=(0, 0, math.pi))
        VP.tow_hook(f"rear_tow_{s}", (-3.10, side * 0.62, CHINE - 0.18), m, hull, size=0.10, rot=(0, 0, math.pi))
        # Jerrycans racked on the leaning rear upper side, in a rail frame:
        # stowage, outside the hull box.
        rack = empty(f"dressing_jerrycans_{s}", parent=hull)
        for k in range(3):
            x = -1.55 - k * 0.42
            VP.jerrycan(f"side_jerrycan_{s}_{k}", (x, side * 1.27, CHINE + 0.08), dict(m, paint=m["dark"]), rack,
                        size=(0.345, 0.165, 0.47), rot=(side * lean, 0, 0))
        box(f"jerrycan_rack_{s}", (1.40, 0.05, 0.05), (-1.97, side * 1.24, CHINE + 0.36), m["steel"], rack,
            rot=(side * lean, 0, 0), lods=NEAR)
        # Bolted applique on the upper side, ahead of the racks.
        for k, x in enumerate((1.15, 0.25) if side > 0 else (1.15,)):
            VP.bolted_panel(f"side_applique_{s}_{k}", (x, side * 1.07, 1.55), (0.84, 0.48, 0.03), m, hull,
                            bolts=(3, 2), rot=(-side * (math.pi / 2 - lean), 0, 0))
    # The exhaust silencer on the right, a big can in its straps.
    cyl("silencer", 0.15, 0.95, (-0.45, -1.17, 1.42), "X", m["dark"], hull, seg=18, bevel=0.02)
    for k, x in enumerate((-0.80, -0.10)):
        box(f"silencer_strap_{k}", (0.05, 0.10, 0.34), (x, -1.20, 1.42), m["paint"], hull, lods=NEAR)
    VP.exhaust("exhaust", (-1.00, -1.18, 1.30), 0.06, 0.35, m, hull, rot=(0, 0.4, math.pi))
    # Troop compartment: two roof hatches, the stowage rails and a tarp.
    for k, y in enumerate((0.45, -0.45)):
        VP.hatch(f"troop_hatch_{k}", (-2.15, y, ROOF), m, hull, size=(0.95, 0.66))
    for side in (-1, 1):
        box(f"roof_rail_{side}", (1.70, 0.04, 0.10), (-2.10, side * 0.88, ROOF + 0.05), m["steel"], hull, lods=NEAR)
    tarp = empty("dressing_roof_tarp", parent=hull)
    VP.tarp_roll("roof_tarp", (-2.80, 0, ROOF + 0.13), 1.40, 0.13, m, tarp, straps=3)
    # Twin doors in the leaning upper rear plate, hinged at the sides, with
    # vision blocks and handles.
    tilt = math.atan(0.20 / (ROOF - CHINE - 0.05))
    for side, s in ((1, "L"), (-1, "R")):
        y = side * 0.40
        box(f"rear_door_{s}", (0.05, 0.74, 0.58), (-3.11, y, 1.55), m["paint"], hull, bevel=0.015,
            rot=(0, tilt, 0), lods=MID)
        box(f"rear_door_glass_{s}", (0.02, 0.18, 0.06), (-3.12, y, 1.72), m["glass"], hull, rot=(0, tilt, 0),
            lods=NEAR)
        for j, z in enumerate((1.35, 1.72)):
            box(f"rear_hinge_{s}_{j}", (0.06, 0.08, 0.10), (-3.13 + (z - 1.35) * 0.29, side * 0.79, z), m["steel"],
                hull, lods=FINE)
        box(f"rear_handle_{s}", (0.05, 0.04, 0.16), (-3.14, side * 0.10, 1.48), m["steel"], hull, lods=FINE)
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (-2.85, side * 0.78, ROOF), m, whip, height=2.6)


def lav_turret(v, turret, gun):
    """The LAV-25's turret on the frame's ring: a faceted body, the mesh
    bustle basket with its stowage, the M242 in its mantlet with the coax,
    smoke banks on each cheek, the gunner's and commander's sight heads and
    hatches, and the commander up in his hatch."""
    m = v.mats
    pz = v.frame["mounts"][0]["pivot_m"][2]
    base = ROOF - pz
    top = 2.45 - pz
    cyl("turret_ring", 0.90, 0.07, (0, 0, base + 0.035), "Z", m["dark"], turret, seg=32, lods=MID)
    foot = [(0.95, 0.30), (0.62, 0.72), (-0.95, 0.72), (-1.15, 0.55), (-1.15, -0.55), (-0.95, -0.72),
            (0.62, -0.72), (0.95, -0.30)]
    crown = [(0.55, 0.22), (0.35, 0.58), (-0.95, 0.60), (-1.10, 0.48), (-1.10, -0.48), (-0.95, -0.60),
             (0.35, -0.58), (0.55, -0.22)]
    loft("turret_shell", [(base + 0.06, foot), (top - 0.20, [(x * 0.99, y) for x, y in foot]), (top, crown)],
         mat=m["paint"], parent=turret, bevel=0.045)
    # The bustle's mesh basket with stowage in it.
    box("bustle_floor", (0.50, 1.30, 0.04), (-1.40, 0, top - 0.32), m["steel"], turret, lods=MID)
    for side in (-1, 1):
        box(f"bustle_side_{side}", (0.50, 0.03, 0.26), (-1.40, side * 0.64, top - 0.19), m["steel"], turret,
            lods=NEAR)
    box("bustle_back", (0.03, 1.30, 0.26), (-1.64, 0, top - 0.19), m["steel"], turret, lods=NEAR)
    VP.tarp_roll("bustle_roll", (-1.36, 0.25, top - 0.17), 0.70, 0.14, m, turret, straps=2)
    VP.stowage_box("bustle_box", (-1.40, -0.38, top - 0.30), (0.40, 0.42, 0.24), dict(m, paint=m["dark"]), turret)
    # Sights and hatches on the roof.
    VP.sight_housing("gunner_sight", (0.15, 0.40, top - 0.01), m, turret, size=(0.36, 0.26, 0.20))
    VP.sight_housing("commander_sight", (-0.05, -0.38, top - 0.01), m, turret, size=(0.30, 0.26, 0.20))
    cyl("commander_ring", 0.33, 0.06, (-0.55, -0.32, top + 0.02), "Z", m["paint"], turret, seg=24, bevel=0.012,
        lods=MID)
    VP.hatch("gunner_hatch", (-0.55, 0.32, top), m, turret, radius=0.28)
    # The commander's lid thrown back flat behind his ring.
    cyl("commander_lid", 0.28, 0.05, (-1.02, -0.32, top + 0.04), "Z", m["paint"], turret, seg=24, bevel=0.01,
        rot=(0, 0.08, 0), lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"smoke_{s}", (0.45, side * 0.70, base + 0.30), m, turret, count=4,
                                 tube_radius=0.045, tube_length=0.24, elevation=0.35, spread=0.3,
                                 rot=(0, 0, side * 0.6))
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("mantlet", (0.42, 0.50, 0.36), (0.90, 0, 0.0), m["paint"], gun, bevel=0.04)
    box("mantlet_cheek", (0.20, 0.62, 0.22), (0.78, 0, -0.02), m["dark"], gun, bevel=0.02, lods=MID)
    cyl("gun_sleeve", 0.075, 0.55, (1.35, 0, 0), "X", m["dark"], gun, seg=16)
    cyl("gun_barrel", 0.042, reach - 1.62, ((reach + 1.62) / 2, 0, 0), "X", m["steel"], gun, seg=14)
    cyl("gun_flash_hider", 0.055, 0.18, (reach - 0.09, 0, 0), "X", m["dark"], gun, seg=12)
    cyl("gun_bore", 0.028, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=NEAR)
    cyl("coax_barrel", 0.02, 0.40, (1.25, -0.20, -0.03), "X", m["steel"], gun, seg=8, lods=NEAR)
    whip = empty("dressing_turret_antenna", parent=turret)
    VP.antenna("turret_antenna", (-1.05, 0.62, top), m, whip, height=2.4)
    # The commander stands in his hatch, head and shoulders out (photos).
    if not v.wreck:
        seat = empty("dressing_commander", parent=turret)
        x, y, z = -0.55, -0.32, top + pz
        v.crew.append(("commander", seat, (x - 0.05, y, z - 0.42),
                       ((x + 0.22, y + 0.20, z + 0.04), (x + 0.22, y - 0.20, z + 0.04)),
                       ((x, y + 0.12, z - 1.25), (x, y - 0.12, z - 1.25))))


def emerson(v, station, pitch, pivot):
    """The LAV-AT's Emerson turret: a low cupola turning on the roof, the arm
    up its side and the TOW hammerhead on it, two tube mouths beside the
    sight's window."""
    m = v.mats
    base = ROOF - pivot.z
    cyl("cupola_ring", 0.72, 0.06, (0, 0, base + 0.03), "Z", m["dark"], station, seg=28, lods=MID)
    loft("cupola", [(base + 0.05, [(0.70, 0.30), (0.40, 0.66), (-0.62, 0.66), (-0.70, 0.30), (-0.70, -0.30),
                                   (-0.62, -0.66), (0.40, -0.66), (0.70, -0.30)]),
                    (base + 0.40, [(0.55, 0.25), (0.30, 0.56), (-0.55, 0.56), (-0.62, 0.25), (-0.62, -0.25),
                                   (-0.55, -0.56), (0.30, -0.56), (0.55, -0.25)])],
         mat=m["paint"], parent=station, bevel=0.04)
    for k, y in enumerate((0.25, -0.25)):
        VP.periscope(f"cupola_vision_{k}", (0.40, y, base + 0.40), m, station, size=(0.12, 0.16, 0.08))
    VP.hatch("cupola_hatch", (-0.25, 0.22, base + 0.40), m, station, radius=0.24)
    arm_top = -0.15
    box("hammerhead_arm", (0.30, 0.26, arm_top - base - 0.40), (-0.20, -0.18, (arm_top + base + 0.40) / 2),
        m["paint"], station, bevel=0.025)
    box("arm_knuckle", (0.34, 0.36, 0.24), (-0.20, -0.18, arm_top), m["dark"], station, bevel=0.03, lods=MID)
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("hammerhead", (reach + 0.20, 1.10, 0.52), (reach / 2 - 0.10, 0, 0.0), m["paint"], pitch, bevel=0.04)
    for k, y in enumerate((-0.12, -0.40)):
        cyl(f"tow_mouth_{k}", 0.12, 0.02, (reach + 0.005, y, 0.0), "X", m["black"], pitch, seg=16, lods=MID)
        cyl(f"tow_rim_{k}", 0.145, 0.05, (reach - 0.01, y, 0.0), "X", m["dark"], pitch, seg=16, lods=NEAR)
    box("sight_window", (0.02, 0.36, 0.22), (reach + 0.005, 0.30, 0.04), m["glass"], pitch, lods=MID)
    box("sight_hood", (0.10, 0.44, 0.04), (reach + 0.03, 0.30, 0.18), m["paint"], pitch, lods=NEAR)
    box("hammerhead_lid", ((reach + 0.20) * 0.9, 1.0, 0.03), (reach / 2 - 0.10, 0, 0.27), m["dark"], pitch,
        lods=NEAR)
    whip = empty("dressing_cupola_antenna", parent=station)
    VP.antenna("cupola_antenna", (-0.55, -0.45, base + 0.40), m, whip, height=2.2)


def wreck(variant, v):
    """The LAV after its fire: the front right pair of wheels blown off and
    the bow down on that side, a rear door blown out and lying beside it, the
    racked jerrycans burnt off and one rack hanging, troop hatches gone,
    plates warped and a dent in the glacis where it was hit, torn plate on
    the ground. The LAV-25 throws its turret (`wreckage.export_wreck`)."""
    from parts import rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_R_1_", "wheel_R_2_", "side_jerrycan_", "troop_hatch_", "roof_tarp", "rear_door_L",
           "rear_door_glass_L", "rear_handle_L", "dressing_spare")
    plate("door_lying", [(-0.36, -0.30), (0.37, -0.31), (0.36, 0.30), (-0.37, 0.29)], 0.05, (-2.5, 1.58, 0.04),
          (0.03, 0.02, 0.4), m["paint"], v.root, seed=71)
    bend(parts("jerrycan_rack_L"), (-1.30, 1.15, CHINE + 0.28), (1, 0, 0), (0, 0, -1), 0.6)
    shell = parts("lav_hull", "side_applique_", "glacis_applique_")
    densify(shell, scale=2.0)
    warp(shell, heat(0.02, 0.8, seed=12.0), dent((2.4, -0.4, 1.60), 0.5, 0.12, (-0.4, 0, -1)))
    for k, (loc, rot, size) in enumerate((((1.6, -1.70, 0.03), (0.03, 0.04, 0.9), 0.30),
                                          ((-1.2, 1.65, 0.03), (-0.04, 0.02, 2.2), 0.26),
                                          ((-2.2, 0.2, ROOF + 0.04), (0.02, 0.04, 0.3), 0.32))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.12, seed=81 + k)
    v.root.rotation_euler = (0.045, 0.04, 0)
    v.root.location.z -= 0.08
    rest_on_ground(0.004)


run("lav", "us_desert_tan", build, wreck, chip=0.6)
