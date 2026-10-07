"""MAN HX cargo truck, HX77 8x8 exterior: upright protected cab and canvas cargo bed.

Rebuild through the pinned asset CLI; optional --preview=<scratch-dir> writes
candidate views after export. Its frame is the unit type's in the resolved
catalog. Stationary service is simulation state, not fictional deployment art.
"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from logistics import build

FAMILY = 'man-hx'
AXLES = [('F', 3.91), ('F2', 2.29), ('M', -2.68), ('R', -4.05)]
CAB_STYLE = 'hx'

build(FAMILY, AXLES, CAB_STYLE)
