"""CAESAR 8x8 self-propelled howitzer (disabled card), from assets/references/caesar/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/caesar.py -- [--wreck]

What the photos settle (Danish Air Show, Ukrainian service 2024): the
Tatra 8x8 chassis, its four axles in two pairs; the long armoured cab-over
with small windows; behind it the ammunition and equipment lockers along
both sides; the 155 mm 52-calibre gun on its cradle at the rear, travelling
forward over the cab on a rest, with its pepperpot muzzle brake; the recoil
spade folded up under the tail. French three-tone (the card is Europe's
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
from parts import box, cyl  # noqa: E402
from truck_chassis import chassis  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

CARD = "europe_caesar_88"


def build(variant, v):
    m, h = v.mats, v.hull
    half = v.length / 2
    deck = chassis(v, [3.45, 2.00, -1.45, -2.85], 0.58, 0.42, 1.02, 1.05, "armoured", 1.90, 3.0, 3.10)
    # Lockers along both sides behind the cab.
    for side, s in ((1, "L"), (-1, "R")):
        for k in range(3):
            VP.stowage_box(f"locker_{s}_{k}", (1.25 - k * 1.05, side * 0.95, deck), (0.95, 0.55, 0.95), m, h,
                           rot=(0, 0, 0 if side > 0 else math.pi))
    box("deck_plate", (5.6, 2.3, 0.08), (-0.9, 0, deck + 0.04), m["paint"], h, bevel=0.01)
    # The gun's cradle at the rear, the barrel forward over the cab on its rest.
    cx = -2.95
    gz = 3.28  # the gun travels over the cab roof (3.10)
    box("gun_carriage", (1.20, 1.50, 0.90), (cx, 0, deck + 0.50), m["paint"], h, bevel=0.04)
    for side in (-1, 1):
        box(f"cradle_cheek_{side}", (0.90, 0.10, 1.00), (cx + 0.10, side * 0.45, deck + 1.40), m["paint"], h,
            bevel=0.02)
        cyl(f"recuperator_{side}", 0.09, 1.60, (cx + 0.80, side * 0.22, gz - 0.18), "X", m["dark"], h, seg=14)
    length = 8.06
    start = cx - 0.60
    end = start + length
    cyl("gun_breech", 0.20, 0.80, (start + 0.40, 0, gz), "X", m["steel"], h, seg=20, bevel=0.02)
    cyl("gun_barrel", 0.10, length - 1.20, ((start + 0.80 + end - 0.40) / 2, 0, gz), "X", m["paint"], h, seg=20)
    cyl("muzzle_brake", 0.15, 0.55, (end - 0.27, 0, gz), "X", m["dark"], h, seg=18, bevel=0.015)
    for k in range(5):
        for side in (-1, 1):
            cyl(f"brake_port_{k}_{side}", 0.030, 0.02, (end - 0.48 + k * 0.10, side * 0.15, gz), "Y", m["black"], h,
                seg=8, lods=FINE)
    cyl("muzzle_bore", 0.078, 0.012, (end + 0.005, 0, gz), "X", m["black"], h, seg=16, lods=MID)
    box("travel_rest", (0.16, 0.40, gz - 3.10), (2.4, 0, (gz + 3.10) / 2 - 0.08), m["dark"], h, lods=MID)
    # The recoil spade folded up under the tail.
    VP.bolted_panel("recoil_spade", (-half + 0.25, 0, 0.55), (0.12, 1.60, 0.60), m, h, bolts=(1, 4), bevel=0.02,
                    rot=(0, -0.35, 0), lods=VP.ALL)


def wreck(variant, v):
    """The CAESAR after its fire: the cab burnt out and its glass gone, the
    left front wheels off and the nose down on that corner, the lockers
    burst, the barrel drooping off its rest."""
    from parts import rest_on_ground
    from wreckage import bend, densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    remove("wheel_L_1_", "wheel_L_2_", "windscreen", "side_window_", "locker_L_1", "mudflap_L")
    bend(parts("gun_barrel", "muzzle_", "brake_port_"), (-3.55, 0, 3.28), (0, 1, 0), (0, 0, 1), -0.06)
    shell = parts("cab_body", "locker_", "gun_carriage")
    densify(shell, scale=2.0)
    warp(shell, heat(0.025, 0.8, seed=91.0), dent((3.4, 1.0, 2.4), 0.6, 0.15, (0, 0, -1)))
    for k in range(3):
        plate(f"debris_{k}", [(-0.4, -0.25), (0.35, -0.3), (0.4, 0.2), (-0.3, 0.32)], 0.03,
              (2.6 - k * 2.0, (-1) ** k * 2.1, 0.03), (0.04, 0.02, 0.6 + k), m["paint"], v.hull, curl=0.12,
              seed=151 + k)
    v.root.rotation_euler = (-0.035, 0.035, 0)
    v.root.location.z -= 0.06
    rest_on_ground(0.004)


if __name__ == "__main__":
    run_disabled("caesar", {CARD: (10.0, 2.55, 3.20)}, "french_three_tone", build, wreck, skip=("dressing_", "gun_", "fume_", "muzzle_", "brake_"), chip=1.0)
