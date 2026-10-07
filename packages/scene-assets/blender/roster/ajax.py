"""Named ajax exports; frames from the resolved catalog."""
import os,sys
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from europe_carriers import build
build("ajax")
