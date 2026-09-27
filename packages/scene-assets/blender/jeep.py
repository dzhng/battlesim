"""The recon jeep: an open-topped 4×4 with a pedestal HMG (and, with --wreck, its burnt wreck).

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/jeep.py -- [out.glb] [--wreck]

Built to the simulation's jeep: hull `physics.jeep_half_extents_m` [2.2, 1.0, 0.95]
and the HMG muzzle `physics.jeep_muzzle_local_m` [1.43, 0, 2.0]. The HMG is the
jeep's turret: its pedestal stands on the hull origin, as the simulation swings
the muzzle about it.

    jeep ─ body ─┬ wheel_{F,R}{L,R}          (radius_m; roll about +Y)
                 └ hmg (yaw on the pedestal) ─ hmg_gun (pitch) ─ hmg_muzzle
"""
import bpy, sys, os, math, json
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import *

ARGS = script_args()
WRECK = "--wreck" in ARGS
POS = [a for a in ARGS if not a.startswith("--")]
OUT = POS[0] if POS else os.path.abspath("jeep.glb")

reset()

# sim authority: physics.jeep_half_extents_m [2.2, 1.0, 0.95], jeep_muzzle_local_m [1.43, 0, 2.0]
TYRE_R = 0.42
LUG = 0.012
WHEEL_Z = TYRE_R + LUG  # seated: the lugs touch the ground
WHEEL_Y = 0.82
AXLES = (("F", 1.45), ("R", -1.45))
PIVOT_Z = 1.68  # the pedestal's yaw bearing; the gun's bore rides 0.32 above it
TUB_Y = 0.64  # the tub's sides stand inside the wheels
FLOOR_Z = 0.55
SIDE_Z = 1.15

if WRECK:
    paint = textured("jeep_paint", "nato_camo", chip=0.9, dirt=0.3, soot=1.0, streak=0.6, ash=0.4)
    dark = textured("chassis", "burnt_metal", chip=0.5, dirt=0.3, soot=0.6, ash=0.25, seed=7.0)
    rubber = textured("tyre", "burnt_metal", colour=(0.02, 0.019, 0.018), chip=0.2, dirt=0.2, seed=9.0)
    steel = textured("steel", "burnt_metal", chip=0.6, dirt=0.3, soot=0.4, ash=0.25, seed=11.0)
    canvas = textured("canvas", "burnt_metal", colour=(0.03, 0.028, 0.025), chip=0.3, dirt=0.2, seed=13.0)
    seat_m = textured("seat", "burnt_metal", colour=(0.03, 0.028, 0.026), chip=0.3, dirt=0.1, seed=15.0)
    ammo = textured("ammo_can", "burnt_metal", chip=0.5, dirt=0.3, soot=0.5, ash=0.3, seed=17.0)
    lamp = flat_paint("lamp", (0.03, 0.03, 0.03), rough=0.9)
    glass = flat_paint("glass", (0.02, 0.02, 0.02), rough=0.9)
else:
    # the side-tint mask (Material.tint): the body paint
    paint = textured("jeep_paint", "nato_camo", tint=1.0, chip=0.9, dirt=1.0, rise=1.1)
    dark = textured("chassis", "olive_paint", colour=(0.045, 0.048, 0.036), chip=0.6, dirt=1.0)
    rubber = textured("tyre", "rubber", dirt=0.55, chip=0.0, streak=0.0, rise=0.9)
    steel = textured("steel", "bare_steel", chip=0.5, dirt=0.6)
    canvas = textured("canvas", "canvas", chip=0.2, dirt=0.6, streak=0.2)
    seat_m = textured("seat", "canvas", colour=(0.05, 0.05, 0.035), chip=0.3, dirt=0.5, streak=0.0)
    ammo = textured("ammo_can", "ammo_paint", chip=0.9, dirt=0.6)
    lamp = flat_paint("lamp", (0.7, 0.7, 0.6), rough=0.2)
    glass = flat_paint("glass", (0.03, 0.05, 0.06), rough=0.05, grime=0.2)

jeep = empty("jeep")
body = empty("body", parent=jeep)

# ------------------------------------------------------------------ chassis
for i, y in enumerate((-0.45, 0.45)):
    box(f"rail_{i}", (4.1, 0.1, 0.16), (0.05, y, 0.45), dark, body)
