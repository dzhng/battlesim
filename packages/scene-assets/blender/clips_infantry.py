"""Bake one infantry clip set and export it as the skeleton's clip source.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/clips_infantry.py rifle|launcher

Writes assets/source/infantry/clips_<family>.glb: the rig (scaled to the
simulation's soldier height, not turned; the catalog's basis yaw turns it)
and one action per clip role, plus the standing-aim reference pose. A family
is a hold (`weapons.FAMILIES`): the weapon rides `hand_r`, so a weapon held
differently needs its own clips.

The method is spike 03's (the frozen winner, `soldier.py`), ported exactly and
then changed only where its critiques asked:
- idle, walk, run and death are library clips with the weapon-hold arm layer:
  IK to wrist targets defined in the weapon's frame, gripping fingers, baked
  to FK, plus small layered rotations (breathing, lean, counter-rotation).
- kneel_fire and prone_pinned are authored on a constraint rig and baked.
- stand_aim is the standing shouldered pose the validator measures height,
  eye and muzzle in.
"""

import math
import os
import sys

import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import packs  # noqa: E402
from common import REPO, empty, export_glb, reparent_keep, script_args  # noqa: E402
from infantry_rig import SCALE, TORSO, Rig  # noqa: E402
from weapons import FAMILIES  # noqa: E402


