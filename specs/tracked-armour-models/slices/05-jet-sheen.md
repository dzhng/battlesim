# 05: jet paint sheen (lane J; ships)

## Contract

Jet paint reads in the battle camera with the sun highlight and sky sheen of the
user's screenshot (`assets/references/jet-sheen-reference.png`), while the grey
still holds at the silhouette. Helicopters keep today's satin finish
(`AIR_ROUGH = 0.55`, `textures_air.py:15-18`).

## Why the paint is matte today

Matte paint was chosen deliberately, with the skins (commit `f5a8a2b59`):
"satin, not gloss: it sheens toward the sun at a grazing angle without
mirroring the sky and the ground". The user now wants the sheen. The old risk,
a jet mirroring the ground until its grey is lost, is the thing to watch.

The renderer already has full PBR sun specular and environment (PMREM) sky
reflections (`lighting.wgsl:150-169`, `models_material.wgsl:61-85`). This is a
tuning change, not renderer work.

## Seam

- **The gloss keys on the aircraft being a jet, not on its paint scheme**
  (decision S5).
  - Schemes are shared between jets and helicopters: `us_compass_grey`,
    `russian_air_grey`, `chinese_air_grey`, `nato_air_grey`.
  - The Su-25 jet wears `russian_helicopter_camo`.
- **The shape of the change:** a roughness factor on the jet's paint material,
  applied where an aircraft's materials are fitted.
  - `parts.textured(rough=)` sets the Principled default, which acts as a factor
    on the recipe (`parts.py:197`).
  - The aircraft's paint is built in `aircraft_parts.fit()` (`aircraft_parts.py:75`).
    That file is 37,191 B, over the ratchet, so it must not grow. Put the jet
    flag and its factor where the jet airframe is assembled
    (`aircraft_jet.py`), or shrink `aircraft_parts.py` first.
- **Values:**
  - start at an effective roughness of about 0.3 (numbers are guesses: tune
    it);
  - metalness stays 0 unless gloss alone can't reach the reference.

## The variable and crop

- **The one variable:** the specular sheen.
- **The crop:** a jet in the battle camera, at the reference's sun angle and
  distance. Use an F-15 or F-16 in `us_compass_grey`, plus a Su-35 in
  `russian_air_grey`.
- **Out of scope:** shape, markings, scheme colours, helicopters.

## Steps

1. **Cheapest check first.**
   - Bake one jet in this worktree.
   - Confirm the factor survives as the glTF material's roughness, using the
     `asset validate` materials output or a bundle read.
   - If the factor is dropped, stop and find which stage drops it.
2. Tune the roughness against the reference in battle-camera captures, run under
   `lockf -k <main>/throwaway/gpu.lock`.
3. **`compare-screenshots`** of the capture against the reference.
4. Check that the grey still reads at the silhouette, and that the jet doesn't
   mirror the ground.
5. **Last check: an unprimed `screenshot-critique`** of the capture.
6. **Re-export every jet family.** Re-export one helicopter (the Apache) and
   prove it byte-identical.
7. **Checkpoint,** which never blocks: `preview-shots` with the reference and
   the capture side by side.
8. **Push milestone:** the coordinator merges and bakes.

## Delegated

- the final roughness value;
- exactly where the factor is applied, within the rules above.

## Feedback that would change it

The user picks a different strength. The planning map's options were:
- **subtle:** about 0.42;
- **strong:** about 0.2, with slight metalness.

## Result (2026-11-11, lane J)

- **Shipped:** an effective roughness of `JET_ROUGH = 0.38` on the airframe
  paint of every fixed-wing jet, against 0.55 satin. Metalness stays 0.
  - `textures_air.jet_finish` registers a jet twin of the scheme's recipe,
    with the same albedo, normal and wear and a smoother ORM.
  - `aircraft_jet.jet` uses that twin when the unit's catalog flight model is
    `fixed_wing`.
  - A roughness factor wouldn't have survived: `textures.attach` resets it to
    1 on worn materials.
- **Re-exported:** all 18 fixed-wing scripts, 25 live appearances. Each GLB
  differs from its last commit only in the paint's ORM image. The wrecks and
  the Apache are byte-identical.
- **Critique,** in three unprimed rounds (sheen, grey at the silhouette,
  artifacts, overall):

  | Roughness | Scores |
  |---|---|
  | 0.35 | 5/4/4/4 |
  | 0.42 | 4/4/5/4 |
  | 0.38 | 5/4/5/5 |

  0.38 is the best of the three, but it is **below the bar**. What remains
  isn't paint:
  - the shaded sides already went dark teal at satin (the scene's light);
  - there is no sun glow into the sun, at any roughness;
  - gloss reveals the generated skins' ripples (slice 06).

  The reference's own scene is brighter and graded.
- **Evidence:** [`assets/experiments/05/`](../assets/experiments/05/README.md).
  **Decisions:** J05-1 to J05-8 in [`decisions.md`](../decisions.md).
