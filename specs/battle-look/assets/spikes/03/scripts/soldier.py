"""Spike 03: rifleman on the Quaternius UBC/UAL rig, modern kit, library + authored clips.

Coordinates while building: Blender rest space of the imported rig (Z up, soldier faces -Y, left is +X).
At the end the rig root is rotated +90 deg about Z so the soldier faces +X (engine basis: Z up, +X forward).
"""
import bpy, bmesh, sys, os, math, json
from mathutils import Vector, Matrix, Quaternion

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import *

DL = os.path.join(HERE, "dl")
UAL = os.path.join(DL, "ual/Universal Animation Library[Standard]/Unreal-Godot/UAL1_Standard.glb")
UBC = os.path.join(DL, "ubc/Universal Base Characters[Standard]/Base Characters/Godot - UE/Superhero_Male_FullBody.gltf")
ARGS = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
MODE = ARGS[0] if ARGS else "all"

SIM_HEIGHT = 1.70  # physics.soldier_height_m
FPS = 30
FWD = Vector((0, -1, 0))
UP = Vector((0, 0, 1))
LEFT = Vector((1, 0, 0))
RIGHT = Vector((-1, 0, 0))

scn = reset()
scn.render.fps = FPS

# ---------------------------------------------------------------- import
bpy.ops.import_scene.gltf(filepath=UBC)
arm = [o for o in bpy.data.objects if o.type == "ARMATURE"][0]
arm.name = "soldier_rig"
body = bpy.data.objects["SuperHero_Male"]
for n in ("Icosphere",):
    if n in bpy.data.objects:
        bpy.data.objects.remove(bpy.data.objects[n], do_unlink=True)
before = set(bpy.data.objects)
bpy.ops.import_scene.gltf(filepath=UAL)
for o in set(bpy.data.objects) - before:
    bpy.data.objects.remove(o, do_unlink=True)
for a in bpy.data.actions:
    a.use_fake_user = True
LIB = {a.name for a in bpy.data.actions}
arm.animation_data_create()


def set_action(act):
    arm.animation_data.action = act
    if act is not None and act.slots:
        arm.animation_data.action_slot = act.slots[0]


def rest_mode(on):
    arm.data.pose_position = "REST" if on else "POSE"
    bpy.context.view_layer.update()


def group_weight(v, names, me_obj):
    idx = {me_obj.vertex_groups[n].index for n in names if n in me_obj.vertex_groups}
    return sum(g.weight for g in v.groups if g.group in idx)


# ---------------------------------------------------------------- clothing shells from the body
def shell(name, keep, offset, smooth_iters, material, cast=0.0):
    """Copy the body, keep verts where keep(v_co, v) is true, smooth away anatomy, push out along normals.
    Keeps every vertex group, so it skins with the body."""
    o = body.copy()
    o.data = body.data.copy()
    o.name = name
    o.data.name = name
    scn.collection.objects.link(o)
    bm = bmesh.new()
    bm.from_mesh(o.data)
    dl = bm.verts.layers.deform.active
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
    kill = [v for v in bm.verts if not keep(v.co, v, dl)]
    bmesh.ops.delete(bm, geom=kill, context="VERTS")
    bm.to_mesh(o.data)
    bm.free()
    o.data.materials.clear()
    o.data.materials.append(material)
    for m in list(o.modifiers):
        o.modifiers.remove(m)
    if smooth_iters:
        sm = o.modifiers.new("smooth", "SMOOTH")
        sm.factor = 0.9
        sm.iterations = smooth_iters
    if cast:
        c = o.modifiers.new("cast", "CAST")
        c.cast_type = "CUBOID"
        c.factor = cast
        c.use_z = True
    d = o.modifiers.new("inflate", "DISPLACE")
    d.strength = offset
    d.mid_level = 0.0
    sol = o.modifiers.new("solid", "SOLIDIFY")
    sol.thickness = 0.006
    a = o.modifiers.new("armature", "ARMATURE")
    a.object = arm
    o.parent = arm
    return o


def gidx(names):
    return {body.vertex_groups[n].index for n in names if n in body.vertex_groups}


HAND_G = gidx([g.name for g in body.vertex_groups if g.name.startswith(("hand_", "index", "middle", "ring", "pinky", "thumb"))])
HEAD_G = gidx(["Head", "neck_01"])
FOOT_G = gidx(["foot_l", "foot_r", "ball_l", "ball_r"])


def w_of(v, dl, groups):
    d = v[dl]
    return sum(w for gi, w in d.items() if gi in groups)


