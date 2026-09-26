"""The infantry rig: the Quaternius UBC body and 65-joint rig, the UAL library
clips, and the weapon-hold machinery every infantry clip and kit is built on.

Ported from spike 03's `soldier.py` (the frozen winner, specs/battle-look/
assets/spikes/03/scripts): same imports, same hand orientation, same IK hold
rig, same bake. While building, space is the imported rig's rest space: Z up,
the soldier faces -Y, his left is +X. The export's +90 deg basis yaw (the
catalog's `basis_yaw_deg`) turns him to face +X.

A hold is a weapon placement. Each hand's IK target is defined in the weapon's
own frame (weapon local -Y is forward, origin at the pistol grip), so the
weapon is rigid on `hand_r` in every clip that uses the weapon: the kit skins
it to `hand_r` at bake time and there is no attachment system.
"""

import math

import bpy
from mathutils import Matrix, Quaternion, Vector

import packs
from common import bone_parent, empty, reparent_keep, reset

SIM_HEIGHT = 1.70  # fixtures/village.json physics.soldier_height_m
BODY_TOP = 1.81  # the UBC male's height in its own units
SCALE = SIM_HEIGHT / BODY_TOP
FPS = 30
FINGER_PREFIXES = ("index", "middle", "ring", "pinky", "thumb")
TORSO = ["root", "pelvis", "spine_01", "spine_02", "spine_03", "neck_01", "Head", "clavicle_l", "clavicle_r",
         "thigh_l", "thigh_r", "foot_l", "foot_r", "ball_l", "ball_r"]


