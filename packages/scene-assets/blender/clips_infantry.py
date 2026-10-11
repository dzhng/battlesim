"""Bake one infantry clip set and export it as the skeleton's clip source.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/clips_infantry.py rifle|launcher

Writes assets/source/infantry/clips_<family>.glb: the rig (scaled to the
simulation's soldier height, not turned; the catalog's basis yaw turns it)
and one action per clip role, plus the standing-aim reference pose. A family
is a hold (`weapons.FAMILIES`): the weapon rides `hand_r`, so a weapon held
differently needs its own clips.

The method is spike 03's (the frozen winner, `soldier.py`), ported exactly and
then changed only where its critiques asked:
- idle, walk and run are library clips with the weapon-hold arm layer:
  IK to wrist targets defined in the weapon's frame, gripping fingers, baked
  to FK, plus small layered rotations (breathing, lean, counter-rotation).
  Walk and run are grounded: each frame his lowest sole touches the ground,
  so the library jog neither bounds into the air nor lands knee-deep.
- kneel_fire and prone_pinned are authored on a constraint rig and baked.
- stand_aim is the standing shouldered pose the validator measures height,
  eye and muzzle in.
- Moving in a posture with the weapon toward the threat, one loop per way he
  can go (`DIRECTIONS`): walk_aim_* and crouch_* step on leg IK along foot
  tracks (`GAITS`), crawl_* works knees and elbows from the prone pose. Their
  strides, times SCALE, are the catalog's `stride_m`.
- death, death_kneel and death_prone fall from standing, kneeling and lying,
  each from that posture's own pose, onto his face; death_back throws a standing
  man onto his back (`deaths`).
"""

import math
import os
import sys

import bpy
from mathutils import Matrix, Quaternion, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import REPO, empty, export_glb, reparent_keep, script_args  # noqa: E402
from infantry_rig import SCALE, TORSO, Rig, iter_fcurves, weapon_matrix  # noqa: E402
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
    res["walk"] = rig.composite("walk", "Walk_Loop", hold_cons, layers=WALK_LAYERS, grounded=True)
    res["run"] = rig.composite("run", "Jog_Fwd_Loop", hold_cons,
                               damp=(["spine_01", "spine_02", "spine_03", "neck_01"], RUN_TWIST),
                               layers=RUN_LAYERS, damp_legs=RUN_STRIDE, grounded=True)

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

    # ---- moving in a posture, each way he can go in it: stepping upright or crouched
    # with the weapon up, and crawling on his belly. Each clip starts from a rig with no
    # constraint left on it (`Rig.release`).
    moves = [(f"{gait}_{d}", lambda gait=gait, d=d: stepping(rig, hold, f"{gait}_{d}", GAITS[gait], d))
             for gait in GAITS for d in DIRECTIONS]
    moves += [(f"crawl_{d}", lambda d=d: crawl(rig, hold, d)) for d in DIRECTIONS]
    for role, make in moves:
        rig.release(list(rig.constraints))
        res[role] = make()
    rig.release(list(rig.constraints))
    # ---- dying from each posture he can die in, each fall ending face down or face up
    res.update(deaths(rig, hold, res))
    rig.release(list(rig.constraints))
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

PRONE_LOOP = 60
# The run is the library jog with its spine and neck twist damped toward rest, so the
# weapon stays forward: spike 03 kept 45%, and its critique still saw the twist.
RUN_TWIST = 0.30
# (frame, chest lift deg, head lift deg): negative X rotations lift toward +Z.
PRONE_KEYS = ((0, -12, 22), (12, -13, 20), (22, -15, 14), (30, -21, 0), (38, -21, -4), (48, -14, 16), (60, -12, 22))


def prone_pose(rig, frame, chest, head, extra=None):
    """Lying on his belly, chest propped by `chest` degrees and head lifted by `head`;
    `extra(P)` adds a move (a crawl's) before the frame is keyed."""
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
    if extra:
        extra(P)
    rig.key_bones(TORSO + ["calf_l", "calf_r"], frame)
    rig.apply_fist(frame)


