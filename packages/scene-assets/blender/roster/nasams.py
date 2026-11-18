"""NASAMS 3 launcher on the MAN HX 8x8, from assets/references/nasams/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/nasams.py -- [--variant=<card id>] [--wreck]

The unit is one drivable vehicle, so it is drawn as the launcher travels on
its truck, as Hungary (MAN), Lithuania (Mercedes 8x8), Finland (Sisu E13TP)
and Norway (Scania) field it, rather than on the trailer it is set down on to
fire. What the photos settle:
- An 8x8 cab-over truck: the HX's tilt cab and eight wheels (the MAN HX
  script's own, `man-hx.py`), its frame the HX77's (hull 10.34 x 2.50 m).
- The loader crane straight behind the cab, its knuckle boom folded upright
  and the spare wheel hung beside it, its outrigger beams across the deck.
- The canister launcher on its own skid frame over the rear deck: six AMRAAM
  canisters in two rows of three on the turntable, stowed flat, ribbed,
  caps forward; the levelling legs folded up at the frame's corners; the
  generator ahead of the canisters, the electronics box and cable reels
  behind them.
- Mudguards over the rear axle pairs, under the deck's edge.

The hull is 3.32 m tall to the canisters' top. The US Army's is the same
launcher and truck in its own tan. The canisters elevate on the turntable.
"""
import importlib
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import hull_fittings as HF  # noqa: E402
import roof_parts as RF  # noqa: E402
import stowage as SW  # noqa: E402
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty  # noqa: E402
from truck_chassis import spare_wheel  # noqa: E402
from launcher_rig import launcher as articulate_launcher  # noqa: E402
from vehicle_export import run  # noqa: E402

hx = importlib.import_module("man-hx")

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

DECK = hx.BED_FLOOR  # top of the subframe on the chassis rails
FLOOR = DECK + 0.10  # top of the launcher's skid frame
CRANE_X = 2.45  # the crane's column, straight behind the cab
SKID = (-5.05, 1.85)  # rear and front of the launcher's skid frame
CAN_X = -1.00  # the canisters' middle; the turntable sits 0.40 ahead of it
CAN_LEN, CAN_W, CAN_H = 4.30, 0.62, 0.58
CAN_PITCH = 0.68  # canister to canister, across
TURNTABLE_X = CAN_X + 0.40

# Each appearance's scheme.
CARDS = {
    "europe_nasams_nasams_3": "german_three_tone",
    "us_nasams_medium": "us_desert_tan",
}


def build(variant, v):
    hx.running_gear(v)
    hx.cab(v)
    m, h = v.mats, v.hull
    half = v.length / 2
    box("subframe", (hx.CAB_REAR - 0.05 + half - 0.10, 2.30, 0.20),
        ((hx.CAB_REAR - 0.05 - half + 0.10) / 2, 0, DECK - 0.10), m["dark"], h, bevel=0.01)
    crane(v)
    launcher(v)
    underside(v)
    articulate_launcher(v, (CAN_X - 1.35, 0, FLOOR + 0.65), 35,
                        ("canister_", "launcher_arm"), ("turntable", "launcher_cradle", "slew_carriage"))


def crane(v):
    """The loader crane behind the cab: its base and outrigger beams across
    the deck, the column on the left as tall as the cab, the knuckle boom
    folded upright behind it with its ram, the hook stowed; the spare wheel
    hung on the column's right flank, its tread blocks round it."""
    m, h = v.mats, v.hull
    x = CRANE_X
    box("crane_base", (0.70, 2.30, 0.30), (x, 0, DECK + 0.15), m["paint"], h, bevel=0.03)
    for side, s in ((1, "L"), (-1, "R")):
        # The outrigger's leg folded up at each end of the beam.
        box(f"crane_leg_{s}", (0.16, 0.16, 0.75), (x, side * 1.06, DECK + 0.62), m["dark"], h, bevel=0.01, lods=MID)
        box(f"crane_leg_pad_{s}", (0.28, 0.28, 0.04), (x, side * 1.06, DECK + 1.01), m["dark"], h, lods=MID)
    cy = 0.45
    cyl("crane_slew", 0.30, 0.20, (x, cy, DECK + 0.40), "Z", m["dark"], h, seg=20, lods=MID)
    box("crane_column", (0.42, 0.42, 1.10), (x, cy, DECK + 1.05), m["paint"], h, bevel=0.03)
    # The main boom folded upright along the column's rear face, the jib beside it.
    box("crane_boom", (0.28, 0.32, 1.25), (x - 0.35, cy, DECK + 0.975), m["paint"], h, bevel=0.03)
    box("crane_jib", (0.20, 0.22, 1.00), (x - 0.35, cy + 0.27, DECK + 0.95), m["paint"], h, bevel=0.02)
    box("crane_knuckle", (0.70, 0.38, 0.22), (x - 0.14, cy, DECK + 1.70), m["dark"], h, bevel=0.03)
    cyl("crane_ram", 0.07, 1.05, (x + 0.26, cy, DECK + 1.05), "Z", m["steel"], h, seg=12, lods=MID)
    HF.tow_hook("crane_hook", (x - 0.35, cy + 0.27, DECK + 0.38), m, h, size=0.10)
    # The spare wheel hung on the column's right flank on its bracket.
    sy, sz, r = -0.45, DECK + 1.05, 0.60
    box("spare_bracket", (0.24, 0.48, 0.24), (x, (sy + 0.20 + cy - 0.21) / 2, sz), m["dark"], h, lods=MID)
    spare_wheel(v, (x, sy, sz), r, 0.40)
    for k in range(18):
        a = k * math.tau / 18
        box(f"spare_tread_{k}", (0.10, 0.42, 0.05), (x + (r + 0.01) * math.cos(a), sy, sz + (r + 0.01) * math.sin(a)),
            m["rubber"], h, rot=(0, -a + math.pi / 2, 0), lods=NEAR)