uniform = camo_mat("uniform", [(0.30, 0.29, 0.20), (0.22, 0.24, 0.15), (0.36, 0.31, 0.22), (0.12, 0.11, 0.08)], scale=9.0, rough=0.85, coord="Object")
gear = mat("gear_ranger_green", (0.16, 0.17, 0.11), rough=0.8)
coyote = mat("gear_coyote", (0.36, 0.29, 0.19), rough=0.85)
vest_m = mat("vest_coyote", (0.33, 0.27, 0.18), rough=0.9)
_nt = vest_m.node_tree
_w = _nt.nodes.new("ShaderNodeTexWave")
_w.wave_type = "BANDS"
_w.bands_direction = "Z"
_w.inputs["Scale"].default_value = 28
_tc = _nt.nodes.new("ShaderNodeTexCoord")
_nt.links.new(_tc.outputs["Object"], _w.inputs["Vector"])
_b = _nt.nodes.new("ShaderNodeBump")
_b.inputs["Strength"].default_value = 0.35
_nt.links.new(_w.outputs["Fac"], _b.inputs["Height"])
_nt.links.new(_b.outputs["Normal"], _nt.nodes["Principled BSDF"].inputs["Normal"])
boots_m = mat("boots", (0.16, 0.12, 0.08), rough=0.7)
glove_m = mat("gloves", (0.10, 0.10, 0.09), rough=0.75)
black = mat("gun_black", (0.035, 0.035, 0.035), rough=0.45, metal=0.3)
gun_tan = mat("gun_fde", (0.30, 0.24, 0.16), rough=0.6)

uniform_o = shell(
    "uniform",
    lambda co, v, dl: w_of(v, dl, HAND_G) < 0.5 and w_of(v, dl, FOOT_G) < 0.5 and co.z > 0.16 and co.z < 1.60 and w_of(v, dl, HEAD_G) < 0.85,
    0.02, 16, uniform)
boots_o = shell("boot_shafts", lambda co, v, dl: 0.08 < co.z < 0.27, 0.022, 2, boots_m)
gloves_o = shell("gloves", lambda co, v, dl: w_of(v, dl, HAND_G) > 0.3, 0.004, 1, glove_m)
PELV_G = gidx(["pelvis", "spine_01"])
vest_o = shell(
    "vest",
    lambda co, v, dl: 1.04 < co.z < 1.42 and abs(co.x) < 0.2,
    0.034, 6, vest_m)
belt_o = shell("belt", lambda co, v, dl: 0.96 < co.z < 1.03 and w_of(v, dl, PELV_G) > 0.6, 0.022, 2, gear)

# hide body under clothes: keep head/neck and a little overlap
bm = bmesh.new()
bm.from_mesh(body.data)
dl = bm.verts.layers.deform.active
bmesh.ops.delete(bm, geom=[v for v in bm.verts if v.co.z < 1.40 or (v.co.z < 1.60 and w_of(v, dl, HEAD_G) < 0.2)], context="VERTS")
bm.to_mesh(body.data)
bm.free()
skin = mat("skin", (0.42, 0.27, 0.19), rough=0.55)
body.data.materials.clear()
body.data.materials.append(skin)
for o in ("Eyes", "Eyebrows"):
    pass


# ---------------------------------------------------------------- rigid kit, bone-parented
def bone_parent(o, bone):
    bpy.context.view_layer.update()
    mw = o.matrix_world.copy()
    o.parent = arm
    o.parent_type = "BONE"
    o.parent_bone = bone
    bpy.context.view_layer.update()
    o.matrix_world = mw


rest_mode(True)
# helmet (FAST/ACH style): half ellipsoid, cover camo, NVG shroud, side rails
bmh = bmesh.new()
bmesh.ops.create_uvsphere(bmh, u_segments=24, v_segments=14, radius=1.0)
bmesh.ops.delete(bmh, geom=[v for v in bmh.verts if v.co.z < -0.12], context="VERTS")
for v in bmh.verts:
    # ear cut: raise the rim at the sides
    x, y, z = v.co
    v.co = Vector((x * 0.122, y * 0.142, z * 0.118))
    if z < 0.05 and abs(x) > 0.6 and y > -0.3:
        v.co.z += 0.03 * (abs(x) - 0.6) / 0.4
for f in bmh.faces:
    f.smooth = True
helmet = obj_from_bm("helmet", bmh, [camo_mat("helmet_cover", [(0.30, 0.28, 0.19), (0.22, 0.23, 0.14), (0.33, 0.27, 0.18), (0.12, 0.11, 0.08)], scale=14, rough=0.9, seed=3.0)])
helmet.location = (0, 0.012, 1.715)
s = helmet.modifiers.new("solid", "SOLIDIFY")
s.thickness = 0.012
nvg = box("helmet_nvg_mount", (0.045, 0.02, 0.035), (0, -0.140, 1.765), black, bevel=0.004)
rl = box("helmet_rail_l", (0.012, 0.09, 0.02), (0.118, 0.0, 1.70), black)
rr = box("helmet_rail_r", (0.012, 0.09, 0.02), (-0.118, 0.0, 1.70), black)
for o in (helmet, nvg, rl, rr):
    bone_parent(o, "Head")
# pouches on the plate carrier (front, facing -Y), radio, admin
kit = []
for i, x in enumerate((-0.085, 0.0, 0.085)):
    kit.append(box(f"mag_pouch_{i}", (0.07, 0.045, 0.11), (x, -0.19, 1.16), coyote, bevel=0.008))
