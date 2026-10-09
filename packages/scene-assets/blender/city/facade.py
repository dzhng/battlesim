"""Source-only coverage of the physical facade's fighting positions."""
import math


def missing_openings(descriptor, openings, heights):
    """Return (edge, bay offset, floor) without an opening covering eye and muzzle.

    Openings are (edge, left, right, foot, head) in the descriptor's frame.
    They describe actual source windows/doors/open fronts, never new physical bays.
    """
    parts = {p["id"]: p for p in descriptor["parts"]}
    missing = []
    for edge in descriptor["edges"]:
        pattern = edge["bays"]
        if not edge["exposed"]:
            continue
        a, b = edge["span_m"]
        pitch, phase = pattern["pitch_m"], pattern["phase_m"]
        first = math.floor((a - phase) / pitch) + 1
        last = math.ceil((b - phase) / pitch) - 1
        part = parts[edge["part"]]
        base, top = part["base_z"], part["base_z"] + 2 * part["half_extents"][2]
        for floor in descriptor["floor_heights_m"][:3]:
            if not base <= floor < top:
                continue
            for k in range(first, last + 1):
                offset = phase + k * pitch
                if not any(name == edge["id"] and left < offset < right
                           and foot <= floor + min(heights) and floor + max(heights) <= head
                           for name, left, right, foot, head in openings):
                    missing.append((edge["id"], offset, floor))
    return missing


def overlapping_openings(openings):
    """Pairs whose visible rectangles intersect on the same source wall."""
    return [(a, b) for i, a in enumerate(openings) for b in openings[i + 1:]
            if a[0] == b[0] and max(a[1], b[1]) < min(a[2], b[2])
            and max(a[3], b[3]) < min(a[4], b[4])]
