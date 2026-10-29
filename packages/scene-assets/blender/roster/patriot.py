"""Patriot PAC-3 launching station on its 8x8 truck, from assets/references/patriot/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/patriot.py -- [--wreck]

The unit is one drivable vehicle, so it is drawn as the Bundeswehr fields
the launcher: on the MAN 15 t mil gl A1 BR 8x8 (the US tows the same
launcher behind a HEMTT on a semi-trailer, 17 m long). Its frame is that
truck's published 10.40 x 2.90 x 3.96 m, the height with four rounds
stowed; the US launcher on its trailer stands 3.99 m loaded (FM 3-01.85,
Appendix B).

What the photos settle:
- The KAT1 cab ahead of the front axle: flat front, a tall two-pane
  windscreen, a roof hatch; the engine and the launcher's generator and
  electronics in a tall louvred box straight behind it.
- Four axles in two pairs, the front pair wider spaced; big single tyres
  outside the narrower body (the "breit" track).
- The launcher: four canisters in two rows of two in one ribbed pack,
  stowed flat over the rear on its elevating beam and ram, the canisters'
  covered mouths at the rear end in a braced end frame; railings and
  cable reels at the pack's front end.
- Four stabiliser jacks, raised: one each side behind the cab and at the
  rear corners; ladders folded up at the rear, warning panels and lamps on
  the rear bumper.

The launcher is drawn stowed and does not articulate yet. The paint is the
US Army's tan, as the roster's other US vehicles.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from truck_chassis import chassis, on, spare_wheel  # noqa: E402
from vehicle_export import run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

AXLES = [3.55, 1.80, -2.05, -3.65]
WHEEL_R, WHEEL_W, TRACK = 0.63, 0.40, 1.24
# The canister pack, stowed: its rear (mouth) end, front end, floor and
# each canister's cross-section.
PACK_REAR, PACK_FRONT, PACK_FLOOR = -5.12, 0.70, 1.97
CAN_W, CAN_H = 1.00, 0.90
# The gaps between the canisters, across and between the rows.
GAP_Y, GAP_Z = 0.08, 0.06


def build(variant, v):
    m, h = v.mats, v.hull
    half = v.length / 2
    # The chassis 0.1 m shorter than the frame: its rear tow hook reaches
    # past the bumper, and the launcher's rear frame is the vehicle's tail.
    deck = chassis(on(v, length=v.length - 0.10), AXLES, WHEEL_R, WHEEL_W, TRACK, 1.18, None, 3.02, 1.98, 2.90)
    kat1_cab(v, 3.02, 5.00, 1.28, 2.90)
    box("deck_plate", (5.9, 2.30, 0.10), (-2.00, 0, deck + 0.05), m["paint"], h, bevel=0.01)
    module(v, deck)
    launcher(v, deck)
    for side, s in ((1, "L"), (-1, "R")):
        # Stabiliser jacks, raised: behind the cab and at the rear corners.
        for k, x in enumerate((2.70, -4.72)):
            jack(v, f"jack_{s}_{k}", x, side * 1.24, deck)
        # Side lockers between the axle pairs, a cable drum on the left.
        VP.stowage_box(f"side_locker_{s}", (-0.15, side * 1.02, deck - 0.62), (1.70, 0.40, 0.55), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
    cyl("cable_drum", 0.30, 0.34, (0.95, 1.05, deck - 0.30), "Y", m["dark"], h, seg=20, lods=MID)
    # Ladders folded up at the rear, the warning panels on the bumper.
    for side, s in ((1, "L"), (-1, "R")):
        ladder(v, f"rear_ladder_{s}", (-half + 0.18, side * 0.95, deck + 0.20), 1.70)
        box(f"warning_panel_{s}", (0.03, 0.42, 0.34), (-half + 0.08, side * 0.78, deck - 0.40), m["tail"], h,
            lods=MID)
        box(f"warning_stripe_{s}", (0.035, 0.08, 0.36), (-half + 0.078, side * 0.78, deck - 0.40), m["lamp"], h,
            rot=(0.7 * side, 0, 0), lods=NEAR)


def kat1_cab(v, x0, front, floor, top):
    """The KAT1 cab ahead of the front axle: an upright, flat-fronted box the
    width of the body, a tall two-pane windscreen barely raked, a door each
    side with its window, the roof hatch, the grille, lights, bumper and
    steps low on the front."""
    m, h = v.mats, v.hull
    half_w = 1.22
    plan = VP.hull_plan(x0, front, half_w, 0.08)
    loft("cab_body", [(floor, plan), (top - 0.20, plan),
                      (top, VP.hull_plan(x0 + 0.04, front - 0.10, half_w - 0.08, 0.10))],
         mat=m["paint"], parent=h, bevel=0.05)
    # The windscreens stand proud of the upright front, in their frames.
    for k, y in enumerate((0.55, -0.55)):
        box(f"windscreen_frame_{k}", (0.03, 1.08, 0.88), (front + 0.015, y, top - 0.62), m["dark"], h,
            lods=NEAR)
        box(f"windscreen_{k}", (0.03, 1.00, 0.80), (front + 0.03, y, top - 0.62), m["glass"], h, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        y = side * (half_w + 0.005)
        VP.weld_line(f"door_seam_{s}", [(front - 0.15, y, floor + 0.25), (front - 0.15, y, top - 0.15),
                                        (front - 1.10, y, top - 0.15), (front - 1.10, y, floor + 0.25)], m, h,
                     radius=0.012)
        box(f"door_window_{s}", (0.80, 0.03, 0.62), (front - 0.62, y, top - 0.55), m["glass"], h, lods=MID)
        box(f"door_handle_{s}", (0.16, 0.04, 0.04), (front - 1.00, side * (half_w + 0.03), top - 1.05),
            m["steel"], h, lods=FINE)
        box(f"rear_window_{s}", (0.35, 0.03, 0.40), (x0 + 0.40, y, top - 0.55), m["glass"], h, lods=MID)
        for k, z in enumerate((0.55, 0.95)):
            box(f"step_{s}_{k}", (0.45, 0.24, 0.04), (front - 0.62, side * (half_w - 0.08), z), m["steel"], h,
                lods=MID)
        VP.light_with_guard(f"headlight_{s}", (front + 0.04, side * (half_w - 0.25), floor + 0.25), 0.08, m, h)
        mirror = empty(f"dressing_mirror_{s}", parent=h)
        box(f"mirror_arm_{s}", (0.04, 0.30, 0.04), (front - 0.15, side * (half_w + 0.12), top - 0.35), m["dark"],
            mirror, lods=MID)
        box(f"mirror_{s}", (0.04, 0.12, 0.30), (front - 0.15, side * (half_w + 0.28), top - 0.50), m["dark"],
            mirror, lods=MID)
    VP.grille("radiator_grille", (front + 0.02, 0, floor + 0.45), (0.35, 1.40), m, h, slats=7,
              rot=(0, -math.pi / 2, 0))
    box("front_bumper", (0.16, 2 * half_w + 0.10, 0.24), (front + 0.07, 0, floor - 0.05), m["dark"], h,
        bevel=0.015)
    VP.hatch("cab_roof_hatch", (x0 + 0.65, 0.40, top), m, h, radius=0.32)
    seat_z = top - 1.05
    seat_base_height = seat_z - floor - 0.25
    box("dash", (0.35, 2 * half_w - 0.3, 0.20), (front - 0.45, 0, seat_z), m["black"], h, lods=NEAR)
    box("dash_support", (0.06, 2 * half_w - 0.1, seat_z - floor - 0.05), (front - 0.45, 0, floor + (seat_z - floor) / 2),
        m["black"], h, lods=NEAR)
    for side in (-1, 1):
        box(f"seat_back_{side}", (0.12, 0.50, 0.60), (x0 + 0.45, side * 0.55, seat_z), m["black"], h,
            lods=NEAR)
        box(f"seat_base_{side}", (0.40, 0.50, seat_base_height), (x0 + 0.35, side * 0.55, floor + seat_base_height / 2 + 0.05),
            m["black"], h, lods=NEAR)
    whip = empty("dressing_antenna_cab", parent=h)
    VP.antenna("antenna_cab", (x0 + 0.20, -(half_w - 0.15), top), m, whip, height=2.2)


def module(v, deck):
    """The engine, generator and electronics box behind the cab: taller than
    the cab, louvred on its sides, a railing round its roof."""
    m, h = v.mats, v.hull
    x0, x1, top = 1.62, 2.98, 3.30
    box("module_body", (x1 - x0, 2.36, top - deck), ((x0 + x1) / 2, 0, (deck + top) / 2), m["paint"], h,
        bevel=0.04)
    for side, s in ((1, "L"), (-1, "R")):
        VP.grille(f"module_louvre_{s}", ((x0 + x1) / 2 + 0.15, side * 1.185, deck + 1.20), (0.80, 0.70), m, h,
                  slats=9, rot=(-side * math.pi / 2, 0, 0))
        VP.bolted_panel(f"module_door_{s}", ((x0 + x1) / 2 - 0.30, side * 1.185, deck + 0.35), (0.55, 1.10, 0.03),
                        m, h, bolts=(2, 3), rot=(-side * math.pi / 2, 0, 0), bevel=0.01)
        box(f"module_rail_{s}", (x1 - x0, 0.04, 0.04), ((x0 + x1) / 2, side * 1.10, top + 0.42), m["dark"], h,
            lods=MID)
        for k, x in enumerate((x0 + 0.08, x1 - 0.08)):
            box(f"module_post_{s}_{k}", (0.04, 0.04, 0.42), (x, side * 1.10, top + 0.21), m["dark"], h, lods=MID)
    VP.exhaust("exhaust", (x1 - 0.20, -1.05, top + 0.05), 0.07, 0.40, m, h)
    box("module_vent", (0.60, 0.80, 0.10), ((x0 + x1) / 2, 0.30, top + 0.05), m["dark"], h, bevel=0.01, lods=MID)
    # The spare wheel lies flat on the roof.
    spare_wheel(v, ((x0 + x1) / 2 - 0.05, -0.40, top + WHEEL_W / 2 + 0.02), WHEEL_R * 0.95, WHEEL_W,
                rot=(math.pi / 2, 0, 0))
    whip = empty("dressing_antenna_module", parent=h)
    VP.antenna("antenna_module", (x0 + 0.15, 0.95, top), m, whip, height=2.2)


def launcher(v, deck):
    """The elevating beam and its ram on the deck, the four-canister pack
    stowed flat on it, mouths to the rear, and its front end's railings and
    cable reels."""
    m, h = v.mats, v.hull
    length = PACK_FRONT - PACK_REAR
    mid = (PACK_FRONT + PACK_REAR) / 2
    # The beam the pack rides on, its rear trunnion and the elevating ram.
    box("launcher_beam", (length - 0.40, 1.30, 0.30), (mid + 0.10, 0, PACK_FLOOR - 0.25), m["paint"], h,
        bevel=0.02)
    for side in (-1, 1):
        box(f"trunnion_{side}", (0.50, 0.20, PACK_FLOOR - deck), (PACK_REAR + 0.85, side * 0.70,
                                                                  (deck + PACK_FLOOR) / 2), m["dark"], h,
            bevel=0.02)
        box(f"beam_rest_{side}", (0.30, 0.25, PACK_FLOOR - deck - 0.40), (PACK_FRONT - 0.70, side * 0.60,
                                                                          deck + (PACK_FLOOR - deck - 0.40) / 2),
            m["dark"], h, lods=MID)
    cyl("trunnion_axle", 0.10, 1.70, (PACK_REAR + 0.85, 0, PACK_FLOOR - 0.35), "Y", m["steel"], h, seg=14,
        lods=MID)
    cyl("elevating_ram", 0.11, 2.10, (-1.30, 0, deck + 0.42), "X", m["steel"], h, seg=14, rot=(0, -0.10, 0))
    cyl("ram_body", 0.16, 1.20, (-0.30, 0, deck + 0.34), "X", m["dark"], h, seg=16, rot=(0, -0.10, 0))
    # The pack: two rows of two canisters, each ribbed round its body, its
    # covered mouth at the rear end and its breech at the front.
    for j in range(2):
        for i in range(2):
            y = (i - 0.5) * (CAN_W + GAP_Y)
            z = PACK_FLOOR + CAN_H / 2 + j * (CAN_H + GAP_Z)
            canister(v, f"canister_{j}_{i}", mid, y, z, length)
    # The end frames: a post each side of each end, a beam across the top,
    # and on each side a diagonal brace from the rear post's foot forward
    # (the mouths at the rear stay open to view).
    span_z = 2 * CAN_H + GAP_Z
    post_y = CAN_W + GAP_Y / 2 + 0.09
    for end, x in (("rear", PACK_REAR - 0.04), ("front", PACK_FRONT + 0.04)):
        for side in (-1, 1):
            box(f"{end}_frame_post_{side}", (0.10, 0.10, span_z + 0.10), (x, side * post_y,
                                                                         PACK_FLOOR + span_z / 2), m["dark"], h,
                lods=MID)
        box(f"{end}_frame_top", (0.10, 2 * post_y + 0.10, 0.10), (x, 0, PACK_FLOOR + span_z + 0.04), m["dark"], h,
            lods=MID)
    run_x = 0.95
    ang = math.atan2(span_z, run_x)
    for side in (-1, 1):
        box(f"side_brace_{side}", (math.hypot(run_x, span_z), 0.06, 0.08),
            (PACK_REAR + run_x / 2, side * (post_y - 0.02), PACK_FLOOR + span_z / 2), m["dark"], h,
            rot=(0, -ang, 0), lods=MID)
    # The front end: railings, the launcher's cable reels and lifting eyes.
    for side, s in ((1, "L"), (-1, "R")):
        box(f"pack_rail_{s}", (0.04, 0.04, 1.10), (PACK_FRONT + 0.32, side * 1.10, PACK_FLOOR + 0.55), m["dark"], h,
            lods=MID)
        box(f"pack_rail_top_{s}", (0.50, 0.04, 0.04), (PACK_FRONT + 0.10, side * 1.10, PACK_FLOOR + 1.10),
            m["dark"], h, lods=MID)
        cyl(f"cable_reel_{s}", 0.22, 0.20, (PACK_FRONT + 0.40, side * 0.70, deck + 0.35), "Y", m["dark"], h,
            seg=16, lods=MID)
        box(f"cable_reel_bracket_{s}", (0.10, 0.08, 0.30), (PACK_FRONT + 0.30, side * 0.60, deck + 0.20),
            m["dark"], h, lods=FINE)
        for k, x in enumerate((PACK_REAR + 0.30, PACK_FRONT - 0.30)):
            box(f"lifting_eye_{s}_{k}", (0.10, 0.05, 0.12), (x, side * (CAN_W - 0.10),
                                                             PACK_FLOOR + 2 * CAN_H + 0.06), m["steel"], h,
                lods=FINE)
    # The antenna mast, telescoped down, upright at the pack's front corner.
    mast = empty("dressing_mast", parent=h)
    cyl("mast_tube", 0.07, 2.60, (PACK_FRONT + 0.55, -1.02, deck + 1.30), "Z", m["paint"], mast, seg=12)
    cyl("mast_head", 0.05, 0.60, (PACK_FRONT + 0.55, -1.02, deck + 2.90), "Z", m["steel"], mast, seg=10, lods=MID)
    VP.cable("pack_cable", [(PACK_FRONT + 0.40, 0.55, deck + 0.40), (PACK_FRONT + 0.20, 0.40, PACK_FLOOR - 0.20),
                            (PACK_FRONT - 0.10, 0.35, PACK_FLOOR + 0.20)], m, h, radius=0.03)


def canister(v, name, mid, y, z, length):
    """One canister: a box ringed by deep stiffening frames, its rear mouth
    closed by a cover inside a raised rim, a breech plate on its front."""
    m, h = v.mats, v.hull
    box(name, (length, CAN_W, CAN_H), (mid, y, z), m["paint"], h, bevel=0.03)
    for k in range(6):
        x = PACK_REAR + 0.35 + k * (length - 0.70) / 5
        box(f"{name}_rib_{k}", (0.10, CAN_W + 0.06, CAN_H + 0.04), (x, y, z), m["paint"], h, bevel=0.015,
            lods=MID)
    box(f"{name}_rim", (0.06, CAN_W - 0.04, CAN_H - 0.04), (PACK_REAR + 0.02, y, z), m["dark"], h, lods=MID)
    box(f"{name}_cover", (0.03, CAN_W - 0.14, CAN_H - 0.14), (PACK_REAR - 0.005, y, z), m["paint"], h, lods=MID)
    box(f"{name}_breech", (0.03, CAN_W - 0.20, CAN_H - 0.20), (PACK_FRONT + 0.005, y, z), m["dark"], h, lods=MID)


def jack(v, name, x, y, deck):
    """A stabiliser jack, raised for travel: its cylinder on a bracket off
    the frame, the foot pad drawn up under it."""
    m, h = v.mats, v.hull
    box(f"{name}_bracket", (0.30, 0.35, 0.25), (x, y * 0.85, deck - 0.10), m["dark"], h, bevel=0.02, lods=MID)
    cyl(f"{name}_cylinder", 0.15, 1.70, (x, y, deck + 0.05), "Z", m["paint"], h, seg=18)
    cyl(f"{name}_collar", 0.18, 0.12, (x, y, deck + 0.85), "Z", m["dark"], h, seg=18, lods=MID)
    cyl(f"{name}_ram", 0.10, 0.20, (x, y, deck - 0.88), "Z", m["steel"], h, seg=12, lods=MID)
    cyl(f"{name}_pad", 0.28, 0.07, (x, y, deck - 1.00), "Z", m["dark"], h, seg=18, lods=MID)


def ladder(v, name, loc, height):
    """A ladder folded upright: two stiles and their rungs."""
    m, h = v.mats, v.hull
    x, y, z = loc
    for k, dy in enumerate((-0.20, 0.20)):
        box(f"{name}_stile_{k}", (0.05, 0.04, height), (x, y + dy, z + height / 2), m["dark"], h, lods=MID)
    for k in range(6):
        box(f"{name}_rung_{k}", (0.03, 0.40, 0.03), (x, y, z + 0.15 + k * (height - 0.30) / 5), m["dark"], h,
            lods=NEAR)


def wreck(variant, v):
    """The launcher after its fire: the right side's rear wheels gone and
    the truck down on that corner, the upper canisters burst and the pack
    warped, the cab's glass out; what it throws is `wreckage.scatter`'s."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, remove, warp
    remove("wheel_R_3_", "wheel_R_4_", "canister_1_1_cover", "canister_1_0_cover", "windscreen", "door_window_",
           "rear_window_", "dressing_mirror_R", "mirror_R", "mirror_arm_R", "mudflap_R", "pack_rail_R",
           "module_rail_R", "module_post_R_")
    shell = parts("canister_", "module_body", "cab_body", "launcher_beam")
    densify(shell, scale=2.0)
    warp(shell, heat(0.03, 0.9, seed=27.0), dent((-2.5, -0.9, 3.7), 0.9, 0.18, (0, 1, -0.4)))
    v.root.rotation_euler = (0.012, -0.02, 0)
    v.root.location.z -= 0.06
    rest_on_ground(0.004)


if __name__ == "__main__":
    run("patriot", "us_desert_tan", build, wreck, chip=0.8)
