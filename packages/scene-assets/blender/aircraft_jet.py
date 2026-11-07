"""A whole fixed-wing airframe from its family's spec, every part named so
`crash` finds it. Built from `aircraft_parts`' pieces, and reached through
it (`A.jet`), as every roster script does. Its shell is either lofted
(`fuselage`, `bodies`, and its flying surfaces) or a generated `skin`
(`aircraft_skin`), on which the rest is built just the same.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from aircraft_parts import (body, canopy, fin, fit, gear, intake, nav_lights, nozzle, paired_surface,  # noqa: E402
                            pitot, pylon, round_intake, store)
from aircraft_canopy import bubble_canopy  # noqa: E402
from aircraft_skin import skin  # noqa: E402


def _both(rows, at):
    """Rows whose location (item `at`) is off the centreline, and their mirrors."""
    out = []
    for row in rows:
        out.append(row)
        loc = row[at]
        if abs(loc[1]) > 1e-6:
            out.append(tuple((loc[0], -loc[1], loc[2]) if i == at else r for i, r in enumerate(row)))
    return out

def jet(v, spec):
    """A fixed-wing airframe from its family's `spec` (all optional but the
    fuselage or the skin), every part named so `crash` finds it: `skin`
    (the `aircraft_skin.skin` keywords but the materials and parent, in
    place of the lofted fuselage, bodies and surfaces; its `canopy` plan
    becomes a bubble canopy fitted to its hump, with frame bows at its
    `bows`), `fuselage` (stations,
    `seg`), `bodies` (more bodies: `(name, stations[, y[, seg]])`, off the
    centreline by `y`), `canopy` (the
    `canopy()` arguments), `wing`, `strake`, `canard`, `stab` (left-side
    sections, mirrored), `fins` (`fin()` keyword sets), `intakes`
    (`(kind, loc, size..)`), `nozzles` (`nozzle()` keyword sets), `gear`
    (`gear()` keyword sets), `pylons` (`(loc, length, depth)`), `stores`
    (`(kind, loc, length, radius[, rail])`, a missile on a launch rail
    unless `rail` says not; an off-centre pylon or store is mirrored),
    `nose_probe` (`(loc, length)`), `wing_hinge`/`stab_hinge`/
    `canard_hinge` (their control surfaces' chord fractions, `hinges`) and
    `nav` (`nav_lights` keywords). Markings go on afterwards (`markings`):
    they are painted onto the skin, so the skin must be built first."""
    m, hull = fit(v), v.hull
    if "skin" in spec:
        fitted = skin(mats=m, parent=hull, **spec["skin"])
        if fitted is not None:
            bubble_canopy("canopy", *fitted, m, hull, bows=spec["skin"]["canopy"].get("bows", ()))
    else:
        body("fuselage", spec["fuselage"], m["paint"], hull, seg=spec.get("seg", 28))
    for name, stations, *at in spec.get("bodies", ()):
        y, seg = (list(at) + [0.0, 18][len(at):])[:2]
        body(name, stations, m["paint"], hull, seg=seg, loc=(0, y, 0))
    if "canopy" in spec:
        canopy("canopy", mats=m, parent=hull, **spec["canopy"])
    # Wings and their strakes and canards are `wing*` (the crash folds them),
    # the tailplane is `tail_stab` (it breaks off with the tail).
    for key, name in (("wing", "wing"), ("strake", "wing_strake"), ("canard", "wing_canard"), ("stab", "tail_stab")):
        if key in spec:
            paired_surface(name, spec[key], m["paint"], hull, hinge=spec.get(f"{key}_hinge", ()), hinge_mat=m["black"])
    for k, f in enumerate(spec.get("fins", ())):
        fin(f"tail_fin_{k}", mats=m, parent=hull, **f)
    for k, (kind, *args) in enumerate(spec.get("intakes", ())):
        if kind == "round":
            round_intake(f"intake_{k}", *args, mats=m, parent=hull)
        else:
            loc, w, h, d, *rot = args
            intake(f"intake_{k}", loc, w, h, d, m, hull, rot=rot[0] if rot else (0, 0, 0))
    for k, n in enumerate(spec.get("nozzles", ())):
        nozzle(f"nozzle_{k}", mats=m, parent=hull, **n)
    for g in spec.get("gear", ()):
        gear(mats=m, parent=hull, **g)
    for k, (loc, length, depth) in enumerate(_both(spec.get("pylons", ()), 0)):
        pylon(f"pylon_{k}", loc, length, depth, m, hull)
    for k, (kind, loc, length, radius, *rail) in enumerate(_both(spec.get("stores", ()), 1)):
        store(f"store_{k}", kind, loc, length, radius, m, hull, rail=rail[0] if rail else kind == "missile")
    if "nose_probe" in spec:
        pitot("nose_probe", *spec["nose_probe"], m, hull)
    if "nav" in spec:
        nav_lights(m, hull, **spec["nav"])
    return m
