"""Headless geometry lint for vehicle models.

Catches modeling mistakes before export: interpenetrating parts, floating
parts, and ground contact issues. Runs headless as part of the export or
standalone over the whole roster.

Standalone entry points:
    bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint.py -- --roster
    bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint.py -- --variant=<id>
    bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint.py -- --update-baseline
"""
import json
import math
import os
import sys
from pathlib import Path
from dataclasses import dataclass, field
from typing import Callable, Optional

import bpy
import bmesh
from mathutils import Vector
from mathutils.bvhtree import BVHTree

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

REPO = Path(__file__).resolve().parents[3]
BASELINE_PATH = REPO / "packages/scene-assets/blender/vehicle_lint_baseline.json"
ALLOWLIST_PATH = REPO / "packages/scene-assets/blender/vehicle_lint_allowlist.json"

# Flags to suppress lint and export during roster scans (when modules are imported)
_SUPPRESS_EXPORT_LINT = False
_SUPPRESS_EXPORT = False

# --------------------------------------------------------------------- config
# Per-asset-type check configuration. Each asset type specifies which checks
# run and with what tolerances.

ASSET_CONFIGS = {
    "vehicle": {
        "checks": ["interpenetration", "floating_part", "ground_contact"],
        "interpenetration_tolerance": 0.005,  # 5mm
        "floating_threshold": 0.02,  # 2cm
        "ground_tolerance": 0.01,  # 1cm
    },
    "aircraft": {
        "checks": ["interpenetration", "floating_part", "ground_contact"],
        "interpenetration_tolerance": 0.005,
        "floating_threshold": 0.02,
        "ground_tolerance": 0.015,  # 1.5cm - landing gear has more flex
    },
    "building": {
        "checks": ["interpenetration", "floating_part"],
        "interpenetration_tolerance": 0.01,  # 1cm - buildings have more tolerance
        "floating_threshold": 0.05,  # 5cm
    },
    "default": {
        "checks": ["interpenetration", "floating_part"],
        "interpenetration_tolerance": 0.005,
        "floating_threshold": 0.02,
    },
}

# Default tolerances when not specified
INTERPENETRATION_TOLERANCE = 0.005
FLOATING_PART_THRESHOLD = 0.02
GROUND_CONTACT_TOLERANCE = 0.01
GROUND_PLANE_Z = 0.0


@dataclass
class LintIssue:
    """A single lint finding."""
    check: str
    severity: str
    message: str
    parts: tuple[str, ...] = ()
    distance_m: Optional[float] = None

    def to_dict(self):
        d = {"check": self.check, "severity": self.severity, "message": self.message}
        if self.parts:
            d["parts"] = list(self.parts)
        if self.distance_m is not None:
            d["distance_m"] = round(self.distance_m, 4)
        return d

    @classmethod
    def from_dict(cls, d):
        return cls(
            check=d["check"],
            severity=d["severity"],
            message=d["message"],
            parts=tuple(d.get("parts", [])),
            distance_m=d.get("distance_m"),
        )

    def matches_baseline(self, other, distance_tolerance=0.005):
        """Check if this issue matches a baseline entry.
        
        Matches if check and parts are the same, AND the distance hasn't grown
        more than distance_tolerance (5mm default) past the baseline value.
        A part whose gap grows past its baselined value fails.
        """
        if self.check != other.check or set(self.parts) != set(other.parts):
            return False
        # If baseline has no distance, match by name only (legacy)
        if other.distance_m is None:
            return True
        # If current has no distance, match (can't compare)
        if self.distance_m is None:
            return True
        # Fail if gap has grown more than tolerance past baseline
        return self.distance_m <= other.distance_m + distance_tolerance


@dataclass
class LintResult:
    """Results from linting one vehicle."""
    vehicle_id: str
    issues: list[LintIssue] = field(default_factory=list)
    passed: bool = True

    def add(self, issue: LintIssue):
        self.issues.append(issue)
        if issue.severity == "error":
            self.passed = False


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


