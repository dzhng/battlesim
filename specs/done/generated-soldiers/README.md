# Generated soldiers

Every soldier in the game wears a body made by an image-to-3D generator: the
roster's 132 kit models, the 12 test soldiers, and the crews riding the six
crewed vehicle families. The body carries the clothing, kit and head in one
mesh. It is fitted onto the shared Quaternius rig and given our own gloves,
weapon and sockets. Six armies field them:
- US Army (OCP);
- US Marines (woodland MARPAT);
- the Bundeswehr (Flecktarn);
- Russia (EMR);
- China's PLA (Type 21 woodland);
- France (CCE woodland).

The procedural clothing it replaced was hand-built shells over a base body. It
read as toy-like next to the vehicles, and every new army needed new code. A
generated body needs a prompt, about $0.64, and a review.

## Why it works this way

- **One front T-pose image, then Tripo H3.1** (Meshy 7.1 as the fallback). The
  trial in [assets/pilot](assets/pilot) settled it:
  - Gemini Argon's Blender scripts came out blocky
    ([US](assets/pilot/trial-us-current-argon-meshy-tripo.png),
    [Russian](assets/pilot/trial-ru-current-argon-meshy-tripo.png));
  - Tripo's multiview mode made people two-faced
    ([here](assets/pilot/trial-tripo-multiview-two-faced.png));
  - Meshy invented weak backs.

  The user's call: "Stop all argon work. Go with tripo with meshy as fallback."
  Vehicles stay on Argon.
- **The generator's texture is kept and only regraded.** Each army's cloth is
  scaled to its `cloth_mean`. Repainting would mean a texture-authoring
  pipeline, and the drawn detail (pouches, folds, faces) is the point.
- **A body is keyed `(army, look, kind)`.**
  - *Looks* are `a`, `b` and `c`, one short clause each (headgear, eyewear,
    face), so a squad never reads as copies.
  - *Kinds* are `rifle` (assault pack and radio whip) and `recon` (a large
    ruck).
  - Packs belong to the kind; look clauses never name them.
  - That makes 30 bodies: four armies × 3 looks × 2 kinds, plus the PLA and
    French riflemen. The PLA's and French teams' men wear their army's
    riflemen, so those armies need no scouts. The US and Russian team guards
    wear their army's scouts.
- **The prompt is composed, never hand-written per body:** studio frame, army,
  look clause, kind cue, all from `roster/armies.json`
  (`tools/skins/prompt.mjs`). The input image is reviewed by eye before
  `generate` is run, because a cue the image drops, the mesh can't recover.
- **Crews wear their vehicle's army** (each crewed family names its
  `crew_army`). The LAV's crew are Marines, and the VBCI's and VBL's are French.
- **France is CCE, not BME,** because BME reads too close to OCP at battle
  distance. The Europe Javelin team is French (France fields the Javelin); the
  Stinger is German.
- **No new factions:** the PLA sits in `eastern`, the French in `europe`.
- **No backward compatibility.** The procedural path is deleted, and no squad
  mixes generated and procedural soldiers.

## What must hold

