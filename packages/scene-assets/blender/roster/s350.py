"""S-350 Vityaz: the 50P6 self-propelled launcher in travel order, from
assets/references/s350/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/s350.py -- [--wreck]

What the photos settle (the export 50P6E at MAKS 2013, Russian green):
- the BAZ-6909 8x8: four axles in two pairs, a big gap between them; the
  cab-forward cab spanning the width, flat-fronted and long, with two tall
  windscreens; equipment housings behind it, one tall, one lower with a
  raised compartment on its right;
- the canister box stowed flat over the rear from behind the housings to the
  tail: twelve light grey canisters in two rows of six, red transport caps
  forward, held in an olive lattice frame of rails and posts in two tiers;
  a light frame round the mouths, a heavy end frame at the tail over the
  elevation hinge, the cradle's rails under it;
- side outriggers stowed upright behind the housings, two rear outriggers
  under the tail.

The box is drawn stowed and does not articulate yet: it raises about the
hinge at the tail (the mount's pivot), mouths up.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, textured  # noqa: E402
from truck_chassis import chassis  # noqa: E402
from wreckage import remove  # noqa: E402
from vehicle_export import run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

# Each appearance's family (its source folder and reference library) and scheme.
CARDS = {
    "eastern_s350_long": ("s350", "russian_green"),
}

# The canister box, in the hull frame: its mouths' face, its tail, the two
# tiers' centres and the canisters' spacing across.
BOX_FRONT, BOX_TAIL = 0.45, -5.58
TIERS = (2.45, 3.13)
PITCH = 0.42
# The heavy frame along the box and the front of the tail's end frame (x).
FRAME_X = -1.50
TAIL_FRAME = -4.65
CAN_R = 0.19


def canister_mats(v):
    """The canisters' light grey and their red transport caps: painted, as
    the photos show them, apart from the scheme."""
    return dict(v.mats,
                canister=textured("canister_grey", "olive_paint", colour=(0.30, 0.31, 0.31), chip=0.1, dirt=0.3,
                                  streak=0.0, role="paint"),
                cap=textured("canister_cap_red", "olive_paint", colour=(0.42, 0.025, 0.02), chip=0.05, dirt=0.15,
                             streak=0.0, role="paint"))


def baz(v):
    """The BAZ-6909 under the launcher: chassis, cab and the housings behind
    it. Returns the deck's height."""
    m, h = v.mats, v.hull
    deck = chassis(v, [3.45, 1.35, -2.45, -4.0], 0.62, 0.50, 1.10, 1.10, "cabover", 3.55, 1.95, 2.85)
    # The tail carries the rear outriggers where a truck's tow hook would be.
    remove("rear_tow")
    # The tall housing right behind the cab: louvred sides, a ladder up its back.
    box("housing_front", (1.10, 2.50, 1.62), (2.95, 0, deck + 0.81), m["paint"], h, bevel=0.04)
    for side in (-1, 1):
        VP.grille(f"housing_louvre_{side}", (3.00, side * 1.255, deck + 1.05), (0.70, 0.55), m, h, slats=7,
                  rot=(-side * math.pi / 2, 0, 0))
    # The lower housing with the raised compartment on its right, a door each side.
    box("housing_rear", (1.75, 2.40, 1.00), (1.45, 0, deck + 0.50), m["paint"], h, bevel=0.03)
    box("housing_raised", (1.25, 1.05, 0.72), (1.40, -0.62, deck + 1.36), m["paint"], h, bevel=0.04)
    for side, s in ((1, "L"), (-1, "R")):
        VP.bolted_panel(f"housing_door_{s}", (1.45, side * 1.205, deck + 0.50), (0.90, 0.70, 0.03), m, h,
                        bolts=(3, 2), rot=(-side * math.pi / 2, 0, 0), bevel=0.01)
        VP.stowage_box(f"step_box_{s}", (0.0, side * 1.10, deck - 0.45), (1.10, 0.30, 0.40), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        # The side outrigger stowed upright behind the housings, its pad clear of the ground.
        box(f"outrigger_{s}", (0.18, 0.18, 1.15), (2.30, side * 1.20, deck - 0.30), m["dark"], h, bevel=0.01)
        cyl(f"outrigger_pad_{s}", 0.16, 0.05, (2.30, side * 1.20, deck - 0.90), "Z", m["dark"], h, seg=16, lods=MID)
        # The rear outriggers under the tail.
        box(f"rear_outrigger_{s}", (0.18, 0.18, 1.05), (-5.30, side * 0.95, deck - 0.35), m["dark"], h, bevel=0.01)
        cyl(f"rear_outrigger_pad_{s}", 0.26, 0.05, (-5.30, side * 0.95, deck - 0.88), "Z", m["dark"], h, seg=16,
            lods=MID)
    # The hangers the cab's steps hang from, down from its floor.
    for side, s in ((1, "L"), (-1, "R")):
        box(f"step_hanger_{s}", (0.06, 0.06, 0.80), (5.5 - 0.70 - 0.32, side * 1.20, 0.80), m["dark"], h, lods=MID)
        box(f"step_hanger_rear_{s}", (0.06, 0.06, 0.80), (5.5 - 0.70 + 0.12, side * 1.20, 0.80), m["dark"], h,
            lods=MID)
    for k in range(4):
        box(f"ladder_rung_{k}", (0.04, 0.50, 0.04), (2.38, 0.55, deck + 0.30 + k * 0.32), m["steel"], h, lods=FINE)
    VP.fuel_tank("housing_extinguisher", (2.90, 0.90, deck + 1.80), 0.45, 0.08, m, h, rot=(0, 0, math.pi / 2))
    # The amber beacon on the cab roof.
    cyl("roof_beacon", 0.09, 0.16, (5.20, 0.70, 2.93), "Z", m["lamp"], h, seg=12, lods=MID)
    return deck


def launcher(v, deck):
    """The canister box stowed flat over the rear, on its cradle."""
    m, h = canister_mats(v), v.hull
    length = BOX_FRONT - BOX_TAIL
    mid = (BOX_FRONT + BOX_TAIL) / 2
    edge = 1.31
    # The cradle's rails on the deck and the hinge at the tail.
    for side in (-1, 1):
        box(f"cradle_rail_{side}", (length - 0.2, 0.16, 0.42), (mid, side * 0.90, deck + 0.40), m["dark"], h,
            bevel=0.015)
        cyl(f"hinge_{side}", 0.16, 0.30, (-4.90, side * 0.90, 1.85), "Y", m["steel"], h, seg=16)
    for k, x in enumerate((-0.2, -2.4, -4.2)):
        box(f"cradle_cross_{k}", (0.20, 1.90, 0.18), (x, 0, deck + 0.52), m["dark"], h, lods=MID)
    lo, hi = TIERS[0] - 0.36, TIERS[1] + 0.36
    box("box_floor", (length, 2.62, 0.10), (mid, 0, lo), m["paint"], h, bevel=0.01)
    # Twelve canisters, two rows of six, red caps forward.
    for j, z in enumerate(TIERS):
        for i in range(6):
            y = (i - 2.5) * PITCH
            cyl(f"canister_{j}_{i}", CAN_R, length - 0.35, (mid + 0.05, y, z), "X", m["canister"], h, seg=16)
            cyl(f"canister_cap_{j}_{i}", CAN_R * 0.95, 0.06, (BOX_FRONT - 0.13, y, z), "X", m["cap"], h, seg=16,
                lods=MID)
            cyl(f"canister_band_{j}_{i}", CAN_R + 0.02, 0.10, (BOX_FRONT - 0.32, y, z), "X", m["canister"], h, seg=16,
                lods=NEAR)
    # Each tier's side: deep rails top and bottom with posts between, the
    # canisters showing through the windows they leave.
    for side, s in ((1, "L"), (-1, "R")):
        for j, z in enumerate(TIERS):
            for k, dz in enumerate((-0.26, 0.26)):
                box(f"rail_{s}_{j}_{k}", (length, 0.09, 0.16), (mid, side * edge, z + dz), m["paint"], h,
                    bevel=0.015)
            for p, x in enumerate(_posts()):
                box(f"post_{s}_{j}_{p}", (0.14, 0.09, 0.40), (x, side * edge, z), m["paint"], h, lods=MID)
        # The heavy frame a third of the way back, spanning both tiers.
        box(f"frame_post_{s}", (0.34, 0.16, hi - lo + 0.10), (FRAME_X, side * edge, (lo + hi) / 2),
            m["paint"], h, bevel=0.03)
        # The mouths' frame, and the tail's deep end frame over the hinge: a
        # post at each end of it, plates top and bottom between.
        box(f"mouth_post_{s}", (0.26, 0.15, hi - lo + 0.06), (BOX_FRONT - 0.13, side * (edge + 0.01), (lo + hi) / 2),
            m["paint"], h, bevel=0.02)
        for k, x in enumerate((TAIL_FRAME + 0.15, BOX_TAIL + 0.15)):
            box(f"tail_post_{s}_{k}", (0.30, 0.20, hi - lo + 0.18), (x, side * (edge - 0.02), (lo + hi) / 2),
                m["paint"], h, bevel=0.03)
        for k, z in enumerate((lo + 0.05, (TIERS[0] + TIERS[1]) / 2, hi - 0.02)):
            box(f"tail_side_{s}_{k}", (TAIL_FRAME - BOX_TAIL, 0.18, 0.22), ((TAIL_FRAME + BOX_TAIL) / 2,
                                                                         side * (edge - 0.02), z), m["paint"], h,
                bevel=0.02)
    for k, z in enumerate((lo, (TIERS[0] + TIERS[1]) / 2, hi)):
        box(f"mouth_bar_{k}", (0.14, 2.70, 0.12), (BOX_FRONT - 0.10, 0, z), m["paint"], h, bevel=0.01, lods=MID)
        box(f"tail_beam_{k}", (0.30, 2.62, 0.20), (BOX_TAIL + 0.15, 0, z), m["paint"], h, bevel=0.02)
    for k in range(2):
        box(f"tail_strut_{k}", (0.26, 0.20, hi - lo), (BOX_TAIL + 0.15, (k - 0.5) * 1.0, (lo + hi) / 2),
            m["paint"], h, bevel=0.02, lods=MID)
    # Cross-members over the upper tier, the tail frame's top plate.
    for k, x in enumerate((BOX_FRONT - 0.13, FRAME_X)):
        box(f"top_bar_{k}", (0.20, 2.70, 0.10), (x, 0, hi + 0.04), m["paint"], h, bevel=0.01)
    box("tail_top", (TAIL_FRAME - BOX_TAIL + 0.1, 2.66, 0.08), ((TAIL_FRAME + BOX_TAIL) / 2, 0, hi + 0.04),
        m["paint"], h, bevel=0.01)
    # The top's lattice: rails along it between the cross-members.
    for k, y in enumerate((-0.66, 0.0, 0.66)):
        box(f"top_rail_{k}", (TAIL_FRAME - BOX_FRONT + 0.25, 0.12, 0.08), ((TAIL_FRAME + BOX_FRONT) / 2, y, hi + 0.04),
            m["paint"], h, bevel=0.01, lods=MID)
    # The tail closed behind the canisters' bases: its plate, the junction
    # boxes on it and the cable runs between them.
    box("tail_plate", (0.04, 2.56, hi - lo), (BOX_TAIL + 0.02, 0, (lo + hi) / 2), m["paint"], h)
    for k, (y, z) in enumerate(((-0.62, TIERS[0]), (0.62, TIERS[0]), (-0.62, TIERS[1]), (0.62, TIERS[1]))):
        VP.stowage_box(f"tail_junction_{k}", (BOX_TAIL - 0.06, y, z - 0.18), (0.12, 0.70, 0.36), m, h)
    for k, y in enumerate((-0.25, 0.25)):
        VP.cable(f"tail_cable_{k}", [(BOX_TAIL - 0.05, y, hi - 0.10), (BOX_TAIL - 0.08, y * 1.4, (lo + hi) / 2),
                                     (BOX_TAIL - 0.05, y, lo + 0.10)], m, h, radius=0.025, eyes=False)


def _posts():
    """The posts' x along a tier: between the mouths' frame and the heavy
    frame, then on to the tail's end frame, at about the photos' spacing."""
    spans = ((BOX_FRONT - 0.26, FRAME_X + 0.17, 3), (FRAME_X - 0.17, TAIL_FRAME + 0.30, 5))
    return [a + (b - a) * k / n for a, b, n in spans for k in range(1, n)]


def build(variant, v):
    deck = baz(v)
    launcher(v, deck)
    whip = empty("dressing_antenna_cab_rear", parent=v.hull)
    VP.antenna("antenna_housing", (2.45, 1.05, 2.87), v.mats, whip, height=1.2)


def wreck(variant, v):
    """The launcher after its fire: the right rear wheels gone and that side
    down, the cab's glass out, canisters burst out of the box and the frame
    and cab warped; what it throws is `wreckage.scatter`'s."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, warp
    remove("wheel_R_3", "wheel_R_4", "mudflap_R", "windscreen", "door_window_", "rear_window_", "dressing_mirror_R",
           "mirror_R", "mirror_arm_R", "canister_cap_", "canister_1_0", "canister_1_1", "canister_band_1_0",
           "canister_band_1_1", "canister_0_0", "canister_band_0_0", "post_R_1_", "rail_R_1_1")
    shell = parts("cab_body", "housing_front", "housing_rear", "housing_raised", "canister_", "rail_", "box_floor")
    densify(shell, scale=2.0)
    warp(shell, heat(0.03, 0.8, seed=35.0), dent((-2.0, -1.3, 2.9), 0.9, 0.18, (0, 1, -0.3)),
         dent((4.6, 0.6, 2.8), 0.6, 0.12, (0, 0, -1)))
    v.root.rotation_euler = (0.02, 0.0, 0)
    v.root.location.z -= 0.06
    rest_on_ground(0.004)


if __name__ == "__main__":
    run("s350", {c: scheme for c, (_, scheme) in CARDS.items()}, build, wreck,
        appearances=list(CARDS), script="s350.py",
        receipt_file="assets/source/roster/s350/s350.source-receipt.json",
        references=[f"assets/references/{family}/references.json" for family, _ in CARDS.values()], chip=1.0)
