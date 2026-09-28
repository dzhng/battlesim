import bpy, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import *

DL = os.path.join(HERE, "dl")
UAL = os.path.join(DL, "ual/Universal Animation Library[Standard]/Unreal-Godot/UAL1_Standard.glb")
UBC = os.path.join(DL, "ubc/Universal Base Characters[Standard]/Base Characters/Godot - UE/Superhero_Male_FullBody.gltf")

reset()
bpy.ops.import_scene.gltf(filepath=UBC)
arm = [o for o in bpy.data.objects if o.type == "ARMATURE"][0]
arm.name = "soldier_rig"
before = set(bpy.data.objects)
bpy.ops.import_scene.gltf(filepath=UAL)
for o in set(bpy.data.objects) - before:
    bpy.data.objects.remove(o, do_unlink=True)
for a in bpy.data.actions:
    a.use_fake_user = True
print("ACTIONS", len(bpy.data.actions))
for o in bpy.data.objects:
    print("OBJ", o.name, o.type, o.parent.name if o.parent else None, [m.type for m in o.modifiers])
setup_render((480, 640), samples=24)
light_rig()
ground()
cam = camera(lens=60)
arm.animation_data_create()
for i, (clip, fr) in enumerate([("A_TPose", 0), ("Idle_Loop", 10), ("Walk_Loop", 8), ("Crouch_Idle_Loop", 10)]):
    act = bpy.data.actions[clip]
    arm.animation_data.action = act
    if act.slots:
        arm.animation_data.action_slot = act.slots[0]
    bpy.context.scene.frame_set(fr)
    aim(cam, (0, 0, 0.9), -60, 10, 4.2)
    render_to(os.path.join(OUT, f"look_{i}.png"))
