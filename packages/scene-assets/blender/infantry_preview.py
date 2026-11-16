"""A review render of an exported infantry look: the source and its hold
family's shared clips reimported, posed in the real exported clips (idle,
stand aim, kneel fire, prone pinned), each from the side and the front.
`roster/infantry_equipment.py --preview=<dir>` runs it.
"""
import os

import bpy
from mathutils import Vector

from common import REPO


def preview(path,hold,directory):
    """Reimport source and shared clip GLB; poses are the actual exported clips."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=path)
    body_arm=next(o for o in bpy.data.objects if o.type=='ARMATURE')
    clip=os.path.join(REPO,'assets/source/infantry/clips_'+hold['family']+'.glb')
    if open(clip,'rb').read(4)!=b'glTF':
        raise SystemExit('Preview requires acquired shared clip source: '+clip)
    before=set(bpy.data.objects);bpy.ops.import_scene.gltf(filepath=clip)
    actions={a.name:a for a in bpy.data.actions}
    for o in set(bpy.data.objects)-before:bpy.data.objects.remove(o,do_unlink=True)
    for o in bpy.data.objects:
        if o.type=='MESH':o.hide_render='_LOD' in o.name and not o.name.endswith('_LOD0')
    scn=bpy.context.scene;scn.render.engine='BLENDER_EEVEE';scn.render.resolution_x=800;scn.render.resolution_y=800
    scn.world=bpy.data.worlds.new('review_world');scn.world.use_nodes=True;scn.world.node_tree.nodes['Background'].inputs['Color'].default_value=(.32,.32,.32,1);scn.world.node_tree.nodes['Background'].inputs['Strength'].default_value=.8
    scn.view_settings.exposure=1.0
    bpy.ops.mesh.primitive_plane_add(size=100,location=(0,0,-.005))
    floor=bpy.context.object;floor.name='review_ground';floor_mat=bpy.data.materials.new('review_ground');floor_mat.diffuse_color=(.22,.24,.20,1);floor.data.materials.append(floor_mat)
    for loc in ((-3,-4,6),(4,3,4)):
        bpy.ops.object.light_add(type='AREA',location=loc);light=bpy.context.object;light.data.energy=450;light.data.size=4;light.rotation_euler=(Vector((0,0,1))-light.location).to_track_quat('-Z','Y').to_euler()
    bpy.ops.object.camera_add();cam=bpy.context.object;scn.camera=cam;cam.data.type='ORTHO'
    os.makedirs(directory,exist_ok=True)
    for pose in ('idle','stand_aim','kneel_fire','prone_pinned'):
        body_arm.animation_data_create();action=next(a for n,a in actions.items() if n.split('.')[0]==pose)
        body_arm.animation_data.action=action
        if action.slots:body_arm.animation_data.action_slot=action.slots[0]
        scn.frame_set(0);bpy.context.view_layer.update()
        dg=bpy.context.evaluated_depsgraph_get()
        points=[o.matrix_world@Vector(p) for o in bpy.data.objects if o.type=='MESH' and o.name.endswith('_LOD0') for p in o.evaluated_get(dg).bound_box]
        lo=Vector(tuple(min(p[i] for p in points) for i in range(3)));hi=Vector(tuple(max(p[i] for p in points) for i in range(3)));target=(lo+hi)/2
        cam.data.ortho_scale=max(hi.y-lo.y,hi.z-lo.z)*1.25
        for label,offset in [('side',(-4,-.1,.6)),('front',(-2.7,-4,1.15))]:
            cam.location=target+Vector(offset);cam.rotation_euler=(target-cam.location).to_track_quat('-Z','Y').to_euler();scn.render.filepath=os.path.join(directory,pose+'_'+label+'.png');bpy.ops.render.render(write_still=True)
