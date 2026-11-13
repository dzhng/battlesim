"""The earthworks (`fixtures/props/military/works.json`): the revetment and HESCO, for
`military.py`."""
import bpy, bmesh, math, random
from mathutils import Euler, Matrix, Vector, noise

from parts import *
import parts
from military_parts import *


# ------------------------------------------------------------------ the revetment
@kind
def revetment():
    """A bin revetment, the kind that walls aircraft and vehicle bays: faces of galvanised
    corrugated steel held 3 m apart in a frame of posts a metre apart, a heavy sill and a
    top rail, tied across, the ends closed and framed the same way, filled with earth to
    just under the rim and grassing over, the bay number stencilled on the +X end. Rust
    weeps from the fixings and dust films the foot."""
    hx, hy, hz = 8.0, 1.5, 1.6
    face_m = textured("bin_face", "cladding", colour=(0.27, 0.27, 0.255), chip=0.4, dirt=0.8, rise=0.9, streak=0.7,
                      mottle=0.2, grain=0.02)
    post_m = textured("bin_post", "enamel", colour=(0.07, 0.065, 0.055), chip=0.7, dirt=0.8, rise=0.8)
    soil = textured("bin_soil", "soil", colour=(0.13, 0.105, 0.075), chip=0.0, dirt=0.0, streak=0.0, mottle=0.25,
                    lichen=0.45, grain=0.04)
    mark = flat_paint("bin_mark", (0.36, 0.33, 0.26), rough=0.85, grime=0.7)
    bin_ = body("bin")
    face_h, post_w, wall = 2 * hz - 0.14, 0.12, hy - 0.16  # the sheet's top; a post's flange; the faces' half spacing
    ex = hx - 0.16  # the ends' sheet
    for k, s in enumerate((-1, 1)):
        box(f"face_{k}", (2 * ex + 0.06, 0.04, face_h), (0, s * wall, face_h / 2), face_m, bin_)
        box(f"end_{k}", (0.04, 2 * wall, face_h), (s * ex, 0, face_h / 2), face_m, bin_)

    def frame(k, run, at):
        """One face's frame: posts every metre along `run` metres, a heavy sill and a top
        rail; `at(t, out)` places a point `t` along the face, `out` off it."""
        n = round(run)
        for i in range(n + 1):
            box(f"post_{k}_{i}", (post_w, post_w, face_h + 0.06), at(-run / 2 + run * i / n, post_w / 2 + 0.02) + Vector((0, 0, (face_h + 0.06) / 2)),
                post_m, bin_, rot=(0, 0, 0), lods=(0, 1, 2))
        for name, z, h, d in (("sill", 0.14, 0.28, 0.1), ("rail", face_h - 0.06, 0.1, 0.07)):
            a, b = at(-run / 2, d / 2 + 0.02), at(run / 2, d / 2 + 0.02)
            bar(f"{name}_{k}", a + Vector((0, 0, z)), b + Vector((0, 0, z)), d, h, post_m, bin_, lods=(0, 1, 2))

    for k, s in enumerate((-1, 1)):
        frame(f"side{k}", 2 * ex, lambda t, out, s=s: Vector((t, s * (wall + out), 0)))
        frame(f"end{k}", 2 * wall, lambda t, out, s=s: Vector((s * (ex + out), t, 0)))
    for i in range(1, 16):
        x = -ex + 2 * ex * i / 16
        bar(f"tie_{i}", (x, -wall, face_h - 0.1), (x, wall, face_h - 0.1), 0.025, 0.025, post_m, bin_, lods=(0, 1))
    # the fill: earth heaped inside the faces to their rim, lumpy

    def fill(bm, lod):
        nu, nv = (64, 32, 12, 4)[lod], (12, 8, 4, 2)[lod]

        def at(u, v):
            x, y = -ex + 0.02 + (2 * ex - 0.04) * u, -wall + 0.02 + (2 * wall - 0.04) * v
            crown = 0.16 * math.sin(math.pi * v) ** 0.6 * min(1.0, (ex - abs(x)) / 0.4 + 0.3)
            lumps = 0.05 * noise.noise(Vector((x * 0.9, y * 1.3, 3.0))) + 0.02 * noise.noise(Vector((x * 4, y * 4, 7.0)))
            return (x, y, face_h - 0.15 + max(0.0, crown + lumps * math.sin(math.pi * v)))

        grid(bm, nu, nv, at)
        return True

    mesh_part("fill", fill, soil, bin_, smooth=True)
    stencil("bay_number", "R4", 0.55, (ex + 0.025, 0, face_h * 0.62), (math.pi / 2, 0, math.pi / 2), mark, bin_,
            lods=(0, 1, 2))
    return [hx, hy, hz]
