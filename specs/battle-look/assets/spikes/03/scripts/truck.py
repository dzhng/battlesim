"""Spike 03: supply truck with wheels, four deploy legs and a telescoping mast, driven by one deploy progress.

Engine basis: Z up, +X forward, origin at the hull centre on the ground. Authority: physics.supply_half_extents_m [3, 1.4, 1.8].
Nodes: truck > body > {wheel_*, deploy_leg_{FL,FR,RL,RR} > deploy_leg_*_jack, deploy_mast > deploy_mast_2 > deploy_mast_3}
"""
import bpy, sys, os, math, json
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import *

ARGS = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
MODE = ARGS[0] if ARGS else "all"
scn = reset()

paint = camo_mat("truck_paint", [(0.14, 0.16, 0.09), (0.12, 0.14, 0.08), (0.14, 0.16, 0.09)], scale=0.4, wear=0.3, dirt=0.7, rough=0.7, seed=5)
dark = mat("chassis", (0.05, 0.05, 0.045), rough=0.7)
rubber = mat("tyre", (0.03, 0.03, 0.03), rough=0.9)
steel = mat("steel", (0.25, 0.25, 0.24), rough=0.4, metal=0.8)
glass = mat("glass", (0.03, 0.05, 0.06), rough=0.05)
canvas = mat("canvas", (0.24, 0.22, 0.15), rough=0.95)

truck = empty("truck", size=0.5)
body = empty("body", (0, 0, 0), truck, 0.3)
# chassis rails, bumper
for y in (-0.45, 0.45):
    box(f"rail_{y}", (5.8, 0.12, 0.25), (0, y, 0.85), dark, body)
box("bumper", (0.18, 2.5, 0.25), (2.95, 0, 0.75), dark, body, bevel=0.02)
# cab-forward cab
prism("cab", [(1.55, 1.0), (2.90, 1.0), (2.95, 1.9), (2.78, 2.85), (1.55, 2.90)], 2.5, mat_=paint, parent=body, bevel=0.05)
box("windscreen", (0.05, 2.1, 0.7), (2.87, 0, 2.35), glass, body, rot=(0, math.radians(-10), 0))
for s in (-1, 1):
    box(f"side_window_{s}", (0.8, 0.03, 0.55), (2.3, s * 1.255, 2.4), glass, body)
    box(f"mirror_{s}", (0.05, 0.22, 0.32), (2.75, s * 1.42, 2.4), dark, body)
    box(f"step_{s}", (0.5, 0.25, 0.06), (2.3, s * 1.2, 0.95), steel, body)
    box(f"fuel_tank_{s}", (0.9, 0.35, 0.45), (0.9, s * 1.05, 1.0), dark, body, bevel=0.05)
box("grille", (0.04, 1.2, 0.5), (2.94, 0, 1.4), dark, body)
for s in (-1, 1):
    cyl(f"headlamp_{s}", 0.11, 0.05, (2.96, s * 0.9, 1.35), "X", mat("lamp", (0.7, 0.7, 0.6), rough=0.2), body)
box("roof_hatch", (0.6, 0.6, 0.06), (2.1, 0.3, 2.93), paint, body, bevel=0.02)
# shelter / cargo body
box("shelter", (4.25, 2.5, 2.0), (-0.95, 0, 2.05), paint, body, bevel=0.04)
box("shelter_floor", (4.3, 2.55, 0.12), (-0.95, 0, 1.0), dark, body)
for i in range(5):
    box(f"shelter_rib_{i}", (0.06, 2.56, 2.02), (-2.9 + i * 0.97, 0, 2.05), paint, body)
box("shelter_door", (0.05, 0.9, 1.7), (-3.08, 0.4, 2.0), paint, body, bevel=0.01)
box("aircon", (0.5, 0.8, 0.45), (1.0, 0, 3.15), paint, body, bevel=0.03)
box("tarp_roll", (3.0, 0.25, 0.25), (-1.0, -1.1, 3.1), canvas, body, bevel=0.1)
box("jerrycans", (0.6, 0.2, 0.45), (-2.5, 1.35, 1.3), paint, body, bevel=0.02)

