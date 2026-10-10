"""Puma (protection level C), from assets/references/puma/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/puma.py -- [--variant=<id>] [--wreck]

What the photos settle (Tag der Bundeswehr 2026, Augustdorf 2019): six road
wheels a side, three return rollers behind the armour, the drive sprocket
raised at the front and the idler at the rear; a sharp prow, its lower plate
and long upper glacis meeting in a chined nose with the lamps and tow cable
on it; tall flat sides under the level C armour modules, the louvred engine
intake in the front left module; the rear in stowage cages round the ramp.
The unmanned turret is a low box: the 30 mm gun in its long perforated
cradle on the right of centre, the MELLS Spike launcher on its arm on the
left, the PERI commander's sight on its mast at the left rear, the gunner's
sight in the turret front, smoke discharger banks on the rear corners and
the MUSS sensors on the corners. No running-gear close-up (gap): the wheel
count is from the side photos.

Built to the catalog frame (hull 7.4 x 4.0 x 3.6 m, turret pivot 0.45 m
behind the hull's middle and 2.55 m up, autocannon muzzle 4.1 m ahead,
launcher on the left at 1.08 m): nothing here moves it; the turret is built
at the frame's pivot. The square-on side photo (side-2) puts the turret
box's middle about 0.45 m behind the hull's; the front-quarter one (side)
exaggerates the setback.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID
TRACK_Y = 1.40
TRACK_W = 0.56
ROAD_R = 0.35
ROAD_Z = 0.43
ROAD_X = [2.30 - 0.90 * k for k in range(6)]
SPROCKET = (3.15, 0.78, 0.30)  # front drive
IDLER = (-3.22, 0.52, 0.30)
RETURNS = [(1.40, 0.90, 0.09), (-0.40, 0.91, 0.09), (-2.10, 0.90, 0.09)]
ROOF_Z = 2.42
SIDE_Y = 2.00
FOOT = -0.06
ROOF = 0.56


def hull_body(v):
    m, hull = v.mats, v.hull
    half = v.length / 2
    prism("hull_upper", [(-half, 0.95), (3.20, 0.95), (half, 1.40), (half - 0.10, 1.56), (1.50, ROOF_Z),
                         (-3.62, ROOF_Z), (-half, 2.30)], 3.40, mat=m["paint"], parent=hull, bevel=0.06)
    prism("hull_lower", [(-3.40, 0.46), (2.90, 0.46), (3.60, 1.40), (-half, 1.00), (-half, 0.78)], 2.40,
          mat=m["paint"], parent=hull, bevel=0.05)
    slope = math.atan((ROOF_Z - 1.56) / (half - 0.10 - 1.50))

    def glacis(x):
        return 1.56 + (half - 0.10 - x) * math.tan(slope)

    VP.hatch("driver_hatch", (2.25, 0.70, glacis(2.25)), m, hull, radius=0.30, rot=(0, slope, 0))
    for k, y in enumerate((0.50, 0.70, 0.90)):
        VP.periscope(f"driver_periscope_{k}", (2.62, y, glacis(2.62)), m, hull, size=(0.12, 0.14, 0.08),
                     rot=(0, slope, 0))
    VP.tarp_roll("glacis_tarp", (3.00, -0.60, glacis(3.00) + 0.12), 1.10, 0.13, m, hull, straps=2,
                 rot=(slope, 0, 0))
    VP.cable("glacis_cable", [(3.40, -1.40, glacis(3.40) + 0.03), (2.80, -0.40, glacis(2.80) + 0.03),
                              (3.30, 0.80, glacis(3.30) + 0.03)], m, hull, radius=0.022)
    for side, s in ((1, "L"), (-1, "R")):
        box(f"nose_lamp_box_{s}", (0.10, 0.36, 0.16), (half - 0.10, side * 1.30, 1.48), m["dark"], hull, bevel=0.02,
            lods=MID)
        box(f"nose_lamp_{s}", (0.012, 0.24, 0.10), (half - 0.045, side * 1.30, 1.48), m["lamp"], hull, lods=NEAR)
        VP.tow_hook(f"front_tow_{s}", (3.40, side * 0.70, 1.00), m, hull, size=0.14, rot=(0, -0.6, 0))
        VP.light_with_guard(f"tail_light_{s}", (-half + 0.10, side * 1.55, 2.05), 0.05, dict(m, lamp=m["tail"]),
                            hull, rot=(0, 0, math.pi))
        # Stowage cages either side of the ramp.
        VP.slat_armour(f"rear_cage_{s}", (-half - 0.04, side * 1.25, 1.10), (1.00, 0.90), m, hull, spacing=0.10,
                       bar=0.014, rot=(0, 0, math.pi / 2))
    VP.bolted_panel("rear_ramp", (-half + 0.02, 0, 0.80), (0.06, 1.40, 1.40), m, hull, bolts=(0, 0),
        bevel=0.02, lods=VP.ALL)
    VP.grille("engine_grille", (1.10, -0.95, ROOF_Z), (0.80, 1.10), m, hull, slats=8)
    VP.hatch("roof_hatch", (-2.60, 0.0, ROOF_Z), m, hull, size=(1.00, 1.30))


def side_modules(v):
    """The level C armour modules down each side: big bolted panels, a front
    one sloped to the prow and carrying the engine intake louvres on the
    left; a lower run over the tracks."""
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        VP.armour_tiles(f"side_module_{s}", (-0.55, side * (SIDE_Y - 0.12), 1.62), (5.90, 1.36), (4, 1), 0.12, m,
                        hull, rot=(-side * math.pi / 2, 0, 0))
        VP.armour_tiles(f"lower_module_{s}", (-0.10, side * (SIDE_Y - 0.14), 0.86), (6.60, 0.18), (6, 1), 0.10, m,
                        hull, rot=(-side * math.pi / 2, 0, 0), bolts=False)
        prism(f"front_module_{s}", [(2.40, 0.95), (3.10, 0.95), (3.30, 1.20), (2.62, 2.30), (2.40, 2.30)], 0.14,
              loc=(0, side * (SIDE_Y - 0.07), 0), mat=m["paint"], parent=hull, bevel=0.05)
        stencil(f"side_number_{s}", "48", 0.20, (-2.80, side * (SIDE_Y + 0.003), 1.90),
                (math.pi / 2, 0, math.pi if side > 0 else 0), m["marking"], hull)
    VP.grille("intake_louvre", (2.10, SIDE_Y + 0.004, 1.70), (0.90, 0.60), m, hull, slats=8,
              rot=(-math.pi / 2, 0, 0))


def turret_body(v, turret):
    m = v.mats
    half = [(1.05, 0.30), (1.00, 0.95), (-1.20, 1.00), (-1.30, 0.85)]

    def ring(pts, lean):
        left = [(x, y - lean) for x, y in pts]
        return left + [(x, -y) for x, y in reversed(left)]

    loft("turret_shell", [(FOOT, ring(half, 0.0)), (ROOF, ring([(x - (0.12 if x > 0.9 else 0.0), y) for x, y in half],
                                                                 0.05))], mat=m["paint"], parent=turret, bevel=0.05)
    cyl("turret_ring_guard", 0.95, 0.06, (0, 0, FOOT - 0.03), "Z", m["dark"], turret, seg=32, lods=MID)
    VP.sight_housing("gunner_sight", (1.02, -0.45, 0.10), m, turret, size=(0.18, 0.40, 0.30))
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"smoke_{s}", (-1.05, side * 0.95, ROOF - 0.20), m, turret, count=4,
                                 tube_radius=0.045, tube_length=0.20, elevation=0.30, spread=0.3,
                                 rot=(0, 0, side * math.pi / 2))
        cyl(f"muss_sensor_{s}", 0.07, 0.06, (0.85, side * 0.88, ROOF + 0.02), "Z", m["glass"], turret, seg=12,
            lods=NEAR)
    whip = empty("dressing_antenna", parent=turret)
    VP.antenna("antenna", (-1.10, -0.65, ROOF), m, whip, height=2.0)
    # The PERI on its mast at the left rear: the tallest thing aboard.
    cyl("peri_mast", 0.10, 0.36, (-0.80, 0.55, ROOF + 0.18), "Z", m["dark"], turret, seg=16)
    for side in (-1, 1):
        box(f"peri_frame_{side}", (0.04, 0.04, 0.34), (-0.80, 0.55 + side * 0.20, ROOF + 0.17), m["paint"], turret,
            lods=MID)
    VP.sight_housing("peri_head", (-0.80, 0.55, ROOF + 0.36), m, turret, size=(0.34, 0.34, 0.20))


def autocannon(v, gun):
    """The 30 mm MK30-2 in its long perforated cradle."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("cradle", (1.30, 0.34, 0.30), (1.50, 0, 0), m["paint"], gun, bevel=0.04, taper=(1.0, 0.85))
    for k in range(6):
        cyl(f"cradle_hole_{k}", 0.05, 0.36, (1.05 + k * 0.18, 0, 0.0), "Y", m["black"], gun, seg=10, lods=FINE)
    cyl("barrel", 0.045, reach - 2.10, ((reach + 2.10) / 2, 0, 0), "X", m["dark"], gun, seg=14)
    cyl("barrel_sleeve", 0.06, 0.40, (2.30, 0, 0), "X", m["paint"], gun, seg=16, lods=MID)
    cyl("muzzle", 0.075, 0.22, (reach - 0.11, 0, 0), "X", m["steel"], gun, seg=16)
    cyl("muzzle_bore", 0.025, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=MID)


