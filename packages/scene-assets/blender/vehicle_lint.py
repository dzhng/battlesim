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

# Flags to suppress lint and export during roster scans (when modules are imported)
_SUPPRESS_EXPORT_LINT = False
_SUPPRESS_EXPORT = False

# Tolerances (metres)
INTERPENETRATION_TOLERANCE = 0.005  # 5mm: allow intended contact points
FLOATING_PART_THRESHOLD = 0.02  # 2cm: parts must be within 2cm of another part
GROUND_CONTACT_TOLERANCE = 0.01  # 1cm: wheels/tracks must touch ground within 1cm
GROUND_PLANE_Z = 0.0


@dataclass
class LintIssue:
    """A single lint finding."""
    check: str  # which check found it
    severity: str  # "error" or "warning"
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

    def matches_baseline(self, other):
        """Check if this issue matches a baseline entry (same check, parts)."""
        return self.check == other.check and set(self.parts) == set(other.parts)


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


def _is_track_or_wheel(name):
    """Check if a part name indicates track or wheel."""
    base = _base_part_name(name).lower()
    return "wheel_" in base or "track_" in base or "_tyre" in base


def _is_dressing(obj):
    """Check if object is under a dressing_* node (crew, antennas)."""
    parent = obj.parent
    while parent is not None:
        if parent.name.startswith("dressing_"):
            return True
        parent = parent.parent
    return False


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
        if name.startswith("wheel_") or name.startswith("track_"):
            return name
        if not name.startswith("dressing_"):
            return name
    return _base_part_name(obj.name)


def check_interpenetration(result: LintResult, tolerance=INTERPENETRATION_TOLERANCE):
    """Check for meaningful mesh-to-mesh intersection between separate parts.
    
    Uses BVH ray casting to detect when meshes from different part groups
    actually intersect beyond the tolerance (intended contact allowance).
    Bounding-box prefilter skips obviously separate pairs.
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
        mesh_data.append((obj, lo, hi, group, None))  # BVH built lazily

    # Check each pair of meshes from different part groups
    checked_pairs = set()
    for i, (obj1, lo1, hi1, group1, _) in enumerate(mesh_data):
        for j, (obj2, lo2, hi2, group2, _) in enumerate(mesh_data):
            if i >= j:
                continue
            if group1 == group2:
                continue
            pair_key = tuple(sorted([_base_part_name(obj1.name), _base_part_name(obj2.name)]))
            if pair_key in checked_pairs:
                continue
            checked_pairs.add(pair_key)

            # Skip if bounds don't even overlap
            if not _bounds_overlap(lo1, hi1, lo2, hi2, margin=tolerance):
                continue

            # Build BVH trees for actual intersection test
            bvh1 = _build_bvh(obj1)
            bvh2 = _build_bvh(obj2)
            if bvh1 is None or bvh2 is None:
                continue

            # Check for overlap using BVH overlap detection
            overlaps = bvh1.overlap(bvh2)
            if not overlaps:
                continue

            # Measure the depth of penetration using ray casting
            max_depth = 0.0
            samples = min(len(overlaps), 50)
            for k in range(samples):
                tri_idx1, tri_idx2 = overlaps[k % len(overlaps)]
                # Sample a point in the overlap region
                me1 = obj1.data
                if tri_idx1 >= len(me1.polygons):
                    continue
                poly1 = me1.polygons[tri_idx1]
                centre1 = obj1.matrix_world @ poly1.center
                normal1 = (obj1.matrix_world.to_3x3() @ poly1.normal).normalized()
                
                # Cast ray into second mesh to measure penetration
                hit, _, _, dist = bvh2.ray_cast(centre1, normal1, 0.5)
                if hit:
                    # Also cast the opposite direction
                    hit2, _, _, dist2 = bvh2.ray_cast(centre1, -normal1, 0.5)
                    if hit2 and dist2 < dist:
                        max_depth = max(max_depth, dist2)
                    else:
                        max_depth = max(max_depth, dist)

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
    
    A floating part is detached from the vehicle - it should be connected
    to at least one other part within the threshold distance.
    """
    bpy.context.view_layer.update()
    meshes = list(_mesh_objects_by_tier(0))
    if len(meshes) < 2:
        return

    # Build bounds for each mesh
    mesh_data = []
    for obj in meshes:
        if _is_dressing(obj):
            continue
        lo, hi = _mesh_bounds(obj)
        if lo is None:
            continue
        centre = (lo + hi) / 2
        mesh_data.append((obj, lo, hi, centre))

    if len(mesh_data) < 2:
        return

    # Check each mesh to see if it's near at least one other mesh
    for i, (obj1, lo1, hi1, c1) in enumerate(mesh_data):
        min_dist = float('inf')
        nearest = None
        for j, (obj2, lo2, hi2, c2) in enumerate(mesh_data):
            if i == j:
                continue
            # Distance between bounding boxes (0 if they overlap)
            dx = max(0, lo2.x - hi1.x, lo1.x - hi2.x)
            dy = max(0, lo2.y - hi1.y, lo1.y - hi2.y)
            dz = max(0, lo2.z - hi1.z, lo1.z - hi2.z)
            dist = math.sqrt(dx*dx + dy*dy + dz*dz)
            if dist < min_dist:
                min_dist = dist
                nearest = obj2

        if min_dist > threshold:
            name = _base_part_name(obj1.name)
            result.add(LintIssue(
                check="floating_part",
                severity="error",
                message=f"Part '{name}' is {min_dist*100:.1f}cm from any other part (threshold: {threshold*100:.0f}cm)",
                parts=(name,),
                distance_m=min_dist,
            ))


