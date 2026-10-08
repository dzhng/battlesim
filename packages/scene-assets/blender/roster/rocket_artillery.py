"""Rocket artillery (disabled cards): M270A2, MARS II, M142 HIMARS (GMLRS and
PrSM), BM-21 Grad and 9K515 Tornado-S, from assets/references/<family>/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/rocket_artillery.py -- [--variant=<card id>] [--wreck]

What the photos settle:
- M270A2 and MARS II: the Bradley-family tracked carrier (six road wheels,
  the sprocket at the front), the armoured three-man cab across the front
  with its shuttered windscreens, the launcher box over the rear holding
  two pods, stowed flat. The MARS II is the German upgrade of the same
  vehicle (German three-tone; its cab armour kit is the A2's twin).
- HIMARS: the FMTV 6x6, its flat-fronted armoured cab, the launcher module
  on a turntable behind with one pod: six GMLRS rounds, or (PrSM) two
  missiles behind two large caps in the same pod.
- BM-21: the Ural-375D 6x6, the cab behind a long bonnet, a spare wheel
  behind it, the forty 122 mm tubes in four rows travelling forward over the
  cab.
- Tornado-S: the MAZ-543M 8x8, its two cabs either side of the engine, the
  twelve 300 mm tubes in three rows lying along the chassis.

Each card's frame is the vehicle's
published length, width and height (launcher stowed), recorded in its
receipt as `references`. The launchers are drawn stowed and do not
articulate until the cards' mechanics land.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from truck_chassis import chassis, spare_wheel  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

CARDS = {
    "us_m270_mlrs_m270a2": ("m270_mlrs", "us_desert_tan", (6.97, 2.97, 2.62)),
    "europe_mars_ii_multiple_launch_rocket_system": ("mars_ii", "german_three_tone", (6.97, 2.97, 2.62)),
    "us_m142_himars_gmlrs": ("m142_himars", "us_desert_tan", (7.00, 2.40, 3.20)),
    "us_m142_himars_prsm": ("m142_himars", "us_desert_tan", (7.00, 2.40, 3.20)),
    "eastern_bm_21_grad_bm_21_rocket_artillery": ("bm_21_grad", "russian_green", (7.35, 2.40, 3.09)),
    "eastern_9k515_tornado_s_heavy_rocket_artillery": ("9k515_tornado_s", "russian_green", (12.10, 3.05, 3.05)),
}


# ---------------------------------------------------------------- pods
def pod(v, name, loc, size, cells, parent, caps=None):
    """A launch pod `size` (x along the rounds, y, z) whose face is at its
    +X end, `cells` (columns, rows) of rounds behind frangible caps, or
    `caps` large caps (PrSM)."""
    m = v.mats
    sx, sy, sz = size
    x, y, z = loc
    box(f"{name}_body", size, loc, m["paint"], parent, bevel=0.03)
    for k in range(4):
        box(f"{name}_rib_{k}", (0.06, sy + 0.03, sz + 0.03), (x - sx / 2 + 0.15 + k * (sx - 0.3) / 3, y, z),
            m["paint"], parent, bevel=0.01, lods=NEAR)
    cols, rows = cells if caps is None else (caps, 1)
    cell = min(sy / cols, sz / rows) * 0.80
    for i in range(cols):
        for j in range(rows):
            cy = y - sy / 2 + sy * (i + 0.5) / cols
            cz = z - sz / 2 + sz * (j + 0.5) / rows
            cyl(f"{name}_cap_{i}_{j}", cell / 2, 0.02, (x + sx / 2 + 0.01, cy, cz), "X", m["dark"], parent, seg=14,
                lods=MID)


# ---------------------------------------------------------------- tracked carriers
def m270(v):
    m, h = v.mats, v.hull
    half = v.length / 2
    deck = 1.30
    prism("hull_upper", [(-half, 0.80), (half - 0.30, 0.80), (half, 1.05), (half - 0.40, deck), (-half, deck)],
          v.width - 0.60, mat=m["paint"], parent=h, bevel=0.04)
    prism("hull_lower", [(-half + 0.30, 0.42), (half - 0.55, 0.42), (half - 0.05, 0.82), (-half, 0.82),
                         (-half, 0.65)], v.width - 1.30, mat=m["paint"], parent=h, bevel=0.04)
    road_x = [2.25 - 0.90 * k for k in range(6)]
    VP.tracked_running_gear(m, h, v.width / 2 - 0.33, 0.53, road_x, 0.39, 0.31, 0.18, (3.05, 0.62, 0.27),
                            (-3.20, 0.58, 0.25), [(1.8, 0.86, 0.08), (0.45, 0.88, 0.08), (-0.9, 0.88, 0.08),
                                                  (-2.25, 0.86, 0.08)], bolts=6, teeth=11, arm=(0.38, -0.30),
                            pitch=0.15, dual=True)
    # The armoured cab across the front.
    cab_rear, cab_front = 1.55, half - 0.10
    loft("cab_body", [(deck, [(cab_rear, -1.40), (cab_front, -1.40), (cab_front, 1.40), (cab_rear, 1.40)]),
                      (2.52, [(cab_rear + 0.05, -1.30), (cab_front - 0.45, -1.30), (cab_front - 0.45, 1.30),
                              (cab_rear + 0.05, 1.30)])], mat=m["paint"], parent=h, bevel=0.04)
    for k, y in enumerate((-0.85, 0.0, 0.85)):
        box(f"windscreen_{k}", (0.03, 0.62, 0.42), (cab_front - 0.22, y, 2.15), m["glass"], h, rot=(0, -0.55, 0),
            lods=MID)
        box(f"windscreen_shutter_{k}", (0.04, 0.66, 0.10), (cab_front - 0.12, y, 2.42), m["paint"], h,
            rot=(0, -0.55, 0), bevel=0.01, lods=NEAR)
    VP.hatch("cab_hatch", (cab_rear + 0.40, -0.70, 2.52), m, h, radius=0.26)
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{s}", (half - 0.05, side * 1.10, 1.30), 0.07, m, h)
        VP.tow_hook(f"front_tow_{s}", (half - 0.02, side * 0.70, 0.85), m, h, size=0.12)
        box(f"side_skirt_{s}", (v.length - 0.40, 0.05, 0.40), (0, side * (v.width / 2 - 0.03), 1.05), m["paint"], h,
            bevel=0.01, lods=MID)
        whip = empty(f"dressing_antenna_{s}", parent=h)
        VP.antenna(f"antenna_{s}", (cab_rear + 0.15, side * 1.20, 2.52), m, whip, height=2.2)
    # The launcher box stowed flat over the rear, two pods in it.
    lx = (-half + cab_rear) / 2 - 0.10
    length = cab_rear - (-half) - 0.25
    box("launcher_base", (1.20, 1.60, 0.30), (lx + 0.6, 0, deck + 0.15), m["dark"], h, bevel=0.02)
    box("launcher_cage", (length, 2.60, 0.18), (lx, 0, deck + 0.38), m["paint"], h, bevel=0.02)
    for k, y in enumerate((-0.65, 0.65)):
        pod(v, f"pod_{k}", (lx, y, deck + 0.85), (length - 0.10, 1.20, 0.80), (3, 2), h)
    VP.stowage_box("rear_box", (-half + 0.20, 0, deck), (0.35, 1.60, 0.30), m, h)
    stencil("rear_number", "A2", 0.20, (cab_front - 1.0, 1.405, 1.8), (math.pi / 2, 0, math.pi), m["marking"], h)


# ---------------------------------------------------------------- trucks
def himars(v, prsm):
    m, h = v.mats, v.hull
    half = v.length / 2
    deck = chassis(v, [2.25, -1.05, -2.45], 0.55, 0.38, 0.95, 0.95, "armoured", 1.20, 2.10, 3.00)
    box("deck_plate", (3.4, 2.2, 0.08), (-1.55, 0, deck + 0.04), m["paint"], h, bevel=0.01)
    cyl("launcher_turntable", 0.75, 0.25, (-1.0, 0, deck + 0.20), "Z", m["dark"], h, seg=28)
    box("launcher_frame", (3.0, 1.10, 0.50), (-1.75, 0, deck + 0.55), m["paint"], h, bevel=0.02)
    pod(v, "pod", (-1.65, 0, deck + 1.32), (3.30, 1.05, 0.95), (3, 2), h, caps=2 if prsm else None)
    VP.stowage_box("side_box_L", (-1.0, 0.98, deck), (1.20, 0.22, 0.40), m, h)
    VP.stowage_box("side_box_R", (-1.0, -0.98, deck), (1.20, 0.22, 0.40), m, h, rot=(0, 0, math.pi))


def bm21(v):
    m, h = v.mats, v.hull
    deck = chassis(v, [2.30, -0.95, -2.35], 0.55, 0.36, 0.98, 0.98, "bonnet", 0.55, 3.05, 2.55)
    spare_wheel(v, (0.30, 0, deck + 0.45), 0.55, 0.36, rot=(0, 0, math.pi / 2))
    cyl("launcher_turntable", 0.65, 0.22, (-1.6, 0, deck + 0.15), "Z", m["dark"], h, seg=24)
    box("launcher_cradle", (1.4, 1.30, 0.75), (-1.6, 0, deck + 0.62), m["paint"], h, bevel=0.02)
    # Forty tubes, four rows of ten, travelling forward over the cab.
    tube_len, rows, cols, r = 3.0, 4, 10, 0.075
    pitch = math.radians(4)
    cx, cz = -0.9, deck + 1.25
    for j in range(rows):
        for i in range(cols):
            y = (i - (cols - 1) / 2) * (2 * r + 0.012)
            z = cz + j * (2 * r + 0.012)
            cyl(f"grad_tube_{j}_{i}", r, tube_len, (cx, y, z), "X", m["paint"], h, seg=10, rot=(0, -pitch, 0),
                lods=(0, 1, 2) if (i % 2 == 0 or j % 2 == 0) else (0, 1))
            cyl(f"grad_bore_{j}_{i}", r * 0.82, 0.01, (cx + tube_len / 2 + 0.005, y, z + tube_len / 2 * math.sin(pitch)),
                "X", m["black"], h, seg=8, rot=(0, -pitch, 0), lods=FINE)
    box("tube_pack_hull", (tube_len - 0.2, cols * (2 * r + 0.012), rows * (2 * r + 0.012)),
        (cx, 0, cz + (rows - 1) * (r + 0.006)), m["paint"], h, rot=(0, -pitch, 0), lods=(3,))
    for k, x in enumerate((-0.9, 0.4)):
        box(f"tube_band_{k}", (0.06, cols * (2 * r + 0.012) + 0.06, rows * (2 * r + 0.012) + 0.06),
            (cx + x, 0, cz + (rows - 1) * (r + 0.006) + x * math.sin(pitch)), m["dark"], h, rot=(0, -pitch, 0),
            lods=MID)


def tornado_s(v):
    m, h = v.mats, v.hull
    half = v.length / 2
    deck = chassis(v, [4.05, 1.85, -1.45, -3.65], 0.76, 0.55, 1.20, 1.25, "maz_split", 3.45, 2.55, 2.65)
    box("deck_plate", (7.0, 2.6, 0.10), (-1.6, 0, deck + 0.05), m["paint"], h, bevel=0.01)
    VP.stowage_box("crane_box", (2.6, 0, deck), (0.70, 1.80, 0.65), m, h, rot=(0, 0, math.pi / 2))
    cyl("launcher_turntable", 0.85, 0.25, (-3.4, 0, deck + 0.20), "Z", m["dark"], h, seg=28)
    box("launcher_cradle", (1.8, 1.8, 0.25), (-3.6, 0, deck + 0.30), m["paint"], h, bevel=0.02)
    tube_len, rows, cols, r = 7.6, 3, 4, 0.20
    cx, cz = -1.15, deck + 0.55
    for j in range(rows):
        for i in range(cols):
            y = (i - (cols - 1) / 2) * (2 * r + 0.03)
            z = cz + j * (2 * r + 0.03)
            cyl(f"smerch_tube_{j}_{i}", r, tube_len, (cx, y, z), "X", m["paint"], h, seg=16)
            cyl(f"smerch_bore_{j}_{i}", r * 0.85, 0.01, (cx + tube_len / 2 + 0.005, y, z), "X", m["black"], h, seg=14,
                lods=MID)
    for k, x in enumerate((-3.0, -0.8, 1.4)):
        box(f"tube_band_{k}", (0.10, cols * (2 * r + 0.03) + 0.08, rows * (2 * r + 0.03) + 0.08),
            (cx + x, 0, cz + (rows - 1) * (r + 0.015)), m["dark"], h, lods=MID)
    box("travel_rest", (0.20, 1.20, deck + 0.35 - 1.25), (2.0, 0, (deck + 0.35 + 1.25) / 2), m["dark"], h, lods=MID)


def build(variant, v):
    ident = variant["id"]
    if ident in ("us_m270_mlrs_m270a2", "europe_mars_ii_multiple_launch_rocket_system"):
        m270(v)
    elif ident.startswith("us_m142"):
        himars(v, prsm=ident.endswith("prsm"))
    elif ident.startswith("eastern_bm_21"):
        bm21(v)
    else:
        tornado_s(v)


def wreck(variant, v):
    """A launcher after its fire: tyres or a track burnt off, the cab's glass
    gone and its roof warped, the pods or tubes split and some lying beside
    it, the chassis settled onto its burnt corner."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, plate, remove, warp
    m = v.mats
    ident = variant["id"]
    if ident in ("us_m270_mlrs_m270a2", "europe_mars_ii_multiple_launch_rocket_system"):
        remove("track_R_band", "wheel_R_2_", "side_skirt_R", "pod_1_cap_", "cab_hatch")
        v.root.rotation_euler = (-0.02, 0.0, 0)
    else:
        remove("wheel_R_1_", "wheel_R_2_", "windscreen", "mudflap_R", "pod_cap_", "grad_tube_3_", "grad_bore_",
               "smerch_tube_2_0", "smerch_tube_2_1", "smerch_bore_", "spare_")
        v.root.rotation_euler = (0.035, 0.03, 0)
        v.root.location.z -= 0.08
    shell = parts("cab_body", "cab_L", "cab_R", "bonnet", "hull_upper", "pod_body", "pod_0_body", "pod_1_body",
                  "launcher_cage", "deck_plate")
    densify(shell, scale=2.0)
    warp(shell, heat(0.025, 0.8, seed=71.0), dent((v.length / 2 - 0.6, -0.8, 2.3), 0.6, 0.12, (0, 0, -1)))
    for k in range(3):
        plate(f"debris_{k}", [(-0.35, -0.2), (0.30, -0.25), (0.35, 0.18), (-0.25, 0.28)], 0.03,
              (-1.5 + k * 1.4, (-1) ** k * (v.width / 2 + 0.7), 0.03), (0.04, 0.02, 0.5 + k), m["paint"], v.hull,
              curl=0.12, seed=131 + k)
    for k in range(2):
        cyl(f"fallen_round_{k}", 0.11, 2.2, (-0.6 + k * 1.5, -(v.width / 2 + 0.5 + k * 0.3), 0.11), "X", m["paint"],
            v.hull, seg=12, rot=(0, 0, 0.3 + k * 0.5))
    rest_on_ground(0.004)


if __name__ == "__main__":
    run_disabled("rocket_artillery", {c: dims for c, (_, _, dims) in CARDS.items()},
                 {c: scheme for c, (_, scheme, _) in CARDS.items()}, build, wreck, chip=1.0)
