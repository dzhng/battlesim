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
PRONE_POCKET = Vector((0.06, -0.05, 0.02))
# How far a shooter's neck cranes forward onto the stock (the head tipped back as far,
# so the face holds its direction), degrees; past it the stock rises to the eye.
NECK_CRANE_DEG = 24.0


def shouldered(rig, hold, pocket=None):
    """Where a weapon sits shouldered. One with a butt rests it in the right shoulder
    pocket (`pocket` off the shoulder joint: `POCKET` up, `PRONE_POCKET` lying), level and
    pointing forward, and the head comes down onto it (`weld_cheek`). A launcher (no butt)
    rides the shoulder with its sight 8 cm ahead of the eye, pulled `cheek_blend` of the
    way across toward the pocket."""
    sh = rig.arm.pose.bones["upperarm_r"].head.copy()
    if hold["butt"] is not None:
        return Matrix.Translation(sh + (pocket if pocket is not None else POCKET) - Vector(hold["butt"]) * hold["scale"])
    eye = rig.eye_point()
    sight = Vector(hold["sight"]) * hold["scale"]
    pocket = sh + POCKET
    k = hold["cheek_blend"]
    grip = Vector((eye.x + (pocket.x - eye.x) * k, eye.y - 0.08, eye.z)) - sight
    return Matrix.Translation(grip)


def weld_cheek(rig, hold, placed, frames):
    """Bring the head to a shouldered weapon's stock the way a shooter does: the neck
    cranes forward and down while the head tips back against it, so the face keeps
    looking where it looked and only the eye comes down. It comes down until it is level
    with the sight, as far as `NECK_CRANE_DEG` allows; past that the stock rises in the
    shoulder to meet it. The same crane is keyed on every one of `frames` (a bob or a
    flinch keeps its motion). Returns the weapon placement, lifted by what the neck could
    not reach. A launcher has no stock: no weld."""
    if hold["butt"] is None:
        return placed
    P = rig.arm.pose.bones
    sight_z = (placed @ (Vector(hold["sight"]) * hold["scale"])).z
    rig.scn.frame_set(frames[0])
    bpy.context.view_layer.update()
    base = {n: (P[n].location.copy(), P[n].rotation_quaternion.copy()) for n in ("neck_01", "Head")}

    def crane(deg):
        for n, (loc, rot) in base.items():
            P[n].location, P[n].rotation_quaternion = loc, rot
        rig.rot_world(P["neck_01"], "X", deg)
        rig.rot_world(P["Head"], "X", -deg)  # the face keeps its direction
        return rig.eye_point().z

    lo, hi = 0.0, NECK_CRANE_DEG
    if crane(lo) > sight_z:
        if crane(hi) > sight_z:
            lo = hi
        else:
            for _ in range(24):
                mid = (lo + hi) / 2
                lo, hi = (mid, hi) if crane(mid) > sight_z else (lo, mid)
    lift = max(0.0, crane(lo) - sight_z)
    for f in frames:
        rig.scn.frame_set(f)
        bpy.context.view_layer.update()
        rig.rot_world(P["neck_01"], "X", lo)
        rig.rot_world(P["Head"], "X", -lo)
        rig.key_bones(("neck_01", "Head"), f)
    rig.scn.frame_set(frames[0])
    bpy.context.view_layer.update()
    return Matrix.Translation((0, 0, lift)) @ placed
