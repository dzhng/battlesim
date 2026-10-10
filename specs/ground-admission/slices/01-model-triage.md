# 01 — Model triage

**Unlocks:** a ranked, per-model fix list for all twelve cards, judged in the battle
renderer beside live peers, before any catalog edit. Scratch evidence plus a short table
in this file; no tracked code changes.

**Why first:** every model is marked `reference_built` and its GLB matches its receipt,
but none has been seen in battle. This slice can invalidate the plan's assumption that the
art is near-ready, cheaply.

## Seam

- `asset sheet <glb> --references --out throwaway/ground-admission/sheets/<id>/before`
  (production battle renderer; `battle-near/mid/far` views on the game's camera curve).
  Disabled sources find references through the manifest's `source_family`.
- Sheet each peer the same way by appearance id.
- Sources: `assets/source/roster/{t14,t15,brm,type15,jaguar,challenger_3,disabled}/`,
  infantry kits under `assets/source/roster/infantry/`.

Peers:

| Card | Peer(s) |
|---|---|
| T-14 | T-90M, KF51 |
| T-15 | BMP-3, Puma |
| BRM-3K | BMP-3 (same frame), Ajax |
| Type 15, M10, CV90120 | each other, plus an MBT of the same faction |
| Centauro II | Boxer RCT30, VBCI |
| Challenger 3 | Challenger 2 TES (same frame) |
| M1E3 | M1A2 SEP v3 Trophy |
| Jaguar | Fennek, VBCI |
| Javelin, Akeron kits | TOW team, Kornet team, RPG team |

## What to produce

1. Before-sheets for the twelve and their peers.
2. A [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) montage per
   card: candidate `battle-mid` and `q-front` beside peer, plus its reference panel.
3. One unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
   per montage — the last check on each shot.
4. Fit gap: each card's exported hull extents against the frame its script states, flagged
   where it exceeds 0.1 m (the live `fit.hull_extents` tolerance).
5. A table here, one row per card: findings sorted into one variable each — frame and
   proportion, silhouette identity, paint and edge wear, hardware density, crew, wreck — and
   ranked. Only findings that break "reads as this vehicle beside its peer at play camera"
   are fixes; the rest are recorded and dropped (models are good enough; no toy-like iteration).
6. Copy the montages that decide something into `../assets/triage/`.

## Decisions this slice settles (non-blocking review)

Open the montages with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md),
wait about five minutes, then decide on the evidence, record it in
[choices.md](../choices.md), close the shots and go on:

- **T-15 turret module** sits about 0.4 m low (pivot 1.82 m, copied from the old
  manifest). Admission makes it the simulation's frame. Keep or raise.
- **M10 Booker** reads as a small Abrams. The earlier model ledger proposes exaggerating
  the nose and the turret's length-to-height ratio. Do it or not.
- **Edge wear** on Jaguar and Centauro (`chip=1.0`) against their wheeled peers (0.6).

## Delegated

Sheet framing; critique wording; which findings rank as fixes, within the bar above.

## Stays green

Nothing tracked changes.

## Would change this slice

If several models are toy-like or fail fit badly, reslice their unit slice into
"art fix" and "admit" before continuing.

## Findings (2026-10-10)

Evidence: production-renderer sheets (`asset sheet`, before) for the twelve and fifteen
live peers; one montage per card (q-front, battle-near, battle-mid ×2 beside peers, plus
the reference panel); one unprimed critique per montage. Deciding montages are in
[`../assets/triage/`](../assets/triage/).

**The bar, calibrated.** As a control, the same critique run on the live T-90M also
returned "does not read as a T-90M" ([control](../assets/triage/control_t90.png)). The
critic's bar ("a knowledgeable player names the exact vehicle") sits above what the live
roster meets. A finding is a fix only where the candidate falls below its live peer:
it reads as another class, it can't be told from its peer, or its defining feature
is absent at play camera. Everything else is recorded and dropped.

