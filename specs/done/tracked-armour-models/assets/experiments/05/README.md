# Slice 05: jet paint sheen, evidence

The jets were shot in the battle camera through a throwaway `jet-sheen` scene
over `/labs/air-high`, at the high graphics profile, with the jets baked into
a local runtime. "Before" is satin, roughness 0.55. "After" is the shipped
`JET_ROUGH = 0.38`.

- `f15ex-reference-before-after.png`: the user's reference, then satin, then
  0.38. Ours are framed from behind and above.
- `<jet>-rear-before-after.png` and `<jet>-sides-before-after.png`: before
  and after at the same camera.
- `<jet>-low-orbit-after.png`: 0.38 at a low orbit, into the sun and with the
  sun behind the camera.
- `<jet>-roughness-ladder.png`: rear crops at satin, 0.42, 0.38, 0.35, 0.30,
  0.25, 0.20 and 0.15, read left to right, top to bottom.
- `compare-telemetry.txt`: `compare-screenshots` distances, and luma crops
  for the whole ladder.

## Critique (unprimed, three rounds)

Scores are sheen against the reference, grey at the silhouette, artifacts,
and overall.

| Roughness | Scores | Note |
|---|---|---|
| 0.35 | 5 / 4 / 4 / 4 | chrome from above; foil on the Su-35S skin |
| 0.42 | 4 / 4 / 5 / 4 | "no different" from satin |
| 0.38 | 5 / 4 / 5 / 5 | "marginally better on sheen"; shipped |

The finish is below the critique bar. The gaps every round named:

- **The dark teal shaded sides** were already there at satin.
- **No sun glow facing the sun,** and roughness doesn't move it.
- **Foil streaks on the generated skins.** Slice 06 finds the code-smoothed
  guides cleaner.

None of these is a paint roughness. See J05-1 in `decisions.md`.
