"""Overlay a model's silhouette on one view of a three-view drawing, at the
drawing's own scale, and print both outlines' widths along the length.

    blender -b --factory-startup --python drawing_overlay.py -- <model.glb> <drawing.png> <x0,y0,x1,y1>
        <view top|side|front> <nose up|down|left|right> <long axis in metres> <out.png> [aircraft|vehicle]

The box crops one view out of the drawing (pixels, top-left origin). The
long axis is the length for top and side views, the span for front. Out:
green where only the model is, pink where only the drawing is, grey where
both agree, the drawing's lines over them.

The last argument picks which of the model's parts are compared:
- aircraft (default): gear, stores and pylons are left out of the model; a
  drawing's own gear or ground line will show as pink.
- vehicle: the running gear stays in (`wheel_*` and the `track_*` belts are
  the hull's lower outline in a drawing), and every `dressing_*` subtree
  (antennas, crew, masts) is left out, so the silhouette is the one the
  battle frame holds. Side and front views stand both outlines on the ground
  line rather than centring them. A drawing's own antennas will show as
  pink."""
import sys, math
import bpy, numpy as np
from mathutils import Vector
a = [v for v in sys.argv[sys.argv.index("--") + 1:] if v != "--"]  # the asset CLI adds its own separator
glb, drw, box, view, nose, real, out = a[0], a[1], [int(v) for v in a[2].split(",")], a[3], a[4], float(a[5]), a[6]
mode = a[7] if len(a) > 7 else "aircraft"
assert mode in ("aircraft", "vehicle"), f"mode is aircraft or vehicle, not {mode}"
bpy.ops.wm.read_factory_settings(use_empty=True)
img = bpy.data.images.load(drw)
W, H = img.size
px = np.array(img.pixels[:]).reshape(H, W, 4)[::-1]  # row 0 = top
g = px[..., :3].mean(-1)
x0, y0, x1, y1 = box
crop = g[y0:y1, x0:x1] < 0.55
ys, xs = np.where(crop)
dy0, dy1, dx0, dx1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
dmask = crop[dy0:dy1, dx0:dx1]
# Filled silhouette of the drawing: per row and column, between first and last dark pixel (both ways, ANDed).
def fill(m):
    out = np.zeros_like(m)
    for i in range(m.shape[0]):
        w = np.where(m[i])[0]
        if len(w): out[i, w[0]:w[-1] + 1] = True
    out2 = np.zeros_like(m)
    for j in range(m.shape[1]):
        w = np.where(m[:, j])[0]
        if len(w): out2[w[0]:w[-1] + 1, j] = True
    return out & out2
dfill = fill(dmask)
# Model silhouette, ortho, engine basis.
bpy.ops.import_scene.gltf(filepath=glb)
def ancestors(o):
    while o.parent:
        o = o.parent
        yield o
def left_out(o):
    if not o.name.endswith("LOD0"):
        return True
    if mode == "vehicle":
        return any(p.name.startswith("dressing_") for p in (o, *ancestors(o)))
    return o.name.startswith(("gear_", "wheel_", "store_", "pylon_", "litter")) or bool(o.parent and o.parent.name.startswith("gear_"))
for o in bpy.data.objects:
    if o.type == "MESH" and left_out(o):
        o.hide_render = True
vs = [o.matrix_world @ v.co for o in bpy.data.objects if o.type == "MESH" and not o.hide_render for v in o.data.vertices]
lo = Vector([min(v[i] for v in vs) for i in range(3)]); hi = Vector([max(v[i] for v in vs) for i in range(3)])
c = (lo + hi) / 2
sc = bpy.context.scene
sc.render.engine = "BLENDER_WORKBENCH"; sh = sc.display.shading; sh.light = "FLAT"; sh.color_type = "SINGLE"; sh.single_color = (1, 1, 1)
sc.world = bpy.data.worlds.new("w"); sc.world.color = (0, 0, 0); sc.view_settings.view_transform = "Standard"
cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam")); sc.collection.objects.link(cam); sc.camera = cam
cam.data.type = "ORTHO"
PXM = 60.0
if view == "top":
    cam.location, cam.rotation_euler = (c.x, c.y, 50), (0, 0, 0); wm, hm = hi.x - lo.x, hi.y - lo.y     # image right=+X, up=+Y
elif view == "side":
    cam.location, cam.rotation_euler = (c.x, 50, c.z), (math.pi / 2, 0, math.pi); wm, hm = hi.x - lo.x, hi.z - lo.z  # image right=-X (nose left), up=+Z