def prone(rig, hold, name="prone_pinned", frames=PRONE_LOOP, pose=None, weapon=None):
    """A clip lying on his belly with the weapon shouldered: the pinned flinch, or a
    crawl's `pose(frame)` keys. `weapon(frame)` is how far the weapon (and both hands)
    is carried from where it lies, in rest-space metres."""
    work = bpy.data.actions.new(f"_work_{name}")
    rig.set_action(work)
    if pose is None:
        for frame, chest, head in PRONE_KEYS:
            prone_pose(rig, frame, chest, head)
    else:
        for frame in range(frames + 1):
            pose(frame)
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
    if weapon is not None:
        carry(rig, anchor, frames, weapon)
    pr = rig.world_empty(f"pole_{name}_r", (sh_r.x - 0.45, sh_r.y + 0.05, -0.4))
    pl = rig.world_empty(f"pole_{name}_l", (-sh_r.x + 0.45, sh_r.y + 0.05, -0.4))
    cons = rig.arm_constraints(tr, tl, pr, pl)
    rig.only(cons)
    act = rig.bake(name, 0, frames)
    rig.only([])
    return act, (0, frames)


def carry(rig, anchor, frames, offset):
    """Key the anchor (riding spine_03) so the weapon sits `offset(frame)` rest-space metres
    from where it rides the chest."""
    bone = rig.arm.pose.bones[anchor.parent_bone]
    basis = anchor.matrix_basis.copy()
    for f in range(frames + 1):
        rig.scn.frame_set(f)
        bpy.context.view_layer.update()
        # world = arm @ bone (moved to its tail) @ parent_inverse @ basis
        parent = (rig.arm.matrix_world @ bone.matrix @ Matrix.Translation((0, bone.length, 0))
                  @ anchor.matrix_parent_inverse)
        local = parent.to_3x3().inverted() @ Vector(offset(f))
        anchor.matrix_basis = Matrix.Translation(local) @ basis
        anchor.keyframe_insert("location", frame=f)


# The ways a soldier moves relative to where he faces (rest space: forward is -Y, his
# left +X). A diagonal plays the nearest; the pose driver picks it.
DIRECTIONS = {"f": (0, -1), "b": (0, 1), "l": (1, 0), "r": (-1, 0)}
# Stepping gaits, in rest-space metres and degrees. `stride` is ground covered per loop
# (two steps), `side_stride` the same sideways, where the trailing foot closes in after
# the leading one rather than crossing it. `drop` lowers the hips (knees bent, as a
# soldier walks with his weapon up), `lean` tips the chest over the weapon, `blade`
# turns the hips right-side back, `lift` is the swinging foot's height, `bob` the hips'
# rise at mid-stance, `sway` their shift over the standing foot, and `duty` the share
# of a loop each foot stands on the ground.
GAITS = {
    "walk_aim": dict(frames=32, stride=1.05, side_stride=0.5, width=0.1, side_width=0.17, drop=0.09, lean=12,
                     blade=-8, lift=0.1, bob=0.02, sway=0.02, duty=0.62),
    "crouch": dict(frames=32, stride=0.95, side_stride=0.5, width=0.13, side_width=0.2, drop=0.3, lean=38,
                   blade=-6, lift=0.08, bob=0.015, sway=0.025, duty=0.65),
}


def foot_track(t, phase, g, stride):
    """Where a foot is at loop phase t: how far along the travel from below the hip
    (ahead > 0), how high, and the share of its swing (None while it stands)."""
    s = (t - phase) % 1.0
    reach = g["duty"] * stride / 2
    if s < g["duty"]:
        return reach * (1 - 2 * s / g["duty"]), 0.0, None
    u = (s - g["duty"]) / (1 - g["duty"])
    ease = u * u * (3 - 2 * u)
    return -reach + 2 * reach * ease, g["lift"] * math.sin(math.pi * u), u


