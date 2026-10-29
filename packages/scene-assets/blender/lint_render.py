"""Uncommitted verification renderer for PR #22. Run via the asset CLI from a worktree.
args: --jobs=<json file> --out=<dir> [--scan-allowlist=<json out>]
jobs: [{"vid":..., "part":..., "tag":...}]"""
import sys, os, json, math, fnmatch
from pathlib import Path
REPO = Path.cwd().parent
BL = REPO / "packages/scene-assets/blender"
sys.path.insert(0, str(BL))
import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree
from bpy_extras.object_utils import world_to_camera_view

argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
A = dict(a[2:].split("=", 1) for a in argv if a.startswith("--") and "=" in a)
OUT = Path(A["out"]); OUT.mkdir(parents=True, exist_ok=True)

import vehicle_lint as VL
VL._SUPPRESS_EXPORT_LINT = True; VL._SUPPRESS_EXPORT = True
sys.modules.setdefault("vehicle_lint", VL)
from parts import reset
from catalog_frames import family_variants

def families():
    for p in sorted((BL / "roster").glob("*.py")):
        if p.name.startswith("_"): continue
        fam = VL._get_family_name_from_script(p) or p.stem
        try: vs = family_variants(fam, repo=REPO)
        except Exception:
            if fam != p.stem:
                try: vs = family_variants(p.stem, repo=REPO)
                except Exception: continue
            else: continue
        sch = VL._extract_scheme_from_script(p)
        if not vs or sch is None: continue
        yield p.stem, vs, sch

FAM = {}
for fam, vs, sch in families():
    for v in vs: FAM[v["id"]] = (fam, v, sch)

def build(vid):
    from vehicle_export import Vehicle, materials
    fam, variant, sch = FAM[vid]
    import vehicle_export as VE
    VE._export = lambda *a, **k: None
    spec = __import__(f"roster.{fam}", fromlist=["build"])
    reset()
    own = sch[vid] if isinstance(sch, dict) else sch
    v = Vehicle(variant, materials(own), own)
    spec.build(variant, v)
    bpy.context.view_layer.update()

def tier0():
    return [o for o in VL._mesh_objects_by_tier(0)]

def find(part):
    for o in tier0():
        if VL._base_part_name(o.name) == part: return o
    return None

def bvh(o):
    return VL._build_bvh(o)

def nearest(obj):
    s = VL._get_mesh_samples(obj, max_samples=200)
    best, bd = None, 1e9
    for o in tier0():
        if o == obj: continue
        b = bvh(o)
        if b is None: continue
        for p in s:
            r = b.find_nearest(p, bd)
            if r[0] is not None and r[3] < bd: bd, best = r[3], o
    return best, bd

def bounds(objs):
    lo = Vector((1e9,)*3); hi = Vector((-1e9,)*3)
    for o in objs:
        for c in o.bound_box:
            w = o.matrix_world @ Vector(c)
            lo = Vector(map(min, lo, w)); hi = Vector(map(max, hi, w))
    return lo, hi

def hide_lods():
    for o in bpy.data.objects:
        if o.type == "MESH":
            n = o.name; i = n.rfind("_LOD")
            t = int(n[i+4:]) if i >= 0 and n[i+4:].isdigit() else None
            o.hide_render = not (t is None or t == 0)

def ground():
    if "VerifyGround" in bpy.data.objects: return
    me = bpy.data.meshes.new("VerifyGround")
    s = 200
    me.from_pydata([(-s,-s,0),(s,-s,0),(s,s,0),(-s,s,0)], [], [(0,1,2,3)])
    ob = bpy.data.objects.new("VerifyGround", me); bpy.context.scene.collection.objects.link(ob)
    m = bpy.data.materials.new("VerifyGroundMat"); m.use_nodes = True
    m.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0.20, 0.17, 0.11, 1)
    m.diffuse_color = (0.35, 0.32, 0.25, 1)
    me.materials.append(m); ob.color = (0.35, 0.32, 0.25, 1)