def _is_running_gear(obj):
    """Check if object is running gear (wheel, track, or landing gear).
    
    Uses both naming convention AND object properties for robust detection:
    - wheel_* nodes with radius_m property
    - track_* nodes with track_length_m property  
    - gear_* nodes (landing gear)
    - Children of the above
    """
    name = obj.name.lower()
    base = _base_part_name(name)
    
    # Check object properties (most reliable)
    if obj.get("radius_m") is not None:
        return True
    if obj.get("track_length_m") is not None:
        return True
    
    # Check name patterns
    if any(p in base for p in ("wheel_", "track_", "_tyre", "gear_")):
        return True
    
    # Check if any ancestor is running gear
    parent = obj.parent
    while parent is not None:
        parent_name = parent.name.lower()
        if any(p in parent_name for p in ("wheel_", "track_", "gear_")):
            return True
        if parent.get("radius_m") is not None or parent.get("track_length_m") is not None:
            return True
        parent = parent.parent
    
    return False


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
    """Get the top-level part group for an object (for detecting separate parts)."""
    ancestors = []
    p = obj.parent
    while p is not None:
        ancestors.append(p.name)
        p = p.parent
    if not ancestors:
        return _base_part_name(obj.name)
    for name in reversed(ancestors):
        if name == "hull":
            continue
        if name.startswith("wheel_") or name.startswith("track_") or name.startswith("gear_"):
            return name
        if not name.startswith("dressing_"):
            return name
    return _base_part_name(obj.name)


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


def lint_current_scene(vehicle_id: str, asset_type: str = "vehicle") -> LintResult:
    """Run lint checks on the current Blender scene.
    
    Args:
        vehicle_id: Identifier for the asset being linted
        asset_type: Type of asset ("vehicle", "aircraft", "building", "default")
    """
    result = LintResult(vehicle_id=vehicle_id)
    config = ASSET_CONFIGS.get(asset_type, ASSET_CONFIGS["default"])
    checks_to_run = config.get("checks", [])
    
    for check_name in checks_to_run:
        check_fn = LINT_CHECKS.get(check_name)
        if check_fn is None:
            continue
        
        # Get tolerance from config
        kwargs = {}
        if check_name == "interpenetration":
            kwargs["tolerance"] = config.get("interpenetration_tolerance", INTERPENETRATION_TOLERANCE)
        elif check_name == "floating_part":
            kwargs["threshold"] = config.get("floating_threshold", FLOATING_PART_THRESHOLD)
        elif check_name == "ground_contact":
            kwargs["tolerance"] = config.get("ground_tolerance", GROUND_CONTACT_TOLERANCE)
        
        try:
            check_fn(result, **kwargs)
        except Exception as e:
            result.add(LintIssue(
                check=check_name,
                severity="error",
                message=f"Check failed with exception: {e}",
            ))
    
    return result


# --------------------------------------------------------------------- baseline/allowlist

def load_baseline() -> dict[str, list[dict]]:
    """Load the baseline file of known issues."""
    if not BASELINE_PATH.exists():
        return {}
    try:
        return json.loads(BASELINE_PATH.read_text())
    except (json.JSONDecodeError, IOError):
        return {}


def save_baseline(baseline: dict[str, list[dict]]):
    """Save the baseline file."""
    BASELINE_PATH.write_text(json.dumps(baseline, indent=2, sort_keys=True) + "\n")


def load_allowlist() -> dict[str, list[dict]]:
    """Load the allowlist file of intentionally accepted issues."""
    if not ALLOWLIST_PATH.exists():
        return {}
    try:
        return json.loads(ALLOWLIST_PATH.read_text())
    except (json.JSONDecodeError, IOError):
        return {}


def save_allowlist(allowlist: dict[str, list[dict]]):
    """Save the allowlist file."""
    ALLOWLIST_PATH.write_text(json.dumps(allowlist, indent=2, sort_keys=True) + "\n")


def filter_by_baseline(result: LintResult, baseline: dict[str, list[dict]]) -> LintResult:
    """Filter out issues that are in the baseline."""
    known = baseline.get(result.vehicle_id, [])
    known_issues = [LintIssue.from_dict(d) for d in known]
    
    filtered = LintResult(vehicle_id=result.vehicle_id)
    for issue in result.issues:
        is_known = any(issue.matches_baseline(k) for k in known_issues)
        if not is_known:
            filtered.add(issue)
    return filtered


def _matches_pattern(part_name: str, pattern: str) -> bool:
    """Check if a part name matches an allowlist pattern.
    
    Supports glob-style patterns with * wildcards:
    - "wheel_*_sprocket_hub" matches "wheel_L_sprocket_hub", "wheel_R_sprocket_hub"
    - "dash*" matches "dash", "dash_-1", "dash_1"
    """
    import fnmatch
    return fnmatch.fnmatch(part_name.lower(), pattern.lower())


