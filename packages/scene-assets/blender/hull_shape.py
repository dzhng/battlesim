"""A hull's shape in plan and on its sloped faces: the plan rings a family
lofts its hull through (`hull_plan`, `mirror_plan`), and where a part sits and
how it tilts on a leaning hull or turret side (`on_side`).

Pure geometry: nothing here makes an object. The parts it places keep the
contracts the kernel documents (`vehicle_parts`).
"""
import math

from mathutils import Matrix


def hull_plan(rear, front, half, chamfer):
    """One horizontal ring of a lofted hull (`parts.loft`), in plan: square
    at the rear (x `rear`), its bow at x `front` with both corners chamfered
    back `chamfer`, `half` wide each side. Every ring a family lofts through
    has these six points, so rings of different sizes loft into one hull."""
    return [(rear, -half), (front - chamfer, -half), (front, -half + chamfer), (front, half - chamfer),
            (front - chamfer, half), (rear, half)]


def mirror_plan(left):
    """A whole plan ring from its left half (`left`, (x, y) points with
    y >= 0, front to rear): those points, then the right half back from rear
    to front. A point on the centre line is not repeated."""
    return list(left) + [(x, -y) for x, y in reversed(left) if y > 0]


def on_side(x, z, side, low, high, proud=0.0, standing=False, fall=0.0):
    """Where a part sits on a hull side face that leans in toward the roof.

    The face is the line through `low` and `high`, two (y, z) points on the
    left (+Y) side, such as the corners of the rings a hull lofts through.
    Returns (loc, rot) for a part at `x` whose origin is at height `z` on the
    `side` (+1 left, -1 right) face, `proud` out along the face's normal:
    a part lying in its local XY plane and facing local +Z (`bolted_panel`,
    `armour_tiles`) gets its back on the face with local x along the hull;
    on a face that also falls toward +X by `fall` (dz/dx, as a band running
    down beside a glacis), local x follows that fall. With `standing`, a part
    standing upright in its own frame (`slat_armour`, `stowage_box`) leans
    with the face, its local +Y facing out on both sides (so the right side's
    is turned half round)."""
    (y0, z0), (y1, z1) = low, high
    lean = math.atan2(y0 - y1, z1 - z0)
    y = y0 - (z - z0) * math.tan(lean)
    loc = (x, side * (y + proud * math.cos(lean)), z + proud * math.sin(lean))
    if standing:
        return loc, (lean, 0, 0 if side > 0 else math.pi)
    # Pitched down by the fall, the part's cross-section is shortened, so the
    # face leans further from upright across it.
    pitch = math.atan(fall)
    across = math.atan2(y0 - y1, (z1 - z0) * math.cos(pitch))
    roll = Matrix.Rotation(-side * (math.pi / 2 - across), 3, "X")
    return loc, tuple((Matrix.Rotation(pitch, 3, "Y") @ roll).to_euler("XYZ"))
