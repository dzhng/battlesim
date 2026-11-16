# Decisions ledger

What was decided where the spec was silent, checked against the shipped code.
Grouped by how sure each call is, least sure first. Each entry stands alone.

## Judgement calls worth a second look

- **The generator's texture is kept as drawn, even where it misses.** The
  fixes would be prompt changes and paid regenerations, not code.
  - *What the critics saw:*
    - Flecktarn reads as a generic green woodland;
    - OCP leans grey-green;
    - sleeves are soft;
    - the Russian body's helmet cover has a sharp fin at the back.
  - *By design, not defects:*
    - The German, Russian and Chinese armies share a green palette (the game
      paints Europe, Russia and China in green woodland).
    - In game every soldier wears the side tint.
- **The eye sits at the top of the sight, and the weapon errs low.** This is the
  user's rule: "make the eye look at the absolute top of the gun … I rather have
  it be too low than too high … the butt of the gun needs to be aligned with the
  shoulder".
  - *How:* the neck cranes up to 36° (`NECK_CRANE_DEG`), and the butt slides at
    most 4 cm on the shoulder (`POCKET_SLIDE`).
  - *Result:*
    - On the M4s the eye is 0.2–1 cm above the optic.
    - On the AK-12 it is 3 cm above, because one hold serves every rifle and
      the AK's low collimator sits about 3 cm under the M4's optic.
- **Face down, every dead man lies rolled 30° onto one side**, not flat on his
  face (`CORPSE["roll_down"]`), so a big rucksack stays under the lying limit.
  - *Why:* flat, a scout's rigid rucksack stood 0.60–0.65 m, over the 0.6 m
    lying limit.
  - *Range:* at 40° a knee or an arm rose to 0.64 m instead.
- **A slung launcher falls off a dead man** and lies flat 0.7 m out at his left
  side (`CORPSE["sling"]`).
  - *How:* the rig gains a `sling` bone (a child of `spine_03`), so the skeleton
    has 66 joints. The living clips hold the bone at rest on the back; the two
    corpse poses put it on the ground.
  - *Ruled out:*
    - any carry position: a rigid tube longer than the torso stands up or
      sinks, whatever its angle;
    - riding the root: a prone soldier's tube would stay standing;
    - a spare leaf bone: it follows its parent.
- **The eye socket sits at the pupil**, measured on three generated heads. The
  model's eye therefore sits 1.5 cm lower than before; the simulation's eye
  height is unchanged, and `fit.eye` holds.
- **The walking and running low-ready support hand is 1.7 cm short of the
  handguard.** Accepted (it was 5.3 cm). The walk animation swings the shoulder
  back, and the hand is already at the magazine well.
- **Two staging tests that fail on staging itself are left to their owners.**
  - *Which:* native `counts` (a browser parity capture against the newer sim
    publication layout) and web `battleRules` (fielding now checks mobility
    first).
  - *Why:* this work's code was identical to staging's there, and both areas
    belonged to other threads' active work.
  - *Also not fetched:* the two audio tests refusing LFS pointers are missing
    assets, per development.md.
- **The LAV and Humvee crews sit low in their hatches.** Their crew poses predate
  this spec, which changed only what crews wear.

## Sound

### The roster and its data

- **Army data lives in `roster/armies.json`:** each army's rifle, gloves,
  webbing, cloth mean, prompt, three look clauses and crew kit, and a light
  machine gun for the four original armies.
  Nothing reads a faction there, so there is none. The kit matrix (`KITS`) stays
  Python, in `infantry_roster.py` at the Blender folder's root, where every
  script can import it.
- **A kit names the body kind its men wear** (`kind="recon"` for scouts and the
  heavy sniper teams; `rifle` otherwise), the same word the pins and the
  generation tool use.
- **A new army's cloth mean is its look `a` body's own measured mean** (French
  `[0.0965, 0.0735, 0.0461]`, PLA `[0.0861, 0.0754, 0.0449]`). With no print
  recipe to match, look `a` keeps its colours and looks `b` and `c` grade to
  it.
- **The PLA army is `pla_type21`**, Type 21 woodland. Shippable photos of the
  uniform, helmet and carrier exist.
  - *What the prompt says:* "a high-cut helmet with side rails in a matching
    camouflage cover, electronic ear defenders", which is what the photos
    show; they don't confirm a QGF-11.
  - *The camouflage:* the flat lay's green with brown patches, after the first
    take read grey.
- **References may be CC-BY, CC-BY-SA or CC0**, as the library already allows. The French
  Army's own photos (Licence Ouverte) are refused, so the French references are
  public-domain US DoD photos from Saber Junction 23.
- **The new rifles (HK416 F, QBZ-191) follow the M4/M27 construction**, with
  the optic's top at the shared hold's sight height. The aim rule holds for them
  with no new hold.