kit.append(box("plate_front", (0.27, 0.04, 0.28), (0, -0.150, 1.28), coyote, bevel=0.02))
kit.append(box("plate_back", (0.27, 0.04, 0.30), (0, 0.135, 1.28), coyote, bevel=0.02))
kit.append(box("admin_pouch", (0.16, 0.03, 0.08), (0, -0.183, 1.30), coyote, bevel=0.008))
kit.append(box("radio_pouch", (0.07, 0.06, 0.13), (-0.14, 0.13, 1.25), coyote, bevel=0.01))
kit.append(box("pack", (0.26, 0.15, 0.36), (0.0, 0.225, 1.23), gear, bevel=0.03))
kit.append(box("pack_lid", (0.24, 0.10, 0.08), (0.0, 0.22, 1.43), gear, bevel=0.025))
kit.append(cyl("radio_antenna", 0.004, 0.30, (-0.10, 0.27, 1.60), "Z", black, seg=6))
for o in kit:
    bone_parent(o, "spine_03")
sole_m = mat("sole", (0.05, 0.045, 0.04), rough=0.8)
for side, bx in (("l", 0.114), ("r", -0.114)):
    bt = box(f"boot_{side}", (0.10, 0.26, 0.085), (bx, -0.025, 0.058), boots_m, bevel=0.03)
    bone_parent(bt, f"foot_{side}")
    sl = box(f"sole_{side}", (0.106, 0.27, 0.025), (bx, -0.025, 0.013), sole_m, bevel=0.008)
    bone_parent(sl, f"foot_{side}")
for side, bx in (("l", 0.12), ("r", -0.12)):
    kp = box(f"knee_pad_{side}", (0.10, 0.05, 0.11), (bx * 0.97, -0.055, 0.53), gear, bevel=0.015)
    bone_parent(kp, f"calf_{side}")
    hp = box(f"hip_pouch_{side}", (0.05, 0.10, 0.10), (bx * 1.55, 0.03, 0.93), coyote, bevel=0.01)
    bone_parent(hp, "pelvis")


# ---------------------------------------------------------------- rifle (modern carbine), local -Y forward, origin at grip
def build_rifle():
    root = empty("rifle", (0, 0, 0), size=0.05)
    S = 0.86
    parts = [
        ("upper", box, dict(size=(0.032, 0.25, 0.055), loc=(0, -0.05, 0.045), mat_=black, bevel=0.003)),
        ("lower", box, dict(size=(0.030, 0.17, 0.045), loc=(0, 0.0, 0.0), mat_=black, bevel=0.003)),
        ("guard", box, dict(size=(0.046, 0.30, 0.05), loc=(0, -0.32, 0.045), mat_=gun_tan, bevel=0.006)),
        ("barrel", cyl, dict(r=0.009, depth=0.14, loc=(0, -0.54, 0.045), axis="Y", mat_=black, seg=10)),
        ("muzzle_dev", cyl, dict(r=0.015, depth=0.06, loc=(0, -0.63, 0.045), axis="Y", mat_=black, seg=10)),
        ("mag", box, dict(size=(0.024, 0.07, 0.17), loc=(0, -0.07, -0.08), mat_=gun_tan, bevel=0.004, rot=(math.radians(-12), 0, 0))),
        ("grip", box, dict(size=(0.028, 0.038, 0.10), loc=(0, 0.06, -0.05), mat_=black, bevel=0.005, rot=(math.radians(22), 0, 0))),
        ("stock", box, dict(size=(0.036, 0.20, 0.07), loc=(0, 0.28, 0.03), mat_=gun_tan, bevel=0.008)),
        ("buffer", cyl, dict(r=0.016, depth=0.12, loc=(0, 0.16, 0.045), axis="Y", mat_=black, seg=10)),
        ("optic", cyl, dict(r=0.018, depth=0.11, loc=(0, -0.05, 0.11), axis="Y", mat_=black, seg=14)),
        ("optic_mount", box, dict(size=(0.02, 0.05, 0.03), loc=(0, -0.05, 0.085), mat_=black)),
        ("fore_grip", box, dict(size=(0.022, 0.03, 0.07), loc=(0, -0.30, -0.01), mat_=black, bevel=0.004)),
        ("light", cyl, dict(r=0.012, depth=0.07, loc=(0.03, -0.40, 0.05), axis="Y", mat_=black, seg=10)),
        ("sling", box, dict(size=(0.004, 0.55, 0.02), loc=(-0.025, -0.05, -0.02), mat_=gear)),
    ]
    for n, fn, kw in parts:
        o = fn("rifle_" + n, **kw)
        o.parent = root
    root.scale = (S, S, S)
    marks = {
        "rifle_muzzle": (0, -0.67, 0.045),
        "rifle_support": (0, -0.30, 0.02),
        "rifle_butt": (0, 0.39, 0.03),
    }
    for n, p in marks.items():
        e = empty(n, p, root, 0.02)
    return root


rifle = build_rifle()


