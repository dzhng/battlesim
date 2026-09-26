"""The supply truck: a 6×6 with four deploy legs and a telescoping mast (and, with --wreck, its burnt wreck).

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/supply_truck.py -- [out.glb] [--wreck]

Ported from spike 03's frozen `truck.py`: the same node tree, pivots and
deploy phases, driven by one progress value (`DeploymentState.progress`).
The phases are custom properties the articulation reads (`DEPLOY_EXTRAS` in
packages/scene-assets/src/articulation.ts), linear inside each window:

    beams out 0–0.3 · jacks down 0.2–0.5 · mast up 0.35–0.65 · telescope 0.6–1.0

    truck ─ body ─┬ wheel_{F,M,R}{L,R}   (radius_m; roll about +Y)
                  ├ deploy_leg_{FL,FR,RL,RR} (beam slides out) ─ _jack ─ _pad (lowers to the ground)
                  └ deploy_mast (hinge: along the roof → vertical) ─ _2 ─ _3 ─ _head
"""
import bpy, bmesh, sys, os, math, json
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import *

ARGS = script_args()
WRECK = "--wreck" in ARGS
POS = [a for a in ARGS if not a.startswith("--")]
OUT = POS[0] if POS else os.path.abspath("supply_truck.glb")

reset()

# sim authority: physics.supply_half_extents_m [3, 1.4, 1.8]
TYRE_R = 0.55
LUG = 0.0175  # tread lugs stand proud of the tyre (radially 0.035 m blocks centred on its surface)
WHEEL_Z = TYRE_R + LUG  # seated: the lugs touch the ground

if WRECK:
    paint_m = burnt("truck_paint")
    dark = burnt("chassis", seed=7.0)
    rubber = burnt("tyre", seed=9.0)
    steel = burnt("steel", seed=11.0)
    glass = flat_paint("glass", (0.02, 0.02, 0.02), rough=0.9)
    canvas = burnt("canvas", seed=13.0)
    lamp = flat_paint("lamp", (0.03, 0.03, 0.03), rough=0.9)
else:
    paint_m = woodland("truck_paint", ((0.068, 0.078, 0.05), (0.06, 0.068, 0.045), (0.068, 0.078, 0.05)), scale=0.4, wear=0.35,
                       dirt=0.7, rough=0.7, seed=5.0)
    paint_m["tint"] = 1.0  # the side-tint mask (Material.tint): the cab and body paint
    dark = flat_paint("chassis", (0.05, 0.05, 0.045), rough=0.7, wear=0.3)
    rubber = flat_paint("tyre", (0.028, 0.028, 0.027), rough=0.9, grime=0.35)
    steel = flat_paint("steel", (0.1, 0.105, 0.09), rough=0.5, metal=0.5, wear=0.8)
    glass = flat_paint("glass", (0.03, 0.05, 0.06), rough=0.05, grime=0.2)
    canvas = flat_paint("canvas", (0.24, 0.22, 0.15), rough=0.95)
    lamp = flat_paint("lamp", (0.7, 0.7, 0.6), rough=0.2)

truck = empty("truck")
body = empty("body", parent=truck)
for i, y in enumerate((-0.45, 0.45)):
    box(f"rail_{i}", (5.8, 0.12, 0.25), (0, y, 0.85), dark, body)
for k, x in enumerate((-2.2, -0.9, 0.5, 1.9)):
    box(f"crossmember_{k}", (0.1, 0.9, 0.12), (x, 0, 0.85), dark, body, lods=(0, 1))
box("bumper", (0.18, 2.5, 0.25), (2.95, 0, 0.75), dark, body, bevel=0.02)
for s in (-1, 1):
    n = "L" if s > 0 else "R"
    box(f"tow_eye_{n}", (0.12, 0.1, 0.12), (2.99, s * 0.55, 0.72), steel, body, bevel=0.01, lods=(0, 1))
    box(f"bumper_step_{n}", (0.2, 0.35, 0.04), (2.98, s * 0.95, 0.9), steel, body, lods=(0,))
# cab-forward cab
prism("cab", [(1.55, 1.0), (2.90, 1.0), (2.95, 1.9), (2.78, 2.85), (1.55, 2.90)], 2.5, mat=paint_m, parent=body, bevel=0.05)
box("windscreen", (0.05, 2.1, 0.7), (2.87, 0, 2.35), glass, body, rot=(0, math.radians(-10), 0))
box("windscreen_divider", (0.06, 0.06, 0.72), (2.885, 0, 2.35), dark, body, rot=(0, math.radians(-10), 0), lods=(0, 1))
box("sun_visor", (0.3, 2.3, 0.04), (2.95, 0, 2.8), paint_m, body, rot=(0, math.radians(-12), 0), bevel=0.01, lods=(0, 1))
for k, y in enumerate((-0.5, 0.5)):
    box(f"wiper_{k}", (0.02, 0.5, 0.03), (2.92, y, 2.05), dark, body, rot=(math.radians(20), 0, 0), lods=(0,))
