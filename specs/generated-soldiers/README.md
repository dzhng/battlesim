# Generated soldiers

Every soldier in the game wears a body made by an image-to-3D generator: the
roster's 24 kits in all their army looks, the vehicle crews cut from them, and
the 12 test soldiers. The procedural cloth shells and kit builders are then
deleted. Two army looks are new: the Chinese PLA and the French army. The
squads that wore the wrong army today move to them.

## Next Agent Prompt

*Status (2026-11-15):* slices 00 (the pilot) and 01 are committed on branch
`t3/soldier-3d-model-comparison`. **Next: slice 02.** Slice 04 can run in
parallel, because it touches only Node tooling.

You are implementing this spec.
1. Pick up at the first unchecked slice below.
2. At the start of every slice, merge `origin/staging` into the branch. Check
   the merge's exit code and grep for conflict markers; never pipe it through
   `tail`. Re-bake if `assets/runtime/catalog.json` conflicts: the bake owns
   that file.
3. Work slice by slice. Each slice file says what to build, what to show and
   what must pass. Commit each slice on the branch. Nothing reaches `staging`
   until [slice 09](slices/09-ship.md), which merges once.
4. Before you end a pass, update this section: the status, the next pickup
   point, and the checklist.

Byte-identity proofs use fresh exports from the pre-change scripts, never the
committed GLBs (some drift). The reference exporter is
`throwaway/soldier-models-trial/refexport.sh <outdir>`, which restores the tree
afterwards.

**Warnings:**
- **Generator spend is real money.** Log every call, and never pay twice for
  one request (slice 04 owns that).
- **The pilot's tooling is throwaway.** `throwaway/soldier-models-trial/`
  holds the trial scripts and evidence; it is never repo code.
- **The main checkout is not yours.** `~/dev/battlegame` has someone else's
  uncommitted work.

**Checklist:**
- [x] Commit the pilot on the branch (`291546b64`)
- [x] 01 One owner each for pins, army data and source paths ([slice](slices/01-data-owners.md))
- [ ] 02 Crews wear their vehicle's army ([slice](slices/02-crew-by-army.md))
- [ ] 03 The wearer handles every soldier; a lying-fit gate for every corpse ([slice](slices/03-wearer.md))
- [ ] 04 One generation tool: pins, resume, cost ledger ([slice](slices/04-generation-tool.md))
- [ ] 05 Probe bodies: a second army, a scout, the kind cue ([slice](slices/05-probes.md))
- [ ] 06 The PLA and French armies, and the teams that move to them ([slice](slices/06-new-armies.md))
- [ ] 07 Every body generated and pinned ([slice](slices/07-batch.md))
- [ ] 08 The procedural path deleted; test soldiers on pins ([slice](slices/08-delete-procedural.md))
- [ ] 09 Ship: every export, crews, budget, closeout, one merge ([slice](slices/09-ship.md))

## Decisions

From the user, 2026-11-15:

- **Generator.** Tripo H3.1 image-to-3D from one front T-pose image, with
  Meshy 7.1 (`pose_mode: t-pose`) as the fallback. Keep the generator's own
  texture and regrade it to the army's print; never repaint it.
- **Argon is out for soldiers** ("Stop all argon work"); tanks stay on Argon.
  The trial is in [assets/pilot](assets/pilot):
  - Argon came out blocky and toy-like;
  - Tripo's multiview mode made people two-faced;
  - Meshy invented weak backs.
- **The whole roster** ("let's do the entire thing").
- **Looks:** "do what you think is best and not over engineered and easiest to
  work with given we're using a different model to generate these." This plan
  settles it in [the look scheme](#the-look-scheme).
- **Delete** the procedural clothing and kit builders. Test soldiers reuse
  generated bodies.
- **Add China and France**, and point the mismatched soldiers at them.
- **No backward compatibility and no migrations:** hard cutovers. No shipped
  squad mixes generated and procedural soldiers.
- **The load budget is arbitrary:** "you can change it however you want. But
  obviously if there's a reason to retire some budget, retire it."
- **Cost per soldier is reported** at the end.

Made in planning:

- **30 bodies**: 4 armies × 3 looks × 2 kinds (rifleman, scout), plus the
  PLA and the French in 3 rifleman looks each. Today the guards of the FN-6
  and Akeron teams wear scout kits; they become their army's riflemen, so the
  new armies need no scout kind. The drafts that counted 36 gave the new
  armies scouts; that is the rejected alternative.
- **One merge to `staging`, at the end.** Five kits span several armies and
  their receipts cover all of them; crews span four armies. Merging army by
  army would re-export the shared kits four times for no runtime safety.
- **Crews wear their vehicle's army.** The LAV moves to MARPAT, and the VBCI
  and VBL to French. The others keep their army.
- **The Europe Javelin team turns French**, guards included: France fields the
  Javelin, Germany doesn't. The Stinger stays German.
- **Camouflage, current issue:**
  - **PLA:** Type 21 ("Xingkong") woodland with the QGF-11 helmet. If
    shippable references can't be found for it, use Type 07 woodland digital
    with the QGF-03 helmet, and record the fallback.
  - **France:** CCE woodland, because BME reads too close to OCP at battle
    distance. The helmet and carrier are whatever the dated references show.
- **Rifles:** the PLA's QBZ-191 and the French HK416F are conventional-layout
  rifles. They get rifle entries built on the M4A1's geometry (same layout at
  game scale), with small proportion changes where the references demand
  them. Full new rifle models are out of scope.