# wheels: front axle + tandem rear, radius 0.55
WHEELS = {}
for name, x in (("F", 2.1), ("M", -1.2), ("R", -2.5)):
    for s, sn in ((1, "L"), (-1, "R")):
        n = f"wheel_{name}{sn}"
        node = empty(n, (x, s * 1.02, 0.55), body, 0.2)
        cyl(n + "_tyre", 0.55, 0.40, (0, 0, 0), "Y", rubber, node, seg=32, bevel=0.04)
        cyl(n + "_rim", 0.30, 0.42, (0, 0, 0), "Y", dark, node, seg=20)
        for k in range(8):
            a = k * math.pi / 4
            cyl(f"{n}_nut_{k}", 0.025, 0.46, (math.cos(a) * 0.18, 0, math.sin(a) * 0.18), "Y", steel, node, seg=6)
        for k in range(16):
            a = k * math.pi / 8
            box(f"{n}_lug_{k}", (0.09, 0.40, 0.03), (math.cos(a) * 0.55, 0, math.sin(a) * 0.55), rubber, node, rot=(0, -a, 0))
        WHEELS[n] = 0.55
    box(f"fender_{name}", (1.25, 2.6, 0.05), (x, 0, 1.18), paint, body, bevel=0.01)

# deploy legs: outrigger beam slides out sideways, then a jack lowers to the ground
LEGS = {}
for n, x, s in (("FL", 0.9, 1), ("FR", 0.9, -1), ("RL", -2.8, 1), ("RR", -2.8, -1)):
    beam = empty(f"deploy_leg_{n}", (x, s * 1.0, 0.95), body, 0.15)
    box(f"leg_{n}_beam", (0.16, 0.6, 0.16), (0, s * 0.15, 0), steel, beam, bevel=0.01)
    jack = empty(f"deploy_leg_{n}_jack", (0, s * 0.42, 0), beam, 0.1)
    cyl(f"leg_{n}_cyl", 0.07, 0.5, (0, 0, -0.1), "Z", dark, jack, seg=12)
    pad = empty(f"deploy_leg_{n}_pad", (0, 0, -0.85), jack, 0.05)
    cyl(f"leg_{n}_rod", 0.045, 0.8, (0, 0, 0.4), "Z", steel, pad, seg=10)
    cyl(f"leg_{n}_pad", 0.16, 0.05, (0, 0, 0), "Z", dark, pad, seg=16)
    LEGS[n] = (beam, jack, pad, s)

# mast: hinged at the shelter roof rear, lies along the roof, raises to vertical, then telescopes
mast = empty("deploy_mast", (-2.7, -0.6, 3.1), body, 0.15)
cyl("mast_base_tube", 0.09, 2.2, (0, 0, 1.1), "Z", paint, mast, seg=14)
m2 = empty("deploy_mast_2", (0, 0, 0.3), mast, 0.1)
cyl("mast_tube_2", 0.07, 2.0, (0, 0, 1.0), "Z", steel, m2, seg=12)
m3 = empty("deploy_mast_3", (0, 0, 0.3), m2, 0.1)
cyl("mast_tube_3", 0.05, 2.0, (0, 0, 1.0), "Z", steel, m3, seg=12)
head = empty("deploy_mast_head", (0, 0, 2.0), m3, 0.1)
box("mast_radio_head", (0.35, 0.35, 0.25), (0, 0, 0.1), dark, head, bevel=0.02)
for k in range(4):
    a = k * math.pi / 2
    cyl(f"mast_whip_{k}", 0.01, 1.0, (math.cos(a) * 0.25, math.sin(a) * 0.25, 0.6), "Z", dark, head, seg=6)
box("mast_cradle", (0.3, 0.3, 0.2), (0.5, -0.6, 3.1), steel, body)