class Rig:
    """The imported rig plus its library actions, in rest space."""

    def __init__(self):
        self.scn = reset(FPS)
        bpy.ops.import_scene.gltf(filepath=packs.path(packs.UBC))
        packs.path(packs.UBC_BIN)  # verified alongside its .gltf
        self.arm = [o for o in bpy.data.objects if o.type == "ARMATURE"][0]
        self.arm.name = "soldier_rig"
        self.body = bpy.data.objects["SuperHero_Male"]
        for n in ("Icosphere",):
            if n in bpy.data.objects:
                bpy.data.objects.remove(bpy.data.objects[n], do_unlink=True)
        before = set(bpy.data.objects)
        bpy.ops.import_scene.gltf(filepath=packs.path(packs.UAL))
        for o in set(bpy.data.objects) - before:
            bpy.data.objects.remove(o, do_unlink=True)
        for a in bpy.data.actions:
            a.use_fake_user = True
        self.arm.animation_data_create()
        self.rest_b = {b.name: b.matrix_local.to_3x3() for b in self.arm.data.bones}
        self.fingers = [b.name for b in self.arm.pose.bones if b.name.startswith(FINGER_PREFIXES)]
        self.rest_mode(False)
        self.fist = self.capture_fingers("Idle_Loop", 0)
        self.set_action(None)
        self.rest_mode(True)
        self.constraints = []  # every hold constraint, so `only` can mute the rest

    # ------------------------------------------------------------ state
    def set_action(self, act):
        self.arm.animation_data.action = act
        if act is not None and act.slots:
            self.arm.animation_data.action_slot = act.slots[0]

    def rest_mode(self, on):
        self.arm.data.pose_position = "REST" if on else "POSE"
        bpy.context.view_layer.update()

    def capture_fingers(self, action, frame):
        self.set_action(bpy.data.actions[action])
        self.scn.frame_set(frame)
        return {n: self.arm.pose.bones[n].rotation_quaternion.copy() for n in self.fingers}

    def bone_parent(self, o, bone):
        bone_parent(o, self.arm, bone)

    # ------------------------------------------------------------ hands and holds
    def hand_orient(self, bone, fingers_dir, palm_dir, palm_rest=Vector((0, 0, -1))):
        """Armature-space rotation pointing the hand bone along fingers_dir, palm facing palm_dir."""
        b0 = self.rest_b[bone]
        p_l = b0.transposed() @ palm_rest
        e = frame_from(Vector((0, 1, 0)), p_l)
        f = frame_from(fingers_dir, palm_dir)
        return f @ e.transposed()

    def hold_targets(self, weapon_mw, grip, support):
        """World (rest-space) wrist targets for a weapon placed at weapon_mw. `grip` and
        `support` are the weapon's pistol grip and support point in its own frame."""
        r = weapon_mw.to_3x3().normalized()
        fwd = r @ Vector((0, -1, 0))
        up = r @ Vector((0, 0, 1))
        side = fwd.cross(up)  # the weapon's right
        # right hand: fingers wrap the grip, forward-down; palm faces the weapon's left side
        rh = self.hand_orient("hand_r", fwd * 0.55 - up * 0.8 + side * 0.25, -side)
        wrist_r = weapon_mw @ Vector(grip) - rh @ Vector((0, 0.075, 0)) + side * 0.03 + up * 0.01
        # left hand: under the handguard, palm up, fingers across to the right
        lh = self.hand_orient("hand_l", fwd * 0.45 + side * 0.85 + up * 0.25, up * 0.9 - side * 0.3)
        wrist_l = weapon_mw @ Vector(support) - lh @ Vector((0, 0.07, 0)) - up * 0.03
        return (Matrix.Translation(wrist_r) @ rh.to_4x4(), Matrix.Translation(wrist_l) @ lh.to_4x4())

    def make_hold(self, tag, weapon_mw, grip, support, spine_follow=True):
        """An anchor at the weapon placement with the two wrist targets under it."""
        anchor = empty(f"anchor_{tag}", (0, 0, 0), size=0.05)
        anchor.matrix_world = weapon_mw
        bpy.context.view_layer.update()
        right, left = self.hold_targets(weapon_mw, grip, support)
        tr = empty(f"ik_hand_r_{tag}", right.translation, size=0.03)
        tl = empty(f"ik_hand_l_{tag}", left.translation, size=0.03)
        tr.rotation_mode = tl.rotation_mode = "QUATERNION"
        tr.rotation_quaternion = right.to_quaternion()
        tl.rotation_quaternion = left.to_quaternion()
        bpy.context.view_layer.update()
        for e in (tr, tl):
            reparent_keep(e, anchor)
        if spine_follow:
            self.bone_parent(anchor, "spine_03")
        else:
            reparent_keep(anchor, self.arm)
        return anchor, tr, tl

    def pole(self, name, loc, bone="spine_03"):
        e = empty(name, loc, size=0.03)
        self.bone_parent(e, bone)
        return e

    def world_empty(self, name, loc):
        e = empty(name, loc, size=0.04)
        e.parent = self.arm
        return e

    def arm_constraints(self, tr, tl, pr, pl):
        cons = []
        for side, t, p in (("r", tr, pr), ("l", tl, pl)):
            pb = self.arm.pose.bones[f"lowerarm_{side}"]
            c = pb.constraints.new("IK")
            c.target = t
            c.pole_target = p
            c.pole_angle = math.radians(-90)
            c.chain_count = 2
            cons.append((pb, c))
            hb = self.arm.pose.bones[f"hand_{side}"]
            c2 = hb.constraints.new("COPY_ROTATION")
            c2.target = t
            cons.append((hb, c2))
        self.constraints.extend(cons)
        return cons

    def leg_constraints(self, fl, fr, pl, pr):
        cons = []
        for side, t, p in (("l", fl, pl), ("r", fr, pr)):
            pb = self.arm.pose.bones[f"calf_{side}"]
            c = pb.constraints.new("IK")
            c.target = t
            c.pole_target = p
            c.pole_angle = math.radians(-90)
            c.chain_count = 2
            cons.append((pb, c))
        self.constraints.extend(cons)
        return cons

    def only(self, cons):
        for _, c in self.constraints:
            c.mute = True
        for _, c in cons:
            c.mute = False

    # ------------------------------------------------------------ posing
    def select_pose(self):
        bpy.context.view_layer.objects.active = self.arm
        self.arm.select_set(True)
        if bpy.context.object.mode != "POSE":
            bpy.ops.object.mode_set(mode="POSE")
        for pb in self.arm.pose.bones:
            pb.select = True

    def bake(self, name, f0, f1):
        """Bake the current action plus live constraints to a plain FK action."""
        self.select_pose()
        bpy.ops.nla.bake(frame_start=f0, frame_end=f1, only_selected=False, visual_keying=True,
                         clear_constraints=False, use_current_action=False, bake_types={"POSE"})
        act = self.arm.animation_data.action
        act.name = name
        act.use_fake_user = True
        bpy.ops.object.mode_set(mode="OBJECT")
        return act

    def apply_fist(self, frame, fist=None):
        for n, q in (fist or self.fist).items():
            pb = self.arm.pose.bones[n]
            pb.rotation_quaternion = q
            pb.keyframe_insert("rotation_quaternion", frame=frame)

    def strip_finger_keys(self, act, keep_frames):
        """Keep only our grip keys on finger curves (the library animates fingers per clip)."""
        for fc in iter_fcurves(act):
            if any(f'"{n}"' in fc.data_path for n in self.fingers):
                pts = fc.keyframe_points
                for k in reversed(list(pts)):
                    if k.co.x not in keep_frames:
                        pts.remove(k)

    def composite(self, role, lib_name, cons, damp=None, layers=(), damp_legs=None):
        """Library clip for legs and torso + weapon-hold arms (IK) + gripping fingers -> baked FK clip.
        damp: (bones, k) slerps those bones' keyed rotations toward rest by k.
        layers: (bone, local axis, degrees(t)) rotations added on top, t the loop's 0..1 phase."""
        src = bpy.data.actions[lib_name]
        work = src.copy()
        work.name = f"_work_{role}"
        self.set_action(work)
        f0, f1 = int(src.frame_range[0]), int(src.frame_range[1])
        for fr in (f0, f1):
            self.apply_fist(fr)
        self.strip_finger_keys(work, (f0, f1))
        for d in (damp, damp_legs):
            if d:
                damp_rotation(work, *d)
        for bone, axis, deg in layers:
            layer_rotation(work, bone, axis, lambda f, deg=deg: deg((f - f0) / max(1, f1 - f0)))
        self.only(cons)
        act = self.bake(role, f0, f1)
        self.only([])
        return act, (f0, f1)

    def rot_world(self, pb, axis, deg, pivot=None):
        """Rotate a pose bone in armature space about its head (or pivot)."""
        bpy.context.view_layer.update()
        m = pb.matrix.copy()
        h = pivot if pivot is not None else m.translation.copy()
        pb.matrix = Matrix.Translation(h) @ Matrix.Rotation(math.radians(deg), 4, axis) @ Matrix.Translation(-h) @ m
        bpy.context.view_layer.update()

    def move_world(self, pb, delta):
        bpy.context.view_layer.update()
        m = pb.matrix.copy()
        m.translation += Vector(delta)
        pb.matrix = m
        bpy.context.view_layer.update()

    def key_bones(self, names, frame):
        for n in names:
            pb = self.arm.pose.bones[n]
            pb.keyframe_insert("rotation_quaternion", frame=frame)
            pb.keyframe_insert("location", frame=frame)

    def clear_pose(self):
        for pb in self.arm.pose.bones:
            pb.rotation_quaternion = (1, 0, 0, 0)
            pb.location = (0, 0, 0)
            pb.scale = (1, 1, 1)
        bpy.context.view_layer.update()

    def eye_point(self):
        """The right eye, in armature space: in the Head bone frame Y runs up the skull and Z to the back."""
        bpy.context.view_layer.update()
        return self.arm.pose.bones["Head"].matrix @ EYE_IN_HEAD