for k, x in enumerate((-1.9, -0.6, 0.6, 1.9)):
    box(f"crossmember_{k}", (0.08, 0.9, 0.1), (x, 0, 0.45), dark, body, lods=(0, 1))
for row, x in AXLES:
    cyl(f"axle_{row}", 0.055, 1.5, (x, 0, WHEEL_Z), "Y", dark, body, seg=10, lods=(0, 1, 2))
    box(f"diff_{row}", (0.28, 0.3, 0.28), (x + (0.1 if row == "F" else -0.1), 0.12, WHEEL_Z), dark, body, bevel=0.04,
        lods=(0, 1))
    for s in (-1, 1):
        box(f"spring_{row}_{'L' if s > 0 else 'R'}", (0.9, 0.08, 0.06), (x, s * 0.45, 0.36), dark, body, lods=(0,))
cyl("exhaust", 0.035, 1.9, (-1.15, -0.3, 0.36), "X", steel, body, seg=10, lods=(0, 1))
box("tow_pintle", (0.12, 0.1, 0.1), (-2.12, 0, 0.5), steel, body, bevel=0.01, lods=(0, 1))

# ------------------------------------------------------------------ the tub: open-topped, its sides inside the wheels
box("floor", (2.62, 2 * TUB_Y, 0.05), (-0.74, 0, FLOOR_Z), dark, body)
# each side dips at the front seats, where the crew climbs in
side_profile = [(-2.05, FLOOR_Z - 0.03), (0.55, FLOOR_Z - 0.03), (0.55, SIDE_Z + 0.05), (0.3, SIDE_Z),
                (0.15, 0.88), (-0.55, 0.88), (-0.7, SIDE_Z), (-2.05, SIDE_Z)]
for s in (-1, 1):
    n = "L" if s > 0 else "R"
    prism(f"tub_side_{n}", side_profile, 0.04, (0, s * TUB_Y, 0), paint, body, bevel=0.012)
    box(f"tub_lip_{n}", (1.35, 0.07, 0.04), (-1.38, s * TUB_Y, SIDE_Z + 0.02), paint, body, bevel=0.01, lods=(0, 1, 2))
    box(f"grab_handle_{n}", (0.25, 0.03, 0.03), (-1.7, s * (TUB_Y + 0.03), 1.0), steel, body, lods=(0,))
box("tub_rear", (0.04, 2 * TUB_Y + 0.04, SIDE_Z - FLOOR_Z + 0.03), (-2.05, 0, (SIDE_Z + FLOOR_Z) / 2), paint, body,
    bevel=0.012)
box("cowl", (0.08, 2 * TUB_Y + 0.04, 0.75), (0.55, 0, 0.9), paint, body, bevel=0.015)
box("dash", (0.18, 1.2, 0.12), (0.46, 0, 1.12), dark, body, bevel=0.02, lods=(0, 1, 2))
for k, y in enumerate((0.25, 0.4, 0.55)):
    cyl(f"gauge_{k}", 0.035, 0.02, (0.37, y, 1.12), "X", glass, body, seg=12, lods=(0,))
# wheel wells and flat rear fenders over the rear wheels, outside the tub
for s in (-1, 1):
    n = "L" if s > 0 else "R"
    box(f"rear_fender_{n}", (1.05, 0.36, 0.035), (-1.45, s * (TUB_Y + 0.16), 0.97), paint, body, bevel=0.01)
    box(f"rear_well_{n}", (1.0, 0.03, 0.4), (-1.45, s * (TUB_Y + 0.33), 1.05 - 0.2), paint, body, lods=(0, 1))
    box(f"tail_light_{n}", (0.03, 0.1, 0.08), (-2.08, s * 0.5, 1.0), lamp, body, lods=(0, 1))

# ------------------------------------------------------------------ bonnet, grille, front fenders
prism("bonnet", [(0.55, 0.82), (2.02, 0.82), (2.05, 1.12), (1.95, 1.17), (0.55, 1.22)], 1.12, (0, 0, 0), paint, body,
      bevel=0.03)
box("bonnet_hinge", (1.4, 0.04, 0.03), (1.28, 0, 1.215), steel, body, lods=(0,))
for s in (-1, 1):
    box(f"bonnet_latch_{'L' if s > 0 else 'R'}", (0.04, 0.02, 0.1), (1.6, s * 0.565, 1.1), steel, body, lods=(0,))