- **The cost ledger is committed**, so a generation is never paid twice and
  the closeout can report actual spend. Every candidate body is committed too,
  rejected ones included (the keep-generated-assets rule).

## Pilot results (slice 00)

The pilot proved one look end to end: US Army OCP, look `a`, rifle kind, on
`rifle_squad`, `machine_gunner` and the US marksman squad (M110), plus the
Humvee gunner.

- **Checks:** `asset validate` has no findings, and the exports repeat byte
  for byte. `asset check` passes, and so do the presentation `pose_behaviour`
  and native `models` tests.
- **Poses and crew:** all 22 clips are clean in the production renderer, and
  the Humvee gunner works.
- **Load:** tier 0 is 25k triangles, and the catalog costs +0.7 MiB on the
  wire.
- **Pictures:**
  - [before and after](assets/pilot/contact-before-after.png)
  - [hard clips](assets/pilot/clips-hard.png)
  - [in the yard](assets/pilot/showcase-yard.png)
  - [crew](assets/pilot/humvee-gunner-before-after.png)

The unprimed critique scored silhouette 7, surface 6, fittings 6, materials 5
and cohesion 7. The tint is subtle by design, and measured at least as strong
as the procedural soldier's.

What the generated body still owes, and the slice that fixes each:
- near-white flecks in Tripo's OCP (03);
- the body is a little brighter than the procedural soldiers (03);
- no assault pack or radio whip, the rifleman's cue (05);
- the hydration hose stands off the vest in the crew's lower tier (03, then
  judged in 09).

Lessons the plan keeps:
- Blender vertex-group handles go stale after `modifier_apply`; use names.
- The arm weight zone must exclude wide-stanced feet.
- Limbs must sit on their bones. A knee 2.4 cm ahead of its joint failed the
  native corpse test.
- A 1024 atlas plus a 42k-triangle body blew the load budget.

Found on the way, not ours: the committed `rifle_squad/active_b.glb` doesn't
match what HEAD's scripts export. Slice 09 re-exports it with everything else.

## Contract

The exported soldier GLB keeps today's contract:
- the 65-joint `soldier_rig`;
- `soldier_LOD0..3` within the tier budgets;
- the `eye` and `muzzle` sockets;
- at most 4 influences per vertex;
- the stand-aim fit (feet, height, eye, muzzle);
- byte-for-byte reproducible exports.

What changes:

- **Every soldier body is a pin.** `(army, look, kind)` keys an entry in
  `skins/infantry.json`. A missing pin is an error once slice 08 lands.
- **Army data is data.** `roster/armies.json` holds each army's faction,
  rifle, LMG, gloves, cloth mean, prompt and look clauses, and the kinds'
  cues. Both the exporter and the generation tool read it.
- **Source files are named by army.** A kit's first army writes
  `<mode>_<look>.glb`, every other army `<army>_<mode>_<look>.glb`. Appearance
  ids don't change; only the catalog's `source` paths do.
- **Crews are army data.** Each crewed family names its `crew_army`; the crew
  soldier is that army's rifleman, look `a`.
- **Every corpse lies low.** A validator finding (`fit.lies`) holds every
  soldier's corpse poses under the catalog's lying height. Today one native
  test checks a single soldier.

### One owner per concept

| Concept | Owner | Introduced |
|---|---|---|
| Pin records: read and verified for every skin, airframes included | `blender/skin_pins.py` | 01 |
| Pin records: written | `tools/skins/pins.mjs` | 04 |
| Army data, look clauses, kind cues | `blender/roster/armies.json` | 01 |
| Source path of a kit look | `infantry_roster.py` (no bpy) | 01 |
| Crew soldier choice | the family's `crew_army` plus `infantry_roster.crew_source` | 02 |
| Generation prompt | `tools/skins/prompt.mjs` (stored in the pin) | 04 |
| Paid-call ledger | `blender/skins/calls.jsonl`, written only by `tools/skins/journal.mjs` | 04 |
| Body fit, limb seating, weights, hands, surface | `generated_body.py` | 00 |
| Atlas embedding | `textures.ATLASES` | 00 |
| Soldier assembly (rig, weapon, gloves, sockets, join, AO, export) | `Kit` in `infantry_kit.py`, cut down to that | 08 |
| Load budget | `CATALOG_LOAD_MAX_BYTES` in `packages/scene-assets/src/schema.ts` | 09 |

