"""S-300PMU2 Favorit: its 5P85SE2 self-propelled launcher, from
assets/references/s300/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/s300.py -- [--wreck]

What the photos settle:
- The chassis is the MAZ-543M 8x8 the Tornado-S stands on
  (`rocket_artillery.maz_543m`): four axles in two pairs, the split cabs
  either side of the engine and its grille.
- Behind the cabs: the boxed equipment enclosures (the power unit with its
  louvres on the right, the electronics on the left), a whip antenna, and a
  bent pipe guard rail rising off each cab's roof.
- The rear deck carries the erector: a beam between two pairs of large
  containers, each ringed by hoop ribs, its base domed, hinged at the
  chassis' rear. Every photo shows the containers raised; travelling, they
  lie along the deck in two rows of two, their bases overhanging the rear
  and their capped tops forward, resting on a frame behind the enclosures.
- Hydraulic outriggers stand stowed beside the frame between the axle pairs
  and at the rear corners.

The frame is the travelling launcher's length, width and height. The
published 13.11 m (5P85S, karopka.ru's figures for the MAZ-543M launcher:
13.11 x 3.15 x 3.8 m) runs past the simulation's longest wheeled hull
(`contract::HullLimits::SHIPPED`), so the card is that limit's length and the containers' overhang
past the chassis is that much shorter. The launcher is a fixed mount drawn
stowed: its catalog pivot is the erector's hinge (`HINGE`), its muzzle the
containers' tops.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, textured  # noqa: E402
import rocket_artillery as RA  # noqa: E402
from truck_chassis import on  # noqa: E402
from vehicle_export import run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

# The containers: two columns either side of the erector beam, two rows.
TUBE_R, TUBE_LEN = 0.40, 7.5
TUBE_Y = 0.62
TUBE_Z = (2.56, 3.36)
HINGE = (-5.6, 1.75)
# The containers' paint: a pale grey-green, paler than the truck's, as every photo
# shows them.
CONTAINER_GREEN = (0.30, 0.32, 0.27)


def maz(v):
    """The MAZ-543M set back so its front tow hooks are the frame's front:
    the launcher's containers overhang its rear. Returns (the chassis'
    offset along x, the deck's height)."""
    dx = v.length / 2 - RA.MAZ_543M_LENGTH / 2 - RA.MAZ_543M_TOW_REACH
    return dx, RA.maz_543m(on(v, dx, RA.MAZ_543M_LENGTH))


def enclosures(v, cab_rear, roof, deck):
    """The equipment behind the cabs: the power unit (right) with its louvres
    and exhaust, the electronics enclosure (left) with its bolted doors, the
    whip antenna, the guard rails off the cab roofs, the cab's beacon and
    searchlight."""
    m, h = v.mats, v.hull
    x0 = cab_rear - 0.10
    length = 1.70
    x = x0 - length / 2
    box("power_unit", (length, 1.15, 1.75), (x, -0.78, deck + 0.875), m["paint"], h, bevel=0.04)
    for k in range(2):
        VP.grille(f"power_louvre_{k}", (x - 0.40 + k * 0.80, -1.36, deck + 0.85), (0.60, 0.70), m, h, slats=7,
                  rot=(math.pi / 2, 0, 0))
    VP.exhaust("power_exhaust", (x + 0.45, -0.55, deck + 1.75), 0.09, 0.30, m, h, rot=(0, -math.pi / 2, 0))
    box("electronics_box", (length, 1.15, 1.70), (x, 0.78, deck + 0.85), m["paint"], h, bevel=0.04)
    box("electronics_top", (length - 0.40, 0.80, 0.25), (x + 0.10, 0.70, deck + 1.82), m["paint"], h, bevel=0.03)
    for k in range(2):
        VP.bolted_panel(f"electronics_door_{k}", (x - 0.40 + k * 0.80, 1.36, deck + 0.75), (0.70, 1.05, 0.03), m, h,
                        bolts=(2, 3), rot=(-math.pi / 2, 0, 0), bevel=0.01)
    box("enclosure_step", (0.30, 0.50, 0.05), (x0 - length - 0.10, 0.85, deck + 0.40), m["steel"], h, lods=MID)
    whip = empty("dressing_antenna", parent=h)
    VP.antenna("antenna", (x - 0.55, 1.20, deck + 1.95), m, whip, height=2.4)
    # The guard rails: a bent pipe rising off each cab's roof to the
    # enclosures' top, as the photos show over both cabs.
    for side, s in ((1, "L"), (-1, "R")):
        y = side * 1.05

        def rail(lod, y=y):
            return [(cab_rear + 1.60, y, roof + 0.02), (cab_rear + 0.90, y, roof + 0.62),
                    (cab_rear - 0.05, y, roof + 0.62), (cab_rear - 0.25, y, deck + 1.70)]

        VP.tube_part(f"guard_rail_{s}", rail, 0.035, m["paint"], h, MID)
    VP.tube_part("guard_rail_cross", lambda lod: [(cab_rear + 0.20, 1.05, roof + 0.62),
                                                  (cab_rear + 0.20, -1.05, roof + 0.62)], 0.03, m["paint"], h,
                 NEAR)
    cyl("beacon", 0.07, 0.14, (cab_rear + 1.0, 0.65, roof + 0.09), "Z", m["tail"], h, seg=12, lods=MID)
    VP.light_with_guard("searchlight", (cab_rear + 1.4, 1.00, roof + 0.12), 0.10, m, h)