- **The soldier GLB contract:**
  - the `soldier_rig`: the Quaternius rig's 65 joints plus our `sling` bone, a
    child of `spine_03`;
  - `soldier_LOD0..3` within the tier budgets;
  - the `eye` socket at the pupil (one offset in the head's frame), and the
    `muzzle` socket;
  - at most 4 influences per vertex;
  - exports on pinned, single-threaded Blender, so they repeat byte for byte.

  Clips and soldier GLBs must agree on the skeleton: the loader checks its
  joints' count, names and parents.
- **The validator holds every soldier to the simulation's soldier**
  (`soldierFit.ts`), in the skeleton's aim reference pose (stand-aim) and every
  corpse pose:
  - *Standing aim:* height, eye and muzzle.
  - *`fit.lies`:* every corpse pose lies under the catalog's lying height. Kit
    weighted wholly to the root (a tripod) is exempt, since it stands by itself
    when its gunner dies.
  - *`fit.facing`:* the aiming head within 8° of level.
  - *`fit.cheek`:* the eye at least the skeleton's `sight_over_bore_m` over the
    muzzle. That's 0.05 for the rifle skeleton; the launcher's has none,
    because its sight sits beside the tube.
- **The aim.** The user's rule: "the eye look[s] at the absolute top of the gun
  … I rather have it be too low than too high … the butt of the gun needs to be
  aligned with the shoulder."
  - The head looks along the bore.
  - The neck cranes to bring the eye to the sight, and the butt slides at most
    4 cm on the shoulder.
  - Any gap left is meant to leave the weapon low; `fit.cheek` holds that in
    the standing aim.
  - The support hand takes the handguard as far forward as the arm reaches.
- **A dead man lies on the ground:**
  - face down, he rolls 30° onto one side, so a rucksack can't prop him up;
  - a slung launcher falls off and lies beside him (the `sling` bone).
- **Every soldier body is a pin.** A missing pin is an error, naming the key and
  the commands that make it. Pins are verified by hash before any export uses
  them (`skin_pins.py`, the exporters' only reader); airframe skins share the
  same pins.
- **A request is never paid twice.**
  - Every accepted submit is logged in `blender/skins/calls.jsonl`, and an
    unresolved one is resumed, not resubmitted.
  - A submit is retried only when the service refused it (a 502 or 503, a
    refused connection), never after a 504 or a timeout, which may come after
    a charge. Such a submit is left for a person to check.
  - Every candidate is committed, rejected ones too.
- **A rejected body is regenerated, never patched.** That covers a body that
  won't lie flat, keeps a stray hand, or comes back untextured.

## Where it lives

Paths are under `packages/scene-assets/` unless they say otherwise.

| Concept | Owner |
|---|---|
| Army data: rifle, LMG, gloves, webbing, cloth mean, prompt, look clauses, crew kit; the kinds' cues | `blender/roster/armies.json` |
| The kit matrix, body kinds, source paths, crew sources | `blender/infantry_roster.py` (`KITS`, `kind_of`, `source_path`, `crew_source`) |
| Pins: read and verified for export | `blender/skin_pins.py` (`required`, `verified`, `WORN`) |
| Pins: written; the paid-call ledger; prompts; generation | `tools/skin-generate.mjs`, `tools/skins/` (`pins.mjs`, `journal.mjs`, `prompt.mjs`, `steps.mjs`, `fal.mjs`) |
| Fitting a body: fit, limb seating, weights, back kit, hands, surface | `blender/generated_body.py` (`wear`, `_strap_packs`) |
| Soldier assembly and export | `blender/infantry_kit.py` (`dressed`, `Kit`, `Kit.sling`, `export_soldier`) |
| The rig, the `sling` bone, holds, the eye socket | `blender/infantry_rig.py` (`Rig`, `SLING_FRAME`, `EYE_IN_HEAD`, `within_reach`) |
| The aim hold | `blender/shouldering.py` (`shouldered`, `weld_cheek`, `lean_in`) |
| The clips, corpses included | `blender/clips_infantry.py` (`corpse`, `CORPSE`) |
| Roster kit export and receipts | `blender/roster/infantry_equipment.py`; team rifles and LMGs in `blender/support_weapons.py` |
| Atlas embedding | `blender/textures.py` (`ATLASES`, `attach`) |
| Validator checks | `src/soldierFit.ts`, `src/skeleton.ts`, `src/pose.ts` (`lyingBounds`) |
| Load budget | `CATALOG_LOAD_MAX_BYTES` in `src/schema.ts` |

The tests that pin the behaviour:
- `web/tests/sceneAssets/soldierFit.test.ts` (the aim, a corpse lying low, a
  tripod standing alone);
- `web/tests/sceneAssets/validate.test.ts` (a golden failure for every
  finding);
- `tools/skins/skins.test.mjs` (paid once, resumed, a timed-out submit not
  repeated);
- `cargo test -p presentation --test pose_behaviour` and `-p native-client
  --test models`.

## Dead ends

- **Argon for soldiers, Tripo multiview, Meshy as first choice** (above).
- **A 1024 atlas on a 42k-triangle body** blew the load budget. The body is
  22k triangles on a 512 atlas.
- **Centring the fit on the torso**, which a ruck pushes back. The fit
  centres on the thighs, which never carry kit.
- **Leg slices centred by extent or median** followed knee pads and vertex
  density, and filtering which vertices moved tore the shins. Every vertex in
  the leg moves now, each slice by its own smoothed amount.
- **Back kit weighted to the nearest body surface:** a ruck's top nodded with
  the head, its sides swung with the arms, and a whip stood upright when he lay
  down. Back kit rides one bone whole.
- **Aim, the first pass:** a 22° bowed head, and a weld that could only raise
  the weapon. It lifted the butt 9–10 cm off the shoulder; the user saw "gun is
  wayyyy too high" and "why are their heads looking down".
- **The clavicle in the arm's IK chain**, to reach the handguard: Blender's
  legacy solver ignored stiffness and limits and hunched the shoulders. The
  aimed poses swing the support shoulder forward instead.
- **Muting a clip's IK instead of releasing it:** stacked muted IK misplaced
  the live chain by 5.4 cm.
- **Fixing slung launchers by where they're carried.** A rigid tube longer than
  the torso stands up on a face-down corpse or sinks under a face-up one,
  whatever its angle. Riding the root would leave a prone soldier's tube
  standing, and a leaf bone follows its parent. Hence the `sling` bone.
- **A deeper corpse roll:** at 40° a knee rose to 0.64 m; rolling the other way
  buried one side.
- **"Pale grey-green" for the PLA,** from washed-out field frames, drew a grey
  pattern. The prompt follows the CC0 flat lay of the Type 21 uniform.

## Visual provenance

- **The pilot.** One look end to end against the procedural soldier, as offline
  Blender renders and production-renderer sheets (`asset sheet`). These shots
  decided the generator and set the bar the whole roster then had to meet:
  - [before and after](assets/pilot/contact-before-after.png);
  - the [hard clips](assets/pilot/clips-hard.png);
  - [in the Armory yard](assets/pilot/showcase-yard.png);
  - the [Humvee gunner](assets/pilot/humvee-gunner-before-after.png);
  - its [input image](assets/pilot/pilot-input-front.png).
- **The reference photos** each body was judged against for national identity
  live in `assets/references/<kit>/` with their licences: public-domain US DoD
  and DVIDS photos, and CC-BY, CC-BY-SA and CC0 Wikimedia Commons images.
  - The aim was held to
    `assets/references/rifle_squad/us_army_ocp-three_quarter_rear.jpg`: butt in
    the shoulder, head on the stock, looking ahead.
  - The PLA camouflage was held to
    `assets/references/rifle_squad/pla_type21-detail-2.jpg`.
- **The result**, as offline Blender renders of the shipped GLBs (the first
  three), production-renderer sheets (the crews) and a capture from the
  Armory scene (`bun run --cwd web scene -- armory`):
  - [every army's bodies](assets/final/lineup-front.png): a row each for OCP,
    MARPAT, Flecktarn and EMR (riflemen, then scouts), then PLA and French;
  - [the aim, side view](assets/final/aim-side.png);
  - [dead launcher gunners](assets/final/corpses-launchers-top.png), their
    launchers on the ground;
  - [four of the six crews](assets/final/crews.png): Humvee, LAV, Tigr and
    VBL;
  - the [Armory's infantry](assets/final/armory-infantry.png), in the game's
    renderer.

## Cost

Calls: $0.04 per input image, $0.60 per Tripo mesh, $1.20 per Meshy mesh (none
was needed).

- **Total: $25.44** over 76 paid calls (36 images, 40 meshes). Every attempt
  is included, rejected ones too. The trial spent about $8.60 before the ledger
  existed; the pilot's $0.64 of it is also in the ledger. That makes **about
  $33.40 in all.**
- **Per body:** $0.64 at the first attempt (23 of 30 bodies), up to $1.88 with
  retakes; **$0.85** on average.
- **Per army:**

  | Army | Bodies | Spend |
  |---|---|---|
  | `us_army_ocp` | 6 | $5.72 |
  | `eastern_emr` | 6 | $5.68 |
  | `pla_type21` | 3 | $4.44 |
  | `german_flecktarn` | 6 | $3.84 |
  | `usmc_marpat` | 6 | $3.84 |
  | `french_cce` | 3 | $1.92 |

- **Per shipped roster soldier GLB: $0.19.** Crews and test soldiers reuse the
  bodies.

`node packages/scene-assets/tools/skin-generate.mjs costs infantry` reports it
from the ledger.

## Known limits

- **Generator textures:** Flecktarn reads as a generic woodland, OCP leans
  grey-green, and sleeves are soft. The fix is a prompt change and a
  regeneration (about $0.64 a body), not code. The gloves are our own plain
  shell (`Kit.gloves`), not the generator's.
- **One rifle hold serves every rifle,** so an AK-12, whose low collimator sits
  about 3 cm under the M4's optic, rides 3 cm below the eye while the M4's optic
  meets it.
- **The walking and running low-ready support hand rides 1.7 cm off the
  handguard,** from the walk animation's shoulder swing.
- **The LAV and Humvee crews sit low in their hatches.** Those poses predate
  this work.

The decisions made along the way, and how sure each is, are in
[decisions.md](decisions.md).