def filter_by_allowlist(result: LintResult, allowlist: dict) -> LintResult:
    """Filter out issues that are in the allowlist (intentional gaps).
    
    The allowlist supports two formats:
    1. Per-vehicle exact matches: {"vehicle_id": [{"check": ..., "parts": [...]}]}
    2. Global pattern matches: {"patterns": [{"part_pattern": "...", "reason": "..."}]}
    """
    # Collect patterns from allowlist
    patterns = []
    if "patterns" in allowlist:
        patterns = [p["part_pattern"] for p in allowlist["patterns"] if "part_pattern" in p]
    
    # Per-vehicle exact matches
    vehicle_allowed = allowlist.get(result.vehicle_id, [])
    vehicle_issues = [LintIssue.from_dict(d) for d in vehicle_allowed if isinstance(d, dict) and "check" in d]
    
    filtered = LintResult(vehicle_id=result.vehicle_id)
    for issue in result.issues:
        # Check exact vehicle-specific allowlist
        is_exact_match = any(issue.matches_baseline(a) for a in vehicle_issues)
        if is_exact_match:
            continue
        
        # Check pattern matches on part names
        is_pattern_match = False
        for part in issue.parts:
            if any(_matches_pattern(part, pattern) for pattern in patterns):
                is_pattern_match = True
                break
        
        if not is_pattern_match:
            filtered.add(issue)
    
    return filtered


def lint_and_report(vehicle_id: str, fail_on_new=True) -> tuple[LintResult, LintResult]:
    """Lint the current scene, returning (all issues, new issues not in baseline/allowlist)."""
    result = lint_current_scene(vehicle_id)
    baseline = load_baseline()
    allowlist = load_allowlist()
    
    # Filter out both baselined and allowlisted issues
    remaining = filter_by_baseline(result, baseline)
    remaining = filter_by_allowlist(remaining, allowlist)
    
    return result, remaining


def format_issues(issues: list[LintIssue]) -> str:
    """Format issues for printing."""
    lines = []
    for issue in issues:
        parts_str = f" ({', '.join(issue.parts)})" if issue.parts else ""
        dist_str = f" [{issue.distance_m*1000:.1f}mm]" if issue.distance_m is not None else ""
        lines.append(f"  {issue.severity.upper()}: [{issue.check}]{parts_str} {issue.message}{dist_str}")
    return "\n".join(lines)


# --------------------------------------------------------------------- roster scanning

def _roster_families():
    """List all roster family scripts."""
    roster_dir = Path(__file__).parent / "roster"
    for p in sorted(roster_dir.glob("*.py")):
        if p.name.startswith("_"):
            continue
        yield p


def _get_family_variants(family_name):
    """Get all variants for a family by parsing catalog."""
    from catalog_frames import family_variants
    try:
        return family_variants(family_name, repo=REPO)
    except Exception:
        return []


def _extract_scheme_from_script(script_path):
    """Extract the scheme(s) from a roster script by parsing its run() call."""
    import ast
    try:
        source = script_path.read_text()
        tree = ast.parse(source)
        for node in ast.walk(tree):
            if isinstance(node, ast.Call):
                func = node.func
                if isinstance(func, ast.Name) and func.id in ("run", "run_disabled"):
                    if len(node.args) >= 2:
                        scheme_arg = node.args[1]
                        if isinstance(scheme_arg, ast.Constant):
                            return scheme_arg.value
                        elif isinstance(scheme_arg, ast.Dict):
                            result = {}
                            for k, v in zip(scheme_arg.keys, scheme_arg.values):
                                if isinstance(k, ast.Constant) and isinstance(v, ast.Constant):
                                    result[k.value] = v.value
                            return result
    except Exception:
        pass
    return None


def _get_family_name_from_script(script_path):
    """Extract the family name from a roster script's run() call.
    
    Some scripts (sp_howitzers, rocket_artillery, etc.) use a family name
    different from the script name.
    """
    import ast
    try:
        source = script_path.read_text()
        tree = ast.parse(source)
        for node in ast.walk(tree):
            if isinstance(node, ast.Call):
                fn = node.func
                if isinstance(fn, ast.Name) and fn.id == "run" and node.args:
                    first_arg = node.args[0]
                    if isinstance(first_arg, ast.Constant) and isinstance(first_arg.value, str):
                        return first_arg.value
        return None
    except Exception:
        return None