box("grille", (0.05, 1.12, 0.52), (2.07, 0, 0.9), paint, body, bevel=0.015)
for k in range(9):
    box(f"grille_slot_{k}", (0.02, 0.05, 0.36), (2.1, -0.4 + k * 0.1, 0.92), dark, body, lods=(0,))
for s in (-1, 1):
    n = "L" if s > 0 else "R"
    cyl(f"headlamp_{n}", 0.085, 0.05, (2.1, s * 0.46, 0.93), "X", lamp, body, seg=16, lods=(0, 1, 2))
    cyl(f"headlamp_rim_{n}", 0.1, 0.03, (2.09, s * 0.46, 0.93), "X", steel, body, seg=16, lods=(0,))
    # the fender: flat over the wheel, its front turned down to the bumper
    box(f"front_fender_{n}", (1.15, 0.44, 0.035), (1.45, s * 0.76, 0.99), paint, body, bevel=0.01)
    box(f"front_fender_nose_{n}", (0.35, 0.44, 0.035), (2.08, s * 0.76, 0.87), paint, body,
        rot=(0, math.radians(40), 0), bevel=0.01, lods=(0, 1, 2))
    box(f"fender_skirt_{n}", (1.15, 0.03, 0.18), (1.45, s * 0.555, 0.9), paint, body, lods=(0, 1, 2))
    box(f"blackout_lamp_{n}", (0.06, 0.08, 0.05), (1.9, s * 0.75, 1.03), lamp, body, lods=(0,))
    box(f"mirror_arm_{n}", (0.03, 0.18, 0.03), (0.6, s * 0.72, 1.28), steel, body, lods=(0, 1))
    box(f"mirror_{n}", (0.03, 0.12, 0.1), (0.6, s * 0.82, 1.32), glass, body, lods=(0, 1))
box("bumper", (0.1, 1.9, 0.14), (2.15, 0, 0.55), dark, body, bevel=0.015)
for s in (-1, 1):
    box(f"bumper_iron_{'L' if s > 0 else 'R'}", (0.4, 0.08, 0.1), (1.95, s * 0.45, 0.55), dark, body, lods=(0, 1))
    box(f"tow_hook_{'L' if s > 0 else 'R'}", (0.08, 0.04, 0.12), (2.18, s * 0.6, 0.47), steel, body, lods=(0,))
cyl("winch_drum", 0.08, 0.6, (2.12, 0, 0.7), "Y", steel, body, seg=12, lods=(0, 1))

# the windscreen, folded flat onto the bonnet and covered, so the HMG sweeps clear over it
box("windscreen_frame", (0.62, 1.2, 0.04), (0.92, 0, 1.25), paint, body, rot=(0, math.radians(-2.5), 0), bevel=0.01)
box("windscreen_cover", (0.56, 1.14, 0.03), (0.92, 0, 1.285), canvas, body, rot=(0, math.radians(-2.5), 0),
    bevel=0.01, lods=(0, 1, 2))
for s in (-1, 1):
    box(f"windscreen_hinge_{'L' if s > 0 else 'R'}", (0.06, 0.05, 0.06), (0.6, s * 0.57, 1.24), steel, body, lods=(0,))

# ------------------------------------------------------------------ cockpit
for s, n in ((1, "L"), (-1, "R")):
    box(f"seat_{n}", (0.45, 0.42, 0.1), (-0.25, s * 0.37, 0.88), seat_m, body, bevel=0.03)
    box(f"seat_base_{n}", (0.4, 0.38, 0.28), (-0.25, s * 0.37, 0.7), dark, body, lods=(0, 1, 2))
    box(f"seat_back_{n}", (0.1, 0.42, 0.45), (-0.5, s * 0.37, 1.15), seat_m, body, rot=(0, math.radians(-10), 0),
        bevel=0.03)
cyl("steering_column", 0.025, 0.55, (0.3, 0.37, 1.1), "X", dark, body, rot=(0, math.radians(-35), 0), seg=8,
    lods=(0, 1))
cyl("steering_wheel", 0.19, 0.03, (0.1, 0.37, 1.24), "X", dark, body, rot=(0, math.radians(-35), 0), seg=18,
    caps=False, lods=(0, 1, 2))