def camera(loc, target, fov_y, w, h):
    sc = bpy.context.scene
    cam = bpy.data.objects.get("VerifyCam")
    if cam is None:
        cam = bpy.data.objects.new("VerifyCam", bpy.data.cameras.new("VerifyCam"))
        sc.collection.objects.link(cam)
    cam.data.sensor_fit = "VERTICAL"; cam.data.angle_y = fov_y
    cam.data.clip_start = 0.05; cam.data.clip_end = 2000
    cam.location = loc
    cam.rotation_euler = (target - loc).to_track_quat("-Z", "Y").to_euler()
    sc.camera = cam; sc.render.resolution_x = w; sc.render.resolution_y = h
    bpy.context.view_layer.update()
    return cam

def world():
    sc = bpy.context.scene
    if sc.world is None: sc.world = bpy.data.worlds.new("W")
    sc.world.use_nodes = True
    bg = sc.world.node_tree.nodes.get("Background")
    if bg: bg.inputs[0].default_value = (0.55, 0.65, 0.8, 1); bg.inputs[1].default_value = 1.0
    if "VerifySun" not in bpy.data.objects:
        ld = bpy.data.lights.new("VerifySun", "SUN"); ld.energy = 4.0; ld.angle = 0.05
        lo = bpy.data.objects.new("VerifySun", ld); sc.collection.objects.link(lo)
        lo.rotation_euler = (math.radians(40), 0, math.radians(30))

def render(path, engine):
    sc = bpy.context.scene
    sc.render.engine = engine
    sc.render.film_transparent = False
    sc.render.image_settings.file_format = "PNG"
    if engine == "BLENDER_WORKBENCH":
        sh = sc.display.shading
        sh.light = "STUDIO"; sh.color_type = "OBJECT"; sh.show_shadows = True
        sh.show_cavity = True; sh.show_object_outline = True
    else:
        try: sc.eevee.taa_render_samples = 32
        except Exception: pass
    sc.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)