- **The new armies' catalog rows are named by army** (`rifle_squad_pla_…`,
  `rifle_squad_french_…`), since two armies now share each faction; the
  original rows keep their faction names (`rifle_squad_eastern_…`,
  `rifle_squad_europe_…`).
  - *Who uses them:* the FN-6 team names the PLA riflemen, the Akeron team the
    French, and the Javelin guard's Europe variant the French rifleman.
  - *What went:* the German Akeron guards and German Javelin exports no longer
    had a user, so they are deleted.
- **Crews wear their vehicle's army.** The crew kit is army data; the LAV's crew
  are Marines, and the VBCI's and VBL's are French.

### Wearing a generated body

- **The fit centres the body front to back on its thighs**, which never carry
  kit. On the torso, a ruck pushed the soldier forward and shrank the scout.
- **The height fit ignores anything behind the head band**, so a ruck's top
  can't set a soldier's height.
- **Leg slices centre on the midpoint of their 15th and 85th percentiles.**
  Each slice moves by its own amount, smoothed over ±4 slices, and every
  vertex in the leg moves. Extents followed knee pads, filtering tore the
  shins, and the median followed vertex density.
- **Back kit rides one bone whole** (`_strap_packs`).
  - *What counts as kit:* anything above mid-thigh behind the back plane, or
    standing off the body (a whip beside the head).
  - *Which bone:* kit joined to kit above the hips rides `spine_03`; separate
    kit lower down (a flap, a dump pouch) rides the pelvis.
  - *The seam:* two rings of body round the kit blend in, so a ruck pivoting
    off the buttocks doesn't tear a spike.
- **The gloves reach back under the sleeve's cut end** (`GLOVE_CUFF`), so no
  wrist shows between them.
- **Cloth and skin each carry their own roughness** in their atlas (0.9 and
  0.6), because the atlas path takes roughness from the image, not the
  material.
- **Rejected candidates are regenerated, never patched.** A body that won't lie
  flat, comes back untextured, or keeps a hand where the hand cut misses gets a
  new mesh, and after two failures a new drawing.

### The aim and the holds

- **One shouldering rule** (`shouldering.py`):
  - the butt rests in the shoulder pocket;
  - the head turns to look along the bore;
  - the neck cranes to bring the eye to the sight;
  - a launcher's eyepiece sits 3 cm ahead of the eye.
- **The support hand takes the handguard as far forward as the arm reaches**
  (`within_reach`), and the aimed poses swing the support shoulder forward 20°
  (`lean_in`). Putting the clavicle in the IK chain hunched the shoulders: the
  solver ignored stiffness and limits.
- **Each authored clip releases its IK once baked.** Muted IK stacked on a bone
  misplaced the live chain by 5.4 cm.
- **One sling** (`Kit.sling`) carries every slung tube and the packed tripods,
  for the roster and the test soldier alike.
- **A packed canister longer than the TOW's rides lower on the back**, as long
  MANPADS tubes do, so its carrier stays under the soldier's height.

### The validator

- **`fit.lies` measures the body and what it carries, not kit standing on the
  ground by itself.** A tripod weighted wholly to the root stays standing when
  its gunner dies.
- **`fit.facing` and `fit.cheek` check the skeleton's aim reference pose**
  (stand-aim); `fit.cheek` skips a soldier serving tripod kit.
  - *`fit.facing`:* the head within 8° of level.
  - *`fit.cheek`:* the eye at least the skeleton's `sight_over_bore_m` over the
    muzzle. That's 0.05 for the rifle skeleton and none for the launcher's,
    whose sight sits beside the tube.
- **A skeleton's own checks live in `skeleton.ts`**, beside the `*Fit` modules
  that fit a model to the simulation.

### Exports, pins and the generation tool

- **Generated bodies are the only path.** A look with no pin fails, naming the
  key and the commands that make it (`skin_pins.required`). `dressed(army,
  look, kind)` and `export_soldier(kit, path)` serve the roster and the test
  soldiers.
- **A kit's receipt lists the skins its looks wore** and hashes `armies.json`.
- **A paid submit is repeated only when it surely never reached the service**
  (a 502/503, a refused connection). A 504 or a timeout may come after it was
  accepted and charged, so a request is never paid twice.
- **Input images are never retried automatically**; an image is keyed by its
  PNG's hash, and a failure costs nothing.
- **The tool's tests run in the root `test` script** (`test:skins`).
- **The catalog load budget is 800 MiB**: the measured 775.3 MiB with every
  unit's art, rounded up to the next 32 MiB.
- **`assets/review/` is left as it is.** Those folders are
  specs/done/infantry-weapon-use's archive of reviewed visuals, which no tool
  regenerates.