box("gear_levers", (0.03, 0.08, 0.3), (0.2, 0.12, 0.72), steel, body, lods=(0,))
# the rear: a bench, the radio and stowage
box("rear_bench", (0.45, 2 * TUB_Y - 0.1, 0.1), (-1.55, 0, 0.9), seat_m, body, bevel=0.03, lods=(0, 1, 2))
box("rear_bench_base", (0.4, 2 * TUB_Y - 0.12, 0.3), (-1.55, 0, 0.72), dark, body, lods=(0, 1))
box("radio", (0.35, 0.5, 0.4), (-0.95, -0.3, 0.8), dark, body, bevel=0.02, lods=(0, 1, 2))
box("radio_face", (0.02, 0.44, 0.3), (-0.765, -0.3, 0.82), steel, body, lods=(0,))
for k in range(3):
    cyl(f"radio_knob_{k}", 0.02, 0.03, (-0.75, -0.45 + k * 0.12, 0.9), "X", lamp, body, seg=8, lods=(0,))
box("ammo_crate", (0.4, 0.35, 0.3), (-0.95, 0.32, 0.75), ammo, body, bevel=0.015, lods=(0, 1, 2))
box("kit_bag", (0.5, 0.35, 0.3), (-1.8, 0.3, 1.12), canvas, body, bevel=0.1, lods=(0, 1, 2))
box("bedroll", (0.25, 0.9, 0.25), (-1.85, -0.2, 1.1), canvas, body, bevel=0.1, lods=(0, 1, 2))

# the roll bar behind the front seats
for s in (-1, 1):
    cyl(f"rollbar_post_{'L' if s > 0 else 'R'}", 0.03, 0.72, (-0.8, s * 0.6, SIDE_Z + 0.34), "Z", dark, body, seg=10,
        lods=(0, 1, 2))
cyl("rollbar_top", 0.03, 1.26, (-0.8, 0, 1.85), "Y", dark, body, seg=10, lods=(0, 1, 2))
for s in (-1, 1):
    cyl(f"rollbar_stay_{'L' if s > 0 else 'R'}", 0.022, 0.95, (-1.25, s * 0.6, 1.47), "X", dark, body,
        rot=(0, math.radians(-40), 0), seg=8, lods=(0, 1))
# the whip antenna at the rear corner, bowed back
cyl("antenna_base", 0.035, 0.14, (-1.95, -0.52, SIDE_Z + 0.07), "Z", dark, body, seg=8, lods=(0, 1))
cyl("antenna", 0.008, 1.7, (-2.03, -0.52, 2.0), "Z", dark, body, rot=(0, math.radians(-6), 0), seg=6, lods=(0, 1))

# the spare wheel and a jerrycan on the tailboard
cyl("spare_tyre", 0.4, 0.2, (-2.1, 0.3, 0.9), "X", rubber, body, seg=24, bevel=0.03, lods=(0, 1, 2))
cyl("spare_rim", 0.22, 0.21, (-2.1, 0.3, 0.9), "X", dark, body, seg=16, lods=(0, 1))
box("jerrycan", (0.14, 0.34, 0.46), (-2.13, -0.35, 0.86), dark, body, bevel=0.02, lods=(0, 1, 2))
box("jerrycan_bracket", (0.03, 0.4, 0.06), (-2.08, -0.35, 0.7), steel, body, lods=(0,))

# ------------------------------------------------------------------ wheels
for row, x in AXLES:
    for s, sn in ((1, "L"), (-1, "R")):
        n = f"wheel_{row}{sn}"
        node = empty(n, (x, s * WHEEL_Y, WHEEL_Z), body, props={"radius_m": TYRE_R + LUG})
        cyl(n + "_tyre", TYRE_R, 0.28, (0, 0, 0), "Y", rubber, node, seg=28, bevel=0.035)
        cyl(n + "_rim", 0.22, 0.29, (0, 0, 0), "Y", dark, node, seg=18, lods=(0, 1, 2))
        cyl(n + "_hub", 0.08, 0.33, (0, 0, 0), "Y", steel, node, seg=10, lods=(0, 1))
        for k in range(5):
            a = k * 2 * math.pi / 5
            cyl(f"{n}_nut_{k}", 0.018, 0.31, (math.cos(a) * 0.13, 0, math.sin(a) * 0.13), "Y", steel, node, seg=6,
                lods=(0,))
        for k in range(20):
            a = k * math.pi / 10
            box(f"{n}_lug_{k}", (0.024, 0.2, 0.06), (math.cos(a) * TYRE_R, 0, math.sin(a) * TYRE_R), rubber, node,
                rot=(0, -a, 0), lods=(0,))