def container(v, name, y, z, x0, x1):
    """One container lying along the deck from its domed base at `x0` to its
    capped top at `x1` inside its rim, ringed by hoop ribs."""
    m, h = v.mats, v.hull
    paint = m["container"]
    base = 0.18
    cyl(f"{name}_body", TUBE_R, x1 - x0 - base, ((x0 + base + x1) / 2, y, z), "X", paint, h, seg=20)
    cyl(f"{name}_base", TUBE_R, base, (x0 + base / 2, y, z), "X", paint, h, seg=20, r2=TUBE_R * 0.75, lods=MID)
    cyl(f"{name}_cap", TUBE_R * 0.92, 0.03, (x1 + 0.01, y, z), "X", paint, h, seg=20, lods=MID)
    cyl(f"{name}_top_rim", TUBE_R + 0.025, 0.08, (x1 - 0.04, y, z), "X", paint, h, seg=20, lods=MID)
    ribs = 12
    for k in range(ribs):
        rx = x0 + base + 0.35 + (x1 - x0 - base - 0.70) * k / (ribs - 1)
        cyl(f"{name}_rib_{k}", TUBE_R + 0.03, 0.06, (rx, y, z), "X", paint, h, seg=20,
            lods=NEAR if k % 2 else MID)


def launcher(v, deck):
    """The erector and its four containers, stowed: the hinge at the
    chassis' rear, the beam between the columns, clamp bands, the elevating
    ram, and the travel rest under the containers' tops."""
    m, h = v.mats, v.hull
    half = v.length / 2
    x0, x1 = -half, -half + TUBE_LEN
    hx, hz = HINGE
    for j, z in enumerate(TUBE_Z):
        for i, side in enumerate((1, -1)):
            container(v, f"container_{j}_{i}", side * TUBE_Y, z, x0, x1)
    mid = sum(TUBE_Z) / 2
    # The erector beam between the columns, its lattice face to each side.
    box("erector_beam", (x1 - hx - 0.40, 0.30, TUBE_Z[1] - TUBE_Z[0] + 0.30), ((hx + x1 - 0.40) / 2, 0, mid),
        m["dark"], h, bevel=0.02)
    for k in range(9):
        bx = hx + 0.40 + k * (x1 - hx - 1.20) / 8
        for side in (-1, 1):
            box(f"erector_lattice_{side}_{k}", (0.05, 0.02, 1.0), (bx, side * 0.16, mid), m["paint"], h,
                rot=(0, 0.6 if k % 2 else -0.6, 0), lods=FINE)
    # Clamp bands round the pack, each with its yoke over the beam.
    pack_w, pack_h = 2 * (TUBE_Y + TUBE_R) + 0.10, TUBE_Z[1] - TUBE_Z[0] + 2 * TUBE_R + 0.08
    for k, bx in enumerate((hx + 0.6, (hx + x1) / 2, x1 - 0.9)):
        box(f"clamp_band_{k}", (0.12, pack_w, 0.08), (bx, 0, TUBE_Z[1] + TUBE_R + 0.01), m["paint"], h, lods=MID)
        box(f"clamp_band_low_{k}", (0.12, pack_w, 0.08), (bx, 0, TUBE_Z[0] - TUBE_R - 0.01), m["paint"], h,
            lods=MID)
        for side in (-1, 1):
            box(f"clamp_side_{side}_{k}", (0.12, 0.05, pack_h), (bx, side * pack_w / 2, mid), m["paint"], h,
                lods=MID)
    # The hinge at the chassis' rear and the cradle under the pack.
    low = TUBE_Z[0] - TUBE_R - 0.06
    for side in (-1, 1):
        box(f"hinge_bracket_{side}", (0.60, 0.10, low - deck + 0.10), (hx, side * 0.45, (low + deck) / 2), m["dark"],
            h, bevel=0.01)
    cyl("hinge_pin", 0.10, 1.10, (hx, 0, hz), "Y", m["steel"], h, seg=14, lods=MID)
    # The erector's cradle: two deep girders under the pack from the hinge
    # forward, cross-members between them.
    cx, clen = (hx + x1) / 2 - 0.30, x1 - hx - 0.60
    for side in (-1, 1):
        box(f"erector_girder_{side}", (clen, 0.16, 0.42), (cx, side * 0.70, low - 0.12), m["paint"], h, bevel=0.02)
    for k in range(4):
        box(f"erector_cross_{k}", (0.16, 1.30, 0.20), (hx + 0.4 + k * (clen - 0.6) / 3, 0, low - 0.05), m["dark"], h,
            lods=MID)
    # The elevating ram, folded from its foot on the deck up to the cradle.
    foot, head = (-0.10, deck + 0.20), (-3.40, low - 0.30)
    reach = math.hypot(head[0] - foot[0], head[1] - foot[1])
    pitch = math.atan2(head[1] - foot[1], foot[0] - head[0])
    for name, r, f0, f1, mat in (("ram_barrel", 0.17, 0.0, 0.55, m["dark"]), ("ram_rod", 0.09, 0.5, 1.0, m["steel"])):
        t = (f0 + f1) / 2
        cyl(name, r, reach * (f1 - f0), (foot[0] + (head[0] - foot[0]) * t, 0, foot[1] + (head[1] - foot[1]) * t), "X",
            mat, h, seg=14, rot=(0, -pitch, 0))
    box("ram_foot", (0.40, 0.50, 0.20), (foot[0], 0, deck + 0.10), m["dark"], h, bevel=0.02, lods=MID)
    # The travel rest: an A-frame under the containers' tops.
    for side in (-1, 1):
        box(f"travel_rest_leg_{side}", (0.12, 0.12, low - deck + 0.10), (x1 - 0.55, side * 0.85, (low + deck) / 2),
            m["dark"], h, rot=(side * 0.25, 0, 0), lods=MID)
    box("travel_rest_bar", (0.16, 1.90, 0.12), (x1 - 0.55, 0, low - 0.05), m["dark"], h, bevel=0.01)


