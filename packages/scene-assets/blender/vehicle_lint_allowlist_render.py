"""Render all allowlist patterns to verify they are intentional gaps.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint_allowlist_render.py -- \
        --output-dir=/opt/cursor/artifacts/vehicle-lint-renders

For each allowlist pattern, finds one representative vehicle part and renders a close-up.
Generates a report of all patterns with their render paths.
"""
import argparse
import fnmatch
import json
import math
import os
import sys
from collections import defaultdict
from pathlib import Path

import bpy
from mathutils import Vector, Euler

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from catalog_frames import roster_variant, all_variants
from vehicle_lint import (
    _mesh_objects_by_tier, _mesh_bounds, _get_mesh_samples,
    _build_bvh, _base_part_name, lint_current_scene, FLOATING_PART_THRESHOLD
)

REPO = Path(__file__).resolve().parents[3]
ALLOWLIST_PATH = Path(__file__).parent / "vehicle_lint_allowlist.json"
ARTIFACTS_DIR = Path("/opt/cursor/artifacts/vehicle-lint-renders")


def setup_workbench_render(resolution=(1280, 720)):
    scene = bpy.context.scene
    scene.render.engine = 'BLENDER_WORKBENCH'
    scene.display.shading.light = 'STUDIO'
    scene.display.shading.studio_light = 'studio.exr'
    scene.display.shading.color_type = 'OBJECT'
    scene.display.shading.show_shadows = True
    scene.render.resolution_x = resolution[0]
    scene.render.resolution_y = resolution[1]
    scene.render.film_transparent = True

    if "RenderLight" not in bpy.data.lights:
        light_data = bpy.data.lights.new(name="RenderLight", type='SUN')
        light_data.energy = 3.0
        light_obj = bpy.data.objects.new(name="RenderLight", object_data=light_data)
        bpy.context.scene.collection.objects.link(light_obj)
        light_obj.location = (5, 5, 10)
        light_obj.rotation_euler = Euler((0.5, 0.3, 0.2))


def find_object_by_name(part_name):
    for obj in _mesh_objects_by_tier(0):
        if _base_part_name(obj.name).lower() == part_name.lower():
            return obj
        if part_name.lower() in obj.name.lower():
            return obj
    return None


def find_nearest_part(part_obj):
    all_meshes = list(_mesh_objects_by_tier(0))
    part_samples = _get_mesh_samples(part_obj, max_samples=60)

    nearest_obj = None
    min_dist = float('inf')

    for obj in all_meshes:
        if obj == part_obj:
            continue
        bvh = _build_bvh(obj)
        if bvh is None:
            continue
        for sample in part_samples:
            loc, normal, idx, dist = bvh.find_nearest(sample, min_dist)
            if loc is not None and dist < min_dist:
                min_dist = dist
                nearest_obj = obj

    return nearest_obj, min_dist


def highlight_part(obj, color):
    mat = bpy.data.materials.new(name=f"Highlight_{obj.name}")
    mat.diffuse_color = color
    mat.use_nodes = False
    orig_mats = list(obj.data.materials)
    obj.data.materials.clear()
    obj.data.materials.append(mat)
    return orig_mats


def restore_materials(obj, orig_mats):
    obj.data.materials.clear()
    for mat in orig_mats:
        obj.data.materials.append(mat)


def setup_camera_for_part(part_obj, nearest_obj=None):
    part_lo, part_hi = _mesh_bounds(part_obj)
    if part_lo is None:
        return None

    part_center = (part_lo + part_hi) / 2
    part_size = (part_hi - part_lo).length

    if nearest_obj:
        near_lo, near_hi = _mesh_bounds(nearest_obj)
        if near_lo is not None:
            near_center = (near_lo + near_hi) / 2
            focus = (part_center + near_center) / 2
            combined_size = max(part_size, (near_hi - near_lo).length)
        else:
            focus = part_center
            combined_size = part_size
    else:
        focus = part_center
        combined_size = part_size

    cam_distance = max(0.8, combined_size * 2.5)

    if "GapCamera" in bpy.data.cameras:
        cam_data = bpy.data.cameras["GapCamera"]
        cam_obj = bpy.data.objects.get("GapCamera")
    else:
        cam_data = bpy.data.cameras.new(name='GapCamera')
        cam_obj = bpy.data.objects.new('GapCamera', cam_data)
        bpy.context.scene.collection.objects.link(cam_obj)

    cam_data.type = 'PERSP'
    cam_data.lens = 50

    direction = Vector((1, 0.7, 0.5)).normalized()
    cam_pos = focus + direction * cam_distance

    cam_obj.location = cam_pos
    direction_to_focus = focus - cam_pos
    cam_obj.rotation_euler = direction_to_focus.to_track_quat('-Z', 'Y').to_euler()

    bpy.context.scene.camera = cam_obj
    return focus


def render_part_closeup(part_name, output_path, variant_id=None):
    part_obj = find_object_by_name(part_name)
    if part_obj is None:
        return False, None

    nearest_obj, distance = find_nearest_part(part_obj)

    part_orig = highlight_part(part_obj, (1, 0.2, 0.2, 1))
    near_orig = None
    if nearest_obj:
        near_orig = highlight_part(nearest_obj, (0.2, 0.5, 1, 1))

    setup_camera_for_part(part_obj, nearest_obj)

    bpy.context.scene.render.filepath = str(output_path)
    bpy.ops.render.render(write_still=True)

    restore_materials(part_obj, part_orig)
    if nearest_obj and near_orig:
        restore_materials(nearest_obj, near_orig)

    return True, distance


