"""M977 HEMTT cargo truck, from assets/references/hemtt/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/hemtt.py -- [--wreck]

What the photos settle: eight black bar-tread tyres on painted rims with a
ring of wheel nuts, two axles close together under the cab and two under the
rear of the bed; the cab forward: a bumper with its big D-shackles, the
grille's mesh panel sloping back up to the headlight shelf, a two-pane
windscreen, amber markers along the roof's front edge, the door with its
window and the steps under it, mirrors on frames; behind the cab the air
cleaner, the exhaust stack and the spare tyre standing on its carrier; the
long drop-side cargo bed on the chassis rails, the fuel tank and tool boxes
under it, and the folded materials-handling crane at its rear end; the load
under a lashed tarp. Cab interior as the dark glass shows it: the dash and
the seat backs.

Built to the catalog frame (hull 10.4 x 2.4 x 3.0 m): nothing here moves it.
The load and the crane's folded boom reach the box's top.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, prism  # noqa: E402
from vehicle_export import run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

AXLES = (3.58, 1.93, -2.45, -3.93)
WHEEL_R = 0.62
WHEEL_W = 0.40
WHEEL_Y = 0.98
RAIL_Z = 1.05
BED_FLOOR = 1.45
CAB_REAR = 3.05
CAB_FLOOR = 1.32
NOSE = 5.20
ROOF = 2.85


def build(variant, v):
    m, hull = v.mats, v.hull
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(AXLES):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * WHEEL_Y, WHEEL_R), WHEEL_R, WHEEL_W, side, m, hull,
                          rim_radius=0.32, tread="bar", hub_bolts=10)
            box(f"axle_{s}_{k}", (0.18, 0.55, 0.18), (x, side * 0.55, WHEEL_R), m["dark"], hull, lods=NEAR)
        box(f"chassis_rail_{s}", (9.80, 0.14, 0.26), (-0.20, side * 0.45, RAIL_Z), m["dark"], hull, bevel=0.01,
            lods=MID)
        VP.mudflap(f"mudflap_{s}", (-4.65, side * WHEEL_Y, BED_FLOOR - 0.15), (0.42, 0.55), m, hull)
        VP.mudflap(f"front_mudflap_{s}", (1.20, side * WHEEL_Y, BED_FLOOR - 0.10), (0.42, 0.50), m, hull)
    cab(v)
    bed(v)


def cab(v):
    m, hull = v.mats, v.hull
    width = v.width - 0.06
    # The cab rides over the front wheels; the engine's block runs down
    # between them to the bumper.
    profile = [(CAB_REAR, CAB_FLOOR), (NOSE - 0.02, CAB_FLOOR), (NOSE - 0.18, 1.92), (NOSE - 0.40, 1.98),
               (NOSE - 0.62, ROOF - 0.05), (NOSE - 0.70, ROOF), (CAB_REAR, ROOF)]
    prism("cab", profile, width, mat=m["paint"], parent=hull, bevel=0.05)
    prism("engine_block", [(CAB_REAR + 0.20, 0.80), (NOSE - 0.06, 0.80), (NOSE - 0.02, CAB_FLOOR),
                           (CAB_REAR + 0.20, CAB_FLOOR)], 1.20, mat=m["paint"], parent=hull, bevel=0.03)
    slope = math.atan(0.16 / (1.92 - CAB_FLOOR))
    # The grille's mesh panel, bumper, D-shackles, headlights in the shelf.
    VP.grille("grille", (NOSE - 0.11, 0, 1.62), (0.56, 1.30), m, hull, slats=9, rot=(0, math.pi / 2 - slope, 0))
    box("front_bumper", (0.20, 2.30, 0.24), (NOSE - 0.02, 0, 0.90), m["paint"], hull, bevel=0.03)
    for side, s in ((1, "L"), (-1, "R")):
        VP.shackle(f"d_shackle_{s}", (NOSE + 0.06, side * 0.55, 0.90), m, hull, size=0.16, rot=(0, 0, math.pi / 2))
        cyl(f"headlight_{s}", 0.09, 0.04, (NOSE - 0.18, side * 0.95, 1.85), "X", m["lamp"], hull, seg=16, lods=MID)
        cyl(f"headlight_bezel_{s}", 0.11, 0.05, (NOSE - 0.20, side * 0.95, 1.85), "X", m["dark"], hull, seg=16,
            lods=MID)
        box(f"marker_{s}", (0.03, 0.08, 0.08), (NOSE - 0.17, side * 0.72, 1.85), m["tail"], hull, lods=FINE)
        # Door, its window and handle; the steps under it; the mirror.
        y = side * (width / 2 + 0.005)
        VP.weld_line(f"door_seam_{s}", [(4.30, y, CAB_FLOOR + 0.05), (4.30, y, ROOF - 0.10), (3.30, y, ROOF - 0.10),
                                       (3.30, y, CAB_FLOOR + 0.05)], m, hull, radius=0.012)
        box(f"door_window_{s}", (0.70, 0.03, 0.50), (3.85, y, 2.40), m["glass"], hull, lods=MID)
        box(f"door_handle_{s}", (0.14, 0.04, 0.04), (3.45, y + side * 0.02, 1.90), m["steel"], hull, lods=FINE)
        for k, z in enumerate((0.55, 0.85)):
            box(f"step_{s}_{k}", (0.30, 0.22, 0.04), (2.78, side * (width / 2 - 0.08), z + 0.20), m["steel"], hull,
                lods=MID)
        mirror = empty(f"dressing_mirror_{s}", parent=hull)
        box(f"mirror_frame_{s}", (0.04, 0.04, 0.60), (4.45, side * 1.32, 2.30), m["black"], mirror, lods=NEAR)
        box(f"mirror_{s}", (0.05, 0.18, 0.40), (4.45, side * 1.34, 2.35), m["black"], mirror, bevel=0.01, lods=MID)
    # The windscreen and, behind it, the dash and seat backs.
    rake = math.atan(0.22 / (ROOF - 0.05 - 1.98))
    for side in (-1, 1):
        box(f"windscreen_{side}", (0.03, 1.02, 0.78), (NOSE - 0.52, side * 0.57, 2.42), m["glass"], hull,
            rot=(0, -rake, 0), lods=MID)
        box(f"seat_back_{side}", (0.12, 0.50, 0.60), (3.55, side * 0.55, 1.90), m["black"], hull, lods=NEAR)
    box("dash", (0.40, 2.00, 0.22), (4.40, 0, 1.92), m["black"], hull, lods=NEAR)
    for k in range(5):
        cyl(f"roof_marker_{k}", 0.03, 0.04, (NOSE - 0.72, -0.80 + k * 0.40, ROOF + 0.02), "Z", m["tail"], hull, seg=8,
            lods=FINE)
    # Behind the cab: air cleaner, exhaust stack, spare tyre on its carrier.
    cyl("air_cleaner", 0.30, 0.70, (2.80, 0.75, 2.30), "Z", m["paint"], hull, seg=18, bevel=0.03)
    cyl("air_cleaner_cap", 0.32, 0.08, (2.80, 0.75, 2.69), "Z", m["dark"], hull, seg=18, lods=MID)
    cyl("exhaust_stack", 0.07, 1.30, (2.85, -0.95, 2.20), "Z", m["dark"], hull, seg=12)
    cyl("exhaust_cap", 0.09, 0.10, (2.85, -0.95, 2.88), "Z", m["black"], hull, seg=12, lods=MID)
    box("spare_carrier", (0.20, 0.90, 0.10), (2.75, -0.15, BED_FLOOR + 0.05), m["dark"], hull, lods=MID)
    cyl("spare_tyre", WHEEL_R * 0.97, WHEEL_W, (2.75, -0.15, BED_FLOOR + 0.70), "X", m["rubber"], hull, seg=28)
    cyl("spare_rim", 0.32, 0.06, (2.95, -0.15, BED_FLOOR + 0.70), "X", m["paint"], hull, seg=18, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=hull)
        VP.antenna(f"antenna_{s}", (CAB_REAR + 0.15, side * 1.05, ROOF), m, whip, height=2.2)


def bed(v):
    m, hull = v.mats, v.hull
    length = 7.80
    centre = 2.62 - length / 2
    VP.cargo_bed("cargo_bed", (centre, 0, BED_FLOOR), (length, 2.36, 0.62), m, hull, stakes=7)
    # The load: pallets of crates under a lashed tarp.
    for k, x in enumerate((centre + 2.0, centre - 0.4)):
        box(f"load_pallet_{k}", (2.00, 1.90, 0.12), (x, 0, BED_FLOOR + 0.06), m["dark"], hull, lods=MID)
        box(f"load_{k}", (1.95, 1.85, 1.30), (x, 0, BED_FLOOR + 0.12 + 0.62), m["canvas"], hull, bevel=0.08)
        for j, y in enumerate((-0.5, 0.5)):
            box(f"load_strap_{k}_{j}", (0.04, 1.90, 1.36), (x + y, 0, BED_FLOOR + 0.12 + 0.62), m["dark"], hull,
                lods=NEAR)
    for k, y in enumerate((-0.55, 0.0, 0.55)):
        VP.stowage_box(f"ammo_crate_{k}", (centre - 2.3, y, BED_FLOOR), (0.90, 0.50, 0.45), dict(m, paint=m["dark"]),
                       hull)
    # The folded crane at the rear: its column on the bed's end, the boom
    # folded forward over the load.
    cx = -4.85
    cyl("crane_base", 0.30, 0.25, (cx, 0.75, BED_FLOOR + 0.12), "Z", m["dark"], hull, seg=18, lods=MID)
    box("crane_column", (0.32, 0.32, 1.10), (cx, 0.75, BED_FLOOR + 0.80), m["paint"], hull, bevel=0.03)
    box("crane_boom", (2.30, 0.24, 0.26), (cx + 1.10, 0.75, BED_FLOOR + 1.45), m["paint"], hull, bevel=0.03)
    box("crane_jib", (1.50, 0.18, 0.18), (cx + 0.80, 0.75, BED_FLOOR + 1.22), m["paint"], hull, bevel=0.02)
    cyl("crane_ram", 0.06, 0.95, (cx + 0.45, 0.75, BED_FLOOR + 1.05), "X", m["steel"], hull, seg=10,
        rot=(0, -0.55, 0), lods=MID)
    VP.tow_hook("crane_hook", (cx + 2.15, 0.75, BED_FLOOR + 1.25), m, hull, size=0.10, rot=(0, math.pi / 2, 0))
    # Under the bed: the fuel tank, tool boxes; the rear lights and pintle.
    VP.fuel_tank("fuel_tank", (0.10, 0.92, 1.05), 1.40, 0.30, m, hull)
    VP.stowage_box("tool_box", (-0.20, -0.95, 0.80), (1.10, 0.40, 0.45), dict(m, paint=m["dark"]), hull)
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"tail_light_{s}", (-5.17, side * 0.95, 1.20), 0.05, dict(m, lamp=m["tail"]), hull,
                            rot=(0, 0, math.pi))
    box("rear_bumper", (0.14, 2.20, 0.16), (-5.12, 0, 0.95), m["dark"], hull, bevel=0.012)
    VP.tow_hook("pintle", (-5.08, 0, 0.92), m, hull, size=0.10, rot=(0, 0, math.pi))


def wreck(variant, v):
    """The HEMTT after its fire: the front left wheel blown off and the cab
    down on that corner, the tarp burnt off its load and two crates tumbled
    across the bed, the crane's boom bent, the windscreen frame warped and the
    cab dented, the spare burnt off. Whole."""
    from parts import rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_L_1_", "spare_", "load_strap_", "ammo_crate_1", "ammo_crate_2", "dressing_mirror_L",
           "mirror_frame_L", "mirror_L")
    for k, (x, y, yaw) in enumerate(((-3.3, 0.45, 0.4), (-2.4, -0.55, -0.3))):
        VP.stowage_box(f"spilled_crate_{k}", (x, y, BED_FLOOR), (0.90, 0.50, 0.45), dict(m, paint=m["dark"]),
                       v.hull, rot=(0.05, 0.02, yaw))
    bend(parts("crane_boom", "crane_jib", "crane_hook"), (-4.85, 0.75, BED_FLOOR + 1.45), (0, 1, 0), (0, 0, 1), 0.35)
    shell = parts("cab", "cargo_bed_side", "load_")
    densify(shell, scale=1.5)
    warp(shell, heat(0.025, 0.7, seed=30.0), dent((4.6, 0.6, 2.2), 0.6, 0.12, (-1, 0, 0)))
    plate("litter_0", [(-0.3, -0.2), (0.27, -0.21), (0.3, 0.15), (-0.21, 0.24)], 0.03, (3.8, 0.3, ROOF + 0.03),
          (0.03, 0.04, 0.9), m["paint"], v.root, curl=0.12, seed=251)
    v.root.rotation_euler = (-0.02, 0.0, 0)
    v.root.location.z -= 0.06
    rest_on_ground(0.004)


run("hemtt", "us_desert_tan", build, wreck, chip=0.6)
