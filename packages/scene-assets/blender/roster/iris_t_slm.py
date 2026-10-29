"""IRIS-T SLM launcher on the MAN HX 8x8, from assets/references/iris_t_slm/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/iris_t_slm.py -- [--wreck]

What the photos settle (German three-tone, as the Luftwaffe's launcher at
Laage): the HX's tilt cab and its eight wheels (the MAN HX script's own,
`man-hx.py`), the tall air intake housing with its big grille behind the
cab; then the equipment shelter on its brown plinth, doors and vents on its
flanks, the two telescopic masts retracted on its rear corners, drum caps on top;
the lower control boxes, side lockers and the outrigger jacks (one pair
amidships, one at the tail) folded up; and at the rear the launcher: eight
canisters, four across and two deep over the whole width, held in two
latch collars in a U frame hinged at the tail, raised by a ram each side.
The reload photo shows the frame lowered flat over the rear deck; the pack
is drawn so, stowed, its canister mouths forward and the hinge base across
the tail.

The frame (hull 10.52 x 2.55 x 3.92 m) is the MAN SX45 8x8's published
travel length, width and height (Danish Army Vehicles, MAN 32.440); the HX
cab and wheels are the MAN HX script's, moved forward so the bumper meets
the frame's nose.
"""
import importlib
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, stencil, textured  # noqa: E402
from truck_chassis import on  # noqa: E402
from vehicle_export import run  # noqa: E402

hx = importlib.import_module("man-hx")

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

DECK = 1.45  # top of the subframe the shelter and launcher stand on
SHELTER = (-1.25, 1.20)  # rear and front of the equipment shelter
PACK_REAR, PACK_FRONT = -5.15, -1.40  # the stowed pack: hinge base to canister mouths
PACK_FLOOR = 1.85
CAN_W, CAN_H = 0.52, 0.46  # one canister's cross-section, across and up
CAN_Y = (-0.84, -0.28, 0.28, 0.84)
COLLARS = (PACK_REAR + 0.55, PACK_FRONT - 1.20)  # the two latch collars round the pack


def build(variant, v):
    # The HX moved forward so its bumper (`hx.NOSE`) meets this frame's nose.
    truck = on(v, v.length / 2 - hx.NOSE, name="hx_chassis")
    hx.running_gear(truck)
    hx.cab(truck)
    m, h = v.mats, v.hull
    half = v.length / 2
    cab_rear = truck.hull.location.x + hx.CAB_REAR
    box("subframe", (half + SHELTER[1] + 0.10, 2.30, 0.20), ((SHELTER[1] + 0.10 - half) / 2, 0, DECK - 0.10),
        m["dark"], h, bevel=0.01)
    intake(v, cab_rear)
    shelter(v)
    launcher(v)
    underside(v)


def intake(v, cab_rear):
    """The HX's air intake housing behind the cab, as tall as the cab, its
    grille on each flank."""
    m, h = v.mats, v.hull
    front, rear = cab_rear - 0.02, SHELTER[1] + 0.12
    roof = hx.ROOF - 0.15
    box("intake_housing", (front - rear, 2.30, roof - DECK), ((front + rear) / 2, 0, (roof + DECK) / 2), m["paint"], h,
        bevel=0.04)
    for side, s in ((1, "L"), (-1, "R")):
        VP.grille(f"intake_grille_{s}", ((front + rear) / 2, side * 1.155, 2.25), (1.10, 1.10), m, h, slats=10,
                  rot=(-side * math.pi / 2, 0, 0))
    VP.exhaust("exhaust", (front - 0.15, -1.05, roof), 0.07, 0.40, m, h)