**Fit gap.** Finest tier at rest, without the gun's pitch node, the HMG's yaw node and
`dressing_*`, measured against the frame its script states (the `fit.hull_extents` rule).
Control: the live Challenger 2 TES measures 0.10 m worst against its roster box, so it
passes.

| Card | Stated L×W×H (m) | Built L×W×H (m) | Faces off by > 0.1 m (model vs box) | Worst (m) |
|---|---|---|---|---|
| T-14 | 8.7×3.5×3.3 | 8.70×3.59×3.33 | none | 0.04 |
| T-15 | 9.5×3.5×3.5 | 9.56×3.62×3.48 | none | 0.06 |
| BRM-3K | 7.14×3.2×2.4 | 7.49×3.16×2.49 | −x −3.80 vs −3.57; +x 3.69 vs 3.57 | 0.23 |
| Type 15 | 7.3×3.35×2.5 | 7.53×3.41×2.58 | −x −3.77 vs −3.65; +x 3.77 vs 3.65 | 0.12 |
| M10 | 7.0×3.4×2.9 | 7.00×3.38×2.94 | none | 0.04 |
| CV90120 | 6.95×3.3×2.75 | 7.03×3.27×2.77 | none | 0.06 |
| Centauro II | 8.2×3.12×2.75 | 8.11×3.18×2.97 | +x 3.98 vs 4.10; +z 2.97 vs 2.75 | 0.22 |
| Challenger 3 | 8.3×4.2×2.49 | 8.56×4.21×2.64 | −x −4.28 vs −4.15; +x 4.29 vs 4.15; +z 2.64 vs 2.49 | 0.15 |
| M1E3 | 7.93×3.66×2.3 | 8.05×3.65×2.50 | +z 2.50 vs 2.30 | 0.20 |
| Jaguar | 7.1×2.99×2.8 | 7.24×3.07×2.77 | −x −3.70 vs −3.55 | 0.15 |

Six of ten vehicles miss 0.1 m, none by more than 0.23 m. Each unit slice settles its own
gap at step 1 of the recipe: copy the frame, then tune it to what the art measures where
the photos allow, else bend the art.

**Ranked fixes per card.** One variable each, most important first. "Dropped" findings
were seen and are not fixes.

