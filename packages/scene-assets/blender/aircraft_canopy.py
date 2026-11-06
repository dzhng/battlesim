"""A one-piece bubble canopy drawn from its references' outlines: the
transparency's plan width, its sill and its top along its length, so it
tapers and rises as the photos show rather than following one profile.
Clear enough to show the cockpit under it: the tub and its side consoles,
the coaming and the head-up display's combiner on it, the ejection seat
reclined under the glass's peak and its headrest.

`aircraft_parts.canopy` is the framed and tinted canopy most airframes
wear; this one's glass is its own material (`CLEAR_TINT`, seen through at
`CLEAR_OPACITY`), its sills and bows in the airframe's paint, and its
opaque tail (the frame behind the last bow, fairing into the spine) is part
of it. Its glass is `<name>_glass`, so a wreck's crash takes it as it takes
every canopy's.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from aircraft_loft import loft  # noqa: E402
from parts import box, flat_paint  # noqa: E402
from vehicle_parts import FINE, MID, NEAR, tube_part  # noqa: E402

# A clear canopy: near black, as all glass is drawn (it shows what it
# reflects), warmed toward the gold of a fighter's coated transparency,
# smooth, and mostly seen through.
CLEAR_TINT, CLEAR_ROUGH, CLEAR_OPACITY = (0.03, 0.022, 0.008), 0.04, 0.32


def _section(hw, sill, top, power, skirt=0.06):
    """A canopy's half section: a superellipse (`power`, above 2 fuller at
    the shoulders) from its top to its sill, then the skirt below the sill
    the body hides."""
    pts = []
    for deg in (0, 22, 42, 60, 75, 90):
        t = math.radians(deg)
        pts.append((hw * math.sin(t) ** (2 / power), sill + (top - sill) * math.cos(t) ** (2 / power)))
    return pts + [(hw * 0.96, sill - skirt), (0.0, sill - skirt)]


def _at(stations, x):
    """A station table `(x, hw, sill, top)`, front to back, at `x`."""
    for a, b in zip(stations, stations[1:]):
        if b[0] <= x <= a[0]:
            t = (a[0] - x) / (a[0] - b[0])
            return tuple(p + (q - p) * t for p, q in zip(a[1:], b[1:]))
    raise ValueError(f"x {x} is outside the canopy")


def bubble_canopy(name, glass, tail, mats, parent, bows=(), power=2.6, seat_x=None, hud_x=None):
    """A bubble canopy whose transparency runs through `glass` and whose
    painted tail runs through `tail` (each `(x, half_width, sill, top)`,
    front to back, the tail's first station the glass's last), with frame
    bows at the X positions `bows`, a sill rail along each side of the
    glass, the seat's back at `seat_x` and the head-up display at `hud_x`."""
    clear = mats.setdefault("canopy_clear", flat_paint("canopy_clear", CLEAR_TINT, rough=CLEAR_ROUGH, grime=0,
                                                       coverage=("blended", CLEAR_OPACITY), role="glass"))
    made = loft(f"{name}_glass", [(x, _section(hw, s, t, power)) for x, hw, s, t in glass], clear, parent)
    loft(f"{name}_tail", [(x, _section(hw, s, t, power)) for x, hw, s, t in tail], mats["paint"], parent)
    whole = list(glass) + list(tail[1:])
    for k, bx in enumerate(bows):
        hw, s, t = _at(whole, bx)
        ring = _section(hw * 1.04 + 0.01, s, t + 0.012, power)
        loft(f"{name}_bow_{k}", [(bx + 0.04, ring), (bx - 0.04, ring)], mats["paint"], parent, lods=MID,
             steps=(1, 1, 1, 1))
    for side, s in ((1, "L"), (-1, "R")):
        tube_part(f"{name}_sill_{s}", lambda lod, side=side: [(x, side * (hw + 0.004), sill)
                                                            for x, hw, sill, _ in glass[1:]],
                  0.012, mats["paint"], parent, MID)
    # The cockpit under the glass: the tub up to its side consoles, opaque so
    # the glass's own skirt never shows through it.
    inner = [st for st in glass[1:-1]]
    loft(f"{name}_cockpit", [(x, [(0, s + 0.02), (hw * 0.55, s + 0.02), (hw * 0.86, s + 0.1, True),
                                  (hw * 0.9, s - 0.05, True), (0, s - 0.05)]) for x, hw, s, _ in inner],
         mats["cockpit"], parent, lods=MID, steps=(1, 1, 1, 1), outline_steps=(1, 1, 1, 1))
    front = glass[0][0]
    hud_x = front - (front - glass[-1][0]) * 0.28 if hud_x is None else hud_x
    hw, s, t = _at(glass, hud_x)
    # The coaming humped over the instruments, the combiner's frame and glass on it.
    loft(f"{name}_coaming", [(hud_x + 0.32, [(0, s + 0.04), (hw * 0.5, s + 0.03), (hw * 0.8, s, True), (0, s)]),
                             (hud_x - 0.05, [(0, s + 0.17), (hw * 0.55, s + 0.15), (hw * 0.82, s + 0.04, True),
                                             (0, s + 0.04)]),
                             (hud_x - 0.2, [(0, s + 0.12), (hw * 0.55, s + 0.1), (hw * 0.82, s, True), (0, s)])],
         mats["cockpit"], parent, lods=MID, steps=(1, 1, 1, 1), outline_steps=(2, 1, 1, 1))
    for side in (1, -1):
        box(f"{name}_hud_post_{'L' if side > 0 else 'R'}", (0.03, 0.02, 0.16), (hud_x, side * 0.08, s + 0.24),
            mats["black"], parent, rot=(0, -0.35, 0), lods=NEAR)
    box(f"{name}_hud_glass", (0.01, 0.15, 0.13), (hud_x + 0.01, 0, s + 0.25), mats["glass"], parent,
        rot=(0, -0.35, 0), lods=NEAR)
    # The ejection seat reclined under the peak, its headrest and its pan.
    seat_x = glass[0][0] - (glass[0][0] - glass[-1][0]) * 0.62 if seat_x is None else seat_x
    hw, s, t = _at(glass, seat_x)
    back = (t - s) * 0.95
    box(f"{name}_seat", (0.12, 0.42, back), (seat_x - back * 0.25, 0, s + back / 2 - 0.05), mats["cockpit"],
        parent, bevel=0.02, rot=(0, -0.5, 0), lods=MID)
    box(f"{name}_headrest", (0.16, 0.26, 0.2), (seat_x - back * 0.5, 0, s + back - 0.08), mats["black"], parent,
        bevel=0.02, rot=(0, -0.5, 0), lods=NEAR)
    box(f"{name}_seat_pan", (0.42, 0.4, 0.1), (seat_x + 0.18, 0, s + 0.02), mats["cockpit"], parent, bevel=0.02,
        lods=FINE)
    return made