# ------------------------------------------------------------------ the pedestal HMG, on the hull origin
cyl("pedestal_foot", 0.16, 0.04, (0, 0, FLOOR_Z + 0.045), "Z", dark, body, seg=16, lods=(0, 1, 2))
cyl("pedestal", 0.055, PIVOT_Z - FLOOR_Z - 0.05, (0, 0, (PIVOT_Z + FLOOR_Z + 0.05) / 2), "Z", dark, body, seg=12)
for k in range(3):
    a = k * 2 * math.pi / 3
    cyl(f"pedestal_stay_{k}", 0.018, 0.5, (math.cos(a) * 0.1, math.sin(a) * 0.1, FLOOR_Z + 0.25), "Z", dark, body,
        rot=(math.sin(a) * -0.35, math.cos(a) * 0.35, 0), seg=6, lods=(0,))
hmg = empty("hmg", (0, 0, PIVOT_Z), body)
cyl("hmg_bearing", 0.075, 0.08, (0, 0, 0.0), "Z", dark, hmg, seg=14, lods=(0, 1, 2))
box("hmg_cradle_post", (0.08, 0.08, 0.3), (0.1, 0, 0.16), dark, hmg, lods=(0, 1, 2))
box("hmg_yoke", (0.12, 0.2, 0.04), (0.14, 0, 0.3), dark, hmg, lods=(0, 1))
hmg_gun = empty("hmg_gun", (0.15, 0, 0.32), hmg)
box("hmg_receiver", (0.32, 0.13, 0.15), (-0.02, 0, 0), dark, hmg_gun, bevel=0.01, lods=(0, 1, 2))
cyl("hmg_jacket", 0.035, 0.45, (0.36, 0, 0.0), "X", dark, hmg_gun, seg=12, lods=(0, 1, 2))
cyl("hmg_barrel", 0.018, 0.75, (0.9, 0, 0.0), "X", dark, hmg_gun, seg=10, lods=(0, 1, 2))
cyl("hmg_flash_hider", 0.028, 0.08, (1.24, 0, 0.0), "X", dark, hmg_gun, seg=10, lods=(0, 1))
box("hmg_grips", (0.08, 0.12, 0.08), (-0.22, 0, 0.0), dark, hmg_gun, lods=(0, 1))
box("hmg_sight", (0.03, 0.03, 0.06), (0.14, 0, 0.1), dark, hmg_gun, lods=(0,))
box("hmg_ammo", (0.26, 0.12, 0.18), (0.02, -0.14, -0.08), ammo, hmg_gun, bevel=0.01, lods=(0, 1))
box("hmg_brass_bag", (0.2, 0.1, 0.16), (0.02, 0.12, -0.1), canvas, hmg_gun, bevel=0.03, lods=(0,))
hmg_muzzle = empty("hmg_muzzle", (1.28, 0, 0), hmg_gun)