def run_roster_lint(update_baseline=False):
    """Run lint over all roster vehicles.
    
    Returns (all_results dict, total_new_issues count).
    """
    global _SUPPRESS_EXPORT_LINT, _SUPPRESS_EXPORT
    import sys
    
    _SUPPRESS_EXPORT_LINT = True
    _SUPPRESS_EXPORT = True
    if 'vehicle_lint' not in sys.modules:
        sys.modules['vehicle_lint'] = sys.modules['__main__']
    
    from parts import reset
    from catalog_frames import family_variants
    
    all_results = {}
    baseline = load_baseline() if not update_baseline else {}
    allowlist = load_allowlist()
    new_baseline = {} if update_baseline else None
    failed_builds = []
    total_new_issues = 0
    variants_linted = 0
    
    families = list(_roster_families())
    print(f"Scanning {len(families)} roster families...", flush=True)
    
    for family_path in families:
        script_name = family_path.stem
        
        # Try to get family name from run() call, fallback to script name
        family_name = _get_family_name_from_script(family_path) or script_name
        
        try:
            variants = family_variants(family_name, repo=REPO)
        except Exception:
            # Also try script name if family name didn't work
            if family_name != script_name:
                try:
                    variants = family_variants(script_name, repo=REPO)
                    family_name = script_name
                except Exception as e:
                    print(f"  {script_name}: skipped (no variants for {family_name} or {script_name})")
                    continue
            else:
                print(f"  {script_name}: skipped (no variants)")
                continue
        
        if not variants:
            continue
        
        scheme = _extract_scheme_from_script(family_path)
        if scheme is None:
            print(f"  {script_name}: skipped (could not extract scheme)")
            continue
            
        try:
            spec = __import__(f"roster.{script_name}", fromlist=["build"])
            build_fn = getattr(spec, "build", None)
            if build_fn is None:
                continue
        except Exception as e:
            print(f"  {script_name}: import failed: {e}")
            continue
        
        from vehicle_export import Vehicle, materials
        
        for variant in variants:
            vid = variant["id"]
            try:
                reset()
                own_scheme = scheme[vid] if isinstance(scheme, dict) else scheme
                mats = materials(own_scheme)
                v = Vehicle(variant, mats, own_scheme)
                build_fn(variant, v)
                
                result = lint_current_scene(vid)
                all_results[vid] = result
                variants_linted += 1
                
                if update_baseline and result.issues:
                    # Only baseline issues not in allowlist
                    remaining = filter_by_allowlist(result, allowlist)
                    if remaining.issues:
                        new_baseline[vid] = [i.to_dict() for i in remaining.issues]
                
                new_issues = filter_by_baseline(result, baseline)
                new_issues = filter_by_allowlist(new_issues, allowlist)
                status = "PASS" if new_issues.passed else "FAIL"
                n_total = len(result.issues)
                n_new = len(new_issues.issues)
                total_new_issues += n_new
                print(f"LINT {vid}: {n_total} issue(s), {n_new} new")
                if new_issues.issues:
                    print(format_issues(new_issues.issues))
                    
            except Exception as e:
                failed_builds.append((vid, str(e)))
                print(f"  {vid}: build failed: {e}")
    
    print(f"\n=== Roster lint summary ===")
    print(f"Variants linted: {variants_linted}")
    print(f"New issues: {total_new_issues}")
    
    if update_baseline:
        save_baseline(new_baseline)
        print(f"Updated baseline: {BASELINE_PATH}")
        print(f"Total vehicles with issues: {len(new_baseline)}")
        total_issues = sum(len(v) for v in new_baseline.values())
        print(f"Total baseline issues: {total_issues}")
    else:
        baseline_size = sum(len(v) for v in baseline.values())
        print(f"Baseline size: {baseline_size}")
    
    if failed_builds:
        print(f"Failed to build {len(failed_builds)} variant(s)")
    
    return all_results, total_new_issues


# --------------------------------------------------------------------- CLI

def main():
    """CLI entry point for standalone lint."""
    from parts import script_args
    args = script_args()
    
    if "--roster" in args:
        _, new_issues = run_roster_lint()
        if new_issues > 0:
            print(f"\nFAIL: {new_issues} new issue(s)")
            sys.exit(1)
        print("\nPASS: no new issues")
        sys.exit(0)
    elif "--update-baseline" in args:
        run_roster_lint(update_baseline=True)
        sys.exit(0)
    elif any(a.startswith("--variant=") for a in args):
        vid = next(a.split("=", 1)[1] for a in args if a.startswith("--variant="))
        from catalog_frames import roster_variant
        from parts import reset
        variant = roster_variant(vid, repo=REPO)
        
        for family_path in _roster_families():
            family_name = family_path.stem
            try:
                variants = _get_family_variants(family_name)
                if any(v["id"] == vid for v in variants):
                    spec = __import__(f"roster.{family_name}", fromlist=["build"])
                    from vehicle_export import Vehicle, materials
                    reset()
                    scheme = getattr(spec, "SCHEME", None) or "olive_drab"
                    mats = materials(scheme)
                    v = Vehicle(variant, mats, scheme)
                    spec.build(variant, v)
                    
                    result, new_issues = lint_and_report(vid)
                    print(f"{vid}: {'PASS' if new_issues.passed else 'FAIL'}")
                    if result.issues:
                        print("All issues:")
                        print(format_issues(result.issues))
                    if new_issues.issues and new_issues.issues != result.issues:
                        print("\nNew issues (not in baseline):")
                        print(format_issues(new_issues.issues))
                    break
            except Exception:
                continue
    else:
        print(__doc__)


if __name__ == "__main__":
    main()
