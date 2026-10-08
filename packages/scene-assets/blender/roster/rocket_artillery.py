"""Rocket artillery (disabled cards): M270A2, MARS II, M142 HIMARS (GMLRS and
PrSM), BM-21 Grad and 9K515 Tornado-S, from assets/references/<family>/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/rocket_artillery.py -- [--variant=<card id>] [--wreck]

What the photos settle:
- M270A2 and MARS II: the Bradley-family tracked carrier (six road wheels,
  the sprocket at the front), the armoured three-man cab across the front
  with its shuttered windscreens, the launcher box over the rear holding
  two pods, stowed flat with their capped faces to the rear. The MARS II is the German upgrade of the same
  vehicle (German three-tone; its cab armour kit is the A2's twin).
- HIMARS: the FMTV 6x6, its flat-fronted armoured cab, the launcher module
  on a turntable behind: a ribbed box filling the rear deck, one pod in it
  whose capped face shows at the rear: six GMLRS rounds, or (PrSM) two
  missiles behind two large caps.
- BM-21: the Ural-375D 6x6 (the roster Ural's chassis and cab), a spare
  wheel behind the cab, flat fenders over the rear bogie, the forty 122 mm
  tubes in four rows on a cradle with its sector gears, travelling with
  their muzzles to the rear.
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
import ural as U  # noqa: E402
from truck_chassis import chassis  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

CARDS = {
    "us_m270_mlrs_m270a2": ("m270_mlrs", "us_desert_tan", (6.97, 2.97, 2.62)),
    "europe_mars_ii_multiple_launch_rocket_system": ("mars_ii", "german_three_tone", (6.97, 2.97, 2.62)),
    "us_m142_himars_gmlrs": ("m142_himars", "us_desert_tan", (7.00, 2.40, 3.20)),
    "us_m142_himars_prsm": ("m142_himars", "us_desert_tan", (7.00, 2.40, 3.20)),
    "eastern_bm_21_grad_bm_21_rocket_artillery": ("bm_21_grad", "russian_green", (7.35, 2.69, 3.09)),
    "eastern_9k515_tornado_s_heavy_rocket_artillery": ("9k515_tornado_s", "russian_green", (12.10, 3.05, 3.05)),
}


# ---------------------------------------------------------------- pods
def pod(v, name, loc, size, cells, parent, caps=None):
    """A launch pod `size` (x along the rounds, y, z) stowed as the photos
    show it, its face at its rear (-X) end: `cells` (columns, rows) of rounds
    behind frangible caps in their steel rims, or `caps` large caps (PrSM),
    in a recessed face; stiffening ribs round its body and lifting eyes on
    its top corners."""
    m = v.mats
    sx, sy, sz = size
    x, y, z = loc
    face = x - sx / 2
    box(f"{name}_body", size, loc, m["paint"], parent, bevel=0.03)
    for k in range(4):
        box(f"{name}_rib_{k}", (0.06, sy + 0.03, sz + 0.03), (x - sx / 2 + 0.25 + k * (sx - 0.4) / 3, y, z),
            m["paint"], parent, bevel=0.01, lods=NEAR)
    box(f"{name}_face", (0.02, sy - 0.08, sz - 0.08), (face - 0.005, y, z), m["black"], parent, lods=MID)
    cols, rows = cells if caps is None else (caps, 1)
    cell = min(sy / cols, sz / rows) * 0.82
    for i in range(cols):
        for j in range(rows):
            cy = y - sy / 2 + sy * (i + 0.5) / cols
            cz = z - sz / 2 + sz * (j + 0.5) / rows
            cyl(f"{name}_rim_{i}_{j}", cell / 2 + 0.02, 0.04, (face - 0.02, cy, cz), "X", m["steel"], parent, seg=16,
                lods=NEAR)
            cyl(f"{name}_cap_{i}_{j}", cell / 2, 0.03, (face - 0.03, cy, cz), "X", m["dark"], parent, seg=16,
                bevel=0.008, lods=MID)
    for i, dy in enumerate((-1, 1)):
        for j, dx in enumerate((-1, 1)):
            cyl(f"{name}_eye_{i}_{j}", 0.04, 0.05, (x + dx * (sx / 2 - 0.15), y + dy * (sy / 2 - 0.10), z + sz / 2 + 0.02),
                "Z", m["steel"], parent, seg=8, lods=FINE)


REAR_LIP = 0.12


def launcher_box(v, name, x0, x1, z0, z1, width, parent):
    """The armoured launcher box a pod rides in (the M270's and HIMARS's
    launcher-loader module): upright ribbed sides, its top chamfered down at
    the front, the loader boom's rail along its top and the cable run down
    its right side. Open at the rear (-X): its shell starts `REAR_LIP` ahead
    of `x0`, where the pod's face shows."""
    m = v.mats
    x0 += REAR_LIP
    prism(f"{name}_shell", [(x0, z0), (x1, z0), (x1, z1 - 0.30), (x1 - 0.30, z1), (x0, z1)], width, mat=m["paint"],
          parent=parent, bevel=0.04)
    ribs = max(3, round((x1 - x0) / 0.75))
    for k in range(ribs):
        x = x0 + 0.20 + (x1 - x0 - 0.70) * k / (ribs - 1)
        for side in (-1, 1):
            box(f"{name}_rib_{side}_{k}", (0.07, 0.04, z1 - z0 - 0.10), (x, side * (width / 2 + 0.02), (z0 + z1) / 2 - 0.05),
                m["paint"], parent, bevel=0.01, lods=NEAR)
    box(f"{name}_rear_frame", (0.08, width + 0.04, 0.10), (x0 + 0.04, 0, z1 - 0.05), m["paint"], parent, bevel=0.015)
    box(f"{name}_rear_sill", (0.08, width + 0.04, 0.10), (x0 + 0.04, 0, z0 + 0.05), m["paint"], parent, bevel=0.015)
    # The loader boom stowed along the top's left edge, its hoist at the rear.
    box(f"{name}_boom", (x1 - x0 - 0.50, 0.14, 0.14), ((x0 + x1) / 2 - 0.20, width / 2 - 0.18, z1 + 0.07),
        m["paint"], parent, bevel=0.02, lods=MID)
    box(f"{name}_hoist", (0.30, 0.22, 0.20), (x0 + 0.30, width / 2 - 0.18, z1 + 0.10), m["dark"], parent,
        bevel=0.02, lods=NEAR)
    VP.cable(f"{name}_cable_run", [(x1 - 0.30, -(width / 2 + 0.05), z1 - 0.25), ((x0 + x1) / 2, -(width / 2 + 0.05), z1 - 0.25),
                                   (x0 + 0.40, -(width / 2 + 0.05), z1 - 0.35)], m, parent, radius=0.025, eyes=False)


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
    # The armoured cab across the front: three windscreens behind their
    # shutters, a door each side with its vision block, the engine's grille
    # on the nose beside the transmission access plate.
    cab_rear, cab_front = 1.55, half - 0.10
    loft("cab_body", [(deck, [(cab_rear, -1.40), (cab_front, -1.40), (cab_front, 1.40), (cab_rear, 1.40)]),
                      (2.52, [(cab_rear + 0.05, -1.30), (cab_front - 0.45, -1.30), (cab_front - 0.45, 1.30),
                              (cab_rear + 0.05, 1.30)])], mat=m["paint"], parent=h, bevel=0.04)
    for k, y in enumerate((-0.85, 0.0, 0.85)):
        box(f"windscreen_{k}", (0.03, 0.62, 0.42), (cab_front - 0.22, y, 2.15), m["glass"], h, rot=(0, -0.55, 0),
            lods=MID)
        box(f"windscreen_frame_{k}", (0.04, 0.72, 0.52), (cab_front - 0.235, y, 2.145), m["paint"], h,
            rot=(0, -0.55, 0), bevel=0.01, lods=NEAR)
        box(f"windscreen_shutter_{k}", (0.04, 0.66, 0.10), (cab_front - 0.12, y, 2.42), m["paint"], h,
            rot=(0, -0.55, 0), bevel=0.01, lods=NEAR)
        box(f"wiper_{k}", (0.02, 0.03, 0.34), (cab_front - 0.19, y - 0.15, 2.12), m["black"], h,
            rot=(-0.5, -0.55, 0), lods=FINE)
    nose = math.atan2(0.25, 0.40)  # the upper glacis, from (half, 1.05) back to (half - 0.40, 1.30)
    VP.grille("engine_grille", (half - 0.20, 0.55, 1.18), (0.36, 0.95), m, h, slats=6, rot=(0, nose, 0))
    VP.bolted_panel("transmission_plate", (half - 0.20, -0.55, 1.18), (0.36, 0.95, 0.03), m, h, bolts=(2, 4),
                    rot=(0, nose, 0), bevel=0.01)
    VP.hatch("cab_hatch", (cab_rear + 0.40, -0.70, 2.52), m, h, radius=0.26)
    VP.hatch("cab_hatch_2", (cab_rear + 0.40, 0.70, 2.52), m, h, radius=0.26)
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{s}", (half - 0.05, side * 1.10, 1.30), 0.07, m, h)
        VP.tow_hook(f"front_tow_{s}", (half - 0.02, side * 0.70, 0.85), m, h, size=0.12)
        box(f"side_skirt_{s}", (v.length - 0.40, 0.05, 0.40), (0, side * (v.width / 2 - 0.03), 1.05), m["paint"], h,
            bevel=0.01, lods=MID)
        VP.bolted_panel(f"cab_door_{s}", (cab_rear + 0.75, side * 1.405, deck + 0.10), (0.80, 0.95, 0.03), m, h,
                        bolts=(3, 3), rot=(-side * math.pi / 2, 0, 0), bevel=0.01)
        VP.periscope(f"door_vision_{s}", (cab_rear + 0.75, side * 1.36, 2.40), m, h, size=(0.12, 0.30, 0.12),
                     rot=(0, 0, side * math.pi / 2))
        box(f"door_handle_{s}", (0.14, 0.04, 0.04), (cab_rear + 0.40, side * 1.45, 1.75), m["steel"], h, lods=FINE)
        VP.cable(f"tow_cable_{s}", [(-1.8, side * (v.width / 2 - 0.01), 1.20), (0.0, side * (v.width / 2 - 0.01), 1.22),
                                    (1.2, side * (v.width / 2 - 0.01), 1.20)], m, h, radius=0.018)
        whip = empty(f"dressing_antenna_{s}", parent=h)
        VP.antenna(f"antenna_{s}", (cab_rear + 0.15, side * 1.20, 2.52), m, whip, height=2.2)
    VP.exhaust("exhaust", (half - 0.55, -1.30, 1.95), 0.07, 0.40, m, h, rot=(0, 0, -math.pi / 2))
    # The launcher-loader module over the rear: two pods in its box.
    x0, x1 = -half + 0.05, cab_rear - 0.10
    box("launcher_base", (1.40, 1.80, 0.20), (x1 - 1.0, 0, deck + 0.10), m["dark"], h, bevel=0.02)
    cyl("launcher_turntable", 0.75, 0.10, (x1 - 1.0, 0, deck + 0.24), "Z", m["dark"], h, seg=28, lods=MID)
    launcher_box(v, "launcher", x0, x1, deck + 0.29, 2.55, 2.60, h)
    for k, y in enumerate((-0.62, 0.62)):
        pod(v, f"pod_{k}", (x0 + (x1 - x0 - 0.20) / 2, y, deck + 0.80), (x1 - x0 - 0.20, 1.18, 0.85), (3, 2), h)
    VP.stowage_box("rear_box", (-half + 0.20, 0, 0.90), (0.30, 1.60, 0.35), m, h)
    stencil("rear_number", "A2", 0.20, (cab_front - 1.0, 1.405, 1.8), (math.pi / 2, 0, math.pi), m["marking"], h)