# ------------------------------------------------------------------ the wreck
if WRECK:
    from wreckage import bend, dent, densify, heat, parts, plate, remove, warp

    # burnt out: the tyres, canvas, glass, seats' cushions and stowage are gone; it settles on
    # its rims; the gun hangs off its pedestal; the bonnet sprang open
    remove("windscreen_cover", "mirror_", "headlamp_", "tail_light_", "blackout_lamp_", "kit_bag", "bedroll",
           "seat_L", "seat_R", "seat_back_", "rear_bench_LOD", "antenna", "radio_knob_", "gauge_", "spare_tyre",
           "hmg_brass_bag", "jerrycan_LOD")
    for o in list(bpy.data.objects):
        if o.type == "MESH" and o.name.startswith("wheel_") and ("_tyre_LOD" in o.name or "_lug_" in o.name):
            bpy.data.objects.remove(o, do_unlink=True)
    debris_m = textured("debris", "burnt_metal", colour=(0.03, 0.028, 0.026), chip=0.3, dirt=0.1, ash=0.08, seed=19.0)
    wire_m = textured("tyre_wire", "burnt_metal", colour=(0.07, 0.04, 0.022), chip=0.2, dirt=0.2, ash=0.05, seed=21.0)
    # the seats burnt to their springs and frames
    for s, n in ((1, "L"), (-1, "R")):
        box(f"seat_frame_{n}", (0.42, 0.4, 0.03), (-0.25, s * 0.37, 0.86), debris_m, body, lods=(0, 1, 2))
        box(f"seat_back_frame_{n}", (0.03, 0.4, 0.42), (-0.52, s * 0.37, 1.1), debris_m, body,
            rot=(0, math.radians(-25), 0), lods=(0, 1))
    box("rear_bench_frame", (0.42, 2 * TUB_Y - 0.12, 0.03), (-1.55, 0, 0.88), debris_m, body, lods=(0, 1, 2))
    # the panels warped by the fire; the bonnet dented and sprung up at the rear
    panels = parts("tub_side_", "tub_rear", "cowl", "bonnet", "front_fender_", "rear_fender_", "grille", "floor")
    densify(panels)
    warp(panels, heat(0.03, 0.6, 5.0), heat(0.01, 0.2, 8.0))
    warp(parts("bonnet"), dent((1.3, 0.2, 1.3), 0.6, 0.1, (0, 0, -1), 3.0))
    bend(parts("bonnet"), (2.0, 0, 1.12), (0, 1, 0), (0, 0, 1), math.radians(-9))
    bend(parts("front_fender_L", "front_fender_nose_L"), (1.0, 0, 0.99), (0, 1, 0), (0, 0, 1), math.radians(10))
    # the HMG hangs off its pedestal, swung aside, its barrel drooping
    hmg.rotation_euler = (math.radians(6), 0, math.radians(-125))
    hmg_gun.rotation_euler = (0, math.radians(30), 0)
    # each rim with the burnt tyre's steel carcass slumped round it
    for row, x in AXLES:
        for s, sn in ((1, "L"), (-1, "R")):
            n = f"wheel_{row}{sn}"
            node = bpy.data.objects[n]
            for k, y in enumerate((-0.13, 0.13)):
                cyl(f"{n}_flange_{k}", 0.25, 0.025, (0, y, 0), "Y", dark, node, seg=18, lods=(0, 1))
            cyl(f"{n}_carcass", 0.36, 0.22, (0, 0, 0), "Y", wire_m, node, seg=22, caps=False, lods=(0, 1, 2))
    cyl("spare_carcass", 0.34, 0.16, (-2.1, 0.3, 0.9), "X", wire_m, body, seg=22, caps=False, lods=(0, 1, 2))
    # thrown clear: the windscreen frame, a jerrycan, a sheet of the tub
    plate("fender_torn", [(-0.3, -0.2), (0.35, -0.18), (0.3, 0.22), (-0.28, 0.19)], 0.02, (1.1, 1.45, 0.03),
          (0.05, -0.04, 0.6), debris_m, body, curl=0.1, seed=41)
    box("jerrycan_thrown", (0.46, 0.14, 0.34), (-2.0, -1.35, 0.075), debris_m, body, rot=(math.radians(90), 0, 0.4),
        bevel=0.02, lods=(0, 1, 2))
    # it settled on its rims, nose down and askew
    jeep.location = (0, 0, -(WHEEL_Z - 0.25) - 0.05)
    jeep.rotation_euler = (math.radians(2.5), math.radians(2.0), 0)

    bpy.context.view_layer.update()
    jw = jeep.matrix_world
    for vent, reach in (((0.0, 0.0, 1.1), 1.8), ((-1.4, 0.0, 1.1), 1.8), ((1.3, 0.0, 1.2), 1.4)):
        SCORCH.append((jw @ Vector(vent), reach))

rest_on_ground(0.006 if WRECK else 0.0)
finish(ao_distance=1.0)
bpy.context.view_layer.update()
info = dict(tris=triangles_by_tier(), muzzle=[round(v, 4) for v in hmg_muzzle.matrix_world.translation],
            nodes=sorted(o.name for o in bpy.data.objects if o.type == "EMPTY"))
print("JEEP", json.dumps(info))
export(OUT)