def load_and_build_variant(variant_id):
    variant = roster_variant(variant_id, repo=REPO)

    bpy.ops.wm.read_homefile(use_empty=True)

    script_path = variant["script"]
    exec(compile(open(script_path).read(), script_path, "exec"),
         {"__name__": "__main__", "__file__": script_path})

    return variant


def load_allowlist():
    with open(ALLOWLIST_PATH) as f:
        data = json.load(f)
    return data.get("patterns", [])


def matches_pattern(part_name, pattern):
    return fnmatch.fnmatch(part_name.lower(), pattern.lower())


GROUND_VEHICLES = [
    "stryker", "lav", "centauro", "acv", "humvee", "hemtt", "ural",
    "man-hx", "abrams", "leopard", "challenger", "challenger_3", "leclerc",
    "t72", "t80", "t90", "t14", "type99", "type15", "cv90", "cv90120",
    "bradley", "bmp", "btr", "boxer", "vbci", "puma", "ajax", "fennek",
    "vbl", "zbl08", "tigr", "brm", "t15", "m10", "kf51", "m1e3", "armata"
]


def get_all_ground_variants():
    variants = []
    try:
        for vid in all_variants(repo=REPO):
            base = vid.split("_")[0] if "_" in vid else vid
            if base in GROUND_VEHICLES or vid in GROUND_VEHICLES:
                variants.append(vid)
    except Exception:
        variants = GROUND_VEHICLES
    return variants


def find_matching_parts(allowlist_patterns):
    all_parts = defaultdict(list)

    for obj in _mesh_objects_by_tier(0):
        base_name = _base_part_name(obj.name)
        for pat_info in allowlist_patterns:
            pattern = pat_info["part_pattern"]
            if matches_pattern(base_name, pattern):
                all_parts[pattern].append(base_name)

    return all_parts


def render_before_after(variant_id, part_name, output_dir, label_prefix):
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    safe_name = part_name.replace("/", "_").replace("\\", "_")
    output_path = output_dir / f"{label_prefix}_{variant_id}_{safe_name}.png"

    success, distance = render_part_closeup(part_name, output_path, variant_id)
    if success:
        return str(output_path), distance
    return None, None


def main():
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1:]
    else:
        argv = []

    parser = argparse.ArgumentParser(description="Render allowlist pattern close-ups")
    parser.add_argument("--output-dir", default=str(ARTIFACTS_DIR), help="Output directory")
    parser.add_argument("--variant", help="Single vehicle variant to test")
    parser.add_argument("--pattern", help="Single pattern to render")
    parser.add_argument("--before-after", action="store_true", help="Render before/after for geometry fixes")
    parser.add_argument("--report", default=str(ARTIFACTS_DIR / "render_report.json"), help="Report output path")

    args = parser.parse_args(argv)
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    allowlist = load_allowlist()
    pattern_renders = {}
    all_renders = []

    if args.variant:
        variants_to_check = [args.variant]
    else:
        variants_to_check = get_all_ground_variants()

    patterns_to_check = allowlist
    if args.pattern:
        patterns_to_check = [p for p in allowlist if p["part_pattern"] == args.pattern]

    for variant_id in variants_to_check:
        print(f"\n=== Processing {variant_id} ===")
        try:
            load_and_build_variant(variant_id)
            setup_workbench_render()
        except Exception as e:
            print(f"  Failed to load {variant_id}: {e}")
            continue

        matches = find_matching_parts(patterns_to_check)

        for pat_info in patterns_to_check:
            pattern = pat_info["part_pattern"]
            if pattern in pattern_renders:
                continue

            if pattern not in matches or not matches[pattern]:
                continue

            part_name = matches[pattern][0]
            safe_pattern = pattern.replace("*", "STAR").replace("/", "_")
            output_path = output_dir / f"allowlist_{safe_pattern}_{variant_id}.png"

            success, distance = render_part_closeup(part_name, output_path, variant_id)
            if success:
                print(f"  Rendered {pattern} -> {part_name} ({distance*100:.1f}cm)")
                pattern_renders[pattern] = {
                    "pattern": pattern,
                    "reason": pat_info["reason"],
                    "vehicle": variant_id,
                    "part": part_name,
                    "distance_cm": round(distance * 100, 1) if distance else None,
                    "render_path": str(output_path)
                }
                all_renders.append({
                    "pattern": pattern,
                    "vehicle": variant_id,
                    "part": part_name,
                    "distance_cm": round(distance * 100, 1) if distance else None,
                    "path": str(output_path)
                })

    report = {
        "patterns_rendered": len(pattern_renders),
        "total_patterns": len(allowlist),
        "renders": all_renders,
        "pattern_details": list(pattern_renders.values())
    }

    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)

    print(f"\n=== Report ===")
    print(f"Rendered {len(pattern_renders)}/{len(allowlist)} patterns")
    print(f"Report saved to {report_path}")
    print(f"Renders saved to {output_dir}")


if __name__ == "__main__":
    main()
