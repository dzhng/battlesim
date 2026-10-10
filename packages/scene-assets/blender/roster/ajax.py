"""Ajax tracked reconnaissance vehicle, from assets/references/ajax/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/ajax.py -- [--variant=<id>] [--wreck]

What the photos settle (Ajax at Bovington; the Ares and an Ajax-family
vehicle for the hull front and rear): seven road wheels a side, three return
rollers, the drive sprocket at the front and the idler at the rear; a long
boxy hull, its flat glacis carrying the driver's hatch and periscope block,
the lamp clusters set into the nose corners; the sides in big flat armour
modules with a chamfered upper run, large stowage bins on the rear quarters.
The turret carries the 40 mm CTA cannon in its mantlet on the front, smoke
discharger clusters on the front corners, a remote weapon station on the
right front, the commander's ORION sight box on the right rear and the
two crew hatches, commander and gunner riding head out. The frame gives the
turret one mount; the remote weapon station is drawn fixed to the turret.

Built to the catalog frame (hull 7.62 x 3.35 x 3.0 m, turret pivot 0.4 m
behind the hull's middle and 1.95 m up, autocannon muzzle 2.65 m ahead):
nothing here moves it. The turret sits behind the front engine and driver,
its rear about one wheel pitch ahead of the last road wheel (side photo);
it, and the crew in its hatches, are built at the frame's pivot.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID
TRACK_Y = 1.25
TRACK_W = 0.53
ROAD_R = 0.32
ROAD_Z = 0.40
ROAD_X = [2.55 - 0.85 * k for k in range(7)]
SPROCKET = (3.30, 0.70, 0.29)  # front drive
IDLER = (-3.35, 0.52, 0.29)
RETURNS = [(1.70, 0.82, 0.08), (0.00, 0.83, 0.08), (-1.70, 0.82, 0.08)]
ROOF_Z = 1.92
SIDE_Y = 1.67
FOOT = -0.04
ROOF = 0.62


def hull_body(v):
    m, hull = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 0.92), (3.55, 0.92), (half, 1.10), (half, 1.34), (2.00, ROOF_Z), (-half, ROOF_Z)],
          3.00, mat=m["paint"], parent=hull, bevel=0.06)
    prism("hull_lower", [(-3.55, 0.44), (3.25, 0.44), (3.70, 0.94), (-half, 0.94), (-half, 0.72)], 2.20,
          mat=m["paint"], parent=hull, bevel=0.04)
    slope = math.atan((ROOF_Z - 1.34) / (half - 2.00))

    def glacis(x):
        return 1.34 + (half - x) * math.tan(slope)

    VP.hatch("driver_hatch", (2.50, 0.70, glacis(2.50)), m, hull, radius=0.28, rot=(0, slope, 0))
    VP.bolted_panel("periscope_block", (3.05, 0.0, glacis(3.05)), (0.36, 0.40, 0.12), m, hull, bolts=(1, 2),
                    bevel=0.02, rot=(0, slope, 0), lods=VP.ALL)
    VP.armour_tiles("glacis_plates", (3.10, -0.80, glacis(3.10) + 0.01), (0.90, 1.00), (2, 2), 0.04, m, hull,
                    rot=(0, slope, 0))
    for side, s in ((1, "L"), (-1, "R")):
        box(f"nose_lamp_box_{s}", (0.12, 0.50, 0.16), (half - 0.04, side * 1.30, 1.18), m["dark"], hull, bevel=0.02,
            lods=MID)
        for k, dy in enumerate((0.12, -0.12)):
            cyl(f"nose_lamp_{s}_{k}", 0.055, 0.02, (half + 0.025, side * 1.30 + dy, 1.18), "X", m["lamp"], hull,
                seg=12, lods=NEAR)
        VP.tow_hook(f"front_tow_{s}", (3.60, side * 0.60, 0.86), m, hull, size=0.13)
        VP.tow_hook(f"rear_tow_{s}", (-3.68, side * 0.80, 0.84), m, hull, size=0.12, rot=(0, 0, math.pi))
        VP.light_with_guard(f"tail_light_{s}", (-half + 0.08, side * 1.30, 1.70), 0.05, dict(m, lamp=m["tail"]),
                            hull, rot=(0, 0, math.pi))
    VP.bolted_panel("rear_door", (-half - 0.01, 0, 0.82), (0.04, 1.10, 0.95), m, hull, bolts=(0, 0),
        bevel=0.02, lods=VP.ALL)
    VP.grille("engine_grille", (1.55, 0.75, ROOF_Z), (0.80, 0.90), m, hull, slats=8)
    VP.hatch("roof_hatch", (-2.70, 0.0, ROOF_Z), m, hull, size=(0.90, 0.90))
    cyl("fire_extinguisher", 0.05, 0.30, (3.40, 1.20, glacis(3.40) + 0.06), "Y", m["tail"], hull, seg=12, lods=NEAR)


def side_modules(v):
    """Big flat armour modules down each side with a chamfered upper run;
    stowage bins standing on the rear quarters."""
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        for k in range(5):
            x0 = 3.30 - k * 1.30
            x1 = x0 - 1.26
            prism(f"side_module_{s}_{k}", [(x1, 0.78), (x0, 0.78), (x0, 1.62), (x0 - 0.04, 1.86), (x1 + 0.04, 1.86),
                                             (x1, 1.62)], 0.14, loc=(0, side * (SIDE_Y - 0.07), 0), mat=m["paint"],
                  parent=hull, bevel=0.04, taper_y=lambda z: 1.0 if z < 1.62 else 0.55)
            for j, x in enumerate((x1 + 0.15, x0 - 0.15)):
                cyl(f"module_bolt_{s}_{k}_{j}", 0.025, 0.02, (x, side * (SIDE_Y + 0.004), 1.50), "Y", m["steel"], hull,
                    seg=6, lods=FINE)
        VP.stowage_box(f"rear_bin_{s}", (-3.05, side * 1.40, ROOF_Z), (1.30, 0.50, 0.42), m, hull,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        stencil(f"side_reg_{s}", "SX 86 AB", 0.12, (2.70, side * (SIDE_Y + 0.003), 1.10),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], hull)


def turret_body(v, turret):
    m = v.mats
    half = [(1.15, 0.36), (1.00, 1.05), (-1.15, 1.08), (-1.32, 0.90)]

    def ring(pts, lean, nose):
        left = [(x - (nose if x > 0.9 else 0.0), y - lean) for x, y in pts]
        return left + [(x, -y) for x, y in reversed(left)]

    loft("turret_shell", [(FOOT, ring(half, 0.0, 0.0)), (0.30, ring(half, -0.04, 0.0)),
                          (ROOF, ring(half, 0.10, 0.25))], mat=m["paint"], parent=turret, bevel=0.05)
    cyl("turret_ring_guard", 1.05, 0.08, (0, 0, FOOT - 0.03), "Z", m["dark"], turret, seg=36, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"smoke_{s}", (0.90, side * 0.90, 0.35), m, turret, count=3, tube_radius=0.05,
                                 tube_length=0.22, elevation=0.30, spread=0.3, rot=(0, 0, side * 0.5))
        VP.bolted_panel(f"turret_cheek_{s}", (0.10, side * 1.08, 0.05), (1.20, 0.08, 0.44), m, turret, bolts=(3, 1),
                        bevel=0.03, lods=VP.ALL)
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.05, side * 0.70, ROOF), m, whip, height=2.2)
    # The remote weapon station on the right front, fixed to the turret.
    cyl("rws_bearing", 0.18, 0.08, (0.55, -0.72, ROOF + 0.04), "Z", m["dark"], turret, seg=20, lods=MID)
    box("rws_body", (0.40, 0.30, 0.26), (0.55, -0.72, ROOF + 0.21), m["paint"], turret, bevel=0.03)
    VP.sight_housing("rws_sight", (0.62, -0.92, ROOF + 0.10), m, turret, size=(0.24, 0.12, 0.20))
    cyl("rws_gun", 0.025, 0.80, (1.10, -0.72, ROOF + 0.26), "X", m["dark"], turret, seg=10)
    # The commander's ORION sight box on the right rear, the hatches.
    cyl("orion_base", 0.18, 0.12, (-0.55, -0.70, ROOF + 0.06), "Z", m["paint"], turret, seg=20, lods=MID)
    VP.sight_housing("orion_sight", (-0.55, -0.70, ROOF + 0.12), m, turret, size=(0.42, 0.40, 0.32))
    VP.cupola("commander_cupola", (-0.25, 0.55, ROOF), m, turret, radius=0.30, periscopes=4)
    VP.cupola("gunner_cupola", (0.15, -0.15, ROOF), m, turret, radius=0.28, periscopes=3, rot=(0, 0, -0.3))


def cta40(v, gun):
    """The 40 mm CTA cannon: a stepped mantlet and the short barrel with
    its muzzle collar."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("mantlet", (0.40, 0.56, 0.42), (1.10, 0, -0.02), m["paint"], gun, bevel=0.05)
    cyl("mantlet_collar", 0.14, 0.24, (1.40, 0, 0), "X", m["paint"], gun, seg=20, bevel=0.015)
    cyl("barrel", 0.055, reach - 1.55, ((reach + 1.55) / 2, 0, 0), "X", m["dark"], gun, seg=16)
    cyl("muzzle", 0.075, 0.14, (reach - 0.07, 0, 0), "X", m["steel"], gun, seg=16)
    cyl("muzzle_bore", 0.03, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=MID)