for k, y in enumerate((-0.9, -0.3, 0.3, 0.9)):
    box(f"cab_marker_{k}", (0.06, 0.1, 0.05), (2.8, y, 2.88), lamp, body, lods=(0,))
for s in (-1, 1):
    n = "L" if s > 0 else "R"
    box(f"door_seam_{n}", (1.02, 0.02, 1.62), (2.2, s * 1.256, 1.9), dark, body, lods=(0,))
    box(f"door_skin_{n}", (0.98, 0.02, 0.96), (2.2, s * 1.262, 1.57), paint_m, body, bevel=0.01, lods=(0, 1))
    box(f"side_window_frame_{n}", (0.9, 0.02, 0.62), (2.2, s * 1.262, 2.38), dark, body, lods=(0, 1, 2))
    box(f"side_window_{n}", (0.8, 0.02, 0.52), (2.2, s * 1.275, 2.38), glass, body, lods=(0, 1, 2))
    box(f"door_handle_{n}", (0.14, 0.03, 0.03), (1.85, s * 1.28, 1.95), steel, body, lods=(0,))
    box(f"mirror_arm_{n}", (0.04, 0.2, 0.04), (2.75, s * 1.3, 2.62), steel, body, lods=(0, 1))
    box(f"mirror_{n}", (0.05, 0.16, 0.34), (2.75, s * 1.4, 2.4), dark, body, bevel=0.01, lods=(0, 1, 2))
    # the cab steps hang behind the front wheel
    box(f"step_{n}", (0.24, 0.25, 0.05), (1.38, s * 1.15, 0.98), steel, body, lods=(0, 1))
    box(f"step_lower_{n}", (0.24, 0.22, 0.05), (1.38, s * 1.15, 0.66), steel, body, lods=(0,))
    box(f"step_hanger_{n}", (0.04, 0.04, 0.4), (1.38, s * 1.25, 0.82), steel, body, lods=(0,))
    box(f"tail_light_{n}", (0.04, 0.2, 0.1), (-2.99, s * 1.0, 0.85), lamp, body, lods=(0, 1))
    box(f"grab_handle_{n}", (0.03, 0.03, 0.7), (1.6, s * 1.27, 1.85), steel, body, lods=(0,))
    box(f"fuel_tank_{n}", (0.9, 0.38, 0.45), (0.0, s * 1.02, 1.0), dark, body, bevel=0.06, lods=(0, 1, 2))
    for k, x in enumerate((-0.3, 0.3)):
        box(f"fuel_strap_{n}_{k}", (0.04, 0.4, 0.47), (x, s * 1.02, 1.0), steel, body, lods=(0,))
    cyl(f"headlamp_{n}", 0.11, 0.05, (2.96, s * 0.9, 1.35), "X", lamp, body, lods=(0, 1))
    box(f"lamp_guard_{n}", (0.04, 0.3, 0.3), (3.0, s * 0.9, 1.35), steel, body, lods=(0,))
    box(f"indicator_{n}", (0.04, 0.1, 0.07), (2.96, s * 1.15, 1.35), lamp, body, lods=(0,))
    box(f"mudflap_{n}", (0.02, 0.45, 0.4), (-2.95, s * 1.02, 0.55), rubber, body, lods=(0, 1))
box("grille", (0.04, 1.2, 0.5), (2.94, 0, 1.4), dark, body, lods=(0, 1, 2))
for k in range(5):
    box(f"grille_bar_{k}", (0.03, 1.22, 0.035), (2.965, 0, 1.2 + k * 0.1), paint_m, body, lods=(0,))