# ---------------------------------------------------------------- orientation helpers
def frame_from(y_axis, n_axis):
    f1 = y_axis.normalized()
    f2 = (n_axis - n_axis.dot(f1) * f1).normalized()
    f3 = f1.cross(f2)
    return Matrix((f1, f2, f3)).transposed()


REST_B = {b.name: b.matrix_local.to_3x3() for b in arm.data.bones}


def hand_orient(bone, fingers_dir, palm_dir, palm_rest=Vector((0, 0, -1))):
    """Armature-space rotation that points the hand bone along fingers_dir with its palm facing palm_dir."""
    B0 = REST_B[bone]
    p_l = B0.transposed() @ palm_rest
    E = frame_from(Vector((0, 1, 0)), p_l)
    F = frame_from(fingers_dir, palm_dir)
    return F @ E.transposed()


# ---------------------------------------------------------------- rifle hold rig (constraints; baked later)
def make_hold(tag, rifle_mw, spine_follow=True):
    """Place the rifle at rifle_mw (rest-space world), make grip/support targets, return targets."""
    anchor = empty(f"anchor_{tag}", (0, 0, 0), size=0.05)
    anchor.matrix_world = rifle_mw
    bpy.context.view_layer.update()
    R = rifle_mw.to_3x3().normalized()
    fwd = R @ Vector((0, -1, 0))
    up = R @ Vector((0, 0, 1))
    side = fwd.cross(up)  # rifle right
    # right hand: fingers wrap the grip: fingers point forward-down, palm faces the rifle's left side
    rh_rot = hand_orient("hand_r", (fwd * 0.55 - up * 0.8 + side * 0.25), -side)
    grip_pt = rifle_mw @ Vector((0, 0.06 / 0.86 * 0.86, -0.035))
    wrist_r = grip_pt - rh_rot @ Vector((0, 0.075, 0)) + side * 0.03 + up * 0.01
    # left hand: under the handguard, palm up, fingers across to the right
    lh_rot = hand_orient("hand_l", (fwd * 0.45 + side * 0.85 + up * 0.25), up * 0.9 - side * 0.3)
    sup_pt = rifle_mw @ Vector((0, -0.30, 0.0))
    wrist_l = sup_pt - lh_rot @ Vector((0, 0.07, 0)) - up * 0.03
    tr = empty(f"ik_hand_r_{tag}", wrist_r, size=0.03)
    tl = empty(f"ik_hand_l_{tag}", wrist_l, size=0.03)
    tr.rotation_mode = tl.rotation_mode = "QUATERNION"
    tr.rotation_quaternion = rh_rot.to_quaternion()
    tl.rotation_quaternion = lh_rot.to_quaternion()
    bpy.context.view_layer.update()
    for e in (tr, tl):
        mw = e.matrix_world.copy()
        e.parent = anchor
        bpy.context.view_layer.update()
        e.matrix_world = mw
    if spine_follow:
        bone_parent(anchor, "spine_03")
    else:
        mw = anchor.matrix_world.copy()
        anchor.parent = arm
        bpy.context.view_layer.update()
        anchor.matrix_world = mw
    return anchor, tr, tl


POLES = {}


def pole(name, loc, bone_parent_name="spine_03"):
    e = empty(name, loc, size=0.03)
    bone_parent(e, bone_parent_name)
    return e


def arm_constraints(tr, tl, pr, plq):
    cons = []
    for side, t, p in (("r", tr, pr), ("l", tl, plq)):
        pb = arm.pose.bones[f"lowerarm_{side}"]
        c = pb.constraints.new("IK")
        c.target = t
        c.pole_target = p
        c.pole_angle = math.radians(-90 if side == "r" else -90)
        c.chain_count = 2
        cons.append((pb, c))
        hb = arm.pose.bones[f"hand_{side}"]
        c2 = hb.constraints.new("COPY_ROTATION")
        c2.target = t
        cons.append((hb, c2))
    return cons


def leg_constraints(fl, fr, pl, pr):
    cons = []
    for side, t, p in (("l", fl, pl), ("r", fr, pr)):
        pb = arm.pose.bones[f"calf_{side}"]
        c = pb.constraints.new("IK")
        c.target = t
        c.pole_target = p
        c.pole_angle = math.radians(-90)
        c.chain_count = 2
        cons.append((pb, c))
    return cons


def rifle_rest_matrix(pos, yaw_deg=0.0, pitch_deg=0.0, roll_deg=0.0):
    """Rifle pointing along -Y (soldier forward), pitched up by pitch_deg, yawed left by yaw_deg."""
    R = (Matrix.Rotation(math.radians(yaw_deg), 4, "Z") @ Matrix.Rotation(math.radians(pitch_deg), 4, "X")
         @ Matrix.Rotation(math.radians(roll_deg), 4, "Y"))
    return Matrix.Translation(Vector(pos)) @ R


# finger grip pose: copy curled finger rotations from a library frame (fists)
FINGERS = [b.name for b in arm.pose.bones if b.name.startswith(("index", "middle", "ring", "pinky", "thumb"))]