def mells(v, launcher, tube):
    """The MELLS Spike launcher: its arm on the turret's left and the
    two-round box."""
    m = v.mats
    reach = v.frame["mounts"][1]["muzzle_m"][0]
    box("mells_arm", (0.30, 0.20, 0.30), (-0.10, -0.12, -0.12), m["dark"], launcher, bevel=0.02)
    box("mells_box", (reach + 0.20, 0.36, 0.42), ((reach - 0.20) / 2, 0.06, 0.0), m["paint"], tube, bevel=0.04)
    for k, z in enumerate((0.10, -0.10)):
        cyl(f"mells_mouth_{k}", 0.075, 0.02, (reach + 0.005, 0.06, z), "X", m["black"], tube, seg=14, lods=MID)


def build(variant, v):
    hull_body(v)
    VP.tracked_running_gear(v.mats, v.hull, TRACK_Y, TRACK_W, ROAD_X, ROAD_Z, ROAD_R, 0.22, SPROCKET, IDLER,
                            RETURNS, bolts=8, teeth=12, arm=(0.46, 0.40), pitch=0.16)
    side_modules(v)
    mounts = rig(v.frame, v.root)
    turret, gun, _, _ = mounts["autocannon"]
    launcher, tube, _, _ = mounts["launcher"]
    turret_body(v, turret)
    autocannon(v, gun)
    mells(v, launcher, tube)