box("number_plate", (0.02, 0.5, 0.12), (3.05, 0, 0.75), lamp, body, lods=(0,))
box("roof_hatch", (0.6, 0.6, 0.06), (2.1, 0.3, 2.93), paint_m, body, bevel=0.02, lods=(0, 1))
# air intake snorkel and exhaust stack behind the cab
cyl("snorkel", 0.09, 1.9, (1.45, -1.05, 2.0), "Z", dark, body, seg=12, lods=(0, 1, 2))
box("snorkel_head", (0.22, 0.22, 0.2), (1.45, -1.05, 3.0), dark, body, bevel=0.03, lods=(0, 1))
cyl("exhaust", 0.07, 1.6, (1.45, 1.05, 2.3), "Z", steel, body, seg=10, lods=(0, 1, 2))
box("exhaust_guard", (0.05, 0.2, 0.8), (1.35, 1.05, 2.6), dark, body, lods=(0,))
# a spare wheel stood behind the cab
cyl("spare_wheel", 0.5, 0.35, (1.33, 0.0, 1.65), "X", rubber, body, seg=24, lods=(0, 1, 2))
cyl("spare_rim", 0.28, 0.37, (1.33, 0.0, 1.65), "X", dark, body, seg=16, lods=(0, 1))
# shelter (the cargo body): ribbed ISO-style box with corner castings
SX, SL = -0.85, 4.25
box("shelter", (SL, 2.5, 2.0), (SX, 0, 2.05), paint_m, body, bevel=0.04)
box("shelter_floor", (SL + 0.05, 2.55, 0.12), (SX, 0, 1.0), dark, body)
for i in range(5):
    box(f"shelter_rib_{i}", (0.06, 2.56, 2.02), (SX - 1.95 + i * 0.97, 0, 2.05), paint_m, body, lods=(0, 1))
for i, (dx, dy, dz) in enumerate([(a, b, c) for a in (-1, 1) for b in (-1, 1) for c in (-1, 1)]):
    box(f"shelter_corner_{i}", (0.16, 0.16, 0.14), (SX + dx * (SL / 2 - 0.06), dy * 1.2, 2.05 + dz * 0.95), steel, body,
        lods=(0,))
box("shelter_door", (0.05, 0.9, 1.7), (SX - SL / 2 - 0.02, 0.4, 2.0), paint_m, body, bevel=0.01, lods=(0, 1, 2))
box("shelter_door_handle", (0.04, 0.06, 0.3), (SX - SL / 2 - 0.06, 0.05, 2.0), steel, body, lods=(0,))
for k, z in enumerate((1.4, 2.6)):
    box(f"shelter_hinge_{k}", (0.05, 0.06, 0.12), (SX - SL / 2 - 0.05, 0.84, z), steel, body, lods=(0,))
for k in range(4):
    box(f"shelter_step_{k}", (0.12, 0.6, 0.04), (SX - SL / 2 - 0.04, 0.4, 0.3 + k * 0.22), steel, body, lods=(0, 1))
for k in range(2):
    box(f"shelter_ladder_rail_{k}", (0.04, 0.04, 1.8), (SX - SL / 2 - 0.06, -0.55 - k * 0.4, 2.2), steel, body, lods=(0,))
for k in range(6):
    box(f"shelter_ladder_rung_{k}", (0.04, 0.4, 0.03), (SX - SL / 2 - 0.06, -0.75, 1.5 + k * 0.3), steel, body, lods=(0,))
for s in (-1, 1):
    n = "L" if s > 0 else "R"
    box(f"shelter_vent_{n}", (0.6, 0.05, 0.35), (SX + 0.8, s * 1.27, 2.6), dark, body, lods=(0, 1))
    for k in range(5):
        box(f"shelter_louvre_{n}_{k}", (0.58, 0.04, 0.02), (SX + 0.8, s * 1.3, 2.46 + k * 0.07), paint_m, body, lods=(0,))
    box(f"cable_box_{n}", (0.7, 0.25, 0.35), (SX - 1.2, s * 1.35, 1.25), paint_m, body, bevel=0.02, lods=(0, 1))
box("nbc_unit", (0.3, 0.8, 0.5), (SX + SL / 2 + 0.15, 0.5, 2.5), paint_m, body, bevel=0.03, lods=(0, 1, 2))
box("aircon", (0.8, 0.9, 0.5), (SX + 1.5, 0.55, 3.3), paint_m, body, bevel=0.03, lods=(0, 1, 2))
box("aircon_grille", (0.7, 0.8, 0.02), (SX + 1.5, 0.55, 3.56), dark, body, lods=(0,))
box("tarp_roll", (3.0, 0.25, 0.25), (SX - 0.05, -1.1, 3.1), canvas, body, bevel=0.1, lods=(0, 1, 2))
for k in range(4):
    box(f"tarp_strap_{k}", (0.05, 0.27, 0.27), (SX - 1.3 + k * 0.85, -1.1, 3.1), dark, body, lods=(0,))
box("jerrycans", (0.6, 0.2, 0.45), (SX - 1.6, 1.35, 1.3), paint_m, body, bevel=0.02, lods=(0, 1))