def shelter(v):
    """The equipment shelter on its brown plinth, doors and vents down each
    flank, and the two masts retracted at its rear corners."""
    m, h = v.mats, v.hull
    rear, front = SHELTER
    mid, length = (rear + front) / 2, front - rear
    top = 3.30  # at the cab roof, as the photos show it
    box("shelter_plinth", (length - 0.10, 2.20, 0.42), (mid, 0, DECK + 0.21), m["dark"], h, bevel=0.02)
    box("shelter", (length, 2.40, top - DECK - 0.42), (mid, 0, (top + DECK + 0.42) / 2), m["paint"], h, bevel=0.04)
    box("shelter_roof", (length - 0.20, 2.20, 0.10), (mid, 0, top + 0.05), m["paint"], h, bevel=0.02)
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate((mid - 0.62, mid + 0.62)):
            VP.bolted_panel(f"shelter_door_{s}_{k}", (x, side * 1.20, 2.58), (1.10, 1.10, 0.03), m, h, bolts=(3, 3),
                            rot=(-side * math.pi / 2, 0, 0), bevel=0.01)
            box(f"shelter_handle_{s}_{k}", (0.14, 0.04, 0.04), (x + 0.40, side * 1.235, 2.55), m["steel"], h,
                lods=FINE)
        VP.grille(f"shelter_vent_{s}", (rear + 0.30, side * 1.205, 2.10), (0.40, 0.40), m, h, slats=5,
                  rot=(-side * math.pi / 2, 0, 0))
    # The masts, retracted: the nested tubes standing on each rear corner,
    # the drum cap on top, the tallest thing on the truck.
    for side, s in ((1, "L"), (-1, "R")):
        x, y = rear + 0.25, side * 0.80
        cyl(f"mast_{s}", 0.10, 0.42, (x, y, top + 0.21), "Z", m["dark"], h, seg=16)
        cyl(f"mast_cap_{s}", 0.24, 0.17, (x, y, top + 0.505), "Z", m["dark"], h, seg=24, bevel=0.02)
        cyl(f"mast_collar_{s}", 0.13, 0.06, (x, y, top + 0.03), "Z", m["steel"], h, seg=16, lods=MID)
    VP.stowage_box("roof_box", (mid + 0.45, 0, top + 0.10), (0.70, 1.20, 0.22), m, h)