def launcher(v):
    """The canister launcher on its skid frame over the rear deck."""
    m, h = v.mats, v.hull
    rear, front = SKID
    for side, s in ((1, "L"), (-1, "R")):
        box(f"skid_{s}", (front - rear, 0.16, FLOOR - DECK), ((front + rear) / 2, side * 0.95, (FLOOR + DECK) / 2),
            m["dark"], h, bevel=0.01)
        # Levelling legs at the frame's corners, folded up, their winding handles.
        for k, x in enumerate((front - 0.20, rear + 0.25)):
            box(f"leg_{s}_{k}", (0.14, 0.14, 0.80), (x, side * 1.12, FLOOR + 0.35), m["dark"], h, lods=MID)
            box(f"leg_pad_{s}_{k}", (0.30, 0.30, 0.05), (x, side * 1.12, FLOOR - 0.03), m["dark"], h, lods=MID)
            box(f"leg_crank_{s}_{k}", (0.03, 0.14, 0.03), (x, side * 1.12, FLOOR + 0.77), m["steel"], h, lods=FINE)
    for k, x in enumerate((front - 0.20, TURNTABLE_X, rear + 0.25)):
        box(f"skid_cross_{k}", (0.14, 2.10, 0.10), (x, 0, FLOOR - 0.05), m["dark"], h, lods=MID)
    box("deck_plate", (3.0, 2.0, 0.08), (TURNTABLE_X, 0, FLOOR + 0.04), m["paint"], h, bevel=0.01)
    cyl("turntable", 0.65, 0.22, (TURNTABLE_X, 0, FLOOR + 0.19), "Z", m["dark"], h, seg=28)
    box("launcher_cradle", (1.0, 1.4, 0.30), (TURNTABLE_X, 0, FLOOR + 0.44), m["paint"], h, bevel=0.03)
    box("slew_carriage", (2.30, 0.90, 0.19), (-1.45, 0, FLOOR + 0.53), m["dark"], h, bevel=0.02)
    # The side arms that hold the pack, against the outer canisters' ribs.
    for side in (-1, 1):
        box(f"launcher_arm_{side}", (2.2, 0.08, 0.45), (CAN_X, side * (CAN_PITCH + 0.32 + 0.04), FLOOR + 1.06),
            m["paint"], h, bevel=0.015)
    # Six canisters, two rows of three: ribbed, caps forward.
    for j in range(2):
        for i in range(3):
            y = (i - 1) * CAN_PITCH
            z = FLOOR + 0.84 + j * 0.62
            box(f"canister_{j}_{i}", (CAN_LEN, CAN_W, CAN_H), (CAN_X, y, z), m["paint"], h, bevel=0.03)
            box(f"canister_cap_{j}_{i}", (0.03, 0.52, 0.50), (CAN_X + CAN_LEN / 2 + 0.01, y, z), m["dark"], h,
                lods=MID)
            box(f"canister_end_{j}_{i}", (0.03, 0.52, 0.50), (CAN_X - CAN_LEN / 2 - 0.01, y, z), m["dark"], h,
                lods=MID)
            for k in range(3):
                box(f"canister_rib_{j}_{i}_{k}", (0.06, 0.64, 0.60), (CAN_X - 1.70 + k * 1.70, y, z), m["paint"], h,
                    lods=NEAR)
    # Ahead of the canisters, behind the crane: the generator, louvred.
    gx = (front + CAN_X + CAN_LEN / 2) / 2
    SW.stowage_box("generator_box", (gx, 0, FLOOR), (0.60, 1.70, 0.75), m, h)
    for side, s in ((1, "L"), (-1, "R")):
        HF.grille(f"generator_louvre_{s}", (gx, side * 0.86, FLOOR + 0.40), (0.40, 0.45), m, h, slats=6,
                  rot=(-side * math.pi / 2, 0, 0))
    # Behind the canisters: the electronics box, the cable reels each side.
    ex = rear + 0.70
    SW.stowage_box("electronics_box", (ex, 0, FLOOR), (0.80, 1.40, 0.60), m, h, rot=(0, 0, math.pi))
    for side, s in ((1, "L"), (-1, "R")):
        cyl(f"cable_reel_{s}", 0.24, 0.26, (ex, side * 0.92, FLOOR + 0.26), "Y", m["dark"], h, seg=16, lods=MID)
    whip = empty("dressing_antenna", parent=h)
    RF.antenna("antenna", (ex - 0.20, 0.50, FLOOR + 0.60), m, whip, height=1.6)