def capture_fingers(action, frame):
    set_action(bpy.data.actions[action])
    scn.frame_set(frame)
    return {n: arm.pose.bones[n].rotation_quaternion.copy() for n in FINGERS}


rest_mode(False)
FIST = capture_fingers("Idle_Loop", 0)
set_action(None)
rest_mode(True)

# ---------------------------------------------------------------- hold rigs
# patrol/low ready: stock at the right chest, muzzle forward-down and slightly left
HOLD_LOW = rifle_rest_matrix((-0.13, -0.22, 1.10), yaw_deg=30, pitch_deg=-30)
# shouldered: butt in the right shoulder pocket, bore at cheek height (kneel_fire), set after torso pose
anchor_low, tr_low, tl_low = make_hold("low", HOLD_LOW)
pole_r = pole("pole_elbow_r", (-0.45, 0.25, 0.95))
pole_l = pole("pole_elbow_l", (0.35, -0.10, 0.85))
rest_mode(False)

# ---------------------------------------------------------------- baking
def select_pose():
    bpy.context.view_layer.objects.active = arm
    arm.select_set(True)
    if bpy.context.object.mode != "POSE":
        bpy.ops.object.mode_set(mode="POSE")
    for pb in arm.pose.bones:
        pb.select = True


def bake(name, f0, f1, cons):
    select_pose()
    bpy.ops.nla.bake(frame_start=f0, frame_end=f1, only_selected=False, visual_keying=True,
                     clear_constraints=False, use_current_action=False, bake_types={"POSE"})
    act = arm.animation_data.action
    act.name = name
    act.use_fake_user = True
    bpy.ops.object.mode_set(mode="OBJECT")
    return act


def set_cons(cons, on):
    for pb, c in cons:
        c.mute = not on


def apply_fist(frame):
    for n, q in FIST.items():
        pb = arm.pose.bones[n]
        pb.rotation_quaternion = q
        pb.keyframe_insert("rotation_quaternion", frame=frame)


def iter_fcurves(act):
    # Blender 5 slotted actions: layers -> strips -> channelbags
    out = []
    for layer in act.layers:
        for strip in layer.strips:
            for cb in strip.channelbags:
                out.extend(cb.fcurves)
    return out


def strip_finger_keys(act, keep_frames):
    """Keep only our fist keys on finger curves (the library animates fingers per clip)."""
    for fc in iter_fcurves(act):
        if any(f'"{n}"' in fc.data_path for n in FINGERS):
            pts = fc.keyframe_points
            for k in reversed(list(pts)):
                if k.co.x not in keep_frames:
                    pts.remove(k)


ALL_CONS = []


def only(cons):
    for pb, c in ALL_CONS:
        c.mute = True
    for pb, c in cons:
        c.mute = False


def damp_rotation(act, bones, k):
    """Slerp each keyed rotation of these bones toward rest by factor k (0 = rest, 1 = unchanged)."""
    fcs = iter_fcurves(act)
    for b in bones:
        comp = {fc.array_index: fc for fc in fcs if fc.data_path == f'pose.bones["{b}"].rotation_quaternion'}
        if len(comp) != 4:
            continue
        frames = sorted({kp.co.x for kp in comp[0].keyframe_points})
        for f in frames:
            q = Quaternion([comp[i].evaluate(f) for i in range(4)])
            q2 = Quaternion((1, 0, 0, 0)).slerp(q, k)
            for i in range(4):
                for kp in comp[i].keyframe_points:
                    if kp.co.x == f:
                        kp.co.y = q2[i]
                        kp.handle_left.y = q2[i]
                        kp.handle_right.y = q2[i]


def composite(role, lib_name, cons):
    """Library clip for legs/torso + rifle-hold arms (IK) + gripping fingers -> baked FK clip."""
    src = bpy.data.actions[lib_name]
    work = src.copy()
    work.name = f"_work_{role}"
    set_action(work)
    f0, f1 = int(src.frame_range[0]), int(src.frame_range[1])
    for fr in (f0, f1):
        apply_fist(fr)
    strip_finger_keys(work, (f0, f1))
    if role == "run":
        damp_rotation(work, ["spine_01", "spine_02", "spine_03", "neck_01"], 0.45)
    only(cons)
    act = bake(role, f0, f1, cons)
    only([])
    return act, (f0, f1)


def rot_world(pb, axis, deg, pivot=None):
    """Rotate a pose bone in armature space about its head (or pivot)."""
    bpy.context.view_layer.update()
    M = pb.matrix.copy()
    h = pivot if pivot is not None else M.translation.copy()
    R = Matrix.Rotation(math.radians(deg), 4, axis)
    pb.matrix = Matrix.Translation(h) @ R @ Matrix.Translation(-h) @ M
    bpy.context.view_layer.update()


def move_world(pb, delta):
    bpy.context.view_layer.update()
    M = pb.matrix.copy()
    M.translation += Vector(delta)
    pb.matrix = M
    bpy.context.view_layer.update()


def key_bones(names, frame):
    for n in names:
        pb = arm.pose.bones[n]
        pb.keyframe_insert("rotation_quaternion", frame=frame)
        pb.keyframe_insert("location", frame=frame)