def stepping(rig, hold, name, g, d):
    """One stepping loop toward direction `d`: the hips and chest keyed, the feet on leg
    IK along their tracks, soles flat but for the toe-off and heel-strike, the weapon
    shouldered on the chest. In place: the battle moves him, the phase follows the
    ground he covers (the clip's stride)."""
    P = rig.arm.pose.bones
    dx, dy = DIRECTIONS[d]
    lateral = dx != 0
    stride = g["side_stride"] if lateral else g["stride"]
    width = g["side_width"] if lateral else g["width"]
    n = g["frames"]
    if lateral:  # the leading foot steps out, the trailing one closes in after it lands
        lead, trail = ("l", "r") if dx > 0 else ("r", "l")
        phases = {lead: 0.0, trail: 1 - g["duty"]}
    else:
        phases = {"l": 0.0, "r": 0.5}
    rest = {s: rig.arm.data.bones[f"foot_{s}"] for s in "lr"}
    sides = {"l": 1, "r": -1}
    work = bpy.data.actions.new(f"_work_{name}")
    rig.set_action(work)
    for f in range(n + 1):
        t = f / n
        rig.scn.frame_set(f)
        rig.clear_pose()
        mid = t - g["duty"] / 2  # the left foot's mid-stance
        rise = g["bob"] * math.cos(4 * math.pi * mid)
        sway = g["sway"] * math.cos(2 * math.pi * mid)
        rig.move_world(P["pelvis"], (sway, 0, -g["drop"] + rise))
        rig.rot_world(P["pelvis"], "Z", g["blade"])
        if not lateral:  # the hips turn with the forward leg; the chest holds the weapon still
            rig.rot_world(P["pelvis"], "Z", 4 * math.sin(2 * math.pi * mid) * dy)
        for n_, k in (("spine_01", 0.5), ("spine_02", 0.3), ("spine_03", 0.2)):
            rig.rot_world(P[n_], "X", g["lean"] * k)
        rig.rot_world(P["spine_03"], "Z", -g["blade"])
        if not lateral:
            rig.rot_world(P["spine_01"], "Z", -4 * math.sin(2 * math.pi * mid) * dy)
        rig.rot_world(P["neck_01"], "X", 5 - g["lean"] * 0.45)
        rig.rot_world(P["Head"], "X", 5 - g["lean"] * 0.3)
        rig.rot_world(P["neck_01"], "Y", -10)
        rig.rot_world(P["Head"], "Y", -10)
        rig.key_bones(TORSO, f)
        rig.apply_fist(f)
    rig.scn.frame_set(0)
    bpy.context.view_layer.update()
    anchor, tr, tl = rig.make_hold(name, shouldered(rig, hold), hold["grip"], hold["support"], spine_follow=True)
    cons = rig.arm_constraints(tr, tl, rig.pole(f"pole_{name}_r", (-0.55, 0.05, 1.25 - g["drop"])),
                               rig.pole(f"pole_{name}_l", (0.20, -0.30, 0.90 - g["drop"])))
    # Animated targets stay unparented: parented to the rig whose IK reads them, they
    # would form a dependency cycle and the legs would solve to a stale frame. The rig
    # stands unturned at the origin while building, so rest space is world space.
    feet, knees, locks = {}, {}, {}
    for s in "lr":
        feet[s] = empty(f"ik_foot_{s}_{name}", (0, 0, 0), size=0.04)
        knees[s] = empty(f"pole_knee_{s}_{name}", (0, 0, 0), size=0.04)
        locks[s] = empty(f"sole_{s}_{name}", (0, 0, 0), size=0.04)
        locks[s].matrix_world = rest[s].matrix_local
        locks[s].rotation_mode = "QUATERNION"
    cons += rig.leg_constraints(feet["l"], feet["r"], knees["l"], knees["r"])
    for s in "lr":
        c = P[f"foot_{s}"].constraints.new("COPY_ROTATION")
        c.target = locks[s]
        cons.append((P[f"foot_{s}"], c))
        rig.constraints.append((P[f"foot_{s}"], c))
    flat = {s: locks[s].rotation_quaternion.copy() for s in "lr"}
    for f in range(n + 1):
        t = f / n
        for s in "lr":
            along, up, swing = foot_track(t, phases[s], g, stride)
            ankle = rest[s].head_local
            feet[s].location = (sides[s] * width + dx * along, ankle.y + dy * along, ankle.z + up)
            feet[s].keyframe_insert("location", frame=f)
            # the knee over its own foot, a little outside it, so he never steps knock-kneed
            knees[s].location = (feet[s].location.x + sides[s] * 0.12, -1.2, 0.6 - g["drop"])
            knees[s].keyframe_insert("location", frame=f)
            # toes down as the foot leaves the ground, up as it reaches to land (along the travel)
            pitch = 0.0 if swing is None or lateral else 22 * math.sin(2 * math.pi * swing) * -dy
            locks[s].rotation_quaternion = Quaternion((1, 0, 0), math.radians(pitch)) @ flat[s]
            locks[s].keyframe_insert("rotation_quaternion", frame=f)
    rig.only(cons)
    act = rig.bake(name, 0, n)
    rig.only([])
    return act, (0, n)