def bake_family(rig, hold):
    P = rig.arm.pose.bones
    grip, support = hold["grip"], hold["support"]

    # patrol / low ready: stock at the right chest, muzzle forward-down and slightly left
    rig.rest_mode(True)
    anchor_low, tr_low, tl_low = rig.make_hold("low", hold["low"], grip, support)
    pole_r = rig.pole("pole_elbow_r", hold["low_poles"][0])
    pole_l = rig.pole("pole_elbow_l", hold["low_poles"][1])
    rig.rest_mode(False)
    hold_cons = rig.arm_constraints(tr_low, tl_low, pole_r, pole_l)
    rig.only([])

    res = {}
    res["idle"] = rig.composite("idle", "Idle_Loop", hold_cons, layers=IDLE_LAYERS)
    res["walk"] = rig.composite("walk", "Walk_Loop", hold_cons, layers=WALK_LAYERS)
    res["run"] = rig.composite("run", "Jog_Fwd_Loop", hold_cons,
                               damp=(["spine_01", "spine_02", "spine_03", "neck_01"], RUN_TWIST),
                               layers=RUN_LAYERS, damp_legs=RUN_STRIDE)

    # ---- kneel_fire (authored)
    work = bpy.data.actions.new("_work_kneel_fire")
    rig.set_action(work)
    rig.clear_pose()
    rig.move_world(P["pelvis"], (0, 0.03, -0.455))
    rig.rot_world(P["pelvis"], "Z", -12)  # blade stance, right side back
    for n, d in (("spine_01", 4), ("spine_02", 4), ("spine_03", 3)):
        rig.rot_world(P[n], "X", d)
    rig.rot_world(P["spine_03"], "Z", 10)  # square the chest back toward the target
    rig.rot_world(P["neck_01"], "X", 6)
    rig.rot_world(P["Head"], "X", 6)
    rig.rot_world(P["neck_01"], "Y", -10)
    rig.rot_world(P["Head"], "Y", -10)  # cheek down onto the stock
    for fr in (0, 36):
        rig.key_bones(TORSO, fr)
        rig.apply_fist(fr)
    rig.scn.frame_set(0)
    bpy.context.view_layer.update()
    kneel_mw = shouldered(rig, hold)
    anchor_k, tr_k, tl_k = rig.make_hold("kneel", kneel_mw, grip, support, spine_follow=True)
    recoil = empty("recoil_kneel", (0, 0, 0), size=0.03)
    recoil.parent = anchor_k
    recoil.rotation_mode = "XYZ"
    for e in (tr_k, tl_k):
        reparent_keep(e, recoil)
    # recoil: shot at frame 3, peak at 5, recovered by 14 (loop 36 frames = 1.2 s)
    for fr, back, up in hold["recoil"]:
        recoil.location = (0, back, 0)
        recoil.rotation_euler = (-up * 3.141592653589793 / 180, 0, 0)
        recoil.keyframe_insert("location", frame=fr)
        recoil.keyframe_insert("rotation_euler", frame=fr)
    pk_r = rig.pole("pole_kneel_r", (-0.55, 0.05, 0.85))
    pk_l = rig.pole("pole_kneel_l", (0.20, -0.30, 0.50))
    kcons = rig.arm_constraints(tr_k, tl_k, pk_r, pk_l)
    fl = rig.world_empty("ik_foot_l", (0.17, -0.40, 0.095))
    fr_ = rig.world_empty("ik_foot_r", (-0.10, 0.44, 0.13))
    pl = rig.world_empty("pole_knee_l", (0.25, -1.2, 0.8))
    pr = rig.world_empty("pole_knee_r", (-0.12, -1.2, -0.2))
    lcons = rig.leg_constraints(fl, fr_, pl, pr)
    rig.only(kcons + lcons)
    res["kneel_fire"] = (rig.bake("kneel_fire", 0, 36), (0, 36))
    rig.only([])

    # ---- prone_pinned (authored). Spike 03's pose lay face-down in the ground with the left
    # hand 0.18 m short of the handguard (critique: "face crater", "no hand on the rifle",
    # "nearly static"). Now: lying on the belly on the ground plane, chest propped on the
    # elbows, the weapon shouldered at the eye as in kneel_fire, and a pinned flinch: the
    # head ducks behind the weapon and the chest sinks, then he looks up again. Breathing
    # moves the chest the rest of the loop.
    res["prone_pinned"] = prone(rig, hold)
    # ---- death: library fall, arms keep the weapon hold (no rifle held aloft)
    # A shouldered launcher would point at the sky once he lies on his back, so a family
    # may carry its weapon across the chest to fall with (`death_hold`).
    if hold.get("death_hold") is None:
        res["death"] = rig.composite("death", "Death01", hold_cons, damp_legs=DEATH_LEGS)
    else:
        rig.rest_mode(True)
        anchor_d, tr_d, tl_d = rig.make_hold("death", fallen_hold(rig, hold), grip, support)
        # he starts the fall with the weapon carried, and lets it swing across as he lands
        fallen = anchor_d.matrix_basis.copy()
        anchor_d.matrix_world = hold["low"]
        carried = anchor_d.matrix_basis.copy()
        anchor_d.rotation_mode = "QUATERNION"
        for frame, basis in ((DEATH_RELEASE[0], carried), (DEATH_RELEASE[1], fallen)):
            anchor_d.matrix_basis = basis
            anchor_d.keyframe_insert("location", frame=frame)
            anchor_d.keyframe_insert("rotation_quaternion", frame=frame)
        pd_r = rig.pole("pole_death_r", (-0.45, 0.25, 0.95))
        pd_l = rig.pole("pole_death_l", (0.35, -0.10, 0.85))
        rig.rest_mode(False)
        cons = rig.arm_constraints(tr_d, tl_d, pd_r, pd_l)
        # the left hand lets go as he lands; the right keeps the grip
        for pb, c in cons:
            if pb.name.endswith("_l"):
                for frame, weight in ((DEATH_RELEASE[0], 1.0), (DEATH_RELEASE[1], 0.0)):
                    c.influence = weight
                    c.keyframe_insert("influence", frame=frame)
        res["death"] = rig.composite("death", "Death01", cons, damp_legs=DEATH_LEGS)

    # ---- stand_aim (new): the standing shouldered reference pose, bladed, knees bent,
    # leaning into the weapon; feet planted by leg IK
    work = bpy.data.actions.new("_work_stand_aim")
    rig.set_action(work)
    rig.clear_pose()
    rig.move_world(P["pelvis"], (0, 0.02, -0.03))
    rig.rot_world(P["pelvis"], "Z", -16)
    for n, d in (("spine_01", 6), ("spine_02", 4), ("spine_03", 2)):
        rig.rot_world(P[n], "X", d)
    rig.rot_world(P["spine_03"], "Z", 10)
    rig.rot_world(P["neck_01"], "X", 5)
    rig.rot_world(P["Head"], "X", 5)
    rig.rot_world(P["neck_01"], "Y", -10)
    rig.rot_world(P["Head"], "Y", -10)
    for fr in (0, 30):
        rig.key_bones(TORSO, fr)
        rig.apply_fist(fr)
    rig.scn.frame_set(0)
    bpy.context.view_layer.update()
    anchor_a, tr_a, tl_a = rig.make_hold("aim", shouldered(rig, hold), grip, support, spine_follow=True)
    pa_r = rig.pole("pole_aim_r", (-0.55, 0.05, 1.25))
    pa_l = rig.pole("pole_aim_l", (0.20, -0.30, 0.90))
    acons = rig.arm_constraints(tr_a, tl_a, pa_r, pa_l)
    fl = rig.world_empty("ik_foot_l_aim", (0.13, -0.08, 0.086))
    fr_ = rig.world_empty("ik_foot_r_aim", (-0.17, 0.2, 0.086))
    pl = rig.world_empty("pole_knee_l_aim", (0.2, -1.2, 0.6))
    pr = rig.world_empty("pole_knee_r_aim", (-0.25, -1.2, 0.5))
    acons += rig.leg_constraints(fl, fr_, pl, pr)
    for side in ("l", "r"):  # soles flat on the ground
        lock = rig.world_empty(f"sole_{side}_aim", (0, 0, 0))
        lock.matrix_world = rig.arm.matrix_world @ rig.arm.data.bones[f"foot_{side}"].matrix_local
        pb = P[f"foot_{side}"]
        c = pb.constraints.new("COPY_ROTATION")
        c.target = lock
        acons.append((pb, c))
    rig.constraints.extend(acons[-2:])
    rig.only(acons)
    res["stand_aim"] = (rig.bake("stand_aim", 0, 30), (0, 30))
    rig.only([])
    return res