def clear_pose():
    for pb in arm.pose.bones:
        pb.rotation_quaternion = (1, 0, 0, 0)
        pb.location = (0, 0, 0)
        pb.scale = (1, 1, 1)
    bpy.context.view_layer.update()


def world_empty(name, loc):
    e = empty(name, loc, size=0.04)
    e.parent = arm
    return e


def reparent_keep(o, parent):
    bpy.context.view_layer.update()
    mw = o.matrix_world.copy()
    o.parent = parent
    bpy.context.view_layer.update()
    o.matrix_world = mw


TORSO = ["root", "pelvis", "spine_01", "spine_02", "spine_03", "neck_01", "Head", "clavicle_l", "clavicle_r",
         "thigh_l", "thigh_r", "foot_l", "foot_r", "ball_l", "ball_r"]


def eye_point():
    bpy.context.view_layer.update()
    H = arm.pose.bones["Head"].matrix
    # right eye, in the head bone frame: head bone Y is up the skull, Z points back (see rest axes)
    return H @ Vector((-0.033, 0.09, 0.09))


def bake_all():
    res = {}
    # ---- library + rifle hold
    for role, lib in (("idle", "Idle_Loop"), ("walk", "Walk_Loop"), ("run", "Jog_Fwd_Loop")):
        res[role] = composite(role, lib, hold_cons)

    # ---- kneel_fire (authored)
    work = bpy.data.actions.new("_work_kneel_fire")
    set_action(work)
    clear_pose()
    P = arm.pose.bones
    move_world(P["pelvis"], (0, 0.03, -0.455))
    rot_world(P["pelvis"], "Z", -12)  # blade stance, right side back
    for n, d in (("spine_01", 4), ("spine_02", 4), ("spine_03", 3)):
        rot_world(P[n], "X", d)
    rot_world(P["spine_03"], "Z", 10)  # square the chest back toward the target
    rot_world(P["neck_01"], "X", 6)
    rot_world(P["Head"], "X", 6)
    rot_world(P["neck_01"], "Y", -10)
    rot_world(P["Head"], "Y", -10)  # cheek down onto the stock
    for fr in (0, 36):
        key_bones(TORSO, fr)
        apply_fist(fr)
    scn.frame_set(0)
    bpy.context.view_layer.update()
    eye = eye_point()
    sh = P["upperarm_r"].head.copy()
    # rifle bore line: optic axis (local z 0.11*0.86) at the eye, 8 cm in front of it
    R = Matrix.Rotation(math.radians(0), 4, "Z")
    optic_off = Vector((0, 0, 0.11 * 0.86))
    grip_to_optic_rear = Vector((0, 0.0, 0.11)) * 0.86
    butt_local = Vector((0, 0.39, 0.03)) * 0.86
    # place so the optic rear sits 8 cm ahead of the eye
    optic_rear_local = Vector((0, 0.005, 0.11)) * 0.86
    pocket = sh + Vector((0.09, -0.05, 0.0))
    grip = Vector(((eye.x + pocket.x) / 2, eye.y - 0.08, eye.z)) - optic_rear_local
    butt = grip + butt_local
    print("KNEEL eye", eye, "shoulder", sh, "butt", butt, "butt-shoulder", (butt - sh).length)
    mw = Matrix.Translation(grip)
    anchor_k, tr_k, tl_k = make_hold("kneel", mw, spine_follow=True)
    recoil = empty("recoil_kneel", (0, 0, 0), size=0.03)
    recoil.parent = anchor_k
    recoil.rotation_mode = "XYZ"
    for e in (tr_k, tl_k):
        reparent_keep(e, recoil)
    # recoil: shot at frame 3, peak at 5, recovered by 14 (loop 36 frames = 1.2 s)
    for fr, back, up in ((0, 0, 0), (3, 0, 0), (5, 0.04, 5.0), (8, 0.02, 2.5), (14, 0, 0), (36, 0, 0)):
        recoil.location = (0, back, 0)
        recoil.rotation_euler = (math.radians(-up), 0, 0)
        recoil.keyframe_insert("location", frame=fr)
        recoil.keyframe_insert("rotation_euler", frame=fr)
    pk_r = pole("pole_kneel_r", (-0.55, 0.05, 0.85))
    pk_l = pole("pole_kneel_l", (0.20, -0.30, 0.50))
    kcons = arm_constraints(tr_k, tl_k, pk_r, pk_l)
    fl = world_empty("ik_foot_l", (0.17, -0.40, 0.095))
    fr_ = world_empty("ik_foot_r", (-0.10, 0.44, 0.13))
    pl = world_empty("pole_knee_l", (0.25, -1.2, 0.8))
    pr = world_empty("pole_knee_r", (-0.12, -1.2, -0.2))
    lcons = leg_constraints(fl, fr_, pl, pr)
    ALL_CONS.extend(kcons + lcons)
    only(kcons + lcons)
    res["kneel_fire"] = (bake("kneel_fire", 0, 36, kcons + lcons), (0, 36))
    only([])

    # ---- prone_pinned (authored)
    work = bpy.data.actions.new("_work_prone")
    set_action(work)
    clear_pose()
    piv = P["pelvis"].matrix.translation.copy()
    rot_world(P["pelvis"], "X", 90)
    move_world(P["pelvis"], (0, -0.05, -0.77))
    rot_world(P["spine_03"], "X", -6)  # shoulders slightly up off the ground
    rot_world(P["thigh_l"], "Z", 9)
    rot_world(P["thigh_r"], "Z", -9)
    rot_world(P["foot_l"], "X", -35)
    rot_world(P["foot_r"], "X", -35)
    for fr, head_down, neck in ((0, -16, -4), (16, -12, -2), (22, 14, 10), (30, 12, 9), (44, -6, -1), (60, -16, -4)):
        clear_keep = [n for n in ("neck_01", "Head")]
        scn.frame_set(fr)
        base_n = P["neck_01"].rotation_quaternion.copy()
        P["neck_01"].rotation_quaternion = (1, 0, 0, 0)
        P["Head"].rotation_quaternion = (1, 0, 0, 0)
        bpy.context.view_layer.update()
        rot_world(P["neck_01"], "X", neck)
        rot_world(P["Head"], "X", head_down)
        # breathing: tiny chest lift
        key_bones(TORSO, fr)
        apply_fist(fr)
    scn.frame_set(0)
    bpy.context.view_layer.update()
    sh_r = P["upperarm_r"].head.copy()
    head = P["Head"].head.copy()
    print("PRONE shoulder_r", sh_r, "head", head)
    # rifle held across in front of the head, muzzle to the left-front, lying on the ground
    mw = rifle_rest_matrix((head.x - 0.24, head.y + 0.02, 0.10), yaw_deg=18, pitch_deg=4, roll_deg=-10)
    anchor_p, tr_p, tl_p = make_hold("prone", mw, spine_follow=False)
    pp_r = world_empty("pole_prone_r", (sh_r.x - 0.6, sh_r.y - 0.2, -0.2))
    pp_l = world_empty("pole_prone_l", (-sh_r.x + 0.6, sh_r.y - 0.2, -0.2))
    pcons = arm_constraints(tr_p, tl_p, pp_r, pp_l)
    ALL_CONS.extend(pcons)
    only(pcons)
    res["prone_pinned"] = (bake("prone_pinned", 0, 60, pcons), (0, 60))
    only([])

    # ---- death: library fall, arms keep the rifle hold (no rifle held aloft)
    res["death"] = composite("death", "Death01", hold_cons)
    return res


