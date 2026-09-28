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
    return H @ Vector((-0.033, 0.075, -0.095))


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
    rot_world(P["Head"], "Y", -8)  # cheek down onto the stock
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
    grip = eye + Vector((0, -0.08, 0)) - optic_rear_local
    butt = grip + butt_local
    print("KNEEL eye", eye, "shoulder", sh, "butt", butt, "butt-shoulder", (butt - sh).length)
    mw = Matrix.Translation(grip)
    anchor_k, tr_k, tl_k = make_hold("kneel", mw, spine_follow=False)
    reparent_keep(anchor_k, None)
    anchor_k.parent = arm
    bone_parent(anchor_k, "spine_03")
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
    pk_r = pole("pole_kneel_r", (-0.55, 0.15, 0.95))
    pk_l = pole("pole_kneel_l", (0.45, -0.20, 0.55))
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
    move_world(P["pelvis"], (0, 0.30, -0.80))
    rot_world(P["spine_03"], "X", -6)  # shoulders slightly up off the ground
    rot_world(P["thigh_l"], "Z", 9)
    rot_world(P["thigh_r"], "Z", -9)
    rot_world(P["foot_l"], "X", -35)
    rot_world(P["foot_r"], "X", -35)
    for fr, head_down, neck in ((0, 20, 10), (18, 24, 12), (24, 38, 22), (30, 36, 20), (44, 22, 12), (60, 20, 10)):
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
    mw = rifle_rest_matrix((head.x - 0.20, head.y - 0.28, 0.09), yaw_deg=40, pitch_deg=0, roll_deg=-25)
    anchor_p, tr_p, tl_p = make_hold("prone", mw, spine_follow=False)
    pp_r = world_empty("pole_prone_r", (sh_r.x - 0.6, sh_r.y - 0.2, -0.2))
    pp_l = world_empty("pole_prone_l", (-sh_r.x + 0.6, sh_r.y - 0.2, -0.2))
    pcons = arm_constraints(tr_p, tl_p, pp_r, pp_l)
    ALL_CONS.extend(pcons)
    only(pcons)
    res["prone_pinned"] = (bake("prone_pinned", 0, 60, pcons), (0, 60))
    only([])

    # ---- death (library as is, fists)
    src = bpy.data.actions["Death01"]
    work = src.copy()
    work.name = "death"
    set_action(work)
    f0, f1 = int(src.frame_range[0]), int(src.frame_range[1])
    for fr in (f0, f1):
        apply_fist(fr)
    strip_finger_keys(work, (f0, f1))
    work.use_fake_user = True
    res["death"] = (work, (f0, f1))
    return res


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
            c = (0.0, 0, 0.5) if role in ("prone_pinned", "death", "kneel_fire") else ctr
            aim(cam, c, 50, 14 if role not in ("prone_pinned", "death") else 30, 3.8)
            p = os.path.join(OUT, f"strip_{role}_{i}.png")
            render_to(p)
            shots.append(p)
        tile(shots, os.path.join(OUT, f"soldier_strip_{role}.png"), 8)

# ---------------------------------------------------------------- export (size measurement only)
if MODE in ("all", "export"):
    for a in bpy.data.actions:
        a.use_fake_user = a.name in {v[0].name for v in RES.values()}
    for a in list(bpy.data.actions):
        if not a.use_fake_user and a.users == 0:
            bpy.data.actions.remove(a)
    arm.animation_data.action = None
    bpy.ops.object.select_all(action="DESELECT")
    for o in bpy.data.objects:
        if o.type in ("MESH", "ARMATURE"):
            o.select_set(True)
    path = os.path.join(OUT, "rifleman.glb")
    bpy.ops.export_scene.gltf(filepath=path, use_selection=True, export_animations=True,
                              export_animation_mode="ACTIONS", export_apply=True, export_yup=True)
    print("GLB", path, os.path.getsize(path))
