"""Headless geometry lint for vehicle models.

Catches modeling mistakes before export: interpenetrating parts, floating
parts, and ground contact issues. Runs headless as part of the export, or
standalone over every appearance the roster exports (`vehicle_lint_roster`).

Standalone entry points:
    bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint.py -- --roster
    bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint.py -- --variant=<id>
    bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint.py -- --update-baseline
"""
import math
import os
import sys
from pathlib import Path
from typing import Callable

import bpy
import bmesh
from mathutils import Vector
from mathutils.bvhtree import BVHTree

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from vehicle_lint_core import (
    LintIssue,
    LintResult,
    filter_by_allowlist,
    filter_by_baseline,
    format_issues,
    load_allowlist,
    load_baseline,
)

HERE = Path(__file__).resolve().parent
BASELINE_PATH = HERE / "vehicle_lint_baseline.json"
ALLOWLIST_PATH = HERE / "vehicle_lint_allowlist.json"

# --------------------------------------------------------------------- config
# Tolerances for vehicle geometry checks.
INTERPENETRATION_TOLERANCE = 0.005  # 5mm
FLOATING_PART_THRESHOLD = 0.02  # 2cm
GROUND_CONTACT_TOLERANCE = 0.01  # 1cm
GROUND_PLANE_Z = 0.0
# Hull children that move apart from the hull: compared with it, not part of it.
RUNNING_GEAR = ("wheel_", "track_", "gear_")


# --------------------------------------------------------------------- helpers

def _mesh_objects_by_tier(tier=0):
    """Get all mesh objects at the given tier (or tierless)."""
    for o in bpy.data.objects:
        if o.type != "MESH":
            continue
        name = o.name
        i = name.rfind("_LOD")
        t = int(name[i + 4:]) if i >= 0 and name[i + 4:].isdigit() else None
        if t == tier or t is None:
            yield o


def _base_part_name(obj_name):
    """Strip LOD suffix to get the base part name."""
    i = obj_name.rfind("_LOD")
    return obj_name[:i] if i >= 0 else obj_name


def _is_dressing(obj):
    """Check if object is under a dressing_* node (crew, antennas, stowage).
    
    Dressing parts are decorative and excluded from most lint checks.
    """
    parent = obj.parent
    while parent is not None:
        if parent.name.lower().startswith("dressing_"):
            return True
        parent = parent.parent
    return False


def _get_world_vertices(obj):
    """Get all vertices of a mesh in world coordinates."""
    bpy.context.view_layer.update()
    me = obj.data
    mw = obj.matrix_world
    return [mw @ v.co for v in me.vertices]


def _get_edge_samples(obj, samples_per_edge=2):
    """Sample points along mesh edges in world coordinates.
    
    For two low-poly cylinders touching at their edges, vertices may be
    far from the other surface, but edge midpoints will be close. This
    fixes false-positive floating-part reports on axles, hubs, springs.
    """
    bpy.context.view_layer.update()
    me = obj.data
    mw = obj.matrix_world
    samples = []
    
    for edge in me.edges:
        v1 = mw @ me.vertices[edge.vertices[0]].co
        v2 = mw @ me.vertices[edge.vertices[1]].co
        # Sample points along the edge (excluding endpoints which are vertices)
        for i in range(1, samples_per_edge + 1):
            t = i / (samples_per_edge + 1)
            samples.append(v1.lerp(v2, t))
    
    return samples


def _get_face_center_samples(obj):
    """Get face centers in world coordinates.
    
    Face centers catch contact that edge samples miss on flat faces.
    """
    bpy.context.view_layer.update()
    me = obj.data
    mw = obj.matrix_world
    samples = []
    
    for poly in me.polygons:
        # Calculate face center from vertex positions
        center = Vector((0, 0, 0))
        for vi in poly.vertices:
            center += me.vertices[vi].co
        center /= len(poly.vertices)
        samples.append(mw @ center)
    
    return samples


def _get_mesh_samples(obj, max_samples=60):
    """Get representative sample points from a mesh for distance checking.
    
    Combines vertices, edge samples, and face centers for robust detection
    of edge-to-edge contact between low-poly meshes like cylinders.
    
    The sampling strategy:
    - Take all vertices (primary geometry definition)
    - Take edge midpoints (catches edge-to-edge contact)
    - Take face centers (catches face-to-face contact)
    Then sample evenly if too many points.
    """
    # Collect all sample types
    verts = _get_world_vertices(obj)
    edge_samples = _get_edge_samples(obj, samples_per_edge=1)  # Just midpoints
    face_samples = _get_face_center_samples(obj)
    
    # Combine all samples
    all_samples = verts + edge_samples + face_samples
    
    if not all_samples:
        return []
    
    # Sample evenly if we have too many points
    if len(all_samples) > max_samples:
        step = len(all_samples) // max_samples
        return [all_samples[i] for i in range(0, len(all_samples), step)][:max_samples]
    
    return all_samples