# ---------------------------------------------------------------- trucks
def himars(v, prsm):
    m, h = v.mats, v.hull
    half = v.length / 2
    deck = chassis(v, [2.25, -1.05, -2.45], 0.55, 0.38, 0.95, 0.95, "armoured", 1.20, 2.10, 3.00)
    box("deck_plate", (4.4, 2.2, 0.08), (-1.20, 0, deck + 0.04), m["paint"], h, bevel=0.01)
    box("launcher_base", (1.60, 1.40, 0.22), (-0.60, 0, deck + 0.19), m["dark"], h, bevel=0.02)
    cyl("launcher_turntable", 0.75, 0.12, (-0.60, 0, deck + 0.36), "Z", m["dark"], h, seg=28)
    # The launcher-loader module over the rear deck, one pod in its box.
    x0, x1 = -half + 0.08, 0.95
    launcher_box(v, "launcher", x0, x1, deck + 0.42, 3.06, 2.28, h)
    pod(v, "pod", (x0 + (x1 - x0 - 0.25) / 2, 0, deck + 1.20), (x1 - x0 - 0.25, 1.95, 1.30), (3, 2), h,
        caps=2 if prsm else None)
    for side, s in ((1, "L"), (-1, "R")):
        VP.stowage_box(f"side_box_{s}", (-1.75, side * 1.02, deck - 0.45), (1.20, 0.30, 0.42), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
    VP.jerrycan("jerrycan", (0.55, -0.98, deck), dict(m, paint=m["dark"]), h, rot=(0, 0, math.pi / 2))
    # Behind the cab: the air intake on its stalk and the exhaust stack.
    cyl("air_intake", 0.16, 0.90, (1.05, 0.90, deck + 1.05), "Z", m["paint"], h, seg=16, bevel=0.02)
    cyl("air_intake_cap", 0.20, 0.12, (1.05, 0.90, deck + 1.55), "Z", m["dark"], h, seg=16, lods=MID)
    cyl("exhaust_stack", 0.07, 1.30, (1.08, -0.98, deck + 0.95), "Z", m["dark"], h, seg=12)


def bm21(v):
    """The BM-21 stands on the roster Ural's chassis and cab (the Ural-375D
    and the 4320 share them), its launcher on the rear frame where the Ural's
    cargo body would be."""
    m, h = v.mats, v.hull
    U.chassis(v)
    U.cab(v)
    rail = U.RAIL_Z + 0.12
    box("sub_frame", (3.70, 1.20, 0.14), (-1.75, 0, rail + 0.07), m["dark"], h, bevel=0.015)
    for side, s in ((1, "L"), (-1, "R")):
        # The flat fenders over the rear bogie, a stowage box on each.
        box(f"rear_fender_{s}", (2.45, 0.52, 0.05), (-1.62, side * U.WHEEL_Y, 1.35), m["paint"], h, bevel=0.012)
        box(f"fender_stay_{s}", (0.06, 0.40, 0.18), (-1.62, side * 0.80, 1.26), m["dark"], h, lods=NEAR)
        VP.stowage_box(f"fender_box_{s}", (-2.35, side * 1.0, 1.375), (0.80, 0.40, 0.32), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        VP.light_with_guard(f"tail_light_{s}", (-3.62, side * 1.00, 1.10), 0.05, dict(m, lamp=m["tail"]), h,
                            rot=(0, 0, math.pi))
    box("rear_bumper", (0.12, 2.00, 0.14), (-3.55, 0, 0.88), m["dark"], h, bevel=0.012)
    VP.tow_hook("hitch", (-3.55, 0, 0.88), m, h, size=0.10, rot=(0, 0, math.pi))
    # The turntable and the cradle's side plates with their sector gears.
    tx = -2.30
    cyl("launcher_turntable", 0.70, 0.18, (tx, 0, rail + 0.23), "Z", m["dark"], h, seg=28)
    for side in (-1, 1):
        prism(f"cradle_plate_{side}", [(tx - 0.70, rail + 0.30), (tx + 0.55, rail + 0.30), (tx + 0.25, 2.18),
                                       (tx - 0.55, 2.18)], 0.06, loc=(0, side * 0.62, 0), mat=m["paint"], parent=h,
              bevel=0.012)
        arc = [(tx + 0.9 * math.cos(a), rail + 0.45 + 0.9 * math.sin(a)) for a in
               [math.radians(20 + 14 * k) for k in range(6)]]
        prism(f"sector_gear_{side}", arc + [(tx + 0.15, rail + 0.55)], 0.05, loc=(0, side * 0.70, 0), mat=m["dark"],
              parent=h, lods=NEAR)
    cyl("elevating_ram", 0.07, 0.95, (tx + 0.45, 0, rail + 0.85), "X", m["steel"], h, seg=10, rot=(0, -0.95, 0),
        lods=MID)
    # Forty tubes, four rows of ten, travelling with their muzzles to the
    # rear and the pack rising slightly toward them, as all three photos show.
    tube_len, rows, cols, r = 3.0, 4, 10, 0.075
    pitch = math.radians(4)
    cx, cz = -1.85, 2.26
    for j in range(rows):
        for i in range(cols):
            y = (i - (cols - 1) / 2) * (2 * r + 0.012)
            z = cz + j * (2 * r + 0.012)
            cyl(f"grad_tube_{j}_{i}", r, tube_len, (cx, y, z), "X", m["paint"], h, seg=10, rot=(0, pitch, 0),
                lods=(0, 1, 2) if (i % 2 == 0 or j % 2 == 0) else (0, 1))
            cyl(f"grad_bore_{j}_{i}", r * 0.82, 0.01, (cx - tube_len / 2 - 0.005, y, z + tube_len / 2 * math.sin(pitch)),
                "X", m["black"], h, seg=8, rot=(0, pitch, 0), lods=FINE)
    pack_z = cz + (rows - 1) * (r + 0.006)
    box("tube_pack_hull", (tube_len - 0.2, cols * (2 * r + 0.012), rows * (2 * r + 0.012)), (cx, 0, pack_z), m["paint"],
        h, rot=(0, pitch, 0), lods=(3,))
    for k, x in enumerate((-1.1, 0.0, 1.1)):
        box(f"tube_band_{k}", (0.06, cols * (2 * r + 0.012) + 0.06, rows * (2 * r + 0.012) + 0.06),
            (cx + x, 0, pack_z - x * math.sin(pitch)), m["dark"], h, rot=(0, pitch, 0), lods=MID)
    # The travel lock under the pack's front, and the firing cable reel.
    box("travel_lock", (0.10, 0.70, 2.30 - rail - 0.10), (-0.55, 0, (2.30 + rail) / 2 - 0.05), m["dark"], h, lods=MID)
    cyl("cable_reel", 0.16, 0.20, (-3.20, -0.70, rail + 0.35), "Y", m["dark"], h, seg=14, lods=NEAR)


def tornado_s(v):
    m, h = v.mats, v.hull
    half = v.length / 2
    deck = chassis(v, [4.05, 1.85, -1.45, -3.65], 0.76, 0.55, 1.20, 1.25, "maz_split", 3.45, 2.55, 2.65)
    box("deck_plate", (7.0, 2.6, 0.10), (-1.6, 0, deck + 0.05), m["paint"], h, bevel=0.01)
    VP.stowage_box("crane_box", (2.6, 0, deck), (0.70, 1.80, 0.65), m, h, rot=(0, 0, math.pi / 2))
    for side, s in ((1, "L"), (-1, "R")):
        # Stowage along the deck's edges, the rear stabiliser jacks, a ladder.
        for k, x in enumerate((0.9, -0.4)):
            VP.stowage_box(f"deck_box_{s}_{k}", (x, side * 1.18, deck + 0.10), (1.10, 0.24, 0.38), m, h,
                           rot=(0, 0, 0 if side > 0 else math.pi))
        box(f"jack_leg_{s}", (0.16, 0.16, deck - 0.25), (-half + 0.45, side * 1.15, (deck + 0.25) / 2 + 0.05), m["dark"],
            h, bevel=0.01)
        box(f"jack_pad_{s}", (0.40, 0.40, 0.06), (-half + 0.45, side * 1.15, 0.30), m["dark"], h, bevel=0.01, lods=MID)
        for k in range(4):
            box(f"ladder_rung_{s}_{k}", (0.40, 0.04, 0.03), (2.30, side * 1.48, 0.45 + k * 0.22), m["steel"], h,
                lods=FINE)
    cyl("launcher_turntable", 0.85, 0.25, (-3.4, 0, deck + 0.20), "Z", m["dark"], h, seg=28)
    box("launcher_cradle", (1.8, 1.8, 0.25), (-3.6, 0, deck + 0.30), m["paint"], h, bevel=0.02)
    cyl("elevating_ram", 0.10, 1.6, (-2.4, 0, deck + 0.45), "X", m["steel"], h, seg=12, lods=MID)
    tube_len, rows, cols, r = 7.6, 3, 4, 0.20
    cx, cz = -1.15, deck + 0.55
    for j in range(rows):
        for i in range(cols):
            y = (i - (cols - 1) / 2) * (2 * r + 0.03)
            z = cz + j * (2 * r + 0.03)
            cyl(f"smerch_tube_{j}_{i}", r, tube_len, (cx, y, z), "X", m["paint"], h, seg=16)
            cyl(f"smerch_ring_{j}_{i}", r + 0.02, 0.10, (cx + tube_len / 2 - 0.05, y, z), "X", m["paint"], h, seg=16,
                lods=NEAR)
            cyl(f"smerch_bore_{j}_{i}", r * 0.85, 0.01, (cx + tube_len / 2 + 0.005, y, z), "X", m["black"], h, seg=14,
                lods=MID)
    for k, x in enumerate((-3.0, -0.8, 1.4)):
        box(f"tube_band_{k}", (0.10, cols * (2 * r + 0.03) + 0.08, rows * (2 * r + 0.03) + 0.08),
            (cx + x, 0, cz + (rows - 1) * (r + 0.015)), m["dark"], h, lods=MID)
    VP.cable("firing_cable", [(-3.6, 0.95, deck + 0.40), (-1.0, 0.95, deck + 0.42), (1.6, 0.95, deck + 0.40)], m, h,
             radius=0.025, eyes=False)
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
               "smerch_tube_2_0", "smerch_tube_2_1", "smerch_ring_2_0", "smerch_ring_2_1", "smerch_bore_", "spare_",
               "dressing_mirror_R", "mirror_R", "mirror_arm_R")
        v.root.rotation_euler = (0.035, 0.03, 0)
        v.root.location.z -= 0.08
    shell = parts("cab_body", "cab_L", "cab_R", "cab", "bonnet", "wing_", "hull_upper", "pod_body", "pod_0_body",
                  "pod_1_body", "launcher_shell", "deck_plate")
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
