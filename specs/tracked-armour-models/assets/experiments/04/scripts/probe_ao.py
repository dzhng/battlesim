"""Mean vertex colour of groove-floor, lip and open-plate vertices on seamed parts."""
import sys, bpy
from mathutils import Vector
a = [v for v in sys.argv[sys.argv.index("--") + 1:] if v != "--"]
bpy.ops.wm.open_mainfile(filepath=a[0])
names = a[1].split(",")
for name in names:
    o = bpy.data.objects[name]
    me = o.data
    col = me.color_attributes["Color"]
    axis, sign = int(a[2]), float(a[3])
    vals = [v.co[axis] * sign for v in me.vertices]
    face = max(vals)
    groups = {"floor": [], "lip_or_plate": []}
    for v, h in zip(me.vertices, vals):
        c = col.data[v.index].color
        lum = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
        if face - float(a[4]) < h < face - 0.002:
            groups["floor"].append(lum)
        elif h > face - 0.001:
            groups["lip_or_plate"].append(lum)
    print("AO", name, {k: (len(v), round(sum(v) / max(1, len(v)), 4)) for k, v in groups.items()})