def _build_bvh(obj):
    """Build a BVH tree for a mesh object in world space."""
    bpy.context.view_layer.update()
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bmesh.ops.transform(bm, matrix=obj.matrix_world, verts=bm.verts)
    bm.verts.ensure_lookup_table()
    bm.faces.ensure_lookup_table()
    verts = [v.co.copy() for v in bm.verts]
    polys = [[v.index for v in f.verts] for f in bm.faces]
    bm.free()
    if not polys:
        return None
    return BVHTree.FromPolygons(verts, polys, epsilon=0.0001)


def _mesh_bounds(obj):
    """Get world-space bounding box of a mesh object."""
    bpy.context.view_layer.update()
    mw = obj.matrix_world
    me = obj.data
    if not me.vertices:
        return None, None
    verts = [mw @ v.co for v in me.vertices]
    lo = Vector((min(v.x for v in verts), min(v.y for v in verts), min(v.z for v in verts)))
    hi = Vector((max(v.x for v in verts), max(v.y for v in verts), max(v.z for v in verts)))
    return lo, hi


def _bounds_overlap(lo1, hi1, lo2, hi2, margin=0.0):
    """Check if two bounding boxes overlap (with optional margin)."""
    return not (
        hi1.x + margin < lo2.x or lo1.x - margin > hi2.x or
        hi1.y + margin < lo2.y or lo1.y - margin > hi2.y or
        hi1.z + margin < lo2.z or lo1.z - margin > hi2.z
    )


def _part_group(obj):
    """The separate body an object belongs to, for the overlap check: the hull
    with everything fixed to it, each piece of running gear on the hull
    (`wheel_*`, `track_*`, `gear_*`), or each node articulated off the
    vehicle's root (the node carrying `unit_id`), such as the turret with all
    it carries. Plates and fittings fixed to one body overlap by design, so
    only different bodies are compared."""
    chain = [obj]
    while chain[-1].parent is not None:
        chain.append(chain[-1].parent)
    below_root = [node.name for node in reversed(chain) if node.get("unit_id") is None]
    if below_root[0] != "hull":
        return _base_part_name(below_root[0])
    if len(below_root) > 1 and below_root[1].startswith(RUNNING_GEAR):
        return _base_part_name(below_root[1])
    return "hull"


def _is_point_inside_mesh(point, bvh, bounds_lo, bounds_hi):
    """Test if a point is inside a mesh using ray-parity.
    
    First checks bounds, then casts rays in multiple directions and votes.
    Point is inside if majority of rays cross an odd number of faces.
    """
    # Quick bounds check with margin
    margin = 0.001
    if not (bounds_lo.x - margin <= point.x <= bounds_hi.x + margin and
            bounds_lo.y - margin <= point.y <= bounds_hi.y + margin and
            bounds_lo.z - margin <= point.z <= bounds_hi.z + margin):
        return False
    
    # Cast rays in multiple directions and vote
    directions = [
        Vector((1, 0, 0)), Vector((-1, 0, 0)),
        Vector((0, 1, 0)), Vector((0, -1, 0)),
        Vector((0, 0, 1)), Vector((0, 0, -1)),
    ]
    
    inside_votes = 0
    for d in directions:
        hits = 0
        origin = point.copy()
        for _ in range(20):
            hit, loc, normal, dist = bvh.ray_cast(origin, d, 1000.0)
            if not hit:
                break
            hits += 1
            origin = loc + d * 0.0001
        if hits % 2 == 1:
            inside_votes += 1
    
    return inside_votes >= 3


