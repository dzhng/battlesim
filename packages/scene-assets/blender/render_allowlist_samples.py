"""Render representative samples for allowlist patterns.

Run with: blender -b --factory-startup --threads 1 --python render_allowlist_samples.py
"""
import json
import math
import os
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import bpy
from mathutils import Vector, Euler

REPO = Path(__file__).resolve().parents[3]
ALLOWLIST_PATH = Path(__file__).parent / "vehicle_lint_allowlist.json"
OUTPUT_DIR = Path("/opt/cursor/artifacts/vehicle-lint-renders")

SAMPLE_VEHICLES = [
    ("stryker", "roster/stryker.py"),
    ("abrams", "roster/abrams.py"),
    ("lav", "roster/lav.py"),
    ("hemtt", "roster/hemtt.py"),
]


def setup_render():
    scene = bpy.context.scene
    scene.render.engine = 'BLENDER_WORKBENCH'
    scene.display.shading.light = 'STUDIO'
    scene.display.shading.studio_light = 'studio.exr'
    scene.display.shading.color_type = 'OBJECT'
    scene.display.shading.show_shadows = True
    scene.render.resolution_x = 800
    scene.render.resolution_y = 600
    scene.render.film_transparent = True

    if "RenderLight" not in bpy.data.lights:
        light_data = bpy.data.lights.new(name="RenderLight", type='SUN')
        light_data.energy = 3.0
        light_obj = bpy.data.objects.new(name="RenderLight", object_data=light_data)
        bpy.context.scene.collection.objects.link(light_obj)
        light_obj.location = (5, 5, 10)
        light_obj.rotation_euler = Euler((0.5, 0.3, 0.2))


def load_vehicle(script_path):
    bpy.ops.wm.read_homefile(use_empty=True)
    full_path = REPO / "packages" / "scene-assets" / "blender" / script_path
    exec(compile(open(full_path).read(), str(full_path), "exec"),
         {"__name__": "__main__", "__file__": str(full_path)})


def find_objects_by_pattern(pattern):
    import fnmatch
    matches = []
    for obj in bpy.data.objects:
        if obj.type == 'MESH' and fnmatch.fnmatch(obj.name.lower(), pattern.lower()):
            matches.append(obj)
    return matches


def get_bounds(obj):
    if obj.type != 'MESH' or not obj.data.vertices:
        return None, None
    verts = [obj.matrix_world @ v.co for v in obj.data.vertices]
    lo = Vector((min(v.x for v in verts), min(v.y for v in verts), min(v.z for v in verts)))
    hi = Vector((max(v.x for v in verts), max(v.y for v in verts), max(v.z for v in verts)))
    return lo, hi


def setup_camera(focus, size):
    cam_distance = max(1.0, size * 3)
    
    if "GapCamera" not in bpy.data.cameras:
        cam_data = bpy.data.cameras.new(name='GapCamera')
        cam_obj = bpy.data.objects.new('GapCamera', cam_data)
        bpy.context.scene.collection.objects.link(cam_obj)
    else:
        cam_obj = bpy.data.objects.get("GapCamera")
    
    cam_obj.data.type = 'PERSP'
    cam_obj.data.lens = 50
    
    direction = Vector((1, 0.7, 0.5)).normalized()
    cam_pos = focus + direction * cam_distance
    cam_obj.location = cam_pos
    direction_to_focus = focus - cam_pos
    cam_obj.rotation_euler = direction_to_focus.to_track_quat('-Z', 'Y').to_euler()
    
    bpy.context.scene.camera = cam_obj


def highlight(obj, color):
    mat = bpy.data.materials.new(name=f"Highlight_{obj.name}")
    mat.diffuse_color = color
    mat.use_nodes = False
    obj.data.materials.clear()
    obj.data.materials.append(mat)


def render_pattern(pattern, reason, output_path):
    matches = find_objects_by_pattern(pattern)
    if not matches:
        return False
    
    obj = matches[0]
    lo, hi = get_bounds(obj)
    if lo is None:
        return False
    
    center = (lo + hi) / 2
    size = (hi - lo).length
    
    highlight(obj, (1, 0.2, 0.2, 1))
    setup_camera(center, size)
    
    bpy.context.scene.render.filepath = str(output_path)
    bpy.ops.render.render(write_still=True)
    
    print(f"  Rendered: {pattern} -> {obj.name}")
    return True


def main():
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    
    with open(ALLOWLIST_PATH) as f:
        allowlist = json.load(f)
    
    patterns = allowlist.get("patterns", [])
    rendered = []
    
    for vehicle_name, script_path in SAMPLE_VEHICLES:
        print(f"\n=== Loading {vehicle_name} ===")
        try:
            load_vehicle(script_path)
            setup_render()
        except Exception as e:
            print(f"  Failed to load: {e}")
            continue
        
        for pat_info in patterns:
            pattern = pat_info["part_pattern"]
            reason = pat_info["reason"]
            
            safe_pattern = pattern.replace("*", "_star_").replace("/", "_")
            output_path = OUTPUT_DIR / f"allowlist_{safe_pattern}_{vehicle_name}.png"
            
            if output_path.exists():
                continue
            
            if render_pattern(pattern, reason, output_path):
                rendered.append({
                    "pattern": pattern,
                    "reason": reason,
                    "vehicle": vehicle_name,
                    "path": str(output_path)
                })
    
    report = {
        "total_patterns": len(patterns),
        "rendered": len(rendered),
        "renders": rendered
    }
    
    report_path = OUTPUT_DIR / "allowlist_render_report.json"
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)
    
    print(f"\n=== Summary ===")
    print(f"Rendered {len(rendered)} samples")
    print(f"Report: {report_path}")


if __name__ == "__main__":
    main()