# A crawl loop's frames, and how it moves (rest-space metres and degrees): each knee
# drawn up and out in turn, the hips rolling toward it, the weapon pushed ahead on the
# opposite elbow.
CRAWL_FRAMES = 48
CRAWL = dict(knee=45, shin=80, roll=7, yaw=6, reach=0.07, surge=0.035, side_leg=22, side_shift=0.05)


def crawl(rig, hold, d):
    """A crawl loop toward direction `d`, from the pinned pose with the head up. Forward:
    knee and opposite elbow work in turn; back: the same, played backward (he pushes
    himself back); sideways: the legs scissor toward the travel while the hips and the
    weapon shift across."""
    dx, dy = DIRECTIONS[d]
    k = CRAWL
    name = f"crawl_{d}"

    def wave(frame):
        t = frame / CRAWL_FRAMES
        return math.sin(2 * math.pi * (1 - t if d == "b" else t))

    def pose(frame):
        w = wave(frame)
        draw_l, draw_r = max(0.0, w), max(0.0, -w)

        def move(P):
            if dx == 0:
                for s, draw, sign in (("l", draw_l, 1), ("r", draw_r, -1)):
                    rig.rot_world(P[f"thigh_{s}"], "Z", -sign * k["knee"] * draw)
                    rig.rot_world(P[f"calf_{s}"], "Z", sign * k["shin"] * draw)  # the shin folds back flat
                rig.rot_world(P["pelvis"], "Y", k["roll"] * w)
                rig.move_world(P["pelvis"], (0, k["surge"] * abs(w), 0))
                rig.rot_world(P["spine_02"], "Z", k["yaw"] * w)
            else:
                rig.rot_world(P["thigh_l"], "Z", -dx * k["side_leg"] * (0.5 + 0.5 * w))
                rig.rot_world(P["thigh_r"], "Z", -dx * k["side_leg"] * (0.5 - 0.5 * w))
                rig.move_world(P["pelvis"], (dx * k["side_shift"] * w, 0, 0))
                rig.rot_world(P["pelvis"], "Y", k["roll"] * 0.5 * w * dx)

        prone_pose(rig, frame, -14, 16, extra=move)

    def weapon(frame):
        # pushed ahead on each elbow in turn, never drawn back past where it lies (into his face)
        w = wave(frame)
        if dx == 0:
            return (0.02 * w, -k["reach"] * abs(w), 0.015 * abs(w))
        return (dx * k["side_shift"] * 0.6 * w, 0, 0.01 * abs(w))

    return prone(rig, hold, name, CRAWL_FRAMES, pose, weapon)


