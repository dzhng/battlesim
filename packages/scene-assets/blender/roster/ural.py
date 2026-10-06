"""Ural-4320 6x6 cargo truck: long bonnet, rounded conventional cab, canvas-covered bed.

Rebuild through the pinned asset CLI; optional --preview=<scratch-dir> writes
candidate views after export. References and frozen fit frame are in the family
manifest. Stationary service is simulation state, not fictional deployment art.
"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from logistics import build

FAMILY = 'ural'
TYPE_ID = 'eastern_ural_4320_general_resupply'
L, W, H = (7.366, 2.5, 3.005)
AXLES = [('F', 2.48), ('M', -1.3), ('R', -2.54)]
CAB_STYLE = 'bonnet'

build(FAMILY, TYPE_ID, (L, W, H), AXLES, CAB_STYLE)
