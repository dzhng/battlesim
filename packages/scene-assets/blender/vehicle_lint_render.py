"""Render close-up gap visualizations for vehicle lint issues.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint_render.py -- \\
        --variant=<id> --part=<part_name> --output=<path.png>

Renders a close-up of the named part with distance measurement annotations.
Uses Workbench render engine for fast, clear visualization of geometry gaps.
"""
import argparse
import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from catalog_frames import roster_variant
from vehicle_lint import _mesh_objects_by_tier, _mesh_bounds, _get_mesh_samples, _build_bvh, _base_part_name

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))


def setup_workbench_render():
    """Configure Workbench for clean geometry visualization."""
    scene = bpy.context.scene
    scene.render.engine = 'BLENDER_WORKBENCH'
    scene.display.shading.light = 'STUDIO'
    scene.display.shading.studio_light = 'studio.exr'
    scene.display.shading.color_type = 'OBJECT'
    scene.display.shading.show_shadows = True
    scene.render.resolution_x = 1920
    scene.render.resolution_y = 1080
    scene.render.film_transparent = True


def find_part_and_nearest(part_name):
    """Find the named part and its nearest neighbor."""
    part_obj = None
    all_meshes = list(_mesh_objects_by_tier(0))
    
    for obj in all_meshes:
        base = _base_part_name(obj.name)
        if base.lower() == part_name.lower() or part_name.lower() in base.lower():
            part_obj = obj
            break
    
    if part_obj is None:
        return None, None, None
    
    part_lo, part_hi = _mesh_bounds(part_obj)
    if part_lo is None:
        return part_obj, None, None
    
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
    
    return part_obj, nearest_obj, min_dist


def setup_camera_for_gap(part_obj, nearest_obj, distance_m):
    """Position camera to show the gap between parts."""
    part_lo, part_hi = _mesh_bounds(part_obj)
    near_lo, near_hi = _mesh_bounds(nearest_obj) if nearest_obj else (None, None)
    
    part_center = (part_lo + part_hi) / 2
    
    if near_lo is not None:
        near_center = (near_lo + near_hi) / 2
        focus = (part_center + near_center) / 2
    else:
        focus = part_center
    
    part_size = (part_hi - part_lo).length
    cam_distance = max(0.5, part_size * 2)
    
    cam_data = bpy.data.cameras.new(name='GapCamera')
    cam_data.type = 'PERSP'
    cam_data.lens = 50
    
    cam_obj = bpy.data.objects.new('GapCamera', cam_data)
    bpy.context.scene.collection.objects.link(cam_obj)
    
    direction = Vector((1, 0.5, 0.3)).normalized()
    cam_pos = focus + direction * cam_distance
    
    cam_obj.location = cam_pos
    direction_to_focus = focus - cam_pos
    cam_obj.rotation_euler = direction_to_focus.to_track_quat('-Z', 'Y').to_euler()
    
    bpy.context.scene.camera = cam_obj
    
    return focus


def highlight_parts(part_obj, nearest_obj):
    """Give distinct colors to the part and its nearest neighbor."""
    part_mat = bpy.data.materials.new(name="PartMaterial")
    part_mat.diffuse_color = (1, 0.2, 0.2, 1)
    
    if part_obj.data.materials:
        part_obj.data.materials[0] = part_mat
    else:
        part_obj.data.materials.append(part_mat)
    
    if nearest_obj:
        near_mat = bpy.data.materials.new(name="NearMaterial")
        near_mat.diffuse_color = (0.2, 0.5, 1, 1)
        if nearest_obj.data.materials:
            nearest_obj.data.materials[0] = near_mat
        else:
            nearest_obj.data.materials.append(near_mat)


def add_distance_annotation(focus, distance_m):
    """Add a text annotation showing the distance."""
    bpy.ops.object.text_add(location=(focus.x, focus.y, focus.z + 0.3))
    text_obj = bpy.context.active_object
    text_obj.data.body = f"Gap: {distance_m*100:.1f}cm ({distance_m*1000:.0f}mm)"
    text_obj.data.size = 0.1
    text_obj.data.align_x = 'CENTER'
    text_obj.rotation_euler = bpy.context.scene.camera.rotation_euler


def render_gap(variant_id, part_name, output_path):
    """Main entry: load variant, find gap, render close-up."""
    variant = roster_variant(variant_id, repo=REPO)
    
    setup_workbench_render()
    
    bpy.ops.wm.read_homefile(use_empty=True)
    
    exec(compile(open(variant["script"]).read(), variant["script"], "exec"),
         {"__name__": "__main__", "__file__": variant["script"]})
    
    part_obj, nearest_obj, distance_m = find_part_and_nearest(part_name)
    
    if part_obj is None:
        print(f"Part '{part_name}' not found in {variant_id}")
        return False
    
    print(f"Part: {part_obj.name}")
    if nearest_obj:
        print(f"Nearest: {nearest_obj.name}")
    print(f"Distance: {distance_m*100:.1f}cm" if distance_m else "Distance: N/A")
    
    highlight_parts(part_obj, nearest_obj)
    
    focus = setup_camera_for_gap(part_obj, nearest_obj, distance_m)
    
    if distance_m is not None:
        add_distance_annotation(focus, distance_m)
    
    bpy.context.scene.render.filepath = output_path
    bpy.ops.render.render(write_still=True)
    
    print(f"Rendered to {output_path}")
    return True


def main():
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1:]
    else:
        argv = []
    
    parser = argparse.ArgumentParser(description="Render gap close-ups")
    parser.add_argument("--variant", required=True, help="Vehicle variant ID")
    parser.add_argument("--part", required=True, help="Part name to focus on")
    parser.add_argument("--output", required=True, help="Output PNG path")
    
    args = parser.parse_args(argv)
    render_gap(args.variant, args.part, args.output)


if __name__ == "__main__":
    main()
