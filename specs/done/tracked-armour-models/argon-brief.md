# The Argon brief

The text every lane sends Gemini 4 Argon for round 1: the common part below
verbatim, then a vehicle part filled in from the frame. It carries the pilot's
lessons (the recipe in [the Argon pilot](family-logs.md#the-argon-pilot-t-72b3-and-abrams-sep-v2)), so
the local edits after round 2 stay few. Round 2 is a numbered list of
measured fixes (template at the end).

## Common part (send verbatim)

Build a game-ready 3D model of a military vehicle yourself, in Blender, with a
Python (bpy) build script. You are the modeller.

WHY: we want a crisp hard-surface model: flat planar armour plates, sharp
edges with small bevels that catch light, clean cylinders for wheels and
barrels, boxes as real boxes. Silhouette and proportions first, then the
details that read at game distance (a camera 20–60 m away, looking down at
35–50 degrees).

SETUP: install Blender (Linux x64 tarball from download.blender.org, 5.2.x)
and run it headless.

THE SCRIPT (hard requirements; the game imports it):
- Module-level helpers, then `def build():`, which builds the vehicle into
  the current scene (no scene reset, render or export) and returns its root
  object; then `def main():`, which resets the scene, calls `build()`, renders
  the five views below, saves `<unit>.blend` and exports `<unit>.glb` to
  `os.environ.get("OUT_DIR", "out")`; then `if __name__ == "__main__": main()`.
- If the brief names variants, `build(variant)` takes the variant id and
  builds that one; `main()` renders and exports each.
- No randomness without a fixed seed.

CONVENTIONS:
- Metres. X forward (the main weapon points +X), Y left, Z up. The hull is
  centred on x = 0, y = 0 and the tracks stand on z = 0.
- Objects, named by what moves them (names are exact):
  - `hull`: the hull and everything fixed to it (skirts, fenders, stowage,
    lights, the rear cage);
  - `turret`: what turns with the turret, origin at the turret ring centre;
  - `gun`: what pitches with the main weapon (mantlet and barrel), origin at
    the trunnion, barrel along +X;
  - `hmg`: the roof gun's turning base (cupola ring or RWS body), origin on
    its pivot; `hmg_gun`: the part that pitches (the gun itself), parented to
    `hmg`;
  - one object per wheel that turns: every road wheel, sprocket, idler and
    return roller, named `wheel_L_<name>` / `wheel_R_<name>`, origin on its
    axle;
  - `track_L`, `track_R`: the track runs (a closed band round the wheels);
  - `dressing_<name>`: sights and sensors standing above the roof line,
    parented to the part they ride.
  Parent `turret` to `hull`, `gun` and `hmg` to `turret` (or to `hull` where
  the vehicle has no turret), everything else to `hull`.
- Hatches closed. No crew. No antenna whips (we add ours). No markings,
  numbers or camouflage.
- Materials: these seven names only, flat colours, no image textures:
  - `armor_paint`: every painted surface, painted bolts and hinges included;
  - `fittings`: dark painted hardware (lamps' housings, tools, latches);
  - `steel`: bare metal only (track pins, tow cables, sprocket teeth,
    machine-gun barrels, the muzzle);
  - `rubber`, `optics` (dark glass), `lamps`, `recesses` (dark gaps and
    grille interiors).
  Painted things are never `steel`: our renderer paints `steel` bright.
- Edges: plate edges as two-segment bevels (1–3 cm) or sharp. Not
  single-segment bevels. Cylinders read round (wheels 24–32 segments,
  barrels 24).
- Budget: under 80,000 triangles. Spend them on the silhouette and what is
  seen from above.

CLEARANCES (the game's geometry lint fails on these):
- Nothing of the hull, skirts or stowage inside the track run's box
  (between the hull's side and the skirts, from the ground to the run's top).
- Nothing on the hull within the turret's swept radius above the ring, so
  the turret turns freely.
- Axle stubs end at the hull's side. Road wheels, return rollers, sprocket
  and idler don't overlap each other seen from the side.
- The track run's top stays under the sponsons and fenders.
- The side skirt's front panel is flush with the front fender: one outer
  plane and a continuous top edge. Nothing of the skirt stands outboard of,
  above or ahead of the fender, and appliqué stops where the glacis takes
  over. (On our last tank a skirt panel stood 8 cm outboard of a narrower
  fender and read as a fin sticking out at the front corner.)

SHAPE RULES:
- The turret's front is swept back. Seen from above, it runs back from the
  mantlet to each side, like an arrowhead or a wedge. It is never flat across
  and never angled in toward the gun. Cheek armour blocks follow that line.

INPUTS (in this Drive folder):
- `refs/*.jpg`: reference photos of the real vehicle. Study all of them first.
  Each file name says the view.
- `our-current-model.glb`: our existing model, for proportions only. Its
  overall dimensions and part positions match the numbers below; its look is
  not what we want. Measure it; don't copy it.
- `example-build.py`, if present: a finished script for another vehicle that
  keeps this contract. Follow its structure, not its shapes.

LOOP: after each build, render with EEVEE on a light grey background at
1600×1000: three-quarter front from the left, left side (orthographic),
head-on front (orthographic), top (orthographic), three-quarter rear. Render
the same views of `our-current-model.glb` once. Compare with the photos and
fix the largest silhouette or proportion mismatch first, then detail. Iterate
up to about 8 times. Watch for soft or lumpy surfaces, gaps, floating parts,
wheels through tracks, the wrong wheel count, and a turret too tall, too big,
or too far forward or back.

DELIVERABLES (write into `out/` in this folder):
- `out/build.py`, `out/<unit>.glb`, `out/<unit>.blend`;
- `out/renders/<view>.png` (five views) and `out/renders/current-<view>.png`;
- `out/compare.png`: rows are views, columns are the closest photo, our
  current model and yours;
- `out/notes.md`: triangle count, object list with origins, dimensions
  measured from the GLB (hull length, length weapon forward, width, height to
  the turret roof), and an honest list of where it still differs from the
  photos.

Reply when done with a short summary and the paths.

## Vehicle part (fill in from the frame)

```
THE VEHICLE: <full name and variant>. Folder: tank-models/<unit> (refs/,
our-current-model.glb[, example-build.py]; write out/ there).
[VARIANTS: build(variant) for <id> (<what differs>), <id> (...).]

Dimensions to hit (metres, from the battle frame): hull length <L>; width <W>
over <skirts/tracks>; height <H> to the turret roof (sights and the roof gun
may stand above it). [Length with the weapon forward <L_gun>.]
Turret ring centre at (<x>, 0). Main weapon trunnion at (<x>, 0, <z>), muzzle
at (<x>, <y>, <z>). Roof gun pivot at (<x>, <y>, <z>), its muzzle at (...).
Road wheels: <n> a side, <diameter>; sprocket at the <front/rear>.

What makes this vehicle (from the photos): <the silhouette features people
recognise, in priority order>.

Must fix (our current model's faults, from the user): "<note, quoted>", meaning
<the measured target>.

Paint: <nation's factory colour as linear sRGB>.
```

The numbers come from the frame (`fixtures/catalog.json`, the variant's
`frame`: `body_dimensions_m`, each mount's `pivot_m` and `muzzle_m`, which is
relative to the pivot) and the survey (`assets/frames/<family>.json`). World
muzzle = pivot + `muzzle_m`.

## Round 2 (template)

```
Round 2, the last round. We measured your GLB against our frame and ran it
through our game's export. Fix these, in order, then re-render and update
out/ (keep round 1's out/ as out-r1/):

1. <measured fault: what, where, by how much, and the target>
...

Keep everything else as it is.
```

Each fix is a number against a target, or a lint pair with where they meet,
or a photo-based shape fault the measurements show. Never more than about
ten.