# The eye socket's offset in the Head bone's frame (spike 03's `eye_point`).
EYE_IN_HEAD = Vector((-0.033, 0.09, 0.09))


def frame_from(y_axis, n_axis):
    f1 = y_axis.normalized()
    f2 = (n_axis - n_axis.dot(f1) * f1).normalized()
    f3 = f1.cross(f2)
    return Matrix((f1, f2, f3)).transposed()


def weapon_matrix(pos, yaw_deg=0.0, pitch_deg=0.0, roll_deg=0.0):
    """A weapon pointing along -Y (the soldier's forward), pitched up, yawed left, rolled."""
    r = (Matrix.Rotation(math.radians(yaw_deg), 4, "Z") @ Matrix.Rotation(math.radians(pitch_deg), 4, "X")
         @ Matrix.Rotation(math.radians(roll_deg), 4, "Y"))
    return Matrix.Translation(Vector(pos)) @ r


def iter_fcurves(act):
    # Blender 5 slotted actions: layers -> strips -> channelbags
    out = []
    for layer in act.layers:
        for strip in layer.strips:
            for cb in strip.channelbags:
                out.extend(cb.fcurves)
    return out


def layer_rotation(act, bone, axis, deg_at):
    """Add a rotation about the bone's local `axis` to each keyed rotation, deg_at(frame) degrees."""
    comp = {fc.array_index: fc for fc in iter_fcurves(act) if fc.data_path == f'pose.bones["{bone}"].rotation_quaternion'}
    if len(comp) != 4:
        raise ValueError(f"{act.name}: {bone} has no keyed rotation to layer onto")
    frames = sorted({kp.co.x for kp in comp[0].keyframe_points})
    for f in frames:
        q = Quaternion([comp[i].evaluate(f) for i in range(4)]) @ Quaternion(Vector(axis), math.radians(deg_at(f)))
        for i in range(4):
            for kp in comp[i].keyframe_points:
                if kp.co.x == f:
                    kp.co.y = q[i]
                    kp.handle_left.y = q[i]
                    kp.handle_right.y = q[i]


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