# The library idle barely moves (critique): breathing in the chest, a slow weight shift
# and a look around, each a whole number of cycles per loop so it stays seamless.
TAU = 6.283185307179586
IDLE_LAYERS = (
    ("spine_01", (1, 0, 0), lambda t: 5.0),  # a combat stance: leaning into the weapon
    ("spine_02", (1, 0, 0), lambda t: 2.5 * math.sin(TAU * 2 * t)),
    ("spine_03", (0, 1, 0), lambda t: 6.0 * math.sin(TAU * t)),
    ("neck_01", (0, 1, 0), lambda t: 16.0 * math.sin(TAU * t + 1.3)),
    ("Head", (1, 0, 0), lambda t: 5.0 * math.sin(TAU * 2 * t + 0.4)),
)
# The library walk is upright and its torso rigid: lean in, and let the chest counter
# the stride (one cycle per loop, the loop being two steps).
WALK_LAYERS = (
    ("spine_01", (1, 0, 0), lambda t: 6.0),
    ("spine_03", (0, 1, 0), lambda t: 8.0 * math.sin(TAU * t)),
    ("spine_02", (1, 0, 0), lambda t: 1.5 * math.sin(TAU * 2 * t)),  # the bob of each step
)
# The library jog is upright, bounding, heels kicked to the hip (critique: "a cartoon
# run"): lean into it, and shorten the leg swing toward a grounded combat jog.
RUN_STRIDE = (["thigh_l", "thigh_r", "calf_l", "calf_r"], 0.6)
RUN_LAYERS = (
    ("spine_01", (1, 0, 0), lambda t: 13.0),
    ("neck_01", (1, 0, 0), lambda t: -9.0),
    ("spine_03", (0, 1, 0), lambda t: 6.0 * math.sin(TAU * t)),
)