def corpse(rig, hold, up=False):
    """Where a fall ends. Face down: flat, head turned onto one cheek, a knee drawn up,
    the left arm by his head and the weapon on its side by his right hand. Face up
    (`up`): on his back, the pack under him, head rolled aside, a knee raised, the left
    arm flung out past his head and the weapon on its side by his right hip."""
    P = rig.arm.pose.bones
    k = CORPSE
    s = -1 if up else 1  # face up, the body turns the other way over
    rig.set_action(None)
    rig.clear_pose()
    rig.rot_world(P["pelvis"], "X", 90 * s)  # face down head forward (-Y); face up head back (+Y)
    rig.move_world(P["pelvis"], (0, 0.02, -k["up_z"] if up else -0.75))
    rig.rot_world(P["pelvis"], "Y", k["roll"] * s)
    rig.rot_world(P["spine_03"], "Z", 8)
    rig.rot_world(P["neck_01"], "Y", k["cheek"] * 0.5 * s)
    rig.rot_world(P["Head"], "Y", k["cheek"] * 0.5 * s)
    if up:
        rig.rot_world(P["thigh_l"], "X", -k["knee"])  # the left knee raised
        rig.rot_world(P["calf_l"], "X", k["knee"] * 1.8)
        rig.rot_world(P["thigh_l"], "Z", 10)
        rig.rot_world(P["thigh_r"], "Z", -8)
        rig.rot_world(P["foot_l"], "Y", -35)  # toes up, falling outward
        rig.rot_world(P["foot_r"], "Y", 35)
        rig.rot_world(P["upperarm_l"], "Y", 15)  # the arms down onto the ground
        rig.rot_world(P["upperarm_l"], "Z", 40)  # the left flung out past his head
        rig.rot_world(P["lowerarm_l"], "Z", 35)
    else:
        rig.rot_world(P["thigh_l"], "Z", -k["knee"])  # the left knee drawn up and out
        rig.rot_world(P["calf_l"], "Z", k["knee"] * 1.6)
        rig.rot_world(P["thigh_r"], "Z", 6)
        rig.rot_world(P["foot_l"], "X", -35)  # insteps toward the ground, feet turned out
        rig.rot_world(P["foot_r"], "X", -35)
        rig.rot_world(P["foot_l"], "Y", 40)
        rig.rot_world(P["foot_r"], "Y", -40)
        rig.rot_world(P["upperarm_l"], "Y", 15)  # the arms down onto the ground
        rig.rot_world(P["upperarm_l"], "Z", -45)  # the left by his head, elbow bent
        rig.rot_world(P["lowerarm_l"], "Z", -70)
    for n, q in rig.fist.items():  # the right hand still grips; the left lies open
        if n.endswith("_r"):
            P[n].rotation_quaternion = q
    # the right arm reaches to the weapon on the ground: solved once by IK, kept as FK
    sh = P["upperarm_r"].head.copy()
    at = (sh.x - 0.2, sh.y - 0.3, k["weapon_z"]) if up else (sh.x - 0.16, sh.y - 0.05, k["weapon_z"])
    placed = weapon_matrix(at, yaw_deg=12 if up else -12, roll_deg=-90)
    _, tr, tl = rig.make_hold(f"corpse_{int(up)}", placed, hold["grip"], hold["support"], spine_follow=False)
    cons = rig.arm_constraints(tr, tl, rig.world_empty(f"pole_corpse_{int(up)}_r", (sh.x - 0.5, sh.y + 0.2, 0.6)),
                               rig.world_empty(f"pole_corpse_{int(up)}_l", (0, 0, 0)))
    rig.only([(pb, c) for pb, c in cons if pb.name.endswith("_r")])
    bpy.context.view_layer.update()
    arm = {n: P[n].matrix.copy() for n in ("upperarm_r", "lowerarm_r", "hand_r")}
    rig.release(cons)
    for n, m in arm.items():
        P[n].matrix = m
        bpy.context.view_layer.update()
    return rig.snapshot()


# A pose's arms, shoulder to fingertip: standing, he folds to his knees still clutching
# the weapon at the carry, not shouldering it as the kneeling pose does.
ARM_BONES = ("clavicle", "upperarm", "lowerarm", "hand", "index", "middle", "ring", "pinky", "thumb")
# The corpses: the hips' roll onto one side and the head's onto a cheek (degrees), how
# far the left knee is drawn up (degrees), the weapon's height off the ground (m), and
# face up, how far the hips sink (m: less than face down, the pack holds him up).
CORPSE = dict(roll=10, cheek=70, knee=30, weapon_z=0.05, up_z=0.66)