def launcher(v):
    """The launcher folded flat over the rear deck: the U frame on its base
    beams, the eight canisters in their two latch collars, mouths forward,
    the hinge base across the tail, a ram each side."""
    m, h = v.mats, v.hull
    length = PACK_FRONT - PACK_REAR
    mid = (PACK_FRONT + PACK_REAR) / 2
    # The base beams and boxes under the pack, the hinge block at the tail.
    for side, s in ((1, "L"), (-1, "R")):
        box(f"launcher_beam_{s}", (length + 0.05, 0.22, PACK_FLOOR - DECK), (mid, side * 0.80, (PACK_FLOOR + DECK) / 2),
            m["paint"], h, bevel=0.02)
        box(f"launcher_rail_{s}", (length - 0.30, 0.10, 0.34), (mid + 0.05, side * 1.21, PACK_FLOOR + 0.30),
            m["paint"], h, bevel=0.015)
        box(f"hinge_{s}", (0.40, 0.30, 0.50), (PACK_REAR + 0.05, side * 1.05, PACK_FLOOR - 0.10), m["dark"], h,
            bevel=0.02)
        cyl(f"hinge_pin_{s}", 0.07, 0.36, (PACK_REAR + 0.05, side * 1.05, PACK_FLOOR - 0.10), "Y", m["steel"], h,
            seg=12, lods=MID)
        # The ram from the deck to the frame's side, lying nearly flat.
        cyl(f"ram_{s}", 0.07, 2.20, (PACK_FRONT - 1.05, side * 1.12, PACK_FLOOR + 0.02), "X", m["dark"], h, seg=12,
            rot=(0, -0.10, 0), lods=MID)
        cyl(f"ram_rod_{s}", 0.04, 0.80, (PACK_FRONT - 2.50, side * 1.12, PACK_FLOOR + 0.19), "X", m["steel"], h,
            seg=10, rot=(0, -0.10, 0), lods=NEAR)
    box("control_box", (1.10, 1.30, 0.32), (PACK_FRONT + 0.25, 0, DECK + 0.16), m["paint"], h, bevel=0.02)
    # The canisters: two rows of four, plain olive (the truck's camouflage
    # stops at them), each a square-section box with its mouth cover forward,
    # its base aft and the blue band near the mouth; the type's name in white
    # on the outer canisters of the top row.
    white = textured("canister_lettering", "marking_paint", colour=(0.55, 0.55, 0.52), chip=0.3, dirt=0.3,
                     streak=0.0, role="marking")
    olive = textured("canister_paint", "olive_paint", colour=(0.042, 0.055, 0.030), chip=0.3, dirt=0.45, role="paint")
    blue = textured("canister_band", "marking_paint", colour=(0.02, 0.09, 0.30), chip=0.3, dirt=0.3, streak=0.0,
                    role="marking")
    for j in range(2):
        z = PACK_FLOOR + 0.04 + CAN_H / 2 + j * (CAN_H + 0.03)
        for i, y in enumerate(CAN_Y):
            box(f"canister_{j}_{i}", (length - 0.10, CAN_W, CAN_H), (mid, y, z), olive, h, bevel=0.04)
            box(f"canister_cap_{j}_{i}", (0.03, CAN_W - 0.10, CAN_H - 0.10), (PACK_FRONT - 0.035, y, z), m["black"], h,
                lods=MID)
            box(f"canister_base_{j}_{i}", (0.03, CAN_W - 0.10, CAN_H - 0.10), (PACK_REAR + 0.035, y, z), m["black"],
                h, lods=MID)
            box(f"canister_band_{j}_{i}", (0.08, CAN_W + 0.006, CAN_H + 0.006), (PACK_FRONT - 0.70, y, z), blue, h,
                lods=NEAR)
    top = PACK_FLOOR + 0.04 + 2 * CAN_H + 0.03
    for side, s in ((1, "L"), (-1, "R")):
        stencil(f"canister_name_{s}", "IRIS-T SLM", 0.20,
                (sum(COLLARS) / 2, side * (CAN_Y[3] + CAN_W / 2 + 0.002), top - CAN_H / 2),
                (math.pi / 2, 0, math.pi if side > 0 else 0), white, h)
    # Two latch collars round the pack, lugs on top; aft, the U frame's
    # base beam and posts round the canister bases, its connectors below.
    for k, x in enumerate(COLLARS):
        box(f"collar_{k}", (0.16, 4 * CAN_W + 0.36, top - PACK_FLOOR + 0.08), (x, 0, (top + PACK_FLOOR) / 2),
            m["paint"], h, bevel=0.02)
        for i, y in enumerate(CAN_Y):
            box(f"collar_lug_{k}_{i}", (0.10, 0.08, 0.06), (x, y, top + 0.07), m["steel"], h, lods=FINE)
    box("launcher_base", (0.16, 2.42, 0.22), (PACK_REAR - 0.04, 0, PACK_FLOOR - 0.02), m["paint"], h, bevel=0.02)
    for side, s in ((1, "L"), (-1, "R")):
        box(f"launcher_post_{s}", (0.16, 0.14, top - PACK_FLOOR + 0.06), (PACK_REAR - 0.04, side * 1.14,
                                                                         (top + PACK_FLOOR) / 2), m["paint"], h,
            bevel=0.015)
    for i, y in enumerate(CAN_Y):
        box(f"connector_{i}", (0.06, 0.30, 0.14), (PACK_REAR - 0.15, y, PACK_FLOOR - 0.02), m["dark"], h, lods=NEAR)


