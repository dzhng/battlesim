"""Flat cockpit glazing and the painted shell along its perimeter."""
from parts import mesh_part


def panel_shell(name, faces, material, parent):
    """Build flat panels with shared vertices along their common edges."""
    def build(bm, lod):
        vertices = {}
        for face in faces:
            for point in face:
                if point not in vertices:
                    vertices[point] = bm.verts.new(point)
            bm.faces.new([vertices[point] for point in face])
    return mesh_part(name, build, material, parent)


def framed_panels(faces, glass, paint, parent, inset):
    """Fill the opening inside each painted perimeter with its glass pane."""
    panes, borders = [], []
    for corners in faces:
        centre = tuple(sum(axis) / len(corners) for axis in zip(*corners))
        pane = [tuple(v + (c - v) * inset for v, c in zip(pt, centre)) for pt in corners]
        panes.append(pane)
        for i, point in enumerate(corners):
            j = (i + 1) % len(corners)
            borders.append((point, corners[j], pane[j], pane[i]))
    panel_shell("cockpit_window_frames", borders, paint, parent)
    panel_shell("canopy_glass", panes, glass, parent)
