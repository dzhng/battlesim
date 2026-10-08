"""CAESAR 8x8 self-propelled howitzer (disabled card), from assets/references/caesar/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/caesar.py -- [--wreck]

What the photos settle (Danish Air Show, Ukrainian service 2024): the
Tatra 8x8 chassis, its four axles in two pairs; the long armoured crew cab
over the front axles, two doors a side with small thick windows; behind it the ammunition and equipment lockers along
both sides; the 155 mm 52-calibre gun on its cradle at the rear, travelling
forward over the cab on a rest, with its pepperpot muzzle brake, the cradle on its trunnions
with recoil cylinders, equilibrators and elevating sector; the recoil
spade folded up under the tail on its rams, with the raised rear jacks. French three-tone (the card is Europe's
CAESAR, French-built; the Danish car is green).

Its frame is the chassis's length,
width and height to its cab roof (10.0 x 2.55 x 3.2 m), measured off the side
photo against its 1.16 m tyres, recorded as `references`; the gun travelling
over the cab is its mount, outside the frame as a tank's gun is. The gun is drawn travelling and does not articulate yet.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, prism  # noqa: E402
from truck_chassis import chassis  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

CARD = "europe_caesar_88"


def build(variant, v):
    m, h = v.mats, v.hull
    half = v.length / 2
    deck = chassis(v, [3.45, 2.00, -1.45, -2.85], 0.58, 0.42, 1.02, 1.05, "armoured", 1.90, 3.0, 3.10, doors=2)
    # Lockers along both sides behind the cab: hinged doors, their latches,
    # the ammunition racks' louvres; a walkway between them.
    for side, s in ((1, "L"), (-1, "R")):
        for k in range(3):
            VP.stowage_box(f"locker_{s}_{k}", (1.25 - k * 1.05, side * 0.95, deck), (0.95, 0.55, 0.95), m, h,
                           rot=(0, 0, 0 if side > 0 else math.pi))
            for j in range(4):
                box(f"locker_louvre_{s}_{k}_{j}", (0.70, 0.02, 0.03), (1.25 - k * 1.05, side * 1.235,
                                                                     deck + 0.30 + j * 0.10), m["dark"], h, lods=FINE)
        box(f"walkway_rail_{s}", (3.2, 0.04, 0.04), (0.20, side * 0.62, deck + 1.20), m["steel"], h, lods=NEAR)
        for k, x in enumerate((1.75, -1.35)):
            box(f"walkway_post_{s}_{k}", (0.04, 0.04, 0.25), (x, side * 0.62, deck + 1.07), m["steel"], h, lods=FINE)
        # The rear stabiliser jack, raised, beside the spade.
        box(f"jack_leg_{s}", (0.16, 0.16, 0.80), (-half + 0.70, side * 0.95, deck - 0.30), m["dark"], h, bevel=0.01)
        box(f"jack_pad_{s}", (0.36, 0.36, 0.06), (-half + 0.70, side * 0.95, deck - 0.72), m["dark"], h, lods=MID)
        for k in range(3):
            box(f"rear_step_{s}_{k}", (0.24, 0.32, 0.03), (-half + 0.12, side * 0.85, 0.50 + k * 0.30), m["steel"], h,
                lods=FINE)
    box("deck_plate", (5.6, 2.3, 0.08), (-0.9, 0, deck + 0.04), m["paint"], h, bevel=0.01)
    # The gun's cradle at the rear on its trunnions, the barrel forward over
    # the cab on its rest: breech ring, cradle with the recoil cylinders
    # under the barrel, the two equilibrators, the elevating sector.
    cx = -2.95
    gz = 3.28  # the gun travels over the cab roof (3.10)
    box("gun_carriage", (1.20, 1.50, 0.90), (cx, 0, deck + 0.50), m["paint"], h, bevel=0.04)
    cyl("carriage_ring", 0.68, 0.10, (cx, 0, deck + 1.00), "Z", m["dark"], h, seg=28, lods=MID)
    for side in (-1, 1):
        box(f"cradle_cheek_{side}", (0.90, 0.10, 1.00), (cx + 0.10, side * 0.45, deck + 1.40), m["paint"], h,
            bevel=0.02)
        cyl(f"trunnion_{side}", 0.11, 0.10, (cx + 0.20, side * 0.52, gz - 0.10), "Y", m["steel"], h, seg=14, lods=MID)
        cyl(f"recuperator_{side}", 0.09, 1.60, (cx + 0.80, side * 0.22, gz - 0.18), "X", m["dark"], h, seg=14)
        cyl(f"equilibrator_{side}", 0.07, 1.10, (cx + 0.35, side * 0.58, deck + 1.35), "X", m["dark"], h, seg=12,
            rot=(0, -0.75, 0), lods=MID)
        arc = [(cx - 0.10 + 0.75 * math.cos(a), deck + 1.20 + 0.75 * math.sin(a)) for a in
               [math.radians(-100 + 12 * k) for k in range(7)]]
        prism(f"elevating_sector_{side}", arc + [(cx - 0.10, deck + 1.20)], 0.05, loc=(0, side * 0.38, 0),
              mat=m["dark"], parent=h, lods=NEAR)
    box("cradle", (1.80, 0.40, 0.30), (cx + 0.85, 0, gz - 0.25), m["paint"], h, bevel=0.03)
    length = 8.06
    start = cx - 0.60
    end = start + length
    box("gun_breech", (0.70, 0.46, 0.46), (start + 0.40, 0, gz), m["steel"], h, bevel=0.04)
    box("gun_breech_block", (0.10, 0.32, 0.32), (start + 0.02, 0, gz), m["dark"], h, bevel=0.02, lods=MID)
    cyl("gun_barrel", 0.10, length - 1.20, ((start + 0.80 + end - 0.40) / 2, 0, gz), "X", m["paint"], h, seg=20,
        r2=0.085)
    cyl("gun_collar", 0.13, 0.12, (start + 1.95, 0, gz), "X", m["dark"], h, seg=20, lods=MID)
    cyl("muzzle_brake", 0.15, 0.55, (end - 0.27, 0, gz), "X", m["dark"], h, seg=18, bevel=0.015)
    for k in range(5):
        for side in (-1, 1):
            cyl(f"brake_port_{k}_{side}", 0.030, 0.02, (end - 0.48 + k * 0.10, side * 0.15, gz), "Y", m["black"], h,
                seg=8, lods=FINE)
    cyl("muzzle_bore", 0.078, 0.012, (end + 0.005, 0, gz), "X", m["black"], h, seg=16, lods=MID)
    # The rest on the cab's roof and its clamp round the barrel.
    box("travel_rest", (0.16, 0.40, gz - 3.10), (2.4, 0, (gz + 3.10) / 2 - 0.08), m["dark"], h, lods=MID)
    box("gun_travel_clamp", (0.14, 0.30, 0.10), (2.4, 0, gz + 0.12), m["dark"], h, bevel=0.02, lods=MID)
    # The recoil spade folded up under the tail.
    VP.bolted_panel("recoil_spade", (-half + 0.25, 0, 0.55), (0.12, 1.60, 0.60), m, h, bolts=(1, 4), bevel=0.02,
                    rot=(0, -0.35, 0), lods=VP.ALL)
    for side in (-1, 1):
        cyl(f"spade_ram_{side}", 0.06, 0.70, (-half + 0.55, side * 0.55, 0.85), "X", m["steel"], h, seg=10,
            rot=(0, 0.6, 0), lods=MID)


def wreck(variant, v):
    """The CAESAR after its fire: the cab burnt out and its glass gone, the
    left front wheels off and the nose down on that corner, the lockers
    burst, the barrel drooping off its rest."""
    from parts import rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, remove, warp
    remove("wheel_L_1_", "wheel_L_2_", "windscreen", "door_window_", "rear_window_", "locker_L_1", "mudflap_L",
           "walkway_", "dressing_mirror_L", "mirror_L", "mirror_arm_L")
    bend(parts("gun_barrel", "gun_collar", "muzzle_", "brake_port_"), (-3.55, 0, 3.28), (0, 1, 0), (0, 0, 1), -0.06)
    shell = parts("cab_body", "locker_", "gun_carriage")
    densify(shell, scale=2.0)
    warp(shell, heat(0.025, 0.8, seed=91.0), dent((3.4, 1.0, 2.4), 0.6, 0.15, (0, 0, -1)))
    v.root.rotation_euler = (-0.035, 0.035, 0)
    v.root.location.z -= 0.06
    rest_on_ground(0.004)


if __name__ == "__main__":
    run_disabled("caesar", {CARD: (10.0, 2.55, 3.20)}, "french_three_tone", build, wreck, skip=("dressing_", "gun_", "fume_", "muzzle_", "brake_"), chip=1.0)
