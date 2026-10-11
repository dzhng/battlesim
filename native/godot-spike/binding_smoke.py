#!/usr/bin/env python3
"""Compare identical native/Godot inputs; usage: binding_smoke.py GODOT SCENARIO.

Build native_binding_probe and godot-binding first in CARGO_TARGET_DIR (debug).
Generated projects, HOME, publication records and logs live in ignored throwaway.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[2]
target = Path(os.environ.get("CARGO_TARGET_DIR", root / "throwaway/target")).resolve()
godot, scenario = map(lambda p: str(Path(p).resolve()), sys.argv[1:3])
(root / "throwaway").mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix="binding-parity-", dir=root / "throwaway") as tmp:
    scratch = Path(tmp)
    project = scratch / "project"
    project.mkdir()
    (project / "project.godot").write_text('[application]\nconfig/name="Binding parity"\n[rendering]\nrenderer/rendering_method="gl_compatibility"\n')
    for name in ["binding_smoke.gd", "binding_smoke.tscn"]:
        shutil.copy(root / "native/godot-spike" / name, project / name)
    addon = project / "addons/battle_binding"
    addon.mkdir(parents=True)
    shutil.copy(root / "native/godot-spike/addons/battle_binding/godot_binding.gdextension", addon)
    shutil.copy(target / "debug/libgodot_binding.dylib", addon)
    cache = project / ".godot"
    cache.mkdir()
    (cache / "extension_list.cfg").write_text('res://addons/battle_binding/godot_binding.gdextension\n')
    env = dict(os.environ, HOME=str(scratch / "home"))
    for side in ["blue", "red"]:
        evidence = scratch / side
        native = subprocess.run([str(target / "debug/examples/native_binding_probe"), scenario, "11", "3", side, str(evidence)], capture_output=True, text=True, check=True, timeout=30)
        result = subprocess.run([godot, "--headless", "--path", str(project), "--scene", "res://binding_smoke.tscn", "--quit-after", "60", "--no-header", "--", scenario, str(evidence)], env=env, capture_output=True, text=True, timeout=30)
        if result.returncode or "ERROR" in result.stderr or "leaked" in result.stderr:
            raise RuntimeError(result.stdout + result.stderr)
        records = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
        assert len(records) == 1 and records[0]["parity"], result.stdout + result.stderr
        print(json.dumps({"native": json.loads(native.stdout), "godot": records[0]}))