else:
    cam.location, cam.rotation_euler = (50, c.y, c.z), (math.pi / 2, 0, math.pi / 2); wm, hm = hi.y - lo.y, hi.z - lo.z
wm += 0.2; hm += 0.2
sc.render.resolution_x, sc.render.resolution_y = int(wm * PXM), int(hm * PXM)
cam.data.ortho_scale = max(wm, hm); cam.data.sensor_fit = "HORIZONTAL" if wm >= hm else "VERTICAL"
sc.render.filepath = "/tmp/_drawover.png"; bpy.ops.render.render(write_still=True)
r = bpy.data.images.load(sc.render.filepath)
mm = np.array(r.pixels[:]).reshape(r.size[1], r.size[0], 4)[::-1][..., 0] > 0.5
ys, xs = np.where(mm); mm = mm[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
# Orient the model render like the drawing: model top render has nose right (+X right); side has nose left.
if view == "top":
    mm = {"right": mm, "left": mm[:, ::-1], "up": np.rot90(mm, 1), "down": np.rot90(mm, -1)}[nose]
elif view == "side" and nose == "right":
    mm = mm[:, ::-1]
# Scale: the drawing's px per metre from its long axis.
long_px = max(dfill.shape) if view != "front" else dfill.shape[1]
dpm = long_px / real
mlong = max(mm.shape) if view != "front" else mm.shape[1]
mpm = mlong / ((hi.x - lo.x) if view != "front" else (hi.y - lo.y))
k = dpm / mpm
th, tw = max(1, int(round(mm.shape[0] * k))), max(1, int(round(mm.shape[1] * k)))
iy = (np.arange(th) / k).astype(int).clip(0, mm.shape[0] - 1); ix = (np.arange(tw) / k).astype(int).clip(0, mm.shape[1] - 1)
ms = mm[iy][:, ix]
# Composite, centred on each other's bbox centre. A vehicle's side and front
# views stand on the ground line instead: a drawing's antennas would otherwise
# lift its centre and float the model off the tracks.
Hh, Ww = max(dfill.shape[0], ms.shape[0]) + 20, max(dfill.shape[1], ms.shape[1]) + 20
D = np.zeros((Hh, Ww), bool); Df = np.zeros((Hh, Ww), bool); M = np.zeros((Hh, Ww), bool)
grounded = mode == "vehicle" and view != "top"
oy, ox = (Hh - 10 - dfill.shape[0] if grounded else (Hh - dfill.shape[0]) // 2), (Ww - dfill.shape[1]) // 2
D[oy:oy + dmask.shape[0], ox:ox + dmask.shape[1]] = dmask; Df[oy:oy + dfill.shape[0], ox:ox + dfill.shape[1]] = dfill
my, mx = (Hh - 10 - ms.shape[0] if grounded else (Hh - ms.shape[0]) // 2), (Ww - ms.shape[1]) // 2
M[my:my + ms.shape[0], mx:mx + ms.shape[1]] = ms
rgb = np.ones((Hh, Ww, 4))
rgb[M & ~Df] = (0.2, 0.85, 0.3, 1)      # model only: green
rgb[Df & ~M] = (1, 0.45, 0.75, 1)       # drawing only: pink
rgb[M & Df] = (0.8, 0.8, 0.8, 1)
rgb[D] = (0, 0, 0, 1)
o = bpy.data.images.new("o", Ww, Hh); o.pixels = rgb[::-1].ravel(); o.filepath_raw = out; o.file_format = "PNG"; o.save()
print(f"DRAWOVER scale {dpm:.2f} px/m; drawing view {dfill.shape[1]/dpm:.2f} x {dfill.shape[0]/dpm:.2f} m; model {ms.shape[1]/dpm:.2f} x {ms.shape[0]/dpm:.2f} m")
# Station profiles: along the length axis (columns for side/top-left/right, rows for top up/down).
along_cols = not (view == "top" and nose in ("up", "down"))
n = 20
for name, A_ in (("drawing", Df), ("model", M)):
    vals = []
    L = A_.shape[1] if along_cols else A_.shape[0]
    idx = np.where(A_.any(0 if along_cols else 1))[0]
    s0, s1 = idx.min(), idx.max()
    for k_ in range(1, n):
        p = int(s0 + (s1 - s0) * k_ / n)
        line = A_[:, p] if along_cols else A_[p, :]
        w = np.where(line)[0]
        vals.append(f"{(w[-1]-w[0]+1)/dpm:.2f}" if len(w) else "-")
    print("DRAWOVER", name.ljust(8), view, "extent across at 5%..95% of length:", " ".join(vals))
