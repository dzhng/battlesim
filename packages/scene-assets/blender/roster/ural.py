"""Ural-4320 6x6 cargo truck: long bonnet, rounded conventional cab, canvas-covered bed.

Rebuild through the pinned asset CLI; optional --preview=<scratch-dir> writes
candidate views after export. Its frame is the unit type's in the resolved
catalog. Stationary service is simulation state, not fictional deployment art.
"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from logistics import build

FAMILY = 'ural'
AXLES = [('F', 2.48), ('M', -1.3), ('R', -2.54)]
CAB_STYLE = 'bonnet'

build(FAMILY, AXLES, CAB_STYLE)
