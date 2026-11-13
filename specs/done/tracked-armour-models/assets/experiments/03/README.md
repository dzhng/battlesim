# Slice 03 evidence: the T-72B3's cast turret dome

Three candidates of the whole T-72B3. They differ only in the dome
(`turret_shell`), and none carries the head-out commander. The verdict and
its reasons are in [`decisions.md`](../../../decisions.md) (E1 entries).

- `candidates/`: the exported GLBs (current, smoothed, skin), built by the
  shipped `roster/t72.py` with only the dome swapped.
- `smoothed_dome.py`: the smoothed candidate's code, a patch to
  `t72.dome_rings` and its loft. Slice 17 applies it.
- `shots/`: `closeups.py` renders of each candidate, shaded and matcap, at
  four turret shots (`battle_zoom`, `cinematic`, `ref_angle`, `dome_close`;
  how they are framed is decision E1-7).
- `sheets/`: each shot as current | smoothed | skin, left to right.
- `compare/`: each candidate's `ref_angle` turret crop against the
  reference photo's turret, with the helper's metrics. The reference crop
  is from a CC-BY-SA-4.0 photo and is for review only.
- `critique.md`: the unprimed critique's scores and findings.

The skin's mesh and its input view are pinned in
`packages/scene-assets/blender/skins/t72.json`, under
`assets/source/skins/t72/`.