def build(variant, v):
    hull_body(v)
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.20, SPROCKET, IDLER,
                            RETURNS, bolts=6, teeth=11, arm=(0.42, 0.40), pitch=0.15)
    side_modules(v)
    mounts = rig(v.frame, v.root)
    turret, gun, _, pivot = mounts["autocannon"]
    turret_body(v, turret)
    cta40(v, gun)
    v.head_out("commander", turret, pivot.x - 0.25, 0.55, pivot.z + ROOF + 0.06)
    v.head_out("gunner", turret, pivot.x + 0.15, -0.15, pivot.z + ROOF + 0.04)


def wreck(variant, v):
    """The vehicle after its fire: the right track off along its side, two
    road wheels gone, an armour module blown off and leaning at its foot,
    the rear door open, the bins burnt, plates warped. `wreckage.burn` heaves
    the turret."""
    from parts import box as solid, rest_on_ground
    from wreckage import densify, dent, heat, parts, remove, warp
    m = v.mats
    remove("track_R_band", "wheel_R_3_", "wheel_R_6_", "side_module_R_2", "module_bolt_R_2", "rear_door",
           "rear_bin_", "side_reg_R", "roof_hatch")
    thrown = solid("thrown_track", (3.8, TRACK_W, 0.05), (-0.5, -1.65, 0.03), m["track"], v.hull, rot=(0, 0, 0),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.03, 0.6, seed=15.0))
    solid("fallen_module", (1.26, 0.14, 0.80), (0.70, -1.72, 0.40), m["paint"], v.hull, rot=(-0.30, 0.0, 0.02),
          bevel=0.03)
    shell = parts("hull_upper", "hull_lower", "side_module_", "turret_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.026, 0.9, seed=16.0), dent((3.20, 0.40, 1.50), 0.5, 0.10, (-0.6, 0, -1)))
    rest_on_ground(0.004)


run("ajax", "british_green", build, wreck, chip=1.0)