hold_cons = arm_constraints(tr_low, tl_low, pole_r, pole_l)


def attach_rifle_to(anchor):
    rifle.parent = None
    bpy.context.view_layer.update()
    rifle.matrix_world = anchor.matrix_world.copy()
    rifle.scale = (0.86, 0.86, 0.86)
    bpy.context.view_layer.update()
    mw = rifle.matrix_world.copy()
    rifle.parent = anchor
    bpy.context.view_layer.update()
    rifle.matrix_world = mw


def studio(res=(560, 700)):
    setup_render(res, samples=32)
    light_rig(strength=3.0, sky=0.5, neutral=True)
    ground(color=(0.36, 0.34, 0.29))
    return camera(lens=70)


ALL_CONS.extend(hold_cons)
only([])
rest_mode(False)
RES = bake_all()

# rifle: rigid on the right hand. Its transform relative to hand_r is the same for every hold, because each
# hold's right-hand target is defined in the rifle's frame.
set_action(RES["idle"][0])
scn.frame_set(0)
only(hold_cons)
attach_rifle_to(anchor_low)
bpy.context.view_layer.update()
rifle_mw = rifle.matrix_world.copy()
only([])
set_action(RES["idle"][0])
scn.frame_set(0)
bpy.context.view_layer.update()
bone_parent(rifle, "hand_r")
rifle.matrix_world = rifle_mw
bpy.context.view_layer.update()

# ---------------------------------------------------------------- basis: Z up, +X forward, scaled to authority
body_top = 1.81
S = SIM_HEIGHT / 1.81
arm.rotation_mode = "XYZ"
arm.rotation_euler = (0, 0, math.radians(90))
arm.scale = (S, S, S)
bpy.context.view_layer.update()


def measure(role, frame):
    set_action(RES[role][0])
    scn.frame_set(frame)
    bpy.context.view_layer.update()
    eye = arm.matrix_world @ eye_point()
    muz = bpy.data.objects["rifle_muzzle"].matrix_world.translation
    sup = bpy.data.objects["rifle_support"].matrix_world.translation
    wl = arm.matrix_world @ arm.pose.bones["hand_l"].head
    toe = arm.matrix_world @ arm.pose.bones["ball_leaf_l"].head
    heel = arm.matrix_world @ arm.pose.bones["foot_l"].head
    return dict(eye=[round(v, 3) for v in eye], muzzle=[round(v, 3) for v in muz],
                left_wrist_to_support_m=round((wl - sup).length, 3), toe_dir=[round(v, 2) for v in (toe - heel).normalized()])


