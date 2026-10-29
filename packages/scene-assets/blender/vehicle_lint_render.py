"""Render close-up gap visualizations for vehicle lint issues.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint_render.py -- \\
        --variant=<id> --part=<part_name> --output=<path.png>
    
Or render all floating_part issues for a vehicle:
    bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint_render.py -- \\
        --variant=<id> --all-floating --output-dir=<dir>

Uses Workbench render engine for fast, clear visualization of geometry gaps.
"""
import argparse
import json
import math
import os
import sys
from pathlib import Path

import bpy
from mathutils import Vector, Euler

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from catalog_frames import roster_variant
from vehicle_lint import (
    _mesh_objects_by_tier, _mesh_bounds, _get_mesh_samples, 
    _build_bvh, _base_part_name, lint_current_scene, FLOATING_PART_THRESHOLD
)

REPO = Path(__file__).resolve().parents[3]
ARTIFACTS_DIR = Path("/opt/cursor/artifacts/vehicle-lint-renders")


def setup_workbench_render(resolution=(1280, 720)):
    """Configure Workbench for clean geometry visualization."""
    scene = bpy.context.scene
    scene.render.engine = 'BLENDER_WORKBENCH'
    scene.display.shading.light = 'STUDIO'
    scene.display.shading.studio_light = 'studio.exr'
    scene.display.shading.color_type = 'OBJECT'
    scene.display.shading.show_shadows = True
    scene.render.resolution_x = resolution[0]
    scene.render.resolution_y = resolution[1]
    scene.render.film_transparent = True
    
    # Add a simple light
    if "RenderLight" not in bpy.data.lights:
        light_data = bpy.data.lights.new(name="RenderLight", type='SUN')
        light_data.energy = 3.0
        light_obj = bpy.data.objects.new(name="RenderLight", object_data=light_data)
        bpy.context.scene.collection.objects.link(light_obj)
        light_obj.location = (5, 5, 10)
        light_obj.rotation_euler = Euler((0.5, 0.3, 0.2))


def find_object_by_part_name(part_name):
    """Find mesh object matching the part name."""
    for obj in _mesh_objects_by_tier(0):
        base = _base_part_name(obj.name)
        if base.lower() == part_name.lower() or part_name.lower() in obj.name.lower():
            return obj
    return None


def find_nearest_part(part_obj):
    """Find the nearest part to the given object and the distance."""
    all_meshes = list(_mesh_objects_by_tier(0))
    part_samples = _get_mesh_samples(part_obj, max_samples=60)
    
    nearest_obj = None
    min_dist = float('inf')
    nearest_point_part = None
    nearest_point_other = None
    
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
                nearest_point_part = sample
                nearest_point_other = loc
    
    return nearest_obj, min_dist, nearest_point_part, nearest_point_other


def highlight_part(obj, color):
    """Give an object a distinct solid color for visualization."""
    mat = bpy.data.materials.new(name=f"Highlight_{obj.name}")
    mat.diffuse_color = color
    mat.use_nodes = False
    
    # Store original materials
    orig_mats = list(obj.data.materials)
    obj.data.materials.clear()
    obj.data.materials.append(mat)
    return orig_mats


def restore_materials(obj, orig_mats):
    """Restore original materials."""
    obj.data.materials.clear()
    for mat in orig_mats:
        obj.data.materials.append(mat)


def setup_camera_for_part(part_obj, nearest_obj=None):
    """Position camera to show the part and its nearest neighbor."""
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
    
    # Camera distance based on part size
    cam_distance = max(0.8, combined_size * 2.5)
    
    # Create or reuse camera
    if "GapCamera" in bpy.data.cameras:
        cam_data = bpy.data.cameras["GapCamera"]
        cam_obj = bpy.data.objects.get("GapCamera")
    else:
        cam_data = bpy.data.cameras.new(name='GapCamera')
        cam_obj = bpy.data.objects.new('GapCamera', cam_data)
        bpy.context.scene.collection.objects.link(cam_obj)
    
    cam_data.type = 'PERSP'
    cam_data.lens = 50
    
    # Position camera at 45 degree angle
    direction = Vector((1, 0.7, 0.5)).normalized()
    cam_pos = focus + direction * cam_distance
    
    cam_obj.location = cam_pos
    direction_to_focus = focus - cam_pos
    cam_obj.rotation_euler = direction_to_focus.to_track_quat('-Z', 'Y').to_euler()
    
    bpy.context.scene.camera = cam_obj
    return focus