End state: the infantry pipeline reads as if generated bodies had been the
design from the start. No procedural fallback, no faction-keyed crew table, no
second pin verifier, no airframe-only generator beside a soldier-only one, and
no print recipe kept alive for a look nobody wears.

### The look scheme

A body is `(army, look a|b|c, kind rifle|recon)`. Its input image's prompt is
composed from four parts, all in `armies.json`:

1. a fixed studio frame: full body, front view, strict T-pose, palms down,
   feet narrow, plain light grey background, soft even light, no weapon, no
   text;
2. the army's base description: uniform and camouflage (with "no white"), helmet, carrier, boots, gloves;
3. the look's clause: one short phrase for what sets that look apart, taken
   from today's look table (headgear, eyewear, balaclava, skin tone);
4. the kind's cue: the rifleman's small assault pack with a radio whip, or the
   scout's large patrol ruck.

Packs belong to the kind; look clauses never name packs. Skin tone varies
across looks so a squad never reads as copies.

## Firewalls

- **Out of scope:** simulation, rules, clips, the rig and presentation. No
  digest changes.
- **No new factions.** The PLA sits in `eastern`, the French in `europe`.
- **Generators supply every kit cue**, so no procedural packs, grenades or
  helmets get "fixed".
- **No repainting the generated texture**, and no texture authoring pipeline.
- **No new rifle models** beyond the two geometry-based entries above.
- **The airframe skin flow keeps working.** It moves onto the shared transport
  in slice 04 and must produce the same request body.
- **Wrecks are untouched.** Crews are frozen into live vehicles only.

## Review gates

Every slice that produces a picture inherits these. Each slice file repeats
them as explicit steps.

1. Production-renderer evidence: `node web/asset.mjs sheet <appearance>`
   (contact views, 22 clip strips, impostor, `--side red`), plus
   `/labs/showcase?unit=<kind>&faction=<f>[&yard=1]` for squads in the game.
2. [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md),
   whenever there is a prior look or a reference photo: the candidate against
   its target.
3. [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md),
   always, as the last check: an unprimed critic with the full capture set and
   crops, and the score rubric from `make-3d-models` (no score below 6, an
   average of 7). Whatever falls short is named in the slice's notes.
4. A non-blocking human checkpoint. Open the shots with
   [preview-shots](../../.agents/skills/preview-shots/SKILL.md), keep working
   for about 5 minutes, then decide on the evidence if the user is silent.
   Record the call and its reason in the slice, close the shots, and go on.

## Cost

| Item | Per call |
|---|---|
| Input image (GPT Image 2.6 Flare, through `bento local model chat`) | $0.04 |
| Tripo H3.1 image-to-3D, HD texture and detailed geometry | $0.60 |
| Meshy 7.1, textured (fallback) | $1.20 |

One body costs $0.64 with Tripo. The 30 bodies, less the pilot's, come to
about $19, or about $25 allowing retries. The trial already spent about $8.60:
- 8 images: $0.32;
- 4 Tripo calls: $2.40;
- 2 Meshy calls: $2.40;
- one Argon round: $3.46.

Slice 09 reports actual spend from the ledger: per body, per army, total, and
per shipped soldier GLB.

## Risks

| Risk | Settled in |
|---|---|
| Carried and tripod launchers crash on a generated body: three `spine_03` back measures find no parts | 03 |
| A whip or tall ruck above the helmet shrinks the height fit | 03 |
| Dark skin takes the side tint (`_skin_like` needs a light red channel) | 03 |
| The regrade darkens skin with the cloth | 03 |
| Two Europe armies write the same file (faction prefix) | 01 |
| Paying twice; 10–20% transient generator failures | 04 |
| The generator drops the back-only kind cue | 05 (check the $0.04 image before paying for Tripo) |
| Collars, balaclavas and non-OCP prints break the fit or the regrade | 05 |
| The reference library refuses a look with only generated views | 06 |
| A team still fields mixed armies (guards, crews) | 06 |
| The load budget overflows | 09 |
| Hidden importers of the procedural code (the test jeep reads `test/rifle.glb`) | 08 |
| Mass-export wall time | 09 (parallel single-threaded Blender processes) |

## Sources

- The pilot's code: `packages/scene-assets/blender/generated_body.py`,
  `skins/infantry.json`, `textures.ATLASES`.
- Generators: the `bento` skill's fal reference, and fal's model pages for
  `tripo3d/h3.1/image-to-3d` and `meshy/v7.1/image-to-3d`.
- [assets/drafting-brief.md](assets/drafting-brief.md) is the brief the three
  independent plan drafts worked from (fewest slices, risk first, seam
  quality). This plan is their synthesis.