def deaths(rig, hold, res):
    """The falls, one from each posture he can die in, each from that posture's own pose
    so he never gets up to die: hit, a jolt; standing, the knees go and he pitches onto
    his face, or, hit from the front, is thrown back onto his back; kneeling, he slumps
    forward; lying, he goes slack. Each ends in a corpse the battle lays: face down, or
    face up (`death_back`), as the schema's `INFANTRY_FALLS` has it."""
    P = rig.arm.pose.bones
    dead = corpse(rig, hold)
    dead_up = corpse(rig, hold, up=True)
    stand = rig.pose_at(res["idle"][0], 0)
    kneel = rig.pose_at(res["kneel_fire"][0], 0)
    pinned = rig.pose_at(res["prone_pinned"][0], 0)

    def moved(pose, *turns, arms=None):
        """`pose` turned by `turns`, its arms (shoulder to fingertip) taken from `arms`."""
        if arms is not None:
            pose = {n: (arms if n.startswith(ARM_BONES) else pose)[n] for n in pose}
        rig.set_pose(pose)
        for bone, axis, deg in turns:
            rig.rot_world(P[bone], axis, deg)
        return rig.snapshot()

    def sagged(pose, *turns):
        """`pose` turned by `turns`, the hips dropped and carried back as the knees go."""
        rig.set_pose(pose)
        rig.move_world(P["pelvis"], (0, 0.18, -0.32))
        for bone, axis, deg in turns:
            rig.rot_world(P[bone], axis, deg)
        return rig.snapshot()

    # the hit throws the chest and head back; then the body folds over the weapon
    hit = (("spine_02", "X", -10), ("Head", "X", -14))
    fold = (("spine_01", "X", 22), ("spine_03", "X", 14), ("Head", "X", 18))
    # hit from the front: thrown back, the knees go and he drops onto his back
    thrown = (("spine_01", "X", -14), ("spine_03", "X", -12), ("Head", "X", -22))
    reel = (("pelvis", "X", -28), ("spine_02", "X", -10), ("Head", "X", -15),
            ("thigh_l", "X", -35), ("thigh_r", "X", -20), ("calf_l", "X", 60), ("calf_r", "X", 45))
    # lying, the hit lifts his head, and he sinks as he goes
    flinch = (("spine_03", "X", -6), ("Head", "X", -16))
    falls = {
        # (frame, pose, ease into the next key as he drops)
        "death": ((0, stand, False), (3, moved(stand, *hit), False), (13, moved(kneel, *fold, arms=stand), True),
                  (25, dead, False), (32, dead, False)),
        "death_back": ((0, stand, False), (3, moved(stand, *thrown), False), (11, sagged(stand, *reel), True),
                       (21, dead_up, False), (28, dead_up, False)),
        "death_kneel": ((0, kneel, False), (3, moved(kneel, *hit), False), (12, moved(kneel, *fold), True),
                        (23, dead, False), (30, dead, False)),
        "death_prone": ((0, pinned, False), (3, moved(pinned, *flinch), False), (16, dead, False),
                        (22, dead, False)),
    }
    return {name: key_fall(rig, name, keys) for name, keys in falls.items()}


def key_fall(rig, name, keys):
    """A one-shot clip through whole-body key poses; a key marked to drop eases in (gravity)."""
    act = bpy.data.actions.new(name)
    act.use_fake_user = True
    rig.set_action(act)
    last = {}
    drops = set()
    for frame, pose, drop in keys:
        for pb in rig.arm.pose.bones:
            loc, rot = pose[pb.name]
            prev = last.get(pb.name)
            if prev is not None and prev.dot(rot) < 0:  # the short way round
                rot = -rot
            last[pb.name] = rot
            pb.location, pb.rotation_quaternion = loc, rot
            pb.keyframe_insert("location", frame=frame)
            pb.keyframe_insert("rotation_quaternion", frame=frame)
        if drop:
            drops.add(frame)
    for fc in iter_fcurves(act):
        for kp in fc.keyframe_points:
            if kp.co.x in drops:
                kp.interpolation = "QUAD"
                kp.easing = "EASE_IN"
    rig.set_action(None)
    return act, (0, keys[-1][0])


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
    export_clips(rig, res, os.path.join(REPO, f"assets/source/infantry/clips_{family}.glb"))


if __name__ == "__main__":
    main()