WHEELS = {}
AXLES = (("F", 2.1), ("M", -1.05), ("R", -2.3))
for row, x in AXLES:
    for s, sn in ((1, "L"), (-1, "R")):
        n = f"wheel_{row}{sn}"
        node = empty(n, (x, s * 1.02, WHEEL_Z), body, props={"radius_m": TYRE_R + LUG})
        cyl(n + "_tyre", TYRE_R, 0.40, (0, 0, 0), "Y", rubber, node, seg=32, bevel=0.04)
        cyl(n + "_rim", 0.30, 0.42, (0, 0, 0), "Y", dark, node, seg=20, lods=(0, 1, 2))
        cyl(n + "_hub", 0.12, 0.5, (0, 0, 0), "Y", steel, node, seg=12, lods=(0, 1))
        for k in range(8):
            a = k * math.pi / 4
            cyl(f"{n}_nut_{k}", 0.025, 0.46, (math.cos(a) * 0.18, 0, math.sin(a) * 0.18), "Y", steel, node, seg=6, lods=(0,))
        for k in range(24):
            a = k * math.pi / 12
            for j, dy in enumerate((-0.1, 0.1)):
                # staggered tread blocks, alternately offset: a cross-country pattern
                box(f"{n}_lug_{k}_{j}", (0.035, 0.18, 0.07), (math.cos(a + j * 0.13) * TYRE_R, dy, math.sin(a + j * 0.13) * TYRE_R),
                    rubber, node, rot=(0, -(a + j * 0.13), 0), lods=(0,) if j else (0, 1))
        WHEELS[n] = TYRE_R
    # wheel arches: a mudguard over each tyre, its ends turned down
    for s in (-1, 1):
        ns = "L" if s > 0 else "R"
        box(f"fender_{row}{ns}", (1.2, 0.5, 0.04), (x, s * 1.02, 1.2), paint_m, body, bevel=0.01, lods=(0, 1, 2))
        for k, sx in enumerate((-1, 1)):
            box(f"fender_{row}{ns}_lip_{k}", (0.26, 0.5, 0.04), (x + sx * 0.66, s * 1.02, 1.1), paint_m, body,
                rot=(0, sx * math.radians(50), 0), lods=(0, 1))
    # the axle and its differential, on leaf springs under the rails
    cyl(f"axle_{row}", 0.07, 1.7, (x, 0, WHEEL_Z), "Y", dark, body, seg=10, lods=(0, 1, 2))
    box(f"diff_{row}", (0.35, 0.4, 0.35), (x, 0, WHEEL_Z), dark, body, bevel=0.05, lods=(0, 1))
    for s in (-1, 1):
        box(f"spring_{row}_{'L' if s > 0 else 'R'}", (1.0, 0.1, 0.1), (x, s * 0.45, 0.7), dark, body, lods=(0,))

# deploy legs: the outrigger beam slides out sideways, then the pad lowers to the ground
# The front legs sit between the front and middle axles, the rear legs behind the rear
# axle; stowed, each beam lies under the body inside the hull's width (the spike's
# stood 0.18 m outside it, and its rear beams ran through the rear tyres).
for n, x, s in (("FL", 0.9, 1), ("FR", 0.9, -1), ("RL", -2.9, 1), ("RR", -2.9, -1)):
    beam = empty(f"deploy_leg_{n}", (x, s * 0.78, 0.95), body,
                 props={"deploy_start": 0.0, "deploy_end": 0.3, "deploy_move_y": s * 0.55})
    box(f"leg_{n}_beam", (0.16, 0.9, 0.16), (0, 0.0, 0), steel, beam, bevel=0.01, lods=(0, 1, 2))
    jack = empty(f"deploy_leg_{n}_jack", (0, s * 0.42, 0), beam)
    cyl(f"leg_{n}_cyl", 0.07, 0.5, (0, 0, -0.1), "Z", dark, jack, seg=12, lods=(0, 1, 2))
    pad = empty(f"deploy_leg_{n}_pad", (0, 0, -0.35), jack,
                props={"deploy_start": 0.2, "deploy_end": 0.5, "deploy_move_z": -0.575})
    cyl(f"leg_{n}_rod", 0.045, 0.8, (0, 0, 0.4), "Z", steel, pad, seg=10, lods=(0, 1, 2))
    cyl(f"leg_{n}_padplate", 0.16, 0.05, (0, 0, 0), "Z", dark, pad, seg=16, lods=(0, 1, 2))
    box(f"leg_{n}_housing", (0.22, 0.5, 0.22), (x, s * 0.72, 0.95), dark, body, bevel=0.015, lods=(0, 1, 2))
    box(f"leg_{n}_stripe", (0.17, 0.02, 0.1), (0, s * 0.45, 0), lamp, beam, lods=(0,))