def add_gap_annotation(text, location):
    """Add a text annotation at the given location."""
    bpy.ops.object.text_add(location=location)
    text_obj = bpy.context.active_object
    text_obj.data.body = text
    text_obj.data.size = 0.05
    text_obj.data.align_x = 'CENTER'
    text_obj.name = "GapAnnotation"
    
    # Make text face camera
    if bpy.context.scene.camera:
        text_obj.rotation_euler = bpy.context.scene.camera.rotation_euler
    
    return text_obj


def render_part_closeup(part_name, output_path, label=None):
    """Render a close-up of a specific part showing its gap."""
    part_obj = find_object_by_part_name(part_name)
    if part_obj is None:
        print(f"Part '{part_name}' not found")
        return False
    
    nearest_obj, distance, pt_part, pt_other = find_nearest_part(part_obj)
    
    # Highlight the parts
    part_orig = highlight_part(part_obj, (1, 0.2, 0.2, 1))  # Red for flagged part
    near_orig = None
    if nearest_obj:
        near_orig = highlight_part(nearest_obj, (0.2, 0.5, 1, 1))  # Blue for nearest
    
    # Setup camera
    focus = setup_camera_for_part(part_obj, nearest_obj)
    
    # Add annotation
    text_obj = None
    if focus and distance is not None:
        gap_text = f"{part_name}\n{distance*100:.1f}cm gap"
        if label:
            gap_text = f"{label}\n{gap_text}"
        text_obj = add_gap_annotation(gap_text, focus + Vector((0, 0, 0.2)))
    
    # Render
    bpy.context.scene.render.filepath = str(output_path)
    bpy.ops.render.render(write_still=True)
    
    # Cleanup
    restore_materials(part_obj, part_orig)
    if nearest_obj and near_orig:
        restore_materials(nearest_obj, near_orig)
    if text_obj:
        bpy.data.objects.remove(text_obj, do_unlink=True)
    
    print(f"Rendered {part_name} to {output_path}")
    return True


def render_all_floating_parts(variant_id, output_dir):
    """Render all floating_part issues for a vehicle."""
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Run lint to get issues
    result = lint_current_scene(variant_id)
    
    floating_issues = [i for i in result.issues if i.check == "floating_part"]
    print(f"Found {len(floating_issues)} floating_part issues")
    
    rendered = []
    for issue in floating_issues:
        for part in issue.parts:
            safe_name = part.replace("/", "_").replace("\\", "_")
            output_path = output_dir / f"{variant_id}_{safe_name}.png"
            if render_part_closeup(part, output_path):
                rendered.append((part, issue.distance_m, str(output_path)))
    
    return rendered


def load_and_build_variant(variant_id):
    """Load and build a vehicle variant."""
    variant = roster_variant(variant_id, repo=REPO)
    
    # Clear scene
    bpy.ops.wm.read_homefile(use_empty=True)
    
    # Run the variant script
    script_path = variant["script"]
    exec(compile(open(script_path).read(), script_path, "exec"),
         {"__name__": "__main__", "__file__": script_path})
    
    return variant


def main():
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1:]
    else:
        argv = []
    
    parser = argparse.ArgumentParser(description="Render gap close-ups")
    parser.add_argument("--variant", required=True, help="Vehicle variant ID")
    parser.add_argument("--part", help="Single part name to render")
    parser.add_argument("--output", help="Output PNG path for single part")
    parser.add_argument("--all-floating", action="store_true", help="Render all floating_part issues")
    parser.add_argument("--output-dir", help="Output directory for all renders")
    
    args = parser.parse_args(argv)
    
    # Load variant
    load_and_build_variant(args.variant)
    setup_workbench_render()
    
    if args.all_floating:
        output_dir = args.output_dir or str(ARTIFACTS_DIR / args.variant)
        rendered = render_all_floating_parts(args.variant, output_dir)
        print(f"Rendered {len(rendered)} parts")
        for part, dist, path in rendered:
            print(f"  {part}: {dist*100:.1f}cm -> {path}")
    elif args.part:
        output = args.output or str(ARTIFACTS_DIR / f"{args.variant}_{args.part}.png")
        Path(output).parent.mkdir(parents=True, exist_ok=True)
        render_part_closeup(args.part, output)
    else:
        print("Specify --part <name> or --all-floating")


if __name__ == "__main__":
    main()
