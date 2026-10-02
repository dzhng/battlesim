"""Detail tiers for kit meshes: one rule, the smallest feature a tier keeps.

A kit mesh is built from many loose primitives (a window's frame bars, an air
conditioner's body, fan, hose and brackets). `simplify(soup, g, bar_m)` keeps
what is larger than `g` metres and replaces what is not, island by island:

- an island smaller than `g` is dropped, and so is a bar thinner than `g / 6`;
- no bar is drawn thinner than `bar_m`: a bar under a pixel is stipple on
  screen and crawls when the camera moves. One thinner than `bar_m` becomes a
  ribbon, a single quad along it `bar_m` wide, and of the bars under
  `bar_m / 2.5` (a cage's, a railing's) every other one is left out, so what
  stays is fewer and thicker rather than a screen. A bar between `bar_m` and
  `g` becomes a four-sided bar (its ends are joints);
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


def _ribbon(lo, hi, order, colour, material, mats, width):
    """One quad along a thin bar, through its middle, showing its wider side, at
    least `width` across; a square bar shows the side that faces along Y (out from a
    wall), or X when it runs along Y."""
    dims = hi - lo
    long_axis, wide, thin = (int(i) for i in order)
    if dims[wide] < 1.05 * dims[thin]:
        thin = 1 if long_axis != 1 else 0
        wide = 3 - long_axis - thin
    mid = (lo + hi) / 2
    half = max(dims[wide], width) / 2
    lo, hi = lo.copy(), hi.copy()
    lo[wide], hi[wide] = mid[wide] - half, mid[wide] + half
    corners = []
    for u, w in ((0, 0), (1, 0), (1, 1), (0, 1)):
        p = mid.copy()
        p[long_axis] = hi[long_axis] if u else lo[long_axis]
        p[wide] = hi[wide] if w else lo[wide]
        corners.append(p)
    return Soup(np.array(corners), np.broadcast_to(colour, (4, 3)), [(0, 1, 2), (0, 2, 3)], [material] * 2, [False] * 2, mats)


def simplify(soup, g, bar_m=None):
    """`soup` with every feature under `g` metres dropped or replaced, and no bar drawn
    thinner than `bar_m` (half of `g` unless given): see the module's notes."""
    if not len(soup) or g <= 0:
        return soup
    bar_m = g / 2 if bar_m is None else bar_m
    labels = islands(soup)
    all_lo, all_hi = soup.bounds()
    out, thin = [], {}
    for label in range(labels.max() + 1):
        part = soup.keep(labels == label)
        lo, hi = part.bounds()
        dims = hi - lo
        order = np.argsort(-dims, kind="stable")
        d0, d1 = dims[order[0]], dims[order[1]]
        if d0 < g or d1 < g / 6:
            continue
        colour = part.c.mean(0)
        material = int(part.m[0])
        if d1 < bar_m and d0 > 3 * d1:
            if d1 < bar_m / 2.5:  # one of many: every other one along each axis goes
                seen = thin[int(order[0])] = thin.get(int(order[0]), 0) + 1
                if seen % 2 == 0:
                    continue
            bar = _ribbon(lo, hi, order, colour, material, part.mats, bar_m)
            # a thickened bar stays inside the mesh's own bounds: a sill's rail does not sink into the floor
            low, high = bar.bounds()
            shift = np.minimum(all_hi - high, 0.0)
            shift += np.maximum(all_lo - (low + shift), 0.0)
            out.append(Soup(bar.v + shift, bar.c, bar.t, bar.m, bar.s, bar.mats))
            continue
        _, area = part.normals()
        box_area = 2 * (dims[0] * dims[1] + dims[1] * dims[2] + dims[0] * dims[2])
        compact = 0.45 <= area.sum() / max(box_area, 1e-12) <= 1.3
        if d1 < g and d0 > 3 * d1:
            simpler = _box(lo, hi, colour, material, part.mats, open_axis=int(order[0]))
        elif compact and d1 < 3 * g:
            simpler = _box(lo, hi, colour, material, part.mats)
        else:
            simpler = _cluster(part, g)
        out.append(simpler if len(simpler) < len(part) else part)
    return Soup.join(out) if out else Soup.empty()


def hull(soup, band_m, caps=True, sides=True):
    """The faces of `soup`'s bounds, banded every `band_m` or so of height. Every
    triangle votes, by the area it shows that way, for the face of the bounds it looks
    out of and the band it is in; a face is drawn in the material that wins it where
    the votes cover half of it (a railing does not make a wall). The face against the
    wall (local +Y) is left out, with `caps` false so are the level ones, and with
    `sides` false only the front is kept: bands of colour on the wall, for the
    farthest tier."""
    lo, hi = soup.bounds()
    dims = np.maximum(hi - lo, 1e-9)
    bands = int(max(1, np.ceil(dims[2] / band_m - 1e-9)))
    step = dims[2] / bands
    normals, area = soup.normals()
    centre = soup.v[soup.t].mean(1)
    axis = np.abs(normals).argmax(1)
    shown = area * np.abs(normals[np.arange(len(area)), axis])
    side = (centre[np.arange(len(area)), axis] > (lo + hi)[axis] / 2).astype(int)
    band = np.clip(np.floor((centre[:, 2] - lo[2]) / step), 0, bands - 1).astype(int)
    level = np.clip(np.round((centre[:, 2] - lo[2]) / step), 0, bands).astype(int)
    votes = {}
    for a, sd, bd, lv, m, w in zip(axis.tolist(), side.tolist(), band.tolist(), level.tolist(), soup.m.tolist(), shown.tolist()):
        key = (2, lv, 0) if a == 2 else (a, sd, bd)
        votes.setdefault(key, {})
        votes[key][m] = votes[key].get(m, 0.0) + w
    out = []
    for key in sorted(votes):
        a, sd, bd = key
        if (a == 1 and sd == 1) or (a == 2 and not caps) or (a == 0 and not sides):
            continue
        face = dims[0] * dims[1] if a == 2 else dims[1 - a] * step
        if sum(votes[key].values()) < 0.5 * face:
            continue
        material = max(sorted(votes[key]), key=lambda m: votes[key][m])
        box_lo, box_hi = lo.copy(), hi.copy()
        if a == 2:
            box_lo[2] = box_hi[2] = lo[2] + sd * step
        else:
            box_lo[a] = box_hi[a] = lo[a] + sd * dims[a]
            box_lo[2], box_hi[2] = lo[2] + bd * step, lo[2] + (bd + 1) * step
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