def do_job(job):
    vid, part, tag = job["vid"], job["part"], job["tag"]
    build(vid); hide_lods(); ground(); world()
    bpy.data.objects["VerifyGround"].hide_render = False
    obj = find(part)
    if obj is None:
        return {**job, "error": "part not found"}
    nb, dist = nearest(obj)
    allm = [o for o in tier0()]
    vlo, vhi = bounds(allm); vc = (vlo + vhi) / 2
    plo, phi = bounds([obj]); pc = (plo + phi) / 2
    # outward horizontal direction from vehicle centre to part
    d = Vector((pc.x - vc.x, pc.y - vc.y, 0))
    if job.get("view_dir"): d = Vector(job["view_dir"])
    if d.length < 0.05: d = Vector((0.3, 1, 0))
    d.normalize()
    # yaw a bit off pure side so depth reads
    ang = math.radians(job.get("yaw_off", 30))
    d = Vector((d.x*math.cos(ang) - d.y*math.sin(ang), d.x*math.sin(ang) + d.y*math.cos(ang), 0))
    # Game view: closest zoom 25 m, pitch 0.22 rad, fov_y 0.8, 1920x1080, EEVEE, real materials
    pitch = 0.22
    gl = pc + (d * math.cos(pitch) + Vector((0,0,math.sin(pitch)))) * 25.0
    cam = camera(gl, pc, 0.8, 1920, 1080)
    gp = OUT / f"{tag}_game.png"
    if job.get("no_game"): gp = None
    eng = "BLENDER_EEVEE" if "BLENDER_EEVEE" in [e.identifier for e in bpy.types.RenderSettings.bl_rna.properties["engine"].enum_items] else "BLENDER_EEVEE_NEXT"
    for o in bpy.data.objects:
        if o.type == "MESH" and o.name != "VerifyGround": o.color = (0.30, 0.31, 0.22, 1)
    bpy.data.objects["VerifyGround"].color = (0.42, 0.38, 0.28, 1)
    if gp: render(gp, "BLENDER_WORKBENCH")
    px = world_to_camera_view(bpy.context.scene, cam, pc)
    for o in bpy.data.objects:
        if o.type == "MESH" and any(fnmatch.fnmatch(VL._base_part_name(o.name), h) for h in job.get("hide", [])):
            o.hide_render = True
    # Close-up: workbench, part red, nearest blue, rest grey
    for o in bpy.data.objects:
        if o.type == "MESH" and o.name != "VerifyGround": o.color = (0.62, 0.62, 0.60, 1)
    obj.color = (0.95, 0.15, 0.1, 1)
    if nb: nb.color = (0.15, 0.45, 0.95, 1)
    frame = [obj] + ([nb] if nb and (bounds([nb])[1]-bounds([nb])[0]).length < 2.5 else [])
    for o in tier0():
        if any(fnmatch.fnmatch(VL._base_part_name(o.name), f) for f in job.get("frame", [])): frame.append(o)
    flo, fhi = bounds(frame); fc = (flo + fhi) / 2
    size = max((fhi - flo).length, 0.3)
    iso = job.get("isolate", 0.6)
    for o in tier0():
        olo, ohi = bounds([o])
        gap = max(0, flo.x-ohi.x, olo.x-fhi.x, flo.y-ohi.y, olo.y-fhi.y, flo.z-ohi.z, olo.z-fhi.z)
        if gap > iso: o.hide_render = True
    if job.get("inboard"): d = -d
    el = math.radians(job.get("close_pitch", 20))
    cd = size * 1.6 / math.tan(math.radians(17.5)) / 2 + 0.3
    cl = fc + (d * math.cos(el) + Vector((0,0,math.sin(el)))) * cd
    camera(cl, fc, math.radians(35), 1280, 960)
    cp = OUT / f"{tag}_close.png"
    if job.get("hide_ground"): bpy.data.objects["VerifyGround"].hide_render = True
    render(cp, "BLENDER_WORKBENCH")
    return {**job, "nearest": nb.name if nb else None, "dist_cm": round(dist*100, 1),
            "game": str(gp) if gp else None, "close": str(cp), "game_px": [px.x * 1920, (1 - px.y) * 1080]}

def scan(patterns):
    worst = {}
    for vid in FAM:
        try:
            build(vid)
            r = VL.lint_current_scene(vid)
        except Exception as e:
            print("scan fail", vid, e); continue
        for iss in r.issues:
            for p in iss.parts:
                for pat in patterns:
                    if fnmatch.fnmatch(p.lower(), pat.lower()):
                        w = worst.setdefault(pat, {"count": 0, "vehicles": set()})
                        w["count"] += 1; w["vehicles"].add(vid)
                        if iss.distance_m and iss.distance_m > w.get("dist", -1):
                            w.update(dist=iss.distance_m, vid=vid, part=p)
    return worst

results = []
if "scan" in A:
    pats = [p["part_pattern"] for p in json.load(open(BL / "vehicle_lint_allowlist.json"))["patterns"]]
    w = scan(pats)
    out = {k: {**v, "vehicles": sorted(v["vehicles"])} for k, v in w.items()}
    for p in pats:
        if p not in out: out[p] = None
    json.dump(out, open(A["scan"], "w"), indent=1)
    print("SCAN DONE", len([v for v in out.values() if v]), "of", len(pats))
if "list" in A:
    for vid in A["list"].split(","):
        build(vid)
        names = sorted(VL._base_part_name(o.name) for o in tier0())
        import re
        print("LIST", vid, [n for n in names if re.search(A.get("grep","."), n)])
if "jobs" in A:
    for job in json.load(open(A["jobs"])):
        try:
            r = do_job(job)
        except Exception as e:
            import traceback; traceback.print_exc(); r = {**job, "error": str(e)}
        print("JOB", json.dumps(r)); results.append(r)
    json.dump(results, open(OUT / "results.json", "w"), indent=1)
