"""M1A2 SEP v2/v3, with explicit Trophy variants. Run in Blender with --preview=DIR.
Seven-wheel suspension, low glacis, broad faceted turret/bustle, short L44 envelope.
"""
import sys, math
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
from armor import *


def build(variant,frame,hull,turret,m):
    L,W,H=frame['body_dimensions_m'];p=frame['mounts'][0]['pivot_m'][2]
    prism('abrams_upper_hull',[(-L/2+.05,.86),(L/2-.25,.83),(L/2,1.01),(2.43,1.41),(-3.72,1.43),(-L/2,1.22)],W-.08,mat=m['armor'],parent=hull,bevel=.035)
    prism('abrams_lower_hull',[(-3.70,.36),(3.10,.36),(3.80,.85),(-3.90,.91)],2.10,mat=m['dark'],parent=hull,bevel=.035)
    for side in (-1,1):
        for k in range(6):
            x=-3.39+k*1.15
            box(f'abrams_skirt_{side}_{k}',(1.12,.075,.48),(x+.48,side*(W/2-.04),1.10),m['armor'],hull,bevel=.015)
            box(f'skirt_seam_{side}_{k}',(.028,.085,.43),(x+.99,side*(W/2-.04),1.10),m['dark'],hull,lods=(0,1))
        box(f'front_fender_{side}',(.65,.63,.055),(3.47,side*1.45,1.14),m['armor'],hull,rot=(0,.22,0),bevel=.015)
        box(f'light_guard_{side}',(.18,.24,.17),(3.69,side*1.30,1.16),m['armor'],hull,bevel=.02,lods=(0,1,2))
        cyl(f'headlamp_{side}',.07,.035,(3.79,side*1.30,1.17),'X',m['lamp'],hull,seg=16,lods=(0,1))
    # Broad angled cheeks, narrow mantlet slot, squared rear bustle, not Leopard wedges.
    bottom=[(-2.35,-1.53),(.65,-1.72),(2.18,-.62),(2.18,.62),(.65,1.72),(-2.35,1.53)]
    top=[(-2.24,-1.42),(.47,-1.52),(1.93,-.53),(1.93,.53),(.47,1.52),(-2.24,1.42)]
    loft('abrams_turret',[(.02,bottom),(.65,top)],mat=m['armor'],parent=turret,bevel=.025)
    # Turret bustle basket frame makes Abrams distinctive in rear and top silhouettes.
    box('bustle_bin',(1.06,2.68,.38),(-2.22,0,.40),m['armor'],turret,bevel=.025)
    for side in (-1,1):
        for k in range(4):box(f'bustle_rail_{side}_{k}',(.065,.055,.50),(-2.68+k*.34,side*1.44,.46),m['steel'],turret,lods=(0,1,2))
        box(f'bustle_toprail_{side}',(1.19,.06,.055),(-2.16,side*1.44,.72),m['steel'],turret,lods=(0,1,2))
        box(f'bustle_pack_{side}',(.52,.52,.36),(-2.26,side*.81,.77),m['canvas'],turret,bevel=.06,lods=(0,1))
        for j in range(6):
            cyl(f'smoke_tube_{side}_{j}',.045,.24,(-.62+(j%3)*.15,side*1.52,.44+(j//3)*.14),'X',m['dark'],turret,seg=12,rot=(0,-.25,side*.50),lods=(0,1,2))
    hatches(turret,.66,m)
    box('citv_base',(.43,.43,.26),(.67,.80,.77),m['armor'],turret,bevel=.025)
    box('citv_head',(.36,.35,.24),(.67,.80,.90),m['armor'],turret,bevel=.025)
    box('citv_optic',(.027,.23,.13),(.865,.80,.92),m['glass'],turret,lods=(0,1,2))
    box('gunners_primary_sight',(.44,.45,.21),(1.00,-.77,.70),m['armor'],turret,bevel=.02,lods=(0,1,2))
    box('gps_glass',(.025,.27,.10),(1.23,-.77,.71),m['glass'],turret,lods=(0,1))
    cyl('driver_hatch',.31,.05,(2.42,0,1.42),'Z',m['armor'],hull,seg=24,lods=(0,1,2))
    for k in range(3):box(f'driver_scope_{k}',(.10,.19,.06),(2.72,-.22+k*.22,1.34),m['glass'],hull,lods=(0,1))
    box('engine_deck_recess',(1.82,2.54,.025),(-2.85,0,1.44),m['black'],hull,lods=(0,1,2))
    for k in range(16):box(f'engine_deck_louvre_{k}',(.055,2.48,.026),(-3.66+k*.11,0,1.46),m['dark'],hull,lods=(0,1))
    box('turbine_exhaust',(.035,2.58,.46),(-3.97,0,1.08),m['black'],hull,lods=(0,1,2))
    for k in range(7):box(f'exhaust_grille_{k}',(.025,2.54,.025),(-3.992,0,.90+k*.057),m['dark'],hull,lods=(0,1))
    if 'SEP v3' in variant['name']:
        box('sepv3_aux_power_pack',(.68,.53,.43),(-3.16,1.11,1.49),m['armor'],hull,bevel=.045,lods=(0,1,2))
        for side in (-1,1):box(f'sepv3_turret_equipment_{side}',(.70,.16,.38),(-1.23,side*1.47,.47),m['armor'],turret,bevel=.02,lods=(0,1,2))
        box('sepv3_front_armor',(1.00,2.16,.13),(2.15,0,1.37),m['armor'],hull,rot=(0,.20,0),lods=(0,1,2))
    if 'Trophy' in variant['name']:trophy(turret,.52,W-.07,m)


export_family('abrams',build)