def _measure_penetration(obj1, obj2, bvh1, bvh2, lo1, hi1, lo2, hi2, tolerance):
    """Measure penetration depth between two meshes.
    
    Uses multiple strategies:
    1. Test vertices of each mesh inside the other
    2. Sample the bounding box intersection region
    
    This handles cases where meshes overlap but no vertices are inside.
    """
    max_depth = 0.0
    
    # Strategy 1: Test mesh vertices
    verts1 = _get_world_vertices(obj1)
    step1 = max(1, len(verts1) // 50)
    for vi in range(0, len(verts1), step1):
        v = verts1[vi]
        if _is_point_inside_mesh(v, bvh2, lo2, hi2):
            loc, normal, idx, dist = bvh2.find_nearest(v, 1.0)
            if loc is not None and dist > max_depth:
                max_depth = dist
    
    verts2 = _get_world_vertices(obj2)
    step2 = max(1, len(verts2) // 50)
    for vi in range(0, len(verts2), step2):
        v = verts2[vi]
        if _is_point_inside_mesh(v, bvh1, lo1, hi1):
            loc, normal, idx, dist = bvh1.find_nearest(v, 1.0)
            if loc is not None and dist > max_depth:
                max_depth = dist
    
    # Strategy 2: Sample the bounding box intersection region
    # This catches cases where no vertices are inside but surfaces cross
    int_lo = Vector((max(lo1.x, lo2.x), max(lo1.y, lo2.y), max(lo1.z, lo2.z)))
    int_hi = Vector((min(hi1.x, hi2.x), min(hi1.y, hi2.y), min(hi1.z, hi2.z)))
    
    if int_lo.x < int_hi.x and int_lo.y < int_hi.y and int_lo.z < int_hi.z:
        # Sample grid in intersection region
        steps = 3
        for ix in range(steps + 1):
            for iy in range(steps + 1):
                for iz in range(steps + 1):
                    t = Vector((ix / steps, iy / steps, iz / steps))
                    p = Vector((
                        int_lo.x + t.x * (int_hi.x - int_lo.x),
                        int_lo.y + t.y * (int_hi.y - int_lo.y),
                        int_lo.z + t.z * (int_hi.z - int_lo.z),
                    ))
                    
                    # Check if point is inside both meshes
                    in1 = _is_point_inside_mesh(p, bvh1, lo1, hi1)
                    in2 = _is_point_inside_mesh(p, bvh2, lo2, hi2)
                    
                    if in1 and in2:
                        # Point is in overlap region - measure depth to both surfaces
                        loc1, _, _, dist1 = bvh1.find_nearest(p, 1.0)
                        loc2, _, _, dist2 = bvh2.find_nearest(p, 1.0)
                        if loc1 and loc2:
                            depth = min(dist1, dist2)  # Depth is min distance to either surface
                            if depth > max_depth:
                                max_depth = depth
    
    return max_depth


# --------------------------------------------------------------------- checks

def check_interpenetration(result: LintResult, tolerance=INTERPENETRATION_TOLERANCE):
    """Check for real mesh-to-mesh penetration between separate parts.
    
    For each pair of meshes from different part groups:
    1. Check if bounding boxes overlap (fast prefilter)
    2. Check if BVH trees have overlapping triangles
    3. For overlapping pairs, test vertices of mesh A that are inside mesh B
    4. Measure penetration depth as distance from inside vertex to surface
    
    This catches real defects like side skirts sinking into tracks/wheels.
    """
    bpy.context.view_layer.update()
    meshes = list(_mesh_objects_by_tier(0))
    if len(meshes) < 2:
        return

    # Build bounds and BVH for each mesh
    mesh_data = []
    for obj in meshes:
        if _is_dressing(obj):
            continue
        lo, hi = _mesh_bounds(obj)
        if lo is None:
            continue
        group = _part_group(obj)
        bvh = _build_bvh(obj)
        if bvh is None:
            continue
        mesh_data.append((obj, lo, hi, group, bvh))

    # Check each pair of meshes from different part groups
    checked_pairs = set()
    for i, (obj1, lo1, hi1, group1, bvh1) in enumerate(mesh_data):
        for j, (obj2, lo2, hi2, group2, bvh2) in enumerate(mesh_data):
            if i >= j:
                continue
            if group1 == group2:
                continue
            
            pair_key = tuple(sorted([_base_part_name(obj1.name), _base_part_name(obj2.name)]))
            if pair_key in checked_pairs:
                continue
            checked_pairs.add(pair_key)

            # Skip if bounds don't overlap at all
            if not _bounds_overlap(lo1, hi1, lo2, hi2, margin=tolerance):
                continue

            # Check for overlapping triangles using BVH
            overlaps = bvh1.overlap(bvh2)
            if not overlaps:
                continue

            # Measure real penetration depth
            max_depth = _measure_penetration(obj1, obj2, bvh1, bvh2, lo1, hi1, lo2, hi2, tolerance)

            if max_depth > tolerance:
                name1 = _base_part_name(obj1.name)
                name2 = _base_part_name(obj2.name)
                result.add(LintIssue(
                    check="interpenetration",
                    severity="error",
                    message=f"Parts '{name1}' and '{name2}' interpenetrate by {max_depth*1000:.1f}mm",
                    parts=(name1, name2),
                    distance_m=max_depth,
                ))


def check_floating_parts(result: LintResult, threshold=FLOATING_PART_THRESHOLD):
    """Check for parts more than the threshold from every other part.
    
    Uses actual mesh-to-mesh distance via BVH find_nearest, not just
    bounding box distance. This catches parts that sit inside a larger
    part's bounding box but don't actually touch it (like a small part
    floating inside a concave hull).
    """
    bpy.context.view_layer.update()
    meshes = list(_mesh_objects_by_tier(0))
    if len(meshes) < 2:
        return

    # Build data for each mesh
    mesh_data = []
    for obj in meshes:
        if _is_dressing(obj):
            continue
        lo, hi = _mesh_bounds(obj)
        if lo is None:
            continue
        # Get mesh samples including vertices, edge midpoints, and face centers
        # This catches edge-to-edge contact that vertex-only sampling misses
        samples = _get_mesh_samples(obj, max_samples=60)
        if not samples:
            continue
        mesh_data.append((obj, lo, hi, samples))

    if len(mesh_data) < 2:
        return

    # Build BVH trees for all meshes upfront
    bvh_cache = {}
    for obj, _, _, _ in mesh_data:
        bvh_cache[obj.name] = _build_bvh(obj)

    # Check each mesh to see if it's near at least one other mesh
    for i, (obj1, lo1, hi1, samples1) in enumerate(mesh_data):
        min_dist = float('inf')
        
        for j, (obj2, lo2, hi2, samples2) in enumerate(mesh_data):
            if i == j:
                continue
            
            bvh1 = bvh_cache.get(obj1.name)
            bvh2 = bvh_cache.get(obj2.name)
            if bvh2 is None:
                continue
            
            # First check bounding box distance as prefilter
            dx = max(0, lo2.x - hi1.x, lo1.x - hi2.x)
            dy = max(0, lo2.y - hi1.y, lo1.y - hi2.y)
            dz = max(0, lo2.z - hi1.z, lo1.z - hi2.z)
            box_dist = math.sqrt(dx*dx + dy*dy + dz*dz)
            
            # Early skip if box distance already exceeds current minimum
            if box_dist >= min_dist:
                continue
            
            # Check samples1 vertices against bvh2 (obj2's surface)
            for v in samples1:
                loc, normal, idx, dist = bvh2.find_nearest(v, min_dist)
                if loc is not None and dist < min_dist:
                    min_dist = dist
                    if dist < 0.001:
                        break
            
            # Also check samples2 vertices against bvh1 (important when obj1 is larger)
            if min_dist > 0.001 and bvh1:
                for v in samples2:
                    loc, normal, idx, dist = bvh1.find_nearest(v, min_dist)
                    if loc is not None and dist < min_dist:
                        min_dist = dist
                        if dist < 0.001:
                            break

        # Use strict comparison with 0.5mm epsilon to avoid threshold noise
        if min_dist > threshold + 0.0005:
            name = _base_part_name(obj1.name)
            result.add(LintIssue(
                check="floating_part",
                severity="error",
                message=f"Part '{name}' is {min_dist*100:.1f}cm from nearest part (threshold: {threshold*100:.0f}cm)",
                parts=(name,),
                distance_m=min_dist,
            ))


def check_ground_contact(result: LintResult, tolerance=GROUND_CONTACT_TOLERANCE):
    """Check that each wheel and track contacts the ground properly.
    
    Unlike a naive single-lowest-point check, this examines each piece of
    running gear individually. A vehicle passes only if:
    - Every track run (left and right) touches the ground
    - Every road wheel touches ground OR is above a track that does
    - Landing gear wheels (for aircraft) touch ground
    
    This catches issues like a single wheel hovering while others touch.
    """
    bpy.context.view_layer.update()
    
    # Collect all running gear by type and side
    gear_groups = {
        "track_L": [], "track_R": [],
        "wheel_L": [], "wheel_R": [],
        "gear": [],  # Landing gear
    }
    
    for obj in bpy.data.objects:
        name = obj.name.lower()
        
        # Track nodes
        if obj.get("track_length_m") is not None or "track_" in name:
            side = "L" if "_l" in name or name.endswith("_l") else "R"
            gear_groups[f"track_{side}"].append(obj)
            continue
        
        # Wheel nodes (with radius property or wheel_ prefix)
        if obj.get("radius_m") is not None or name.startswith("wheel_"):
            # Determine side from name
            if "_l_" in name or "_l" in name.split("_"):
                gear_groups["wheel_L"].append(obj)
            elif "_r_" in name or "_r" in name.split("_"):
                gear_groups["wheel_R"].append(obj)
            elif name.startswith("wheel_") and "gear_" in _get_ancestor_names(obj):
                # Landing gear wheel
                gear_groups["gear"].append(obj)
            continue
        
        # Landing gear nodes
        if name.startswith("gear_"):
            gear_groups["gear"].append(obj)

    # Determine if this is likely an aircraft (has landing gear, no tracks)
    is_aircraft = (len(gear_groups["gear"]) > 0 and 
                   len(gear_groups["track_L"]) == 0 and 
                   len(gear_groups["track_R"]) == 0)
    
    # Check each group that should touch ground
    groups_to_check = []
    if gear_groups["track_L"]:
        groups_to_check.append(("left track", gear_groups["track_L"]))
    if gear_groups["track_R"]:
        groups_to_check.append(("right track", gear_groups["track_R"]))
    if not gear_groups["track_L"] and not gear_groups["track_R"]:
        # Wheeled vehicle or aircraft - check wheels
        if gear_groups["wheel_L"]:
            groups_to_check.append(("left wheels", gear_groups["wheel_L"]))
        if gear_groups["wheel_R"]:
            groups_to_check.append(("right wheels", gear_groups["wheel_R"]))
    if gear_groups["gear"]:
        groups_to_check.append(("landing gear", gear_groups["gear"]))

    if not groups_to_check:
        return  # No running gear found

    for group_name, group_objs in groups_to_check:
        # Find all meshes in this group
        group_meshes = []
        for obj in group_objs:
            # Get all mesh children recursively
            group_meshes.extend(_get_mesh_descendants(obj))
        
        if not group_meshes:
            continue
        
        # Find lowest point in group
        lowest_z = float('inf')
        lowest_part = None
        for mesh in group_meshes:
            lo, hi = _mesh_bounds(mesh)
            if lo is not None and lo.z < lowest_z:
                lowest_z = lo.z
                lowest_part = mesh
        
        if lowest_z == float('inf'):
            continue
        
        ground_dist = lowest_z - GROUND_PLANE_Z
        
        if ground_dist > tolerance:
            name = _base_part_name(lowest_part.name) if lowest_part else group_name
            result.add(LintIssue(
                check="ground_contact",
                severity="error",
                message=f"{group_name.title()} hovers {ground_dist*100:.1f}cm above ground ('{name}' lowest)",
                parts=(name,),
                distance_m=ground_dist,
            ))
        elif ground_dist < -tolerance:
            name = _base_part_name(lowest_part.name) if lowest_part else group_name
            result.add(LintIssue(
                check="ground_contact",
                severity="error",
                message=f"{group_name.title()} sinks {-ground_dist*100:.1f}cm below ground ('{name}' lowest)",
                parts=(name,),
                distance_m=abs(ground_dist),
            ))


def _get_ancestor_names(obj):
    """Get names of all ancestors of an object."""
    names = []
    p = obj.parent
    while p:
        names.append(p.name.lower())
        p = p.parent
    return names


def _get_mesh_descendants(obj):
    """Get all mesh objects that are descendants of obj (including obj itself if it's a mesh)."""
    meshes = []
    if obj.type == "MESH":
        meshes.append(obj)
    for child in obj.children:
        meshes.extend(_get_mesh_descendants(child))
    return meshes


# Registry of all lint checks
LINT_CHECKS: dict[str, Callable[[LintResult], None]] = {
    "interpenetration": check_interpenetration,
    "floating_part": check_floating_parts,
    "ground_contact": check_ground_contact,
}


def lint_current_scene(vehicle_id: str) -> LintResult:
    """Run lint checks on the current Blender scene."""
    result = LintResult(vehicle_id=vehicle_id)
    
    for check_name, check_fn in LINT_CHECKS.items():
        try:
            check_fn(result)
        except Exception as e:
            result.add(LintIssue(
                check=check_name,
                severity="error",
                message=f"Check failed with exception: {e}",
            ))
    
    return result


# --------------------------------------------------------------------- baseline

def lint_and_report(vehicle_id: str) -> tuple[LintResult, LintResult]:
    """Lint the current scene, returning (all issues, new issues not in baseline/allowlist)."""
    result = lint_current_scene(vehicle_id)
    remaining = filter_by_baseline(result, load_baseline(BASELINE_PATH))
    return result, filter_by_allowlist(remaining, load_allowlist(ALLOWLIST_PATH))


if __name__ == "__main__":
    # The roster walk imports this module by name: let it find this run.
    sys.modules["vehicle_lint"] = sys.modules[__name__]
    from parts import script_args
    from vehicle_lint_roster import main
    main(script_args())
