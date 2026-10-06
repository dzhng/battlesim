"""T-90M Proryv: welded angular turret with deep bustle, Relikt and remote HMG."""
import sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
from eastern_tanks import *


def build(variant,frame,hull,turret,m):
    chassis(frame,hull,m,modern=True);slats(hull,frame,m)
    bottom=[(-2.18,-1.19),(-.50,-1.42),(1.51,-1.05),(1.79,-.43),(1.79,.43),(1.51,1.05),(-.50,1.42),(-2.18,1.19)]
    top=[(-2.10,-1.10),(-.50,-1.28),(1.32,-.93),(1.49,-.41),(1.49,.41),(1.32,.93),(-.50,1.28),(-2.10,1.10)]
    loft('t90m_welded_turret',[(.015,bottom),(.60,top)],mat=m['armor'],parent=turret,bevel=.018)
    box('t90m_armored_bustle',(.86,2.32,.50),(-1.98,0,.30),m['armor'],turret,bevel=.02)
    for side in (-1,1):
        lower=[(.35,side*.43),(1.87,side*.53),(1.17,side*1.54),(-.32,side*1.47)]
        upper=[(.25,side*.45),(1.47,side*.53),(1.01,side*1.40),(-.32,side*1.36)]
        if side<0:lower=lower[::-1];upper=upper[::-1]
        loft(f't90m_relikt_cheek_{side}',[(.11,lower),(.51,upper)],mat=m['armor'],parent=turret,bevel=.012)
        for k in range(4):box(f't90m_cheek_seam_{side}_{k}',(.024,.39,.02),(-.11+k*.34,side*1.17,.525),m['dark'],turret,lods=(0,1))
        box(f't90m_bustle_side_bin_{side}',(.87,.18,.40),(-1.68,side*1.25,.32),m['armor'],turret,bevel=.02,lods=(0,1,2))
    roof(turret,m,modern=True)
    # Rear bustle cage is part of the turret and yaws with it.
    for k in range(7):box(f't90m_bustle_slat_{k}',(.023,2.42,.023),(-2.45,0,.12+k*.065),m['steel'],turret,lods=(0,1))
    for side in (-1,1):box(f't90m_bustle_frame_{side}',(.035,.035,.47),(-2.45,side*1.23,.31),m['steel'],turret,lods=(0,1,2))


export_family('t90',build)