| Card | Fixes (ranked) | Dropped |
|---|---|---|
| T-14 | 1. Silhouette: the turret is small against a tall hull (hull top 2.43 m, turret only 0.9 m above it). At battle-near it reads as an Armata IFV, not a tank. | Thin gun; parade red stripe; skirts hide wheels; rear cage |
| T-15 | 1. Frame: raise the module (decision below). 2. Hardware: re-check after the raise that the 30 mm and the Kornet pair read at battle-near; enlarge only if they don't. | Crate-grid side seams; red stripe; blunt nose |
| BRM-3K | 1. Silhouette: the recon turret is a low slab (0.5 m over the hull), so it reads as a BMP-3 with a flattened turret. Its mass and sight box are the card's identity. 2. Frame: the tall rear corner boxes outrank the turret in profile. | Pale deck patches; blunt bow; bare rear; no crew |
| Type 15 | 1. Silhouette: the turret merges into a stepped hull, so at battle-near it matches the CV90120's profile. | Same green as CV90120/99A (faction scheme); skirts; hatch lids; the "floating object" in the top view is the scale figure |
| M10 | 1. Silhouette: exaggerate the nose and the turret's proportions (decision below). | Same tan as the Abrams (faction scheme); wheel count; engine deck layout; gun thickness |
| CV90120 | 1. Silhouette: the turret is a tall narrow box set back on the hull, with no wide wedge front, so it can't be told from the Type 15 at play camera. 2. References: the side and rear photos show CV90 IFVs, not the CV90120; replace them before the art pass. | Road-wheel count; splinter paint at range; nose shape |
| Centauro II | None. It reads as an 8×8 gun vehicle, clearly apart from the Boxer and VBCI. | Boxy turret; short bow; sparse roof; thin barrel |
| Challenger 3 | 1. Hardware: the `trophy_radar_*`/`trophy_launcher_*` nodes exist but don't read at battle-near. They are what tells it from the C2 TES there. | Low wedge turret (it is the C3's turret, not a defect at range); paint same as C2; wheel count; length |
| M1E3 | None beyond slice 09's Trophy art. It reads as a lower-turreted Abrams beside the SEPv3. | Same tan (faction scheme); two of three references are older Abrams; thin gun |
| Jaguar | 1. Frame: the hull is low with a pointed VBCI-like nose; the Jaguar is a tall, flat-sided box on big wheels. 2. Silhouette: the turret and its `akeron_box` are too small to read; that launcher pod is the card's signature. | Sand-brown photos vs three-tone (faction scheme); long gun; rear grille; no crew |
| Javelin kit | 1. Frame: the tube sits almost wholly ahead of the shoulder and the CLU blends into the camo, so the active pose reads as an RPG gunner. A Javelin rides centred on the shoulder with the CLU at the face. 2. Silhouette: the carried pose shows a rifle and no Javelin, so it reads as a rifleman. | No second crewman (team composition is slice 11's); CLU detail; tube colour |
| Akeron kit | 1. Silhouette: the uniform dark tube lacks the oversized black end caps, so it reads as a Javelin. 2. Paint: the tube is near-black where the real one is light olive with black caps. | No tripod (shoulder fire is a real mode); stride pose; no crew |

**Reslice (per "Would change this slice").** Fixes 1 on T-14, T-15, BRM-3K, Type 15,
M10, CV90120 and Jaguar are art passes on proportions, not polish. No model is
toy-like, and no fit gap exceeds 0.23 m. A recurring pattern is a turret too small or low
for its hull: four tracked vehicles and the Jaguar. Slices 06, 07, 08 and 12 should
each open with an art-fix commit (sheet → fix → one critique), then admit. 05, 09 and 11
stay as planned, with their one hardware or kit fix folded in.

### Decisions

- **T-15 turret module: raise it.** The module's pivot (1.82 m) sits 0.10 m *below* its own
  deck (`DECK` 1.92 m), so at battle-near the module barely clears the rear modules (hull
  top 2.43 m). The vehicle reads as a turretless heavy APC beside the BMP-3 and Puma
  ([montage](../assets/triage/t15.png)). Raise the pivot by about 0.4 m (to about 2.22 m;
  30 mm axis about 2.72 m, matching the real module). The frame takes the raised module.
  Today's top, 3.47 m, comes from the turret's own mast, so slice 06 either shortens that
  mast or grows the box height. Rejected: keeping it low. The simulation's eye and the
  muzzle would inherit a module that reads as missing.
- **M10 Booker silhouette: exaggerate it.** At battle-mid the M10 and the SEPv3 are the same
  tan shape a size apart ([montage](../assets/triage/m10.png)). The critique ranks the
  Abrams-recipe turret and the low nose first. Within the 2.9 m frame: raise and blunt the
  nose, and shorten and heighten the turret (lower its length-to-height ratio toward the
  photos' boxy turret). Keep the faction tan. Rejected: a green repaint, which would
  separate it from the Abrams but merge it into the green Type 15 and CV90120, and break the
  faction scheme.
- **Jaguar and Centauro II edge wear: 0.6, like their wheeled peers.** Chips don't read
  at play camera on either model (both critiques), so this is not a reading fix. Every
  live wheeled family uses 0.6 and every tracked one 1.0; at 1.0 these two are per-card
  exceptions to that class rule with nothing gained. The Jaguar also shows pale washed
  patches on its turret and side panels that the 0.6 VBCI lacks, faintly lighter at
  battle-mid. Whether the chip value causes them has not been isolated, so slice 12's
  after-sheet checks. The Centauro's pale areas are its vegetata sand, which is correct.
  ([Jaguar](../assets/triage/jaguar-surface.png), [Centauro](../assets/triage/centauro2-surface.png),
  [VBCI](../assets/triage/vbci-surface.png).) Set 0.6 at the admission re-export (slices 08
  and 12), where it costs nothing. Rejected: keeping 1.0.
