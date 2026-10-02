"""Detail tiers for kit meshes: one rule, the smallest feature a tier keeps.

A kit mesh is built from many loose primitives (a window's frame bars, an air
conditioner's body, fan, hose and brackets). `simplify(soup, g)` keeps what is
larger than `g` metres and replaces what is not, island by island:

- an island smaller than `g` is dropped, and so is a bar thinner than `g / 8`;
- a bar thinner than `g / 2` becomes a ribbon, one quad along it, and one thinner
  than `g` a four-sided bar (its ends are joints);
- a compact island narrower than `3 g` becomes its bounding box;
- anything else is clustered: its vertices snap to a lattice of about `g`
  inside its own bounds, which keeps its extents exactly and its shape roughly.

`hull(soup, band_m)` is for the far tiers of a boxy thing that hangs on a wall (a
balcony): the faces of its bounds, banded by height, each band in the material
most of it shows there.

Nothing here calls a Blender operator, so the result is the same bytes every
run. Faces are drawn two-sided, so a sheet keeps one side.
"""
import numpy as np

from graph import Soup


def islands(soup):
    """A label per triangle: triangles that share a vertex are one island."""
    parent = np.arange(len(soup.v))

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    for a, b, c in soup.t.tolist():
        ra, rb, rc = find(a), find(b), find(c)
        low = min(ra, rb, rc)
        parent[ra] = parent[rb] = parent[rc] = low
    roots = np.array([find(i) for i in soup.t[:, 0].tolist()], dtype=np.int64)
    _, labels = np.unique(roots, return_inverse=True)
    return labels


def _box(lo, hi, colour, material, mats, open_axis=None):
    """A box over the bounds as a soup of flat faces; `open_axis` leaves the two
    faces across that axis out (a bar's ends), and a flat box is one quad."""
    dims = hi - lo
    flat = [i for i in range(3) if dims[i] < 1e-4]
    faces = []
    for axis in range(3):
        if axis == open_axis or (flat and axis not in flat):
            continue
        a, b = [i for i in range(3) if i != axis]
        for side in ((0,) if flat else (0, 1)):
            corners = []
            for u, w in ((0, 0), (1, 0), (1, 1), (0, 1)):
                p = np.array(lo, dtype=np.float64)
                p[axis] = hi[axis] if side else lo[axis]
                p[a] = hi[a] if u else lo[a]
                p[b] = hi[b] if w else lo[b]
                corners.append(p)
            # counter-clockwise seen from outside
            outward = (axis == 1) != bool(side)
            faces.append(corners if outward else corners[::-1])
    v = np.array([p for f in faces for p in f]).reshape(-1, 3)
    t = np.array([[4 * i + k for k in tri] for i in range(len(faces)) for tri in ((0, 1, 2), (0, 2, 3))]).reshape(-1, 3)
    return Soup(v, np.broadcast_to(colour, v.shape), t, np.full(len(t), material), np.zeros(len(t), bool), mats)


def _cluster(part, g):
    """Snap an island's vertices to a lattice of about `g` inside its bounds."""
    lo, hi = part.bounds()
    dims = hi - lo
    thin = dims < g  # a plate collapses onto its middle: one side of a sheet is enough
    cells = np.maximum(1, np.ceil(dims / g - 1e-9))
    step = np.where(thin, 1.0, dims / cells)
    node = np.where(thin, 0, np.round((part.v - lo) / step)).astype(np.int64)
    key = (node[:, 0] * 4096 + node[:, 1]) * 4096 + node[:, 2]
    used, first, inverse = np.unique(key, return_index=True, return_inverse=True)
    v = np.where(thin, (lo + hi) / 2, lo + node[first] * step)
    colour = np.zeros((len(used), 3))
    np.add.at(colour, inverse, part.c)
    colour /= np.bincount(inverse, minlength=len(used))[:, None]
    t = inverse[part.t]
    alive = (t[:, 0] != t[:, 1]) & (t[:, 1] != t[:, 2]) & (t[:, 0] != t[:, 2])
    t, m = t[alive], part.m[alive]
    # two triangles on the same three vertices are the two sides of a collapsed sheet
    _, once = np.unique(np.sort(t, axis=1), axis=0, return_index=True)
    once.sort()
    out = Soup(v, colour, t[once], m[once], np.zeros(len(once), bool), part.mats)
    normals, area = out.normals()
    return out.keep(area > 1e-10)


