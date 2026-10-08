"""ZTZ-99A, from assets/references/type99/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/type99.py -- [--variant=<id>] [--wreck]

What the photos settle (Military Museum Beijing 2017, the 2015 parade): six
large road wheels a side, the sprocket at the rear and the idler at the
front; heavy side armour panels from nose to tail, their lower edges cut in
V scallops between the wheels; rounded front fenders either side of a
glacis in reactive armour blocks, a lamp in a box on the left fender. The
turret is the arrowhead: wedge armour either side of the gun, flat sides,
a bank of five smoke tubes on each side behind the wedge, a long bustle with
a cylindrical stowage container across its rear, the gunner's sight on the
left front, the commander's independent sight on a pedestal at the right
rear and the machine gun by his hatch. The rear is from the earlier Type 99
(gap); the hull rear is assumed the same.

Built to the catalog frame (hull 7.6 x 3.7 x 2.35 m, turret pivot 1.41 m,
cannon muzzle 7.2 m ahead): nothing here moves it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from t72 import kord  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID
TRACK_Y = 1.40
TRACK_W = 0.60
ROAD_R = 0.36
ROAD_Z = 0.44
ROAD_X = [2.40 - 0.95 * k for k in range(6)]
SPROCKET = (-3.28, 0.55, 0.32)
IDLER = (3.28, 0.52, 0.30)
RETURNS = [(1.45, 0.86, 0.10), (-0.45, 0.87, 0.10), (-2.35, 0.86, 0.10)]
DECK = 1.38
PANEL_Y = 1.80
FOOT = -0.04
ROOF = 0.74
TRUNNION = 1.05


def glacis_z(x):
    if x >= 2.40:
        return DECK - (x - 2.40) / 1.40 * 0.42
    return DECK


def hull_body(v):
    m, hull = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 0.94), (3.70, 0.94), (half, 0.96), (2.40, DECK), (-3.75, DECK), (-half, 1.30)],
          3.20, mat=m["paint"], parent=hull, bevel=0.05)
    prism("hull_lower", [(-3.55, 0.46), (3.25, 0.46), (3.78, 0.96), (-half, 0.96), (-half, 0.80)], 2.20,
          mat=m["paint"], parent=hull, bevel=0.04)
    slope = math.atan(0.42 / 1.40)
    VP.armour_tiles("glacis_era", (3.15, 0, glacis_z(3.15) + 0.005), (1.10, 1.70), (2, 4), 0.10, m, hull,
                    rot=(0, slope, 0))
    VP.hatch("driver_hatch", (2.15, 0, DECK), m, hull, radius=0.30)
    VP.periscope("driver_periscope", (2.42, 0, DECK), m, hull, size=(0.12, 0.26, 0.09))
    for side, s in ((1, "L"), (-1, "R")):
        # The rounded front fenders over the tracks.
        prism(f"front_fender_{s}", [(2.30, DECK - 0.02), (3.40, 1.15), (3.75, 0.96), (3.75, 0.90), (2.30, 0.90)],
              0.80, loc=(0, side * 1.38, 0), mat=m["paint"], parent=hull, bevel=0.10)
        VP.tow_hook(f"front_tow_{s}", (3.70, side * 0.55, 0.80), m, hull, size=0.14)
        VP.tow_hook(f"rear_tow_{s}", (-3.72, side * 0.80, 0.84), m, hull, size=0.13, rot=(0, 0, math.pi))
        VP.cable(f"tow_cable_{s}", [(-3.4, side * 1.48, DECK + 0.02), (-1.8, side * 1.50, DECK + 0.02),
                                    (-0.2, side * 1.50, DECK + 0.02), (1.6, side * 1.48, DECK + 0.02)], m, hull,
                 radius=0.022)
    VP.bolted_plate("lamp_box", (3.00, 1.05, 1.20), (0.26, 0.30, 0.22), m, hull, bolts=(1, 1), bevel=0.03)
    box("lamp", (0.012, 0.18, 0.12), (3.135, 1.05, 1.32), m["lamp"], hull, lods=MID)
    VP.grille("engine_grille", (-2.70, 0, DECK), (1.50, 1.80), m, hull, slats=12)
    VP.bolted_plate("engine_door", (-1.55, 0.0, DECK), (0.70, 1.90, 0.03), m, hull, bolts=(2, 3), bevel=0.01,
                    lods=MID)
    VP.grille("rear_louvre", (-half, 0, 1.10), (0.30, 2.30), m, hull, slats=7, rot=(0, -math.pi / 2, 0))


def side_panels(v):
    """The armour panels along each side, their lower edges scalloped in V
    between the road wheels, bolted along the top."""
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        for k in range(6):
            x0 = 3.40 - k * 1.20
            x1 = x0 - 1.16
            mid = (x0 + x1) / 2
            outline = [(x1, 0.80), (mid, 0.62), (x0, 0.80), (x0, DECK + 0.02), (x1, DECK + 0.02)]
            if k == 0:
                outline = [(x1, 0.80), (mid, 0.62), (x0 - 0.10, 0.80), (x0 + 0.22, 1.20), (x0 + 0.22, DECK + 0.02),
                           (x1, DECK + 0.02)]
            prism(f"side_panel_{s}_{k}", outline, 0.14, loc=(0, side * (PANEL_Y - 0.07), 0), mat=m["paint"],
                  parent=hull, bevel=0.04)
            for j, x in enumerate((x1 + 0.18, x0 - 0.18)):
                cyl(f"panel_bolt_{s}_{k}_{j}", 0.026, 0.02, (x, side * (PANEL_Y + 0.004), DECK - 0.08), "Y",
                    m["steel"], hull, seg=6, lods=FINE)
            box(f"panel_port_{s}_{k}", (0.08, 0.02, 0.08), (mid, side * (PANEL_Y + 0.002), 0.88), m["black"], hull,
                lods=NEAR)
        stencil(f"star_{s}", "*", 0.40, (0.30, side * (PANEL_Y + 0.003), 1.08),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], hull)


def turret_body(v, turret):
    m = v.mats
    half = [(1.30, 0.40), (1.28, 1.36), (-1.70, 1.40), (-2.55, 1.30), (-2.65, 1.10)]

    def ring(pts, lean):
        left = [(x, y - lean) for x, y in pts]
        return left + [(x, -y) for x, y in reversed(left)]

    loft("turret_shell", [(FOOT, ring(half, 0.0)), (ROOF, ring(half, 0.06))], mat=m["paint"], parent=turret,
         bevel=0.05)
    cyl("turret_ring_guard", 1.10, 0.10, (0, 0, FOOT - 0.03), "Z", m["dark"], turret, seg=40, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        back = [(1.30, 0.42), (1.75, 0.46), (1.10, 1.44), (0.20, 1.46)]
        ridge = [(1.30, 0.42), (2.30, 0.50), (1.30, 1.50), (0.20, 1.50)]
        top = [(1.30, 0.42), (1.68, 0.46), (1.05, 1.40), (0.20, 1.42)]
        rings = []
        for z, pts in ((FOOT + 0.04, back), (0.32, ridge), (ROOF - 0.02, top)):
            p = [(x, side * y) for x, y in pts]
            rings.append((z, p if side > 0 else p[::-1]))
        loft(f"wedge_{s}", rings, mat=m["paint"], parent=turret, bevel=0.045)
        VP.armour_tiles(f"wedge_tiles_{s}", (1.40, side * 0.95, ROOF - 0.02), (0.60, 0.90), (2, 3), 0.06, m, turret,
                        rot=(0, 0.30, 0))
        VP.smoke_discharger_bank(f"smoke_{s}", (0.05, side * 1.46, 0.40), m, turret, count=5, tube_radius=0.05,
                                 tube_length=0.26, elevation=0.35, spread=0.4, rot=(0, 0, side * 0.6))
        VP.bolted_plate(f"side_plate_{s}", (-0.90, side * 1.40, 0.30), (1.40, 0.56, 0.05), m, turret, bolts=(4, 2),
                        bevel=0.015, rot=(-side * math.pi / 2, 0, 0), lods=MID)
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-2.20, side * 0.95, ROOF), m, whip, height=2.4)
    # The cylindrical stowage container across the bustle's rear.
    cyl("rear_container", 0.26, 2.20, (-2.92, 0, 0.42), "Y", m["paint"], turret, seg=24, bevel=0.03)
    for k in (-0.70, 0.70):
        box(f"container_bracket_{k}", (0.40, 0.08, 0.10), (-2.72, k, 0.30), m["dark"], turret, lods=MID)
    VP.sight_housing("gunner_sight", (0.85, 0.75, ROOF - 0.04), m, turret, size=(0.44, 0.36, 0.26))
    sight = empty("dressing_commander_sight", parent=turret)
    cyl("commander_sight_post", 0.12, 0.22, (-1.40, -0.85, ROOF + 0.11), "Z", m["paint"], sight, seg=16,
        bevel=0.015, lods=MID)
    VP.sight_housing("commander_sight", (-1.40, -0.85, ROOF + 0.18), m, sight, size=(0.36, 0.34, 0.26))
    VP.cupola("commander_cupola", (-0.80, -0.62, ROOF), m, turret, radius=0.34, periscopes=4, lid_open=False)
    VP.hatch("gunner_hatch", (-0.60, 0.62, ROOF), m, turret, radius=0.30)
    VP.bolted_plate("roof_plate", (-1.70, 0.30, ROOF), (0.80, 0.70, 0.03), m, turret, bolts=(2, 2), bevel=0.01,
                    lods=MID)


def main_gun(v, gun):
    """The 125 mm smoothbore: the mantlet cover, the thermal sleeve, the fume
    extractor a third of the way out and the muzzle reference box."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0] - TRUNNION
    box("mantlet_cover", (0.60, 0.70, 0.46), (0.25, 0, 0.0), m["canvas"], gun, bevel=0.12)
    cyl("barrel_root", 0.14, 0.40, (0.70, 0, 0), "X", m["paint"], gun, seg=24, bevel=0.015)
    evac = 2.40
    for k, (a, b) in enumerate([(0.90, evac - 0.36), (evac + 0.36, reach - 0.20)]):
        cyl(f"thermal_sleeve_{k}", 0.094, b - a, ((a + b) / 2, 0, 0), "X", m["paint"], gun, seg=24)
    for k, x in enumerate((1.40, evac + 1.00, evac + 2.10, reach - 0.70)):
        cyl(f"sleeve_band_{k}", 0.102, 0.05, (x, 0, 0), "X", m["dark"], gun, seg=20, lods=NEAR)
    cyl("fume_extractor", 0.15, 0.62, (evac, 0, 0), "X", m["paint"], gun, seg=28, bevel=0.02)
    cyl("muzzle_end", 0.095, 0.20, (reach - 0.10, 0, 0), "X", m["steel"], gun, seg=24)
    cyl("muzzle_bore", 0.064, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=20, lods=MID)
    box("muzzle_reference", (0.12, 0.09, 0.08), (reach - 0.30, 0, 0.15), m["dark"], gun, lods=NEAR)


