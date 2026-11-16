"""How a weapon is shouldered and the head brought to it, for every clip that
aims (kneel-fire, stand-aim, prone and its crawls, the aimed and crouched walks):
a stocked weapon rests in the right shoulder pocket and the shooter cranes his neck
onto it; a launcher rides the shoulder with its sight at the eye. Rest space, as
`clips_infantry` builds in: Z up, the soldier facing -Y.
"""
import bpy
from mathutils import Matrix, Vector

# Where a stock's butt rests off the right shoulder joint, rest-space metres: up (standing,
# kneeling, walking) and lying, where the rolled shoulder brings the pocket lower and in.
POCKET = Vector((0.09, -0.05, 0.0))
PRONE_POCKET = Vector((0.06, -0.05, -0.02))
# How far a shooter's neck cranes, either way, to bring the eye to the sight (forward to
# lower it, back to raise it), degrees.
NECK_CRANE_DEG = 36.0
# How far a stock's butt may slide up or down the shoulder's front from the pocket to meet
# the eye, metres: the butt pad stays against the shoulder. Past it the eye stays above the
# sight (the weapon a little low, still a real hold), or below it.
POCKET_SLIDE = 0.04
# How far ahead of the eye a launcher's eyepiece sits, metres: its eyecup against the face.
EYE_RELIEF = 0.03
# How far the support shoulder swings forward to reach the handguard, degrees.
SUPPORT_SHOULDER_DEG = 20.0
# How far the head rolls right onto the stock, degrees.
CHEEK_ROLL_DEG = 10.0


def lean_in(rig):
    """The upper body of a shouldered aim, on the posed torso: the support shoulder swings
    forward to the handguard, and the head rolls right onto the stock (`weld_cheek` then
    brings the eye to the sight, looking along the bore)."""
    P = rig.arm.pose.bones
    rig.rot_world(P["clavicle_l"], "Z", -SUPPORT_SHOULDER_DEG)
    rig.rot_world(P["neck_01"], "Y", -CHEEK_ROLL_DEG)
    rig.rot_world(P["Head"], "Y", -CHEEK_ROLL_DEG)



def shouldered(rig, hold, pocket=None):
    """Where a weapon sits shouldered. One with a butt rests it in the right shoulder
    pocket (`pocket` off the shoulder joint: `POCKET` up, `PRONE_POCKET` lying), level and
    pointing forward, and the head comes down onto it (`weld_cheek`). A launcher (no butt)
    rides the shoulder with its eyepiece `EYE_RELIEF` ahead of the eye."""
    sh = rig.arm.pose.bones["upperarm_r"].head.copy()
    if hold["butt"] is not None:
        return Matrix.Translation(sh + (pocket if pocket is not None else POCKET) - Vector(hold["butt"]) * hold["scale"])
    eye = rig.eye_point()
    sight = Vector(hold["sight"]) * hold["scale"]
    return Matrix.Translation(Vector((eye.x, eye.y - EYE_RELIEF, eye.z)) - sight)


def _turn(pb, q):
    """Rotate a pose bone by `q` in armature space, about its head."""
    bpy.context.view_layer.update()
    m = pb.matrix.copy()
    h = m.translation.copy()
    pb.matrix = Matrix.Translation(h) @ q.to_matrix().to_4x4() @ Matrix.Translation(-h) @ m
    bpy.context.view_layer.update()


def facing(rig):
    """Where the face points, in armature space: the Head bone's Z runs out of the face
    (the eye sits at +Z, `EYE_IN_HEAD`)."""
    bpy.context.view_layer.update()
    return rig.arm.pose.bones["Head"].matrix.to_3x3().col[2].normalized()


def weld_cheek(rig, hold, placed, frames):
    """Bring the head to a shouldered weapon the way a shooter does: he looks down the
    bore at the target, his face level with it, and his eye on the sight line. The neck
    cranes forward to lower the eye or back to raise it, as far as `NECK_CRANE_DEG`, and
    the head turns to look along the bore; past the crane, the butt slides up or down the
    shoulder to meet the eye, as far as `POCKET_SLIDE`. A launcher's sight sits beside the tube: the head only
    turns, and the launcher comes to the eye where it now is (`shouldered`). The same crane
    and turn are keyed on every one of `frames` (a bob or a flinch keeps its motion).
    Returns the weapon placement."""
    P = rig.arm.pose.bones
    sight_z = (placed @ (Vector(hold["sight"]) * hold["scale"])).z
    bore = (placed.to_3x3() @ Vector((0, -1, 0))).normalized()
    rig.scn.frame_set(frames[0])
    bpy.context.view_layer.update()
    base = {n: (P[n].location.copy(), P[n].rotation_quaternion.copy()) for n in ("neck_01", "Head")}

    def crane(deg):
        for n, (loc, rot) in base.items():
            P[n].location, P[n].rotation_quaternion = loc, rot
        rig.rot_world(P["neck_01"], "X", deg)
        turn = facing(rig).rotation_difference(bore)
        _turn(P["Head"], turn)
        return turn, rig.eye_point().z

    lo, hi = -NECK_CRANE_DEG, NECK_CRANE_DEG  # the eye falls as the neck cranes forward
    if hold["butt"] is None:
        hi = 0.0
    elif crane(lo)[1] < sight_z:
        hi = lo
    elif crane(hi)[1] < sight_z:
        for _ in range(24):
            mid = (lo + hi) / 2
            lo, hi = (mid, hi) if crane(mid)[1] > sight_z else (lo, mid)
    turn, eye_z = crane(hi)
    for n, (loc, rot) in base.items():
        P[n].location, P[n].rotation_quaternion = loc, rot
    for f in frames:
        rig.scn.frame_set(f)
        bpy.context.view_layer.update()
        rig.rot_world(P["neck_01"], "X", hi)
        _turn(P["Head"], turn)
        rig.key_bones(("neck_01", "Head"), f)
    rig.scn.frame_set(frames[0])
    bpy.context.view_layer.update()
    if hold["butt"] is None:
        crane(hi)
        return shouldered(rig, hold)
    return Matrix.Translation((0, 0, max(-POCKET_SLIDE, min(POCKET_SLIDE, eye_z - sight_z)))) @ placed