def smooth(t):
    t = max(0.0, min(1.0, t))
    return t * t * (3 - 2 * t)


def set_deploy(p, distance_m=0.0):
    """p = DeploymentState.progress in [0, 1]. 0-0.3 beams out, 0.2-0.5 jacks down, 0.35-0.65 mast up, 0.6-1 telescope."""
    out = smooth(p / 0.3) * 0.55
    down = smooth((p - 0.2) / 0.3) * 0.85
    for n, (beam, jack, pad, s) in LEGS.items():
        beam.location.y = s * (1.0 + out)
        jack.location.z = -down * 0.0
        pad.location.z = -0.35 - down / 0.85 * 0.575
        # jack rod extends: pad from -0.35 (stowed) to about -0.9 below the beam (ground)
    raise_ = smooth((p - 0.35) / 0.3)
    mast.rotation_euler = (0, math.radians(90 * (1 - raise_)), 0)  # lies forward along the roof when stowed
    tele = smooth((p - 0.6) / 0.4)
    m2.location.z = 0.3 + tele * 1.8
    m3.location.z = 0.3 + tele * 1.6
    for n, r in WHEELS.items():
        bpy.data.objects[n].rotation_euler = (0, distance_m / r, 0)
    bpy.context.view_layer.update()


set_deploy(0.0)
# checks: pads reach the ground when deployed; mast top height
set_deploy(1.0)
pads = {n: round(LEGS[n][2].matrix_world.translation.z, 3) for n in LEGS}
top = bpy.data.objects["deploy_mast_head"].matrix_world.translation.z
set_deploy(0.0)
stowed_pads = {n: round(LEGS[n][2].matrix_world.translation.z, 3) for n in LEGS}
dg = bpy.context.evaluated_depsgraph_get()
tris = 0
for o in bpy.data.objects:
    if o.type == "MESH":
        me = o.evaluated_get(dg).to_mesh()
        me.calc_loop_triangles()
        tris += len(me.loop_triangles)
        o.evaluated_get(dg).to_mesh_clear()
info = dict(pad_z_deployed=pads, pad_z_stowed=stowed_pads, mast_head_z_deployed=round(top, 2), tris=tris,
            nodes=sorted(o.name for o in bpy.data.objects if o.type == "EMPTY"))
print("TRUCK", json.dumps(info))
json.dump(info, open(os.path.join(OUT, "truck_info.json"), "w"), indent=1)

setup_render((560, 560), samples=48)
light_rig(strength=3.2, sky=0.5, neutral=True)
ground(color=(0.36, 0.34, 0.29))
cam = camera(lens=50)
if MODE in ("all", "sheet"):
    set_deploy(0.0)
    shots = []
    for name, yaw, pitch in VIEWS8:
        aim(cam, (0, 0, 1.5), yaw, pitch, 16 if name not in ("battle", "top") else 20)
        p = os.path.join(OUT, f"truck_view_{name}.png")
        render_to(p)
        shots.append(p)
    tile(shots, os.path.join(OUT, "truck_sheet.png"), 4)
if MODE in ("all", "strips"):
    shots = []
    for i in range(8):
        set_deploy(i / 7)
        aim(cam, (-0.5, 0, 2.6), -35, 18, 19)
        p = os.path.join(OUT, f"truck_deploy_{i}.png")
        render_to(p)
        shots.append(p)
    tile(shots, os.path.join(OUT, "truck_strip_deploy.png"), 8)
if MODE in ("all", "export"):
    set_deploy(0.0)
    for o in list(bpy.data.objects):
        if o.type in ("CAMERA", "LIGHT") or o.name == "ground":
            bpy.data.objects.remove(o, do_unlink=True)
    path = os.path.join(OUT, "truck.glb")
    bpy.ops.export_scene.gltf(filepath=path, export_apply=True, export_yup=True)
    print("GLB", path, os.path.getsize(path))