def _ribbon(lo, hi, order, colour, material, mats):
    """One quad along a thin bar, through its middle, showing its wider side; a
    square bar shows the side that faces along Y (out from a wall), or X when it
    runs along Y."""
    dims = hi - lo
    long_axis, wide, thin = (int(i) for i in order)
    if dims[wide] < 1.05 * dims[thin]:
        thin = 1 if long_axis != 1 else 0
        wide = 3 - long_axis - thin
    mid = (lo + hi) / 2
    corners = []
    for u, w in ((0, 0), (1, 0), (1, 1), (0, 1)):
        p = mid.copy()
        p[long_axis] = hi[long_axis] if u else lo[long_axis]
        p[wide] = hi[wide] if w else lo[wide]
        corners.append(p)
    return Soup(np.array(corners), np.broadcast_to(colour, (4, 3)), [(0, 1, 2), (0, 2, 3)], [material] * 2, [False] * 2, mats)


def simplify(soup, g):
    """`soup` with every feature under `g` metres dropped or replaced (see the module's notes)."""
    if not len(soup) or g <= 0:
        return soup
    labels = islands(soup)
    out = []
    for label in range(labels.max() + 1):
        part = soup.keep(labels == label)
        lo, hi = part.bounds()
        dims = hi - lo
        order = np.argsort(-dims, kind="stable")
        d0, d1 = dims[order[0]], dims[order[1]]
        if d0 < g or d1 < g / 8:
            continue
        colour = part.c.mean(0)
        material = int(part.m[0])
        _, area = part.normals()
        box_area = 2 * (dims[0] * dims[1] + dims[1] * dims[2] + dims[0] * dims[2])
        compact = 0.45 <= area.sum() / max(box_area, 1e-12) <= 1.3
        if d1 < g / 2 and d0 > 3 * d1:
            simpler = _ribbon(lo, hi, order, colour, material, part.mats)
        elif d1 < g and d0 > 3 * d1:
            simpler = _box(lo, hi, colour, material, part.mats, open_axis=int(order[0]))
        elif compact and d1 < 3 * g:
            simpler = _box(lo, hi, colour, material, part.mats)
        else:
            simpler = _cluster(part, g)
        out.append(simpler if len(simpler) < len(part) else part)
    return Soup.join(out) if out else Soup.empty()


def hull(soup, band_m, caps=True):
    """The faces of `soup`'s bounds, banded every `band_m` or so of height. Every
    triangle is snapped onto the bounds and votes, by its area, for the face and band
    it lands on; a face is drawn in the material that wins it where the votes cover
    half of it (a railing does not make a wall). The face against the wall (local +Y)
    is left out, and with `caps` false so are the level ones."""
    lo, hi = soup.bounds()
    dims = np.maximum(hi - lo, 1e-9)
    bands = int(max(1, np.ceil(dims[2] / band_m - 1e-9)))
    cells = np.array([1, 1, bands])
    node = np.round((soup.v - lo) / dims * cells).astype(np.int64)[soup.t]  # (m, 3 corners, 3 axes)
    _, area = soup.normals()
    votes = {}  # (axis, side, band) -> area per material
    for tri, a, m in zip(node.tolist(), area.tolist(), soup.m.tolist()):
        for axis in range(3):
            side = tri[0][axis]
            if tri[1][axis] != side or tri[2][axis] != side:
                continue
            if axis == 2:
                keys = [(2, side, 0)]
            else:
                z = [corner[2] for corner in tri]
                keys = [(axis, side, band) for band in range(min(z), max(z))]
            for key in keys:
                votes.setdefault(key, {})
                votes[key][m] = votes[key].get(m, 0.0) + a / len(keys)
            break
    step = dims / cells
    out = []
    for key in sorted(votes):
        axis, side, band = key
        if (axis == 1 and side == 1) or (axis == 2 and not caps):
            continue
        a, b = [i for i in range(3) if i != axis]
        face = dims[a] * (dims[b] if axis == 2 else step[2])
        material = max(sorted(votes[key]), key=lambda m: votes[key][m])
        if sum(votes[key].values()) < 0.5 * face:
            continue
        box_lo, box_hi = lo.copy(), hi.copy()
        if axis == 2:
            box_lo[2] = box_hi[2] = lo[2] + side * step[2]
        else:
            box_lo[axis] = box_hi[axis] = lo[axis] + side * dims[axis]
            box_lo[2], box_hi[2] = lo[2] + band * step[2], lo[2] + (band + 1) * step[2]
        colour = soup.c[np.unique(soup.t[soup.m == material])].mean(0)
        out.append(_box(box_lo, box_hi, colour, material, soup.mats))
    return Soup.join(out) if out else Soup.empty()


def front_quad(soup, material, out_m, mats):
    """The opening a module fills, as one quad on its wall: the module's bounds
    across X and Z, `out_m` in front of the wall plane (local -Y is outward)."""
    lo, hi = soup.bounds()
    y = -out_m
    v = np.array([(lo[0], y, lo[2]), (hi[0], y, lo[2]), (hi[0], y, hi[2]), (lo[0], y, hi[2])])
    return Soup(v, np.ones((4, 3)), [(0, 1, 2), (0, 2, 3)], [material, material], [False, False], mats)