def underside(v):
    """Below the deck: the side lockers and fuel tank between the axles, the
    outrigger jacks folded up amidships and at the tail, the rear underrun
    bar, the tail lights and the pintle; the whip antennas on the cab."""
    m, h = v.mats, v.hull
    half = v.length / 2
    for side, s in ((1, "L"), (-1, "R")):
        VP.stowage_box(f"locker_{s}", (-1.20, side * 0.95, 0.72), (1.30, 0.42, 0.58), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        for k, x in enumerate((-2.30, -half + 0.40)):
            cyl(f"jack_{s}_{k}", 0.07, 1.10, (x, side * 1.15, DECK - 0.40), "Z", m["dark"], h, seg=14)
            cyl(f"jack_pad_{s}_{k}", 0.16, 0.05, (x, side * 1.15, DECK - 0.98), "Z", m["dark"], h, seg=16, lods=MID)
            box(f"jack_bracket_{s}_{k}", (0.22, 0.20, 0.24), (x, side * 1.08, DECK - 0.05), m["paint"], h, bevel=0.01,
                lods=MID)
        VP.light_with_guard(f"tail_light_{s}", (-half + 0.08, side * 0.85, 1.10), 0.05, dict(m, lamp=m["tail"]), h,
                            rot=(0, 0, math.pi))
        VP.mudflap(f"rear_mudflap_{s}", (-half + 0.70, side * 1.03, 0.95), (0.42, 0.50), m, h)
    VP.fuel_tank("fuel_tank", (0.40, 0.95, 1.00), 1.30, 0.30, m, h)
    box("battery_box", (1.10, 0.40, 0.50), (0.40, -0.98, 0.95), m["paint"], h, bevel=0.02)
    box("battery_bracket", (0.10, 0.50, 0.08), (0.40, -0.72, 1.18), m["dark"], h, lods=MID)
    box("rear_underrun", (0.12, 2.20, 0.14), (-half + 0.10, 0, 0.70), m["dark"], h, bevel=0.012)
    for side, s in ((1, "L"), (-1, "R")):
        box(f"underrun_bracket_{s}", (0.08, 0.10, 0.40), (-half + 0.10, side * 0.45, 0.90), m["dark"], h, lods=MID)
    VP.tow_hook("pintle", (-half + 0.18, 0, 0.92), m, h, size=0.10, rot=(0, 0, math.pi))
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=h)
        VP.antenna(f"antenna_{s}", (half - 2.10, side * 1.05, hx.ROOF), m, whip, height=2.0)


def wreck(variant, v):
    """The launcher after its fire: the front right wheel gone and the cab
    down on that corner, two canisters burst out of the top row and their
    covers blown, the shelter and pack warped and dented, a mast cap and a
    mirror gone, a door torn off and lying on the pack. Whole."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_R_1_", "canister_cap_", "canister_1_1", "canister_band_1_1", "canister_1_2", "canister_band_1_2",
           "mast_cap_R", "dressing_mirror_R", "mirror_arm_R", "mirror_R", "shelter_door_R_0")
    shell = parts("cab", "intake_housing", "shelter", "canister_", "collar_", "launcher_base")
    densify(shell, scale=1.5)
    warp(shell, heat(0.03, 0.8, seed=47.0), dent((4.7, -0.7, 2.4), 0.6, 0.12, (-1, 0, 0)),
         dent((-3.0, 0.0, 2.9), 0.9, 0.18))
    plate("litter_door", [(-0.55, -0.62), (0.52, -0.66), (0.56, 0.60), (-0.5, 0.64)], 0.03,
          (-3.6, 0.3, PACK_FLOOR + 2 * CAN_H + 0.10), (0.05, 0.08, 0.6), m["paint"], v.root, curl=0.10, seed=313)
    v.root.rotation_euler = (0.025, 0.015, 0)
    v.root.location.z -= 0.06
    rest_on_ground(0.004)


if __name__ == "__main__":
    run("iris_t_slm", "german_three_tone", build, wreck, chip=0.6)