def deck_fittings(v, dx, deck):
    """The deck and what it carries along the sides: lockers between the
    axle pairs and behind the rear axle, a cable reel, the outriggers stowed
    beside the frame."""
    m, h = v.mats, v.hull
    axles = [a + dx for a in RA.MAZ_543M_AXLES]
    rear = dx - RA.MAZ_543M_LENGTH / 2
    box("deck_plate", (axles[1] - rear - 0.55, 2.50, 0.10), ((axles[1] - 0.55 + rear) / 2, 0, deck + 0.05),
        m["paint"], h, bevel=0.01)
    mid = (axles[1] + axles[2]) / 2
    for side, s in ((1, "L"), (-1, "R")):
        # Lockers hung under the deck's edge between the axle pairs.
        VP.stowage_box(f"locker_{s}", (mid, side * 1.15, 0.70), (1.70, 0.40, 0.70), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        VP.stowage_box(f"rear_locker_{s}", ((axles[3] + rear) / 2 - 0.25, side * 1.15, 0.80), (0.80, 0.40, 0.50), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        # Outriggers: a jack between each axle pair and at the rear corners,
        # stowed up, their pads clear of the ground.
        for k, x in enumerate(((axles[0] + axles[1]) / 2, (axles[2] + axles[3]) / 2, rear + 0.35)):
            y = side * 1.38
            box(f"outrigger_arm_{s}_{k}", (0.20, 0.80, 0.18), (x, side * 1.00, deck - 0.12), m["dark"], h,
                bevel=0.01, lods=MID)
            cyl(f"outrigger_jack_{s}_{k}", 0.12, 0.60, (x, y, deck - 0.20), "Z", m["dark"], h, seg=14)
            cyl(f"outrigger_rod_{s}_{k}", 0.06, 0.12, (x, y, deck - 0.56), "Z", m["steel"], h, seg=10, lods=MID)
            cyl(f"outrigger_pad_{s}_{k}", 0.24, 0.06, (x, y, deck - 0.64), "Z", m["dark"], h, seg=16, lods=MID)
    cyl("cable_reel", 0.20, 0.30, (rear + 1.20, -0.90, deck + 0.30), "Y", m["dark"], h, seg=16, lods=MID)
    VP.stowage_box("deck_box", (axles[2] - 0.30, 0.95, deck + 0.10), (1.10, 0.36, 0.40), m, h)


def build(variant, v):
    v.mats["container"] = textured("containers", "olive_paint", colour=CONTAINER_GREEN, chip=0.3, dirt=0.35,
                                   role="paint")
    dx, deck = maz(v)
    cab_x, roof = RA.MAZ_543M_CAB
    enclosures(v, dx + cab_x, roof, deck)
    deck_fittings(v, dx, deck)
    launcher(v, deck)


def wreck(variant, v):
    """The launcher after its fire: the right side's middle wheels burnt off
    and the chassis down on that side, the cabs' glass gone, the upper right
    container burst from its clamps (what it throws is `wreckage.scatter`'s),
    the enclosures and cabs warped."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, remove, warp
    remove("wheel_R_2_", "wheel_R_3_", "windscreen", "door_window_", "mudflap_R", "container_1_1_", "clamp_band_1",
           "dressing_mirror_R", "mirror_R", "mirror_arm_R", "beacon", "searchlight")
    shell = parts("cab_L", "cab_R", "engine_hood", "power_unit", "electronics_box", "container_0_0_body",
                  "container_0_1_body", "container_1_0_body")
    densify(shell, scale=2.0)
    warp(shell, heat(0.03, 0.8, seed=93.0), dent((v.length / 2 - 3.2, -1.0, 2.4), 0.7, 0.14, (0, 1, -0.2)))
    v.root.rotation_euler = (0.035, 0.01, 0)
    v.root.location.z -= 0.08
    rest_on_ground(0.004)


if __name__ == "__main__":
    run("s300", "russian_green", build, wreck, chip=1.0)