MEAS = {r: measure(r, RES[r][1][0]) for r in RES}
MEAS["kneel_fire_recoil_peak"] = measure("kneel_fire", 5)
print("MEAS", json.dumps(MEAS))
json.dump({"clips": {r: [RES[r][0].name, RES[r][1]] for r in RES}, "measure": MEAS, "scale": S}, open(os.path.join(OUT, "soldier_meas.json"), "w"), indent=1)

# ---------------------------------------------------------------- stats: triangles per object (evaluated), bones
dg = bpy.context.evaluated_depsgraph_get()
tri = {}
for o in bpy.data.objects:
    if o.type == "MESH" and not o.hide_render:
        me = o.evaluated_get(dg).to_mesh()
        me.calc_loop_triangles()
        tri[o.name] = len(me.loop_triangles)
        o.evaluated_get(dg).to_mesh_clear()
print("TRIS", sum(tri.values()), json.dumps(tri))
json.dump(tri, open(os.path.join(OUT, "soldier_tris.json"), "w"), indent=1)

# ---------------------------------------------------------------- renders
cam = studio((420, 560))
ctr = (0, 0, 0.88)
if MODE in ("all", "sheet"):
    set_action(RES["idle"][0])
    scn.frame_set(0)
    shots = []
    for name, yaw, pitch in VIEWS8:
        d = 3.6 if name != "battle" else 5.0
        aim(cam, ctr, yaw, pitch, d)
        p = os.path.join(OUT, f"soldier_view_{name}.png")
        render_to(p)
        shots.append(p)
    tile(shots, os.path.join(OUT, "soldier_sheet.png"), 4)
if MODE in ("all", "strips"):
    for role in ("idle", "walk", "run", "kneel_fire", "prone_pinned", "death"):
        act, (f0, f1) = RES[role]
        set_action(act)
        shots = []
        for i in range(8):
            fr = f0 + round((f1 - f0) * i / 8) if role != "death" else f0 + round((f1 - f0) * i / 7)
            scn.frame_set(fr)
            if role == "prone_pinned":
                aim(cam, (0.0, 0, 0.15), 70, 32, 4.0)
            elif role == "death":
                aim(cam, (-0.7, 0, 0.5), 70, 30, 5.0)
            elif role == "kneel_fire":
                aim(cam, (0.1, 0, 0.6), 50, 14, 3.6)
            else:
                aim(cam, ctr, 50, 14, 3.8)
            p = os.path.join(OUT, f"strip_{role}_{i}.png")
            render_to(p)
            shots.append(p)
        tile(shots, os.path.join(OUT, f"soldier_strip_{role}.png"), 8)

# ---------------------------------------------------------------- export (size measurement only)
if MODE in ("all", "closeups"):
    shots = []
    for role, fr, tgt, views in (("kneel_fire", 5, (0.25, 0, 0.95), ((60, 10, 2.0), (-30, 15, 2.0), (180, 20, 2.2))),
                                 ("prone_pinned", 24, (0.6, 0, 0.1), ((40, 30, 2.2), (-40, 30, 2.2), (90, 70, 2.6)))):
        set_action(RES[role][0])
        scn.frame_set(fr)
        for i, (yaw, pitch, d) in enumerate(views):
            aim(cam, tgt, yaw, pitch, d)
            p = os.path.join(OUT, f"close_{role}_{i}.png")
            render_to(p)
            shots.append(p)
    tile(shots, os.path.join(OUT, "soldier_closeups.png"), 3)
if MODE in ("all", "export"):
    for a in bpy.data.actions:
        a.use_fake_user = a.name in {v[0].name for v in RES.values()}
    for a in list(bpy.data.actions):
        if not a.use_fake_user and a.users == 0:
            bpy.data.actions.remove(a)
    arm.animation_data.action = None
    for pb in arm.pose.bones:
        for c in list(pb.constraints):
            pb.constraints.remove(c)
    for o in list(bpy.data.objects):
        if o.type == "EMPTY" and o.name.startswith(("anchor_", "ik_", "pole_", "recoil_")):
            bpy.data.objects.remove(o, do_unlink=True)
    for a in list(bpy.data.actions):
        if a.name not in {v[0].name for v in RES.values()}:
            bpy.data.actions.remove(a)
    bpy.ops.object.select_all(action="DESELECT")
    for o in bpy.data.objects:
        if o.type in ("MESH", "ARMATURE"):
            o.select_set(True)
    for o in list(bpy.data.objects):
        if o.type in ("CAMERA", "LIGHT") or o.name == "ground":
            bpy.data.objects.remove(o, do_unlink=True)
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(OUT, "rifleman.blend"))
    path = os.path.join(OUT, "rifleman.glb")
    bpy.ops.export_scene.gltf(filepath=path, use_selection=False, export_animations=True,
                              export_animation_mode="ACTIONS", export_apply=True, export_yup=True)
    print("GLB", path, os.path.getsize(path))