def roof_gun(v, hmg, hmg_gun):
    """The commander's 12.7 mm on its post with a small shield."""
    m = v.mats
    muzzle = v.frame["mounts"][1]["muzzle_m"]
    lift = muzzle[2]
    cyl("mg_post", 0.04, lift + 0.10, (0.10, 0, (lift + 0.10) / 2 - 0.10), "Z", m["steel"], hmg, seg=10)
    box("mg_cradle", (0.24, 0.12, 0.10), (0.08, 0, lift - 0.07), m["dark"], hmg, lods=MID)
    box("mg_shield", (0.04, 0.34, 0.24), (0.42, 0, -0.04), m["paint"], hmg_gun, bevel=0.01, lods=MID)
    kord(hmg_gun, muzzle[0], m)


def build(variant, v):
    hull_body(v)
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.24, SPROCKET, IDLER,
                            RETURNS, bolts=8, teeth=12, arm=(0.48, -0.35), pitch=0.16)
    side_panels(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret_body(v, turret)
    main_gun(v, gun)
    roof_gun(v, hmg, hmg_gun)


def wreck(variant, v):
    """The tank after its cook-off: the right track off with two road wheels
    gone, two right side panels blown off and lying by the hull, another bent
    out, plates warped, the glacis dented, the rear container burst.
    `wreckage.burn` then throws the turret."""
    from parts import box as solid, rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("track_R_band", "wheel_R_3_", "wheel_R_4_", "side_panel_R_2", "side_panel_R_3", "panel_bolt_R_2",
           "panel_bolt_R_3", "panel_port_R_2", "panel_port_R_3", "rear_container", "gunner_hatch", "star_R")
    thrown = solid("thrown_track", (4.0, TRACK_W, 0.05), (-0.3, -2.10, 0.03), m["track"], v.hull, rot=(0, 0, 0.06),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.035, 0.6, seed=9.0))
    for k, (loc, rot) in enumerate((((1.6, -2.45, 0.08), (1.5, 0.2, 0.3)), ((-1.4, -2.50, 0.08), (1.5, -0.1, -0.4)))):
        solid(f"fallen_panel_{k}", (1.16, 0.14, 0.70), loc, m["paint"], v.hull, rot=rot, bevel=0.03)
    bend(parts("side_panel_L_4"), (-1.40, PANEL_Y, DECK), (1, 0, 0), (0, 0, -1), 0.5)
    shell = parts("hull_upper", "hull_lower", "side_panel_", "turret_shell", "wedge_", "front_fender_")
    densify(shell, scale=2.0)
    warp(shell, heat(0.022, 0.9, seed=13.0), dent((3.20, -0.40, 1.15), 0.5, 0.10, (-0.6, 0, -1)))
    for k, (loc, rot, size) in enumerate((((2.4, 2.2, 0.03), (0.04, 0.02, 0.4), 0.40),
                                          ((-3.0, 2.1, 0.03), (-0.03, 0.05, 1.8), 0.32),
                                          ((4.1, 0.6, 0.03), (0.0, 0.06, 2.6), 0.26))):
        plate(f"debris_{k}", [(-size, -size * 0.6), (size * 0.9, -size * 0.7), (size, size * 0.5),
                              (-size * 0.7, size * 0.8)], 0.03, loc, rot, m["paint"], v.hull, curl=0.15, seed=81 + k)
    rest_on_ground(0.004)


run("type99", "chinese_digital", build, wreck, chip=1.0)
