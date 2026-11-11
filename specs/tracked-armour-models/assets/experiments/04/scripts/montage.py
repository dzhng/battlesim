"""Grid of PNGs, each row scaled (nearest) to one height, rows stacked with a
dark gutter. Rows are separated by '/' in the argument list.

    blender -b --python montage.py -- <out.png> <height> a.png b.png / c.png d.png ..."""
import sys

import bpy
import numpy as np

a = [v for v in sys.argv[sys.argv.index("--") + 1:] if v != "--"]
out, height = a[0], int(a[1])
rows, row = [], []
for p in a[2:]:
    if p == "/":
        rows.append(row)
        row = []
    else:
        row.append(p)
rows.append(row)
GUT = 6


def load(p):
    img = bpy.data.images.load(p)
    w, h = img.size
    arr = np.array(img.pixels[:], dtype=np.float32).reshape(h, w, 4)[::-1]
    bpy.data.images.remove(img)
    ys = (np.arange(height) * h / height).astype(int)
    width = max(1, round(w * height / h))
    xs = (np.arange(width) * w / width).astype(int)
    return arr[ys][:, xs]


built = []
for r in rows:
    tiles = [load(p) for p in r]
    gut = np.full((height, GUT, 4), (0.08, 0.08, 0.08, 1.0), np.float32)
    parts = []
    for t in tiles:
        parts += [t, gut]
    built.append(np.concatenate(parts[:-1], axis=1))
width = max(b.shape[1] for b in built)
canvas = []
for b in built:
    pad = np.full((height, width - b.shape[1], 4), (0.08, 0.08, 0.08, 1.0), np.float32)
    canvas += [np.concatenate([b, pad], axis=1), np.full((GUT, width, 4), (0.08, 0.08, 0.08, 1.0), np.float32)]
full = np.concatenate(canvas[:-1], axis=0)
h, w = full.shape[:2]
img = bpy.data.images.new("montage", w, h, alpha=True)
img.pixels.foreach_set(full[::-1].ravel())
img.filepath_raw = out
img.file_format = "PNG"
img.save()
print("MONTAGE", out, w, h)