# the mast: hinged at the shelter roof's rear, stowed lying forward along the roof
mast = empty("deploy_mast", (-2.7, -0.6, 3.1), body, rot=(0, math.radians(90), 0),
             props={"deploy_start": 0.35, "deploy_end": 0.65, "deploy_turn_y": -90.0})
cyl("mast_base_tube", 0.09, 2.2, (0, 0, 1.1), "Z", paint_m, mast, seg=14)
m2 = empty("deploy_mast_2", (0, 0, 0.3), mast, props={"deploy_start": 0.6, "deploy_end": 1.0, "deploy_move_z": 1.8})
cyl("mast_tube_2", 0.07, 2.0, (0, 0, 1.0), "Z", steel, m2, seg=12)
m3 = empty("deploy_mast_3", (0, 0, 0.3), m2, props={"deploy_start": 0.6, "deploy_end": 1.0, "deploy_move_z": 1.6})
cyl("mast_tube_3", 0.05, 2.0, (0, 0, 1.0), "Z", steel, m3, seg=12, lods=(0, 1, 2))
head = empty("deploy_mast_head", (0, 0, 2.0), m3)
box("mast_radio_head", (0.35, 0.35, 0.25), (0, 0, 0.1), dark, head, bevel=0.02, lods=(0, 1, 2))
for k in range(4):
    a = k * math.pi / 2
    cyl(f"mast_whip_{k}", 0.01, 1.0, (math.cos(a) * 0.25, math.sin(a) * 0.25, 0.6), "Z", dark, head, seg=6, lods=(0, 1))
box("mast_cradle", (0.3, 0.3, 0.2), (0.5, -0.6, 3.1), steel, body, lods=(0, 1, 2))

if WRECK:
    # burnt out: the tyres, glass, lamps and canvas are gone; it settles on its rims
    gone = ("wheel_", "windscreen", "side_window_", "headlamp_", "tarp_", "mudflap_", "mast_whip_", "spare_wheel", "cab_marker_",
            "indicator_", "wiper_", "shelter_door", "aircon", "nbc_unit", "cable_box_", "sun_visor", "number_plate", "tail_light_",
            "fender_FL_lip", "mirror_L")
    keep = ("_rim", "_hub", "_nut_")
    for o in list(bpy.data.objects):
        if o.type == "MESH" and o.name.startswith(gone) and not any(k in o.name for k in keep):
            bpy.data.objects.remove(o, do_unlink=True)
    hole = burnt("hole", seed=21.0)
    box("shelter_door_hole", (0.02, 0.9, 1.7), (SX - SL / 2 - 0.01, 0.4, 2.0), hole, body, lods=(0, 1, 2))
    for k, (x, y) in enumerate(((SX + 0.6, 0.3), (SX - 1.1, -0.4))):
        box(f"roof_burn_through_{k}", (1.1, 0.9, 0.02), (x, y, 3.06), hole, body, lods=(0, 1, 2))
    box("roof_sheet_fallen", (1.2, 1.0, 0.03), (SX + 0.6, 0.3, 2.8), paint_m, body, rot=(0.3, -0.25, 0.2), lods=(0, 1))
    box("aircon_fallen", (0.8, 0.9, 0.5), (SX + 1.2, -0.5, 3.38), paint_m, body, rot=(0.3, 0.05, 0.8), lods=(0, 1, 2))
    # the glazing is gone: black openings where the windows were
    box("windscreen_hole", (0.03, 2.1, 0.7), (2.9, 0, 2.35), hole, body, rot=(0, math.radians(-10), 0), lods=(0, 1, 2))
    for s_ in (-1, 1):
        box(f"side_window_hole_{s_}", (0.8, 0.02, 0.52), (2.2, s_ * 1.276, 2.38), hole, body, lods=(0, 1, 2))
    truck.location = (0, 0, -(WHEEL_Z - 0.30) + 0.005)
    truck.rotation_euler = (math.radians(1.5), math.radians(-0.8), 0)
    body.location = (0, 0, 0)

rest_on_ground()
finish(ao_distance=1.2)
info = dict(tris=triangles_by_tier(), nodes=sorted(o.name for o in bpy.data.objects if o.type == "EMPTY"))
print("TRUCK", json.dumps(info))
export(OUT)