def check_ground_contact(result: LintResult, tolerance=GROUND_CONTACT_TOLERANCE):
    """Check that tracks or wheels touch the ground plane.
    
    Every vehicle's running gear must actually contact the ground in its
    rest pose. Flag vehicles that hover above it or sink below it.
    """
    bpy.context.view_layer.update()
    
    # Find all track and wheel meshes
    contact_meshes = []
    for obj in _mesh_objects_by_tier(0):
        if _is_dressing(obj):
            continue
        if _is_track_or_wheel(obj.name):
            contact_meshes.append(obj)

    if not contact_meshes:
        # No tracks or wheels found - might be aircraft, skip
        return

    # Find the lowest point of all contact meshes
    lowest_z = float('inf')
    lowest_part = None
    for obj in contact_meshes:
        lo, hi = _mesh_bounds(obj)
        if lo is None:
            continue
        if lo.z < lowest_z:
            lowest_z = lo.z
            lowest_part = obj

    if lowest_z == float('inf'):
        return

    # Check distance from ground
    ground_dist = lowest_z - GROUND_PLANE_Z
    
    if ground_dist > tolerance:
        # Vehicle hovers above ground
        name = _base_part_name(lowest_part.name) if lowest_part else "running gear"
        result.add(LintIssue(
            check="ground_contact",
            severity="error",
            message=f"Vehicle hovers {ground_dist*100:.1f}cm above ground ('{name}' lowest)",
            parts=(name,),
            distance_m=ground_dist,
        ))
    elif ground_dist < -tolerance:
        # Vehicle sinks below ground
        name = _base_part_name(lowest_part.name) if lowest_part else "running gear"
        result.add(LintIssue(
            check="ground_contact",
            severity="error",
            message=f"Vehicle sinks {-ground_dist*100:.1f}cm below ground ('{name}' lowest)",
            parts=(name,),
            distance_m=abs(ground_dist),
        ))


# Registry of all lint checks - add new checks here
LINT_CHECKS: list[Callable[[LintResult], None]] = [
    check_interpenetration,
    check_floating_parts,
    check_ground_contact,
]


def lint_current_scene(vehicle_id: str) -> LintResult:
    """Run all lint checks on the current Blender scene."""
    result = LintResult(vehicle_id=vehicle_id)
    for check in LINT_CHECKS:
        try:
            check(result)
        except Exception as e:
            result.add(LintIssue(
                check=check.__name__,
                severity="error",
                message=f"Check failed with exception: {e}",
            ))
    return result


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


def lint_and_report(vehicle_id: str, fail_on_new=True) -> tuple[LintResult, LintResult]:
    """Lint the current scene, returning (all issues, new issues not in baseline)."""
    result = lint_current_scene(vehicle_id)
    baseline = load_baseline()
    new_issues = filter_by_baseline(result, baseline)
    return result, new_issues


def format_issues(issues: list[LintIssue]) -> str:
    """Format issues for printing."""
    lines = []
    for issue in issues:
        parts_str = f" ({', '.join(issue.parts)})" if issue.parts else ""
        dist_str = f" [{issue.distance_m*1000:.1f}mm]" if issue.distance_m is not None else ""
        lines.append(f"  {issue.severity.upper()}: [{issue.check}]{parts_str} {issue.message}{dist_str}")
    return "\n".join(lines)


