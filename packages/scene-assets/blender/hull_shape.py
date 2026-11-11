"""A hull's shape in plan, in side elevation and on its sloped faces: the
plan rings a family lofts its hull through (`hull_plan`, `mirror_plan`), the
side outline its skirts and fenders keep inside (`Outline`), and where a part
sits and how it tilts on a leaning hull or turret side (`on_side`).

Pure geometry: nothing here makes an object. The parts it places keep the
contracts the kernel documents (`vehicle_parts`).
"""
import math

from mathutils import Matrix

# How far inside its convex hull a profile's point may sit and still count
# as on it: authored points are centimetres, so a millimetre is float noise,
# and a dent that shallow is lost under any bevel.
ON_HULL_M = 1e-3


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


class Outline:
    """The hull's side elevation, in the hull's XZ plane (x forward, z up):
    the convex shape its side skirts and fenders keep inside
    (`armour_parts.side_skirt`, `track_fender`).

    Built from the XZ profiles a family already extrudes (`Outline(*profiles)`,
    as passed to `parts.prism`), or from the rings it lofts through
    (`Outline.from_rings(rings)`); the outline is the convex hull of them all,
    so an upper and a lower hull make one. A profile that is not convex raises
    ValueError: a panel clipped to its envelope would stand in its notch.
    `envelope=True` takes the envelope anyway, for a hull whose notch no panel
    reaches."""

    def __init__(self, *profiles, envelope=False):
        for profile in profiles:
            if not envelope and not _convex(profile):
                raise ValueError(f"not a convex profile: {profile}")
        self.points = _hull([p for profile in profiles for p in profile])

    @classmethod
    def from_rings(cls, rings, envelope=False):
        """The side elevation of a loft through `rings` ([(z, [(x, y)...])...],
        as passed to `parts.loft`): each ring's rear and front at its height."""
        ordered = sorted(rings, key=lambda r: r[0])
        rear = [(min(x for x, _ in pts), z) for z, pts in ordered]
        front = [(max(x for x, _ in pts), z) for z, pts in reversed(ordered)]
        return cls(rear + front, envelope=envelope)

    def top(self, x):
        """The outline's highest point at `x`, or None off its ends."""
        across = span(self.points, x, 0)
        return across and across[1]

    def front(self, z):
        """The outline's foremost point at height `z`, or None above or below it."""
        across = span(self.points, z, 1)
        return across and across[1]

    def rear(self, z):
        """The outline's rearmost point at height `z`, or None above or below it."""
        across = span(self.points, z, 1)
        return across and across[0]

    def clip(self, polygon, inset):
        """`polygon` (XZ points, in order) cut to the outline's top, front
        and rear, each moved `inset` inward: never above
        `top(x) - inset`, never ahead of `front(z) - inset` or behind
        `rear(z) + inset`. The belly (its lowest edge) doesn't cut, so a
        skirt may hang below the hull. Returns the points left, possibly
        none."""
        n, belly = len(self.points), min(z for _, z in self.points) + 1e-6
        for k in range(n):
            (x0, z0), (x1, z1) = self.points[k], self.points[(k + 1) % n]
            if z0 < belly and z1 < belly:
                continue
            length = math.hypot(x1 - x0, z1 - z0)
            nx, nz = (z1 - z0) / length, (x0 - x1) / length
            polygon = clip_half_plane(polygon, nx, nz, nx * x0 + nz * z0 - inset * max(abs(nx), abs(nz)))
        return polygon


def clip_half_plane(polygon, nx, nz, c):
    """The part of `polygon` (XZ points, in order) where nx*x + nz*z <= c, in
    the same order and turning the same way (Sutherland-Hodgman). Points on
    the line are kept; a point repeated within a micrometre is dropped."""
    out = []
    for k, (x1, z1) in enumerate(polygon):
        x0, z0 = polygon[k - 1]
        d0, d1 = nx * x0 + nz * z0 - c, nx * x1 + nz * z1 - c
        if (d0 > 1e-9) != (d1 > 1e-9):
            t = d0 / (d0 - d1)
            out.append((x0 + (x1 - x0) * t, z0 + (z1 - z0) * t))
        if d1 <= 1e-9:
            out.append((x1, z1))
    return [p for k, p in enumerate(out) if math.dist(p, out[k - 1]) > 1e-6] if len(out) > 1 else out


def span(polygon, at, axis):
    """Where the convex `polygon` (XZ points) crosses the line through `at`
    on `axis` (0: a vertical line at x, 1: a horizontal one at z): its
    (low, high) along the other axis, or None if it misses."""
    other = 1 - axis
    hits = []
    for k, a in enumerate(polygon):
        b = polygon[k - 1]
        if abs(b[axis] - a[axis]) < 1e-12:
            if abs(a[axis] - at) < 1e-9:
                hits += [a[other], b[other]]
        elif min(a[axis], b[axis]) - 1e-9 <= at <= max(a[axis], b[axis]) + 1e-9:
            t = min(1.0, max(0.0, (at - a[axis]) / (b[axis] - a[axis])))
            hits.append(a[other] + (b[other] - a[other]) * t)
    return (min(hits), max(hits)) if hits else None


def _cross(o, a, b):
    return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])


def _hull(points):
    """The convex hull of 2D `points`, anticlockwise from the lowest x, with
    no repeated or collinear point (Andrew's monotone chain)."""
    pts = sorted(set(points))
    if len(pts) < 3:
        return pts

    def chain(seq):
        out = []
        for p in seq:
            while len(out) >= 2 and _cross(out[-2], out[-1], p) <= 1e-12:
                out.pop()
            out.append(p)
        return out[:-1]

    return chain(pts) + chain(reversed(pts))


def _convex(profile):
    """Whether every point of `profile` lies on its convex hull's boundary,
    within `ON_HULL_M`."""
    hull = _hull(profile)
    if len(hull) < 3:
        return True
    edges = [(a, hull[(k + 1) % len(hull)]) for k, a in enumerate(hull)]
    return all(min(_cross(a, b, p) / math.dist(a, b) for a, b in edges) <= ON_HULL_M for p in profile)