# Death01 kicks both legs into the air as he lands (critique): damp the swing.
DEATH_LEGS = (["thigh_l", "thigh_r", "calf_l", "calf_r"], 0.85)
# Frames of Death01 over which the weapon passes from the carry to lying across him.
DEATH_RELEASE = (8, 34)

PRONE_LOOP = 60
# The run is the library jog with its spine and neck twist damped toward rest, so the
# weapon stays forward: spike 03 kept 45%, and its critique still saw the twist.
RUN_TWIST = 0.30
# (frame, chest lift deg, head lift deg): negative X rotations lift toward +Z.
PRONE_KEYS = ((0, -12, 22), (12, -13, 20), (22, -15, 14), (30, -21, 0), (38, -21, -4), (48, -14, 16), (60, -12, 22))


def prone_pose(rig, frame, chest, head):
    P = rig.arm.pose.bones
    rig.scn.frame_set(frame)
    rig.clear_pose()
    rig.rot_world(P["pelvis"], "X", 90)  # face down, head toward forward (-Y)
    rig.move_world(P["pelvis"], (0, 0.02, -0.71))
    rig.rot_world(P["spine_02"], "X", chest * 0.4)
    rig.rot_world(P["spine_03"], "X", chest * 0.6)
    rig.rot_world(P["spine_02"], "Z", -12)  # the chest angled to the weapon's line, left
    rig.rot_world(P["spine_03"], "Z", -22)  # shoulder forward onto the handguard
    rig.rot_world(P["neck_01"], "X", head * 0.45)
    rig.rot_world(P["Head"], "X", head * 0.55)
    rig.rot_world(P["thigh_l"], "X", -8)  # legs down onto the ground
    rig.rot_world(P["thigh_r"], "X", -8)
    rig.rot_world(P["thigh_l"], "Z", 16)  # legs splayed, one knee drawn up a little
    rig.rot_world(P["thigh_r"], "Z", -13)
    rig.rot_world(P["thigh_l"], "Y", -14)
    rig.rot_world(P["calf_l"], "Z", -4)
    rig.rot_world(P["foot_l"], "X", -35)  # insteps toward the ground
    rig.rot_world(P["foot_r"], "X", -35)
    rig.rot_world(P["foot_l"], "Y", 40)  # feet turned out, insides of the boots down
    rig.rot_world(P["foot_r"], "Y", -40)
    rig.key_bones(TORSO + ["calf_l", "calf_r"], frame)
    rig.apply_fist(frame)


def prone(rig, hold):
    work = bpy.data.actions.new("_work_prone")
    rig.set_action(work)
    for frame, chest, head in PRONE_KEYS:
        prone_pose(rig, frame, chest, head)
    rig.scn.frame_set(0)
    bpy.context.view_layer.update()
    sh_r = rig.arm.pose.bones["upperarm_r"].head.copy()
    if hold["prone_butt"] is not None:
        # the butt in the right shoulder pocket, the weapon level and pointing forward
        pocket = sh_r + Vector((0.06, -0.05, 0.02))
        placed = Matrix.Translation(pocket - Vector(hold["prone_butt"]) * hold["scale"])
    else:
        placed = shouldered(rig, hold)
    # prone, the support hand may take the rear of the handguard, nearer the grip;
    # the anchor rides the chest, so the flinch carries the weapon and both hands
    support = Vector(hold["support"]) + Vector((0, hold["prone_support_back"], 0))
    anchor, tr, tl = rig.make_hold("prone", placed, hold["grip"], support, spine_follow=True)
    pr = rig.world_empty("pole_prone_r", (sh_r.x - 0.45, sh_r.y + 0.05, -0.4))
    pl = rig.world_empty("pole_prone_l", (-sh_r.x + 0.45, sh_r.y + 0.05, -0.4))
    cons = rig.arm_constraints(tr, tl, pr, pl)
    rig.only(cons)
    act = rig.bake("prone_pinned", 0, PRONE_LOOP)
    rig.only([])
    return act, (0, PRONE_LOOP)