def underside(v):
    """Below the deck: the lockers and fuel tank between the axles, the rear
    underrun bar, the tail lights and pintle; the whip antennas on the cab."""
    m, h = v.mats, v.hull
    half = v.length / 2
    rear_axles = hx.AXLES[2:]
    fx = sum(rear_axles) / 2
    flen = abs(rear_axles[0] - rear_axles[1]) + 2 * hx.WHEEL_R + 0.20
    for side, s in ((1, "L"), (-1, "R")):
        # One mudguard over each rear pair, under the deck's edge, its lip clear of the tyres.
        box(f"fender_{s}", (flen, 0.46, 0.04), (fx, side * hx.WHEEL_Y, 2 * hx.WHEEL_R + 0.07), m["paint"], h,
            bevel=0.01, lods=MID)
        box(f"fender_lip_{s}", (flen, 0.03, 0.08), (fx, side * (hx.WHEEL_Y + 0.215), 2 * hx.WHEEL_R + 0.05),
            m["paint"], h, lods=NEAR)
        SW.stowage_box(f"locker_{s}", (-1.20, side * 0.95, 0.72), (1.30, 0.42, 0.58), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        HF.light_with_guard(f"tail_light_{s}", (-half + 0.08, side * 0.85, 1.10), 0.05, dict(m, lamp=m["tail"]), h,
                            rot=(0, 0, math.pi))
    HF.fuel_tank("fuel_tank", (0.40, 0.95, 1.00), 1.30, 0.30, m, h)
    box("battery_box", (1.10, 0.40, 0.50), (0.40, -0.98, 0.95), m["paint"], h, bevel=0.02)
    box("battery_bracket", (0.10, 0.50, 0.08), (0.40, -0.72, 1.18), m["dark"], h, lods=MID)
    box("rear_underrun", (0.12, 2.20, 0.14), (-half + 0.10, 0, 0.70), m["dark"], h, bevel=0.012)
    for side, s in ((1, "L"), (-1, "R")):
        box(f"underrun_bracket_{s}", (0.08, 0.10, 0.40), (-half + 0.10, side * 0.45, 0.90), m["dark"], h, lods=MID)
    HF.tow_hook("pintle", (-half + 0.18, 0, 0.92), m, h, size=0.10, rot=(0, 0, math.pi))
    RF.whip_pair(h, (hx.CAB_REAR + 0.15, 1.05, hx.ROOF), m, height=2.0)


def wreck(variant, v):
    """The launcher after its fire: the front right wheel gone and the cab
    down on that corner, a top-row canister burst out and every cap blown,
    the cab and canisters warped and dented, a mirror gone. Whole."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, remove, warp
    remove("wheel_R_1_", "canister_cap_", "canister_1_2", "canister_rib_1_2", "dressing_mirror_R", "mirror_arm_R",
           "mirror_R")
    shell = parts("cab", "canister_", "crane_column", "crane_boom")
    densify(shell, scale=1.5)
    warp(shell, heat(0.03, 0.8, seed=59.0), dent((4.7, -0.7, 2.4), 0.6, 0.12, (-1, 0, 0)),
         dent((CAN_X, -0.6, FLOOR + 1.50), 0.8, 0.16))
    v.root.rotation_euler = (0.025, 0.015, 0)
    v.root.location.z -= 0.06
    rest_on_ground(0.004)


if __name__ == "__main__":
    run("nasams", CARDS, build, wreck, chip=0.6)