def wreck(variant, v):
    """The vehicle after its fire: the left track off along its side, two
    road wheels gone, armour modules blown off and lying at its foot, the
    ramp blown down, plates warped. `wreckage.burn` heaves the turret."""
    from parts import box as solid, rest_on_ground
    from wreckage import densify, dent, heat, parts, remove, warp
    m = v.mats
    remove("track_L_band", "wheel_L_2_", "wheel_L_5_", "side_module_L_1_", "side_module_L_2_", "glacis_tarp",
           "side_number_L", "roof_hatch")
    thrown = solid("thrown_track", (3.6, TRACK_W, 0.05), (-0.3, 1.95, 0.03), m["track"], v.hull, rot=(0, 0, 0),
                   lods=(0, 1, 2, 3))
    densify(thrown)
    warp(thrown, heat(0.03, 0.6, seed=12.0))
    for k, x in enumerate((0.0, -1.40)):
        solid(f"fallen_module_{k}", (1.40, 0.12, 0.80), (x, 2.00, 0.42), m["paint"], v.hull, rot=(0.45, 0.0, 0.03),
              bevel=0.03)
    shell = parts("hull_upper", "hull_lower", "side_module_", "front_module_", "turret_shell")
    densify(shell, scale=2.0)
    warp(shell, heat(0.026, 0.9, seed=14.0), dent((3.20, -0.50, 1.80), 0.5, 0.10, (-0.6, 0, -1)))
    rest_on_ground(0.004)


run("puma", "german_three_tone", build, wreck, chip=1.0)