def fallen_hold(rig, hold):
    """A rest-space weapon placement that, riding spine_03 through Death01, ends lying level
    across the fallen chest (`death_hold` metres in front of spine_03), pointing past his
    left foot."""
    P = rig.arm.pose.bones
    rig.rest_mode(False)
    lib = bpy.data.actions["Death01"]
    rig.set_action(lib)
    rig.scn.frame_set(int(lib.frame_range[1]))
    bpy.context.view_layer.update()
    end = P["spine_03"].matrix.copy()
    rig.set_action(None)
    rig.clear_pose()
    rig.rest_mode(True)
    rest = rig.arm.data.bones["spine_03"].matrix_local.copy()
    # in rest space the chest faces -Y, his left is +X and up is +Z
    r = end.to_3x3() @ rest.to_3x3().inverted()
    front, left, up = r @ Vector((0, -1, 0)), r @ Vector((1, 0, 0)), r @ Vector((0, 0, 1))
    # lying level across his chest, pointing past his left foot, his hands still on it
    along = left * 0.6 - up * 0.8
    along = Vector((along.x, along.y, 0)).normalized()
    z = Vector((0, 0, 1))
    side = (-along).cross(z)  # the weapon's +X
    frame = Matrix((side, -along, z)).transposed().to_4x4()
    grip = end.translation + front * hold["death_hold"] - along * 0.1
    placed_end = Matrix.Translation(grip) @ frame
    return rest @ end.inverted() @ placed_end


def shouldered(rig, hold):
    """A weapon placement with its sight 8 cm ahead of the eye, pulled `cheek_blend` of the
    way across toward the right shoulder pocket (spike 03's kneel_fire bore line)."""
    eye = rig.eye_point()
    sh = rig.arm.pose.bones["upperarm_r"].head.copy()
    sight = Vector(hold["sight"]) * hold["scale"]
    pocket = sh + Vector((0.09, -0.05, 0.0))
    k = hold["cheek_blend"]
    grip = Vector((eye.x + (pocket.x - eye.x) * k, eye.y - 0.08, eye.z)) - sight
    return Matrix.Translation(grip)


def export_clips(rig, res, path):
    keep = {act.name for act, _ in res.values()}
    rig.set_action(None)
    for pb in rig.arm.pose.bones:
        for c in list(pb.constraints):
            pb.constraints.remove(c)
    for a in list(bpy.data.actions):
        if a.name not in keep:
            bpy.data.actions.remove(a)
    for o in list(bpy.data.objects):
        if o.type == "EMPTY" or (o.type == "MESH" and o != rig.body):
            bpy.data.objects.remove(o, do_unlink=True)
    # A clip source is the rig and its actions. glTF carries a skin only with a
    # skinned mesh, so the body is cut to one triangle weighted to the pelvis.
    body = rig.body
    me = body.data
    import bmesh
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.faces.ensure_lookup_table()
    keep_face = bm.faces[0]
    bmesh.ops.delete(bm, geom=[f for f in bm.faces if f is not keep_face], context="FACES")
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS")
    bmesh.ops.triangulate(bm, faces=bm.faces[:])
    bm.to_mesh(me)
    bm.free()
    body.vertex_groups.clear()
    g = body.vertex_groups.new(name="pelvis")
    g.add([v.index for v in me.vertices], 1.0, "REPLACE")
    body.name = "skeleton_anchor"
    rig.arm.scale = (SCALE, SCALE, SCALE)
    bpy.context.view_layer.update()
    export_glb(path, [rig.arm, body], animations=True)
    print("CLIPS", path, os.path.getsize(path), sorted(keep))


def main():
    args = script_args()
    family = args[0] if args else "rifle"
    rig = Rig()
    res = bake_family(rig, FAMILIES[family])
    rel = f"assets/source/infantry/clips_{family}.glb"
    export_clips(rig, res, os.path.join(REPO, rel))
    packs.record_source(rel, "clips_infantry.py", (packs.UBC, packs.UBC_BIN, packs.UAL))


if __name__ == "__main__":
    main()