# ---------------------------------------------------------------- roster scanning

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
                # Look for run(...) or run_disabled(...) calls
                if isinstance(func, ast.Name) and func.id in ("run", "run_disabled"):
                    if len(node.args) >= 2:
                        scheme_arg = node.args[1]
                        if isinstance(scheme_arg, ast.Constant):
                            return scheme_arg.value
                        elif isinstance(scheme_arg, ast.Dict):
                            # scheme is a dict per variant
                            result = {}
                            for k, v in zip(scheme_arg.keys, scheme_arg.values):
                                if isinstance(k, ast.Constant) and isinstance(v, ast.Constant):
                                    result[k.value] = v.value
                            return result
    except Exception:
        pass
    return None


def run_roster_lint(update_baseline=False):
    """Run lint over all roster vehicles."""
    global _SUPPRESS_EXPORT_LINT
    import sys
    
    # Suppress lint and export during roster scan BEFORE any imports that might trigger roster modules.
    # We need to set this on both __main__ and vehicle_lint modules since the script
    # runs as __main__ but _run_lint imports vehicle_lint.
    _SUPPRESS_EXPORT_LINT = True
    _SUPPRESS_EXPORT = True
    if 'vehicle_lint' not in sys.modules:
        # Register this module under vehicle_lint name so other modules see the same flags
        sys.modules['vehicle_lint'] = sys.modules['__main__']
    
    from parts import reset
    from catalog_frames import family_variants
    
    all_results = {}
    baseline = load_baseline() if not update_baseline else {}
    new_baseline = {} if update_baseline else None
    failed_builds = []
    
    families = list(_roster_families())
    print(f"Scanning {len(families)} roster families...", flush=True)
    
    for family_path in families:
        family_name = family_path.stem
        try:
            variants = family_variants(family_name, repo=REPO)
        except Exception as e:
            print(f"  {family_name}: skipped (no variants: {e})")
            continue
        
        if not variants:
            continue
        
        # Extract scheme from the script
        scheme = _extract_scheme_from_script(family_path)
        if scheme is None:
            print(f"  {family_name}: skipped (could not extract scheme)")
            continue
            
        # Import and run the family's build
        try:
            spec = __import__(f"roster.{family_name}", fromlist=["build"])
            build_fn = getattr(spec, "build", None)
            if build_fn is None:
                continue
        except Exception as e:
            print(f"  {family_name}: import failed: {e}")
            continue
        
        from vehicle_export import Vehicle, materials
        
        for variant in variants:
            vid = variant["id"]
            try:
                reset()
                # Get scheme for this variant
                own_scheme = scheme[vid] if isinstance(scheme, dict) else scheme
                mats = materials(own_scheme)
                v = Vehicle(variant, mats, own_scheme)
                build_fn(variant, v)
                
                result = lint_current_scene(vid)
                all_results[vid] = result
                
                if update_baseline and result.issues:
                    new_baseline[vid] = [i.to_dict() for i in result.issues]
                
                new_issues = filter_by_baseline(result, baseline)
                status = "PASS" if new_issues.passed else "FAIL"
                n_total = len(result.issues)
                n_new = len(new_issues.issues)
                print(f"  {vid}: {status} ({n_new} new, {n_total} total)")
                if new_issues.issues:
                    print(format_issues(new_issues.issues))
                    
            except Exception as e:
                failed_builds.append((vid, str(e)))
                print(f"  {vid}: build failed: {e}")
    
    if update_baseline:
        save_baseline(new_baseline)
        print(f"\nUpdated baseline: {BASELINE_PATH}")
        print(f"Total vehicles with issues: {len(new_baseline)}")
        total_issues = sum(len(v) for v in new_baseline.values())
        print(f"Total baseline issues: {total_issues}")
    
    if failed_builds:
        print(f"\nFailed to build {len(failed_builds)} variant(s)")
    
    return all_results


# ---------------------------------------------------------------- CLI

def main():
    """CLI entry point for standalone lint."""
    from parts import script_args
    args = script_args()
    
    if "--roster" in args:
        run_roster_lint()
    elif "--update-baseline" in args:
        run_roster_lint(update_baseline=True)
    elif any(a.startswith("--variant=") for a in args):
        vid = next(a.split("=", 1)[1] for a in args if a.startswith("--variant="))
        # Build and lint a single variant
        from catalog_frames import roster_variant
        from parts import reset
        variant = roster_variant(vid, repo=REPO)
        
        # Find and run the family's build
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
