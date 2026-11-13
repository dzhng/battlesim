# Family logs

What each family's pass recorded: the user's notes it had to fix, its Argon sessions and costs, its critique scores, and what it left below the bar. The decisions behind them are in [`decisions.md`](decisions.md); each body's rounds and local edits are in its folder's README under `assets/source/skins/<family>/`.

## T-72 family

### The user's notes (the done list)

- T-80BVM: "turret seems too small".
- T-90M: "turret seems too forward".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Result (lane A, 2026-11-12)

All three wear Argon bodies through `scripted_body`, two Argon rounds each,
local edits numbered in each `assets/source/skins/<family>/argon-r2/README.md`.
Every model: lint 0 issues live and wreck, `asset validate` 0 findings, two
exports byte-identical (R6). Compare sheets in `assets/after/<family>/`.

| | T-80BVM | T-90M | ZTZ-99A |
|---|---|---|---|
| Session | jn7apfv3bda3bpmk1mj6jxqhcx8j2pyw | jn71k7ab1h76cs7dmeqg5f9vrx8j2yga | jn78ttc97afrpveddsafyh0kd98j3x2a |
| Argon cost r1 / r2 | $1.95 / $2.18 | $2.40 / $2.27 | $1.15 / $1.24 |
| Tiers (live) | 70,212 / 31,302 / 7,342 / 2,586 | 62,980 / 30,380 / 9,614 / 3,002 | 73,434 / 32,374 / 8,828 / 3,080 |
| Local edits | 1 | 3 | 3 |
| User note | turret 3.40 x 3.45 m (was 3.38 x 3.15) | turret front x = +1.52, bustle to -3.0 (front was +2.26) | (none) |

Costs include a few cents a turn of the session's helper model
(`openai/gpt-6-luna`); Argon built every round (decision LA2).

**Critique (unprimed; silhouette / surface / fittings / materials / cohesion):**

- T-80BVM: r1 6/6/5/5/6; r2 6/6/6/5/6 (avg 5.8).
- T-90M: r1 5/6/5/5/5; r2 5/7/6/5/6 (avg 5.8).
- ZTZ-99A: r1 5/7/6/5/6; r2 7/7/6/5/6; r3 7/7/7/5/6 (avg 6.4).

**Below the bar, all three.** What is left:

- **Materials (5) is the shared paint look**, as in the pilot. The critics
  read the scheme's mottle and dust as bleached glacis tops and the
  workbench's side light as near-black flanks. Measured, the new bodies are
  no darker than the procedural ones in the same shots (lane decision LA6).
- **T-90M:** the critic wants the turret moved forward and taller. Moving it
  forward would undo the user's note and the drawing. Its height is the
  frame's 2.22 m.
- **T-80BVM:** the critic reads the cast dome ringed by tilted Relikt tiles
  as an older Kontakt turret. It wants flat blunt blocks, which means
  rebuilding the turret front (more Argon or hand modelling). Its smoke
  launchers don't read.
- **ZTZ-99A:** the turret still reads a little low. Its roof is bound by the
  frame's 2.5 m top and 0.1 m tolerance, and the roof gun's 2.21 m pivot.

## Armata (T-14, T-15)

### The user's notes (the done list)

- T-15 Armata: "gun barrel is too small".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Result (lane A, 2026-11-12)

Both wear Argon bodies through `scripted_body`, two Argon rounds each, with
the local edits numbered in each `assets/source/skins/<family>/argon-r2/README.md`.
Each model has lint 0 issues live and wreck, `asset validate` 0 findings,
and two exports byte-identical (R6).

| | T-14 | T-15 |
|---|---|---|
| Session | jn7fjespaej463s31fyd06zz558j2h8q | jn7f7j32g37hv6qsgc5n4w393h8j2hk0 |
| Argon cost r1 / r2 | $2.65 / $1.32 | $2.25 / $1.17 |
| Tiers (live) | 59,862 / 33,848 / 9,038 / 3,672 | 92,028 / 33,988 / 8,572 / 2,800 |
| Local edits | 3 | 3 |
| User note | (none) | 2A42 barrel 0.112 m, jacket 0.18 m, 2.2 m showing (was 0.10 m, 1.5 m) |

The T-15's second Kornet pair is fixed turret art. The frame aims one
`launcher` mount (decision LA9).

**Critique (unprimed; silhouette / surface / fittings / materials / cohesion):**

- T-14: r1 4/5/5/5/5; r2 5/5/4/4/5 (avg 4.6).
- T-15: r1 6/6/4/5/5; r2 7/7/6/5/6; r3 7/7/6/5/6 (avg 6.2).

**Below the bar, both.** What is left:

- **The shared paint look (materials):** see slice 17.
- **T-15:** the skirts read as a tiled grid, and the nose's side cheeks as
  stacked strips. Both are Argon's geometry. The module reads small; its
  height is the frame's 3.2 m.
- **T-14, the weakest of the lane:**
  - The turret is a rounded multi-facet shell where the photos show a tall,
    slab-sided box. That is a rebuild.
  - Two sloped turret-front panels read as glass.
  - The slat cages read tan although they are painted. Their rods are so
    thin that the paint's edge-wear tone covers them whole (shared-part
    request LA-S1).
  - The panoramic sight is short, held to the frame's dressing allowance
    (2.8 m + 0.3 m).

### R12 sweep (2026-11-13, lane C's post-merge pass)

The user's rule: every turret's front sweeps back from the mantlet (R12),
with the coordinator's floor of 0.5 m back per metre outboard (decisions
F2–F4). T-15, local edit 4 (`t15/argon-r2/README.md`): each cheek now runs
back in one straight line at 0.75 m per metre (~37°). It used to run back
13° from the gun slot and then turn steeply to the side.

- **Sweep:** the coordinator's whole-turret number stays 0.13: the
  right-hand Kornet pair (fixed turret art, LA9) stands at the outer edge.
  The turret body (`armor_paint` shell, the same formula) went 0.41 → 0.66.
- **Export:** lint 0, `asset validate` clean live and wreck, two exports
  byte-identical. Tiers 91,978 / 27,476 / 8,512 / 2,744.
- **Sheet:** [`t15-compare.png`](assets/after/t15/t15-compare.png).

| Critique (silhouette, surface, fittings, materials, cohesion) | after edit 4 |
|---|---|
| T-15 | 6 5 5 4 6 |

The critic read the front as a wedge from above, and partly from the
three-quarter front. It named small grey squares floating beside the
turret's sides. Those were the flank bolts, which floated off the core
before this pass; they and the panel now sit on its surface, a change made
after the score. The rest is what lane A named: the low turret, the stubby
Kornets and the mottled paint (LA6, LA8). Still below the bar.

## ZTQ-15

### The user's notes (the done list)

- ZTQ-15: "turret shape seems wrong".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Result (lane A, 2026-11-12)

The ZTQ-15 wears its Argon body through `scripted_body`, with the local
edits numbered in `assets/source/skins/type15/argon-r2/README.md`. It has
lint 0 issues live and wreck, `asset validate` 0 findings, and two exports
byte-identical (R6).

- **Session:** jn764tmmw39kza8t15w09ar93d8j3fpa.
- **Argon cost:** r1 $3.99, then r2 $1.54, which failed on a provider error
  (`finish_reason: other`) before writing anything; resumed once for $4.74
  (decision LA3). Total $10.27, the lane's dearest.
- **Tiers (live):** 51,174 / 28,786 / 7,410 / 2,004 (`tracked_medium` caps
  90k / 34k / 10k / 3k).
- **User note, "turret shape seems wrong":**
  - Round 1's turret had the angular wedge face, but it sat 0.22 m too tall,
    0.12 m forward and on a gun axis 0.3 m low. Round 2 put it on the frame:
    ring at x = 0, roof flat at 2.50 m, gun axis 2.18 m.
  - It is now a squat slab-sided box with appliqué wedges on each cheek and
    a bustle cage, where the old model was a tall box with blocks stuck on.

**Critique (unprimed; silhouette / surface / fittings / materials / cohesion):**
r1 6/5/5/5/6; r2 6/7/5/5/6 (avg 5.8).

**Below the bar.** What is left:

- **Materials (5):** the shared paint look (slice 17); the rear face goes
  near-black in the workbench light.
- **The roof weapon station reads small** next to the photo's tall mount. It
  stands on the frame's pivot, under the 0.3 m dressing allowance.
- **The turret reads a little low.** Its roof is the frame's 2.5 m.
- **The cheek appliqué and the smoke dischargers read busy close up.**

## Challengers

### The user's notes (the done list)

- Challenger 2 TES: "turret shape wrong".
- Challenger 3: "turret shape wrong".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Lane B log (2026-11-12)

**Session:** `jn707pgsw1baychrscp48kt8nx8j26vm` ("challenger argon r1"), one
session building `build(variant)` for `challenger_2_tes` and `challenger_3`.
Brief: the common part plus `throwaway/argon/briefs/challenger.txt` (lane B's
worktree). Created 12:51; Bento's session dispatch was down workspace-wide
(503 `integration_unavailable` on create, then `lastDeliveredInputSeq` 0 on
every lane's sessions), so round 1 had not started. Cost so far: $0.

**Frame targets the brief carries** (both variants): hull 8.30 × 4.20 (over
the packs); turret roof 2.49 (the Challenger 3's box top 2.70, its sight to
3.00 as dressing); ring (0.20, 0); gun axis 1.94; muzzle x 7.40 (CR2) and
7.15 (CR3); roof gun pivot (-0.05, -0.58, 2.34).

**Before (the committed procedural GLBs, measured):** Challenger 2 turret
x -2.22..+1.98, so its front stops 2.2 m behind the nose (the real turret's
front is about +3.0); Challenger 3 turret x -2.17..+2.05. Both boxes run
0.09–0.11 m long.

### Lane B result (2026-11-13)

Both wear one Argon script, `assets/source/skins/challenger/argon-r2/build.py`
(`build(variant)`), through `roster/challenger.py` and `roster/challenger_3.py`.
Two Argon rounds: $3.27 + $0.47 = **$3.74** (Argon $3.67, Bento's helper model
$0.07; A5 passed, decision B6). Eight local edits (folder README), among them the user's
rear-deck columns removed, the Challenger 3's sight mast in the turret so its
body reaches the 2.70 m box (B9), broad `fittings` plates painted as armour
(B8), and the turret fronts swept back (R12, B10). Lint 0 issues, validate
clean, tiers inside `tracked_heavy` (61k/31k/8.6k/3.7k).

| Critique round | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| CR2 r1 | 6 | 7 | 4 | 5 | 6 | 5.6 |
| CR2 r2 | 6 | 7 | 5 | 6 | 6 | 6.0 |
| CR2 r3 | 6 | 7 | 5 | 6 | 6 | 6.0 |
| CR3 r1 | 4 | 7 | 3 | 5 | 5 | 4.8 |
| CR3 r2 | 4 | 7 | 4 | 6 | 5 | 5.2 |
| CR3 r3 | 4 | 7 | 4 | 6 | 5 | 5.2 |

**Below the bar, named:**
- **Challenger 3:** the turret is still the Challenger 2's box with a new sight
  and Trophy panels, not the faceted, chamfered turret of the generated views.
  That needs a rebuild of its turret function, which local edits can't do. A
  third Argon round, or a lane-local rebuild, is the user's call.
- **Challenger 2:** the commander's sight reads small within the 0.3 m roof
  allowance; the TES turret-side packs and the bustle cage are thinner than
  the photos.
- **Both:** a pale cheek face in the sun, the shared occlusion and paint
  contrast (the coordinator's, R13 in progress).
- **The user's note** ("turret shape wrong"): the Challenger 2's turret now runs
  x -2.30..+2.99 against the old -2.22..+1.98, about 5.2 m long, reaching
  over the glacis. It reads as fixed in `assets/after/challenger/challenger_2_tes-compare.png`.
  On the Challenger 3 it is only partly fixed (above).

## Abrams and M10

### The user's notes (the done list)

- M1E3 Abrams: "back needs to shift up, front too pointy".
- M10 Booker: "turret shape looks wrong".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Lane B log (2026-11-12)

**Sessions** (all created 12:51–12:53; Bento's dispatch was down workspace-wide,
`lastDeliveredInputSeq` 0, so no round 1 had started; cost so far $0):
- Abrams variants: `jn7ad1dwaez1skassvj1sz4ckn8j30mv`. The SEP v2's
  `argon-r2/build.py` extended with `build(variant)` for `sep_v2`,
  `sep_v2_trophy` and `sep_v3_trophy`, the SEP v2's geometry unchanged (the
  brief asks for a per-object vertex hash as proof; we diff it ourselves too).
- M1E3: `jn78bm1wsgt4afa91snv1pd7718j34q7`. Example script: the SEP v2's (B3).
- M10: `jn70jg8d7mdznefq10mm645v998j2dwf`.

**Frame targets:** the Abrams and M1E3 hull 7.93 × 3.66 × 2.44, ring (0.25, 0),
muzzle (5.805, 0, 1.903); the M1E3's RWS pivot (-0.20, +0.30, 2.42). The M10:
hull 7.00 × 3.40, box top 2.60 (sight dome to 2.89 as dressing), ring
(-0.30, 0), muzzle (5.45, 0, 2.10), roof gun pivot (-0.40, 0.62, 2.80).

**Before (measured):** M1E3 hull top 1.70 m only at the rear, and a wedge nose
(the note); the M10 turret runs x -2.34..+1.09 and z 1.60..2.68 (1.08 m tall
sides, 3.42 m long) against the photos' 0.7 m and 3.7 m.

### Lane B result (2026-11-13)

**Abrams SEPv2, SEPv2 Trophy, SEPv3 Trophy:** one script for all three,
`assets/source/skins/abrams/argon-variants-r1/build.py`, from one Argon
round on the pilot's SEPv2 script, **$1.52**. Round 2 wasn't needed (B7). `build("sep_v2")` was
proved unchanged against the pilot by a per-object geometry hash. Ten local edits (folder README): Trophy within the
roof allowance, a deterministic script, belt and stub clearances the
corrected lint found, a readable SEPv3 APU box and data link. The family
places the `trophy_aps` nodes. `abrams.py` lost its procedural body once the
M1E3 stopped importing it. Lint 0, validate clean, byte-identical twice.

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| SEPv2 (r1) | 7 | 6 | 6 | 5 | 7 | 6.2 |
| SEPv2 Trophy (r1) | 7 | 6 | 7 | 5 | 7 | 6.4 |
| SEPv3 Trophy r1 | 6 | 6 | 5 | 5 | 7 | 5.8 |
| SEPv3 Trophy r2 | 6 | 7 | 7 | 6 | 6 | 6.4 |

Below the bar, named: the paint mottling (the shared scheme recipe; R13's
matte paint lands on re-export), and the pilot's turret cheeks, which
read as a forward wedge rather than the Abrams' steep blunt faces.

**Abrams R12 sweep (2026-11-13):** the three variants' turret fronts measured
0.29–0.30 m back per metre outboard, under rule R12's floor of 0.5. The front
cheek stations sweep back about 0.75 (edit 11 in the folder README, the M1E3's
method), and the plan sweep now measures 0.64 (SEPv2) and 0.61 (both Trophy variants).
See `assets/after/abrams/<variant>-compare.png`. Lint 0, validate clean live
and wreck, byte-identical twice. Unprimed critique of the after shots:

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| SEPv2 (R12) | 7 | 6 | 7 | 5 | 7 | 6.4 |
| SEPv2 Trophy (R12) | 7 | 6 | 6 | 5 | 6 | 6.0 |
| SEPv3 Trophy (R12) | 7 | 6 | 7 | 5 | 6 | 6.2 |

The critic read "swept front cheeks" as correct. Its findings were the
shared look (olive-tinted shaded sides, which persist with textures off, so
the lighting; grey mottling on the tops; grainy shadow edges), the cheek
smoke launchers reading as flat marks at battle-near, and, at low
confidence, cheeks sharper in plan than the real turret (no top-view photo
settles it).

**M1E3:** `assets/source/skins/m1e3/argon-r2/`, two rounds, $3.85 + $4.78 =
**$8.63** (round 2 after the user's review and two replaced reference
photos). The turret runs x -2.25..+2.35, 3.5 m across, up to the 2.44 m roof.
The engine deck stands above the front deck and the nose is blunt. Both user
notes ("back needs to shift up, front too pointy"; "turret a lot longer")
read as fixed in `assets/after/m1e3/m1e3-compare.png`. Lint 0, validate clean.

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| M1E3 (r2 body) | 7 | 7 | 6 | 5 | 7 | 6.4 |

Below the bar, named: the paint (shared), dark strips on the front deck, and
the cheeks squarer than the photos'.

**M10 Booker:** `assets/source/skins/m10/argon-r2/`, two rounds, $2.31 + $0.91
= **$3.22**. The turret is set back, long and low, with a blunt front and
bustle bulk; the front is swept back from the mantlet (R12 edit). Lint 0,
validate clean, tiers inside `tracked_medium`.

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| M10 r1 | 6 | 6 | 6 | 5 | 6.5 | 5.9 |
| M10 r2 (R12 sweep) | 6 | 6 | 5 | 6 | 6 | 5.8 |

Below the bar, named. The critics want a taller turret, which contradicts the
frame's 2.5 m roof (from the photo survey) and the user's own note asking for
a lower turret, so it was kept (B13). Also named: the smoke dischargers are
flush, and there's a dark pit on the roof front. The user's note ("turret
shape looks wrong", "a small Abrams") is partly fixed: the critics now read
the turret as one blunt M10-like piece, but still say "generic".

## Leopard 2

### The user's notes (the done list)

- (No new note.) Known before the walk: the frame is 3.0 m tall against about 2.6 m in photos (`specs/done/unit-models/choices.md`).

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Lane B log (2026-11-12)

**Session:** `jn769mrz3ycr34y57q2gkjg2en8j3249`, `build(variant)` for `2a6`,
`2a7v` and `2a8`. Created 12:52; Bento's dispatch was down workspace-wide
(`lastDeliveredInputSeq` 0), so round 1 had not started. Cost so far: $0.

**Frame targets:** 2A6 hull 7.70 × 3.75, roof 2.64, box top 2.70, ring
(0.30, 0), muzzle (7.12, 0, 2.05); 2A7V and 2A8 hull 8.05 (the front module),
roof 2.64 / 2.72, box top 2.83, ring (0.125, 0), muzzle (6.945, 0, 2.05);
PERI to 3.00 / 3.13 as `dressing_peri`.

**Before (measured):** both current boxes top out at 2.76 m (2A6 0.06 over
its 2.70 box), widths 3.84 against 3.75, the 2A7V 0.15 m long.

### Lane B result (2026-11-13)

One script for all three, `assets/source/skins/leopard/argon-r2/build.py`
(`build(variant)`), two Argon rounds, $1.10 + $2.78 = **$3.88**. Round 2 put
the PERI's mast in the turret, so the 2A7V and 2A8 bodies reach their 2.83 m
box (B9), and added the arrowhead wedge armour. Local edit 1 sweeps the turret
front back (R12, B10), with the meshes cut on every piece boundary so no face
twists. The family places the 2A8's `trophy_aps` nodes. All three lint 0 and
validate clean. The known note is fixed: the art stands at its real roof
(2.64 m 2A6, 2.80 m 2A7V/2A8 against the 2.83 m box), not the old 3.0 m.

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| 2A6 r1 | 7 | 7 | 5 | 6 | 7 | 6.4 |
| 2A7V r1 | 6 | 7 | 5 | 6 | 6 | 6.0 |
| 2A8 r1 | 7 | 7 | 6 | 6 | 7 | 6.6 |
| 2A6 r2 | 6 | 5 | 5 | 6 | 6 | 5.6 |
| 2A7V r2 | 6 | 5 | 6 | 6 | 6 | 5.8 |
| 2A8 r2 | 6 | 4 | 6 | 6 | 6 | 5.6 |

Round 2 saw the sweep's first version, which twisted faces across its kink
("crumpled sheet"). The shipped sweep cuts on every boundary and adds no bent
face (probe in `decisions.md` B10). Below the bar, named: the roof weapons
read as boxes on posts, the 2A7V's front module reads thin, the wedge tops
are pale (shared paint), and thin rails at the hull's front corners read as
stray rods.

Argon's lofts carry a few quads up to 9.7 cm out of plane, measured before the
sweep (the sweep adds none). In the re-exported three-quarter-front and top
sheets they don't read as a crease at the battle camera, so they were left.

## Leclerc and KF51

### The user's notes (the done list)

- KF51 Panther: "turret barrel shape wrong".
- Leclerc XLR: no note. It must still meet the bar.

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Lane B log (2026-11-12)

**Sessions:** Leclerc `jn75q527pbvjyparz5n555s2rs8j2j25`, KF51
`jn7cfznahkd0tpse99trrgqxth8j27mx`. Created 12:52; Bento's dispatch was down
workspace-wide (`lastDeliveredInputSeq` 0), so round 1 had not started.
Cost so far: $0.

**Frame targets:** Leclerc hull 6.88 × 3.60 × 2.53, ring (0, 0), muzzle
(6.43, 0, 1.97), roof gun (-0.25, -0.58, 2.38). KF51 hull 7.80 × 3.70 × 2.50,
ring (0.30, 0), muzzle (6.40, 0, 1.95) (B4), roof gun (0.05, -0.58, 2.35).

**Before (measured):** the KF51 turret runs x -2.76..+2.92; the user's
"turret barrel shape wrong" is its wedge profile and the thin generic tube,
not its length. The Leclerc's model is 3.80 m wide against the 3.60 m frame
and 0.08 m over its roof.

### Lane B result (2026-11-13)

**KF51:** `assets/source/skins/kf51/argon-r2/`, two rounds, $1.70 + $1.42 =
**$3.12**. Four local edits: the arrowhead's prow blunted round a proud
mantlet, a raised launcher box, a larger commander's sight, and the front swept
back (R12). The 130 mm is a thick sleeve with a heavy muzzle, and the turret
a long flat faceted wedge over the glacis, so the user's note ("turret
barrel shape wrong") reads as fixed in `assets/after/kf51/kf51-compare.png`.
Lint 0, validate clean.

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| KF51 r1 | 6 | 7 | 7 | 6 | 7 | 6.6 |
| KF51 r2 | 7 | 7 | 6 | 6 | 7 | 6.6 |
| KF51 r3 | 7 | 7 | 6 | 6 | 7 | 6.6 |

Below the bar, named: the commander's sight can't grow taller within the
0.3 m roof allowance; there's a pale glare on one cheek (shared); the turret
reads a little tall-sided.

**Leclerc XLR:** `assets/source/skins/leclerc/argon-r1/`, one round,
**$2.46**. Round 2 wasn't needed (B7). Three local edits: the armour blocks painted as
armour, the cheek corners filled, and the cheek fronts swept back instead of
caving in (the user's note: "why does the side cave outwards?"). Lint 0,
validate clean, byte-identical twice.

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| Leclerc r1 | 7 | 7 | 6.5 | 6 | 6.5 | 6.6 |
| Leclerc r2 | 6 | 6 | 6.5 | 6 | 6 | 6.1 |
| Leclerc r3 (cheeks swept) | 6 | 7 | 6 | 6 | 6 | 6.2 |

Below the bar, named: the right cheek's appliqué reads as a flat slab, the
road wheels read brassy, and the turret and engine deck merge into one long
box from the side.

**Leclerc R12 sweep (2026-11-13):** the turret front measured 0.42 m back per
metre outboard, under rule R12's floor of 0.5. The cheek yaw alone couldn't
reach it: the shell's nose corner stood forward at the turret's edge. Edit 4
(folder README) ends the build with lane B's `sweep_turret_front` (B10),
0.25 per metre more on the cheeks, shell nose and corner blocks together. It
now measures 0.62. See `assets/after/leclerc/leclerc-compare.png`. Lint 0, validate clean
live and wreck, byte-identical twice. Unprimed critique of the after shots:

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| Leclerc XLR (R12) | 6 | 6 | 6 | 5 | 6 | 5.8 |

Below the bar, named. The critic's first finding is that the front is
"swept too far back; the Leclerc's is a broad, nearly flat face". That is
the user's R12 note reversed, and the note was made on this tank, so it
stands. Also named, all present before the edit: hard-edged dark facets on
the far cheek block (Argon's tapered block, in the before shot too), the
camouflage breaking at each skirt panel, the roof weapon station's gun not
reading, the appliqué reading as a stuck-on slab, and the dark shadow side
(the shared lighting).

## Air defence and rockets

### The user's notes (the done list)

- Buk-M3: "needs more detail".
- M270A2: "the pivot for the rocket launcher needs to go in the back".
- MARS II: "the pivot for the rocket launcher needs to go in the back".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Lane D record

**Sessions and Argon cost** (tier `google-recommended`; every turn's
`usageByModel` was `google/gemini-4-argon`, plus a cent or less of
`openai/gpt-6-luna` for the session title, as in the pilot; A5 holds).
Bento's dispatch was down for three hours after the creates; the sessions
were delivered at 15:57 local.

| Vehicle | Session | Round 1 | Round 2 | Total |
|---|---|---|---|---|
| Gepard 1A2 | `jn74m5sakaab67mvd77fh41b5h8j343w` | $3.74 | $1.23 | $4.97 |
| Buk-M3 | `jn76e3qxg3xxxm399wgbt10y858j3t1q` | $0.26 (provider error) + $1.99 (finished, D5) | $0.90 | $3.15 |
| M270A2 + MARS II | `jn73fbacmdh3f2p0p1vft94f5d8j234z` | $1.65 | $2.73 (provider error after writing its outputs, D5) | $4.38 |

**State** (each folder's README numbers its local edits):

- **M270A2 and MARS II: done to the export bar, below the critique bar.**
  One script, a variant each (`scripted_body.wear(..., variant=)`). The
  user's note is fixed: the launcher box hinges at the rear of the box (pin
  at x -3.21, z 1.19, the frame's pivot), its turntable under the rear;
  [`m270a2-hinge.png`](assets/after/rocket_artillery/m270a2-hinge.png)
  swings the box 35 degrees on that origin, front end over the cab. Lint 0,
  validate clean, byte-identical; compare sheets
  [`m270a2`](assets/after/rocket_artillery/m270a2-compare.png),
  [`mars_ii`](assets/after/rocket_artillery/mars_ii-compare.png).
  Critique round 1 / 2: M270A2 7/7/6/6/5, 7/7/5/7/7; MARS II
  7/6.5/6.5/6/6.5, 7/7/6/6/6. Still below: the bare hull nose (no tow
  hooks, guarded lights or fender lips), the plain cab roof and sides, and
  the box's side panels (edit 4 after round 2 paints them).
- **Gepard 1A2: done to the export bar, below the critique bar.** Its gun
  mount is now drawn by the gun rig (D6): the turret turns, both guns
  pitch as one `gun`, the folded search radar rides the turret. Its wreck
  takes a 1.2 m footprint allowance (D7). Lint 0, validate clean,
  byte-identical; [`gepard-compare.png`](assets/after/air_defence/gepard-compare.png).
  Critique 5/7/6/6/6, then 6/7/6/6/6. Still below: the turret reads low and
  flat (its roof is the frame's 2.76 m), the tracking radar small, the
  folded search radar box-like.
- **Buk-M3: done to the export bar, below the critique bar.** The layout
  is rebuilt (no cab; the turret forward with the 9S36M radar at its front
  and six canisters lying behind; the raised engine compartment at the
  rear; seven road wheels, rear sprocket) and the note "needs more detail"
  reads as fixed: [`buk-compare.png`](assets/after/air_defence/buk-compare.png).
  Lint 0, validate clean, byte-identical; tiers 67.4k / 32.6k / 9.7k / 2.5k
  inside `tracked_medium`'s caps after the coordinator's reducer change
  (R11, tier-1 target 19k). Critique 6/6/6/5/5.5, then 5/5/5/6/6. Still
  below: the radar's "pillow" rounding (the brief asked for it; the critic
  reads the photos as a chamfered box), canisters too fat with domed ends,
  the turret's stepped tiers, and the running gear's sprocket, idler and
  upper run.
- `air_defence.py` was not split (S14): wearing the bodies removes
  `gepard()`, `buk()` and `tracked_hull()`, about 170 lines; the file is
  now 16.8 KB. Pantsir, Skyranger 30 and NASAMS are untouched.
- Siblings: HIMARS x2 and the S-300 were re-exported with the rocket
  family; their committed GLBs were stale against the shared modules (the
  committed script exports the same bytes), 0 new lint issues, validate
  clean. BM-21 and Tornado-S came out byte-identical.

**Gepard R12 check (2026-11-13): no change.** The whole-turret probe scores
it negative, but that's a probe fault: the gun pods flank the turret at |y| 0.88,
so the mantlet half-width the probe reads off the guns hits its 0.8 m cap,
and it scores from there. Read band by band, the front isn't flat and doesn't
come forward. The tracking radar stands foremost (x 1.59–1.67 to |y| 0.4).
Each cheek wedge beside the radar pocket sweeps back about 0.3 m per metre
on its own face (x 1.27 at |y| 0.53 to 1.20 at 0.68). The outline from that
wedge to the turret's side falls back about 1.1 per metre (x 0.88 at |y|
0.88). Only a 3 cm lip comes forward at |y| 0.75. R12's rule is to change it only if the
front is flat or comes forward, so it stays.

## CV90 family

### The user's notes (the done list)

- CV9040C: "front side panels sticks out".
- CV90 Mk IV: "front side panels sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Lane C progress (2026-11-12)

**Delayed by Bento, then run.** Round 1 was briefed and created for every vehicle at
08:53 UTC (`--tier google-recommended`, the common brief plus a vehicle
part; LC1-LC4 in `decisions.md`). Every create returned HTTP 503
`integration_unavailable`, the sessions exist but were never delivered
(`lastDeliveredInputSeq` 0), and new creates still failed the same way: Bento's
session dispatch was down workspace-wide. It recovered at about 16:00 UTC
and the sessions then ran as briefed (results below).

| Session | Bento id | State |
|---|---|---|
| CV9040C and CV90 Mk IV (`cv90 argon r1`) | `jn7eent2jvymp2dvxk3b0bwxvs8j30rk` | integrated; Argon only (A5) |
| CV90120 (`cv90120 argon r1`) | `jn77678qa4hz4dmmja62gk8bfd8j2hbs` | integrated; Argon only (A5) |

Each brief's vehicle part is archived as `brief.md` in its round-1 folder
(`assets/source/skins/<family>/[<vehicle>-]argon-r1/`). Ready for when it
runs: the before sheets (game renderer) of the committed
GLBs, the frame targets with the lane's trunnions, and a GLB-against-frame
measuring script (lane worktree `throwaway/argon/tools/`).

### CV90120: integrated, below the bar

- **Argon:** round 1 $1.34, round 2 $1.88 (`google/gemini-4-argon`, plus
  1-2 cents of `openai/gpt-6-luna`, Bento's own helper, on every turn).
- **Export:** lint 0 issues; `asset validate` clean, live and wreck; tiers
  52,232 / 26,682 / 7,510 / 2,334 (`tracked_medium` caps 90k / 34k / 10k /
  3k); two exports byte-identical.
- **Sheet:** [`assets/after/cv90/cv90120-compare.png`](assets/after/cv90/cv90120-compare.png).
- **Local edits:** 2 (`assets/source/skins/cv90120/argon-r2/README.md`).

| Critique round | silhouette | surface | fittings | materials | cohesion |
|---|---|---|---|---|---|
| 1 (Argon round 2 as built) | 6 | 5 | 5 | 4 | 6 |
| 2 (after edit 1, the gun) | 6 | 5 | 6 | 5 | 6 |
| 3 (after edit 2, the mantlet) | 6 | 5 | 5 | 5 | 6 |

Still below the bar (average 5.4). What the critics kept naming:

- **The turret reads low and set back.** Its roof (2.40 m) and ring
  (0.3 m behind the middle) are the frame's (P9); not moved.
- **Washed-out camouflage on the top faces, metal and rubber reading
  alike:** the shared paint and specular look (the pilot's "not ours here").
- **A bare-looking rear and bow at close range:** the rear door, lamps and
  headlight clusters are built but small; the workbench's rear view is
  backlit. A `build.py` detail pass for the lane's sweep.
- After round 3 the canvas cover and bumper moved from `fittings` (which
  paints tan in the Swedish scheme) to `rubber`; not re-critiqued.

### CV9040C and CV90 Mk IV: integrated, below the bar

- **Argon:** $2.06 + $0.24 = $2.30 for both variants. Round 2's turn ended
  on a provider error after writing all its outputs.
- **The user's note, fixed for both:** the side armour stops where the
  glacis takes over, its top following the glacis line down, flush with the
  fender. See the three-quarter and side rows of the sheets.
- **Export:** lint 0 for both, from round 1 on; `asset validate` clean, live
  and wreck; two exports byte-identical.
- **Tiers:** CV9040C 53,190 / 26,428 / 6,702 / 2,074; Mk IV 45,738 / 25,656 /
  6,550 / 2,006 (`tracked_medium`).
- **Sheets:** [`assets/after/cv90/cv9040c-compare.png`](assets/after/cv90/cv9040c-compare.png),
  [`assets/after/cv90/mk_iv-compare.png`](assets/after/cv90/mk_iv-compare.png).
- **Local edits:** 2 (`assets/source/skins/cv90/argon-r2/README.md`).

| Critique (silhouette, surface, fittings, materials, cohesion) | round 1 | round 2 (after the lamps and the drum sight) |
|---|---|---|
| CV9040C | 6 6 5 5 6 | 5 6 5 5 6 |
| CV90 Mk IV | 6 5 5 5 6 | 6 5 5 5 6 |

Stopped at two rounds; still below the bar. What the critics named:

- **The Mk IV's sight mast and Spike pod read short and flat:** both already
  stand at the frame's limits (the sight at the 0.3 m dressing allowance
  over its 2.8 m box, the pod at the box's top).
- **The CV9040C's drum sight reads small.**
- **The nose reads boxy with no front cheeks or mudguards.**
- **The shared look:** the splinter print's stair-stepped edges at 2x, a
  washed-out glacis, tan `fittings`, and the plain belt.

### R12 sweep (2026-11-13)

The user's rule: every turret's front sweeps back from the mantlet, never
flat across (R12; decisions F2–F4). Local edit 3 (`cv90/argon-r2/README.md`).

- **CV9040C:** the front-cheek C-kit was flat across from the gun to |y|
  0.90 and is now swept ~25°. The coordinator's sweep (m back per m
  outboard, cheek peak to edge) went 0.70 → 0.79. It already passed the
  0.5 floor through its corner chamfer; the edit removes the flat run.
- **CV90 Mk IV:** the shell was flat across to |y| 0.84 and is now swept
  ~24° from the mantlet's edge. Sweep 0.54 → 0.54: the Spike pod at the
  outer edge sets the edge either way.
- **Export:** lint 0, `asset validate` clean live and wreck, two exports
  byte-identical. Tiers: CV9040C 53,042 / 23,554 / 6,690 / 2,062; Mk IV
  45,722 / 22,462 / 6,560 / 2,010 (`tracked_medium`).
- **Sheets:** [`cv9040c-compare.png`](assets/after/cv90/cv9040c-compare.png),
  [`mk_iv-compare.png`](assets/after/cv90/mk_iv-compare.png).

| Critique (silhouette, surface, fittings, materials, cohesion) | after edit 3 |
|---|---|
| CV9040C | 6 6 5 6 6 |
| CV90 Mk IV | 6 6 5 5 6 (after its facets were made planar) |

Both critics read the turret front as a wedge from above. Other notes:
- the Mk IV's first critic found triangle shading on the new cheek
  facets (non-planar quads), which is fixed;
- the CV9040C's dark notch behind the gun was already there before;
- the turrets read low, the sights small and the rear black (frame-bound
  or the shared look, LC9 and LC12).

Still below the bar.

## Bradley

### The user's notes (the done list)

- M2A4 Bradley: "front side panels sticks out".
- M3A3 Bradley: "front side panels sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Lane C progress (2026-11-12)

**Delayed by Bento, then run.** Round 1 was briefed and created for every vehicle at
08:53 UTC (`--tier google-recommended`, the common brief plus a vehicle
part; LC1-LC4 in `decisions.md`). Every create returned HTTP 503
`integration_unavailable`, the sessions exist but were never delivered
(`lastDeliveredInputSeq` 0), and new creates still failed the same way: Bento's
session dispatch was down workspace-wide. It recovered at about 16:00 UTC
and the sessions then ran as briefed (results below).

| Session | Bento id | State |
|---|---|---|
| M2A4 and M3A3 (`bradley argon r1`) | `jn74wyvgjabx5bxr2qkw4wy6rd8j3amq` | integrated; Argon only (A5) |

Each brief's vehicle part is archived as `brief.md` in its round-1 folder
(`assets/source/skins/<family>/[<vehicle>-]argon-r1/`). Ready for when it
runs: the before sheets (game renderer) of the committed
GLBs, the frame targets with the lane's trunnions, and a GLB-against-frame
measuring script (lane worktree `throwaway/argon/tools/`).

### Result: both integrated, below the bar

Every turn ran on `google/gemini-4-argon`, plus about a cent of
`openai/gpt-6-luna` (Bento's own helper).

- **Argon:** round 1 $3.00 (stopped on a provider error) + $1.14 to continue
  it; round 2 $0.42 (stopped on a stream error) + $0.29 to continue it.
  Total $4.87 for both variants.
- **The user's note, fixed for both:** the applique tiles and skirt panels
  stop where the glacis takes over and stay inside the hull's outline at the
  nose. See the three-quarter and side rows of the compare sheets.
- **Export:** lint 0 for both; `asset validate` clean, live and wreck; two
  exports byte-identical. The 25 mm and the TOW box now sit on the frame's
  pivots and muzzles (Argon's round 1 had the gun 0.21 m low).
- **Tiers:** M2A4 39,808 / 26,628 / 7,368 / 1,794; M3A3 42,056 / 27,336 /
  7,600 / 1,828 (`tracked_medium`).
- **Sheets:** [`assets/after/bradley/m2a4-compare.png`](assets/after/bradley/m2a4-compare.png),
  [`assets/after/bradley/m3a3-compare.png`](assets/after/bradley/m3a3-compare.png).
- **Local edits:** 1 (`assets/source/skins/bradley/argon-r2/README.md`).

| Critique (silhouette, surface, fittings, materials, cohesion) | round 1 | round 2 (after the ramp door) |
|---|---|---|
| M2A4 | 5 6 5 6 6 | 6 7 5 6 6 |
| M3A3 | 5 6 5 5 5 | 5 5 4 6 6 |

Stopped at two rounds; below the bar. What every round named:

- **The turret reads low and narrow:** the frame holds its roof at 2.565 m,
  but the real turret's bulk, its bolted armour blocks and the tall CIV on
  its mast make it dominate the profile.
- **Undersized items:** the TOW box, the 25 mm (no mantlet sleeve or muzzle
  brake) and the smoke banks.
- **The glacis tiles read as planks,** and the headlights sit low.
- **The shared look:** the shaded sides cast olive, and the belt is plain.

### R12 sweep (2026-11-13)

The user's rule: every turret's front sweeps back from the mantlet (R12),
with the coordinator's floor of 0.5 m back per metre outboard (decisions
F2–F4). Local edit 3 (`bradley/argon-r2/README.md`): the box turret's
front plates angle back 0.70 m per metre (~35°) from the mantlet's edge.

- **Sweep** (coordinator's probe, cheek peak to edge): M2A4 0.18 → 0.56,
  M3A3 0.17 → 0.64. The probe folds |y| about the hull's axis and this
  turret sits 0.2 m right of it, so its right cheek decides the edge;
  measured about the turret's own axis the cheeks run back 0.70.
- **Export:** lint 0, `asset validate` clean live and wreck, two exports
  byte-identical. Tiers: M2A4 39,776 / 23,632 / 7,342 / 1,792; M3A3
  42,024 / 24,070 / 7,590 / 1,826 (`tracked_medium`).
- **Sheets:** [`m2a4-compare.png`](assets/after/bradley/m2a4-compare.png),
  [`m3a3-compare.png`](assets/after/bradley/m3a3-compare.png).

| Critique (silhouette, surface, fittings, materials, cohesion) | after edit 3 |
|---|---|
| M2A4 | 6 6 5 6 6 |
| M3A3 | 6 6 5 6 6 |

Both critics read the front clearly as a wedge from above, and mostly from
the three-quarter front. Both named the pale disc round the turret base.
That is the hull's ring coaming, which the swept cheeks had uncovered; it
is now 0.86 m across and under the turret, a change made after these
scores. What is left is the turret's small, low mass and the plain sights
(frame-bound, LC9). Still below the bar.

## Puma and Ajax

### The user's notes (the done list)

- Puma: "body shape wrong, front side panels sticks out".
- Ajax: "front side panels sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Lane C progress (2026-11-12)

**Delayed by Bento, then run.** Round 1 was briefed and created for every vehicle at
08:53 UTC (`--tier google-recommended`, the common brief plus a vehicle
part; LC1-LC4 in `decisions.md`). Every create returned HTTP 503
`integration_unavailable`, the sessions exist but were never delivered
(`lastDeliveredInputSeq` 0), and new creates still failed the same way: Bento's
session dispatch was down workspace-wide. It recovered at about 16:00 UTC
and the sessions then ran as briefed (results below).

| Session | Bento id | State |
|---|---|---|
| Puma (`puma argon r1`) | `jn76chmjyvrn4yy1amtxm9dres8j3fzt` | integrated; Argon only (A5) |
| Ajax (`ajax argon r1`) | `jn72xwdb9cppgxhn9ysbp1cfqx8j3efa` | integrated; Argon only (A5) |

Each brief's vehicle part is archived as `brief.md` in its round-1 folder
(`assets/source/skins/<family>/[<vehicle>-]argon-r1/`). Ready for when it
runs: the before sheets (game renderer) of the committed
GLBs, the frame targets with the lane's trunnions, and a GLB-against-frame
measuring script (lane worktree `throwaway/argon/tools/`).

### Result: both integrated, below the bar

Every turn ran on `google/gemini-4-argon` (plus 1-5 cents of
`openai/gpt-6-luna`, Bento's own helper).

- **Argon:** Puma $2.71 + $5.00 = $7.71; Ajax $2.68 + $0.61 = $3.29.
- **The user's notes, fixed:**
  - **Puma, "body shape wrong":** a single sharp prow chine at about 1.1 m
    under the long upper glacis (no vertical nose plate), and the side
    modules' upper row chamfered inward to a shoulder line, so the roof
    reads narrower than the modules.
  - **Both, "front side panels sticks out":** the front modules end flush
    with the glacis and the nose corners.
  - See the compare sheets' three-quarter and side rows.
- **Export:** lint 0 for both (under 7e6877829's corrected inside test);
  `asset validate` clean, live and wreck; two exports byte-identical.
- **Tiers:** Puma 77,110 / 29,705 / 7,411 / 2,403 (`tracked_heavy`); Ajax
  54,100 / 28,332 / 7,320 / 2,266 (`tracked_medium`).
- **Sheets:** [`assets/after/puma/puma-compare.png`](assets/after/puma/puma-compare.png),
  [`assets/after/ajax/ajax-compare.png`](assets/after/ajax/ajax-compare.png).
- **Local edits:** Puma 2, Ajax 1 (each `argon-r2/README.md`).

| Critique (silhouette, surface, fittings, materials, cohesion) | round 1 | round 2 |
|---|---|---|
| Puma | 4 5 4 5 5 | 5 5 4 4 5 |
| Ajax | 5 5 4 4 5 | 5 5 5 5 5 |

Round 2 followed the local edits: the Puma's PERI drum, the Ajax's smoke
banks. Stopped at two rounds; well below the bar. What every round named:

- **Turrets reading low, plain and small:** the Puma's should be a tall
  cluttered box, the Ajax's tall and angular with a raised sight box. Both
  turrets are at the frames' roofs (3.10 m and 2.57 m), so the gap is
  their shape and clutter, a rebuild beyond local edits.
- **The Ajax's nose:** the critics see a tall, near-vertical lower plate
  between the tracks in the photos, where Argon built a long glacis.
- **The shared look:** a washed-out glacis, tan `fittings` (the Ajax's deck
  panels), dark shaded sides, and the plain belt.

## BMP and BRM

### The user's notes (the done list)

- BMP-2M: "front slopes down too much, bottom should actually slope up more".
- BRM-3K: "front slopes down too much, bottom should actually slope up more".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Lane C progress (2026-11-12)

**Delayed by Bento, then run.** Round 1 was briefed and created for every vehicle at
08:53 UTC (`--tier google-recommended`, the common brief plus a vehicle
part; LC1-LC4 in `decisions.md`). Every create returned HTTP 503
`integration_unavailable`, the sessions exist but were never delivered
(`lastDeliveredInputSeq` 0), and new creates still failed the same way: Bento's
session dispatch was down workspace-wide. It recovered at about 16:00 UTC
and the sessions then ran as briefed (results below).

| Session | Bento id | State |
|---|---|---|
| BMP-2M (`bmp2m argon r1`) | `jn7d8f6c09s3cbn1ak0h74g08s8j3amn` | integrated; Argon only (A5) |
| BMP-3 and BRM-3K (`bmp3 argon r1`) | `jn7c9yatjycn23ctyqsj2xwgzs8j38gc` | integrated; Argon only (A5) |

Each brief's vehicle part is archived as `brief.md` in its round-1 folder
(`assets/source/skins/<family>/[<vehicle>-]argon-r1/`). Ready for when it
runs: the before sheets (game renderer) of the committed
GLBs, the frame targets with the lane's trunnions, and a GLB-against-frame
measuring script (lane worktree `throwaway/argon/tools/`).

### Result: all three integrated, below the bar

Bento recovered at about 16:00 UTC and every session then ran on
`google/gemini-4-argon` (plus 1-2 cents of `openai/gpt-6-luna`, Bento's own
helper, per turn).

- **Argon:** BMP-2M $2.34 + $3.23 = $5.57. BMP-3 and BRM-3K together $3.17:
  round 1 $1.13, which stopped on a provider error, plus $1.07 to continue
  it; round 2 $0.98.
- **The user's notes, fixed:** the nose stands high and the underside rises
  to it. BMP-2M nose 1.21-1.24 m (was 0.95), upper glacis about 10 degrees.
  BMP-3/BRM-3K nose 1.39-1.44 m (was 0.95-1.04), lower plate about 58
  degrees, upper glacis about 11 degrees. See the side rows of the compare
  sheets.
- **Export:** lint 0 for all three; `asset validate` clean, live and wreck
  (the wrecks under R10's 1 m rubble floor); two exports byte-identical.
- **Tiers:** all inside `tracked_medium`.
  - BMP-2M 79,694 / 30,770 / 7,845 / 2,038.
  - BMP-3 60,962 / 28,796 / 7,664 / 1,752.
  - BRM-3K 59,392 / 28,520 / 7,482 / 1,770.
- **Sheets:**
  - [`assets/after/bmp/bmp2m-compare.png`](assets/after/bmp/bmp2m-compare.png)
  - [`assets/after/bmp/bmp3-compare.png`](assets/after/bmp/bmp3-compare.png)
  - [`assets/after/brm/brm3k-compare.png`](assets/after/brm/brm3k-compare.png)
- **Local edits:** BMP-2M 2, BMP-3/BRM-3K 5 (each `argon-r2/README.md`).

| Critique (silhouette, surface, fittings, materials, cohesion) | round 1 | round 2 | round 3 |
|---|---|---|---|
| BMP-2M | 6 5 5 5 6 | 5 5 4 5 6 | 6 5 5 6 6 |
| BMP-3 | 5.5 6 5 5 6 | 6 5 5 5 6 | 5 5 4 4 5 |
| BRM-3K | 5 5 4 5 5 | 6 6 5 5 6 | 6 5 4 4 6 |

Round 1 was Argon's round 2 as built. Each later round followed one set of
local edits:

- BMP-2M: round 2 after the Kornet covers came off; round 3 after the
  right-hand pair was added.
- BMP-3: round 2 after the 2A70 and the trim vane; round 3 after the taller
  sights.
- BRM-3K: round 2 after the radar housing and the plain sides.

Still below the bar. What every round kept naming:

- **The shared look:**
  - `fittings` paints tan in the Russian scheme, so Argon's large fittings
    (stowage lids, grille slats, side boxes) read as wood or cardboard;
  - the top faces wash out pale;
  - the paint's mottle reads as smudges;
  - the belt has no links (P4).
- **Turret detail Argon left plain:** the BMP-3's mantlet and sights, the
  BRM-3K's smoke tubes and searchlight, the BMP-2M's sight heads. These are
  `build.py` detail passes for slice 30's sweep.
- **Faceted shading** on the BMP-2M's bow sides; and the idlers and
  sprockets hidden under the nose and deck (BMP-3/BRM-3K).

### R12 sweep (2026-11-13)

The user's rule: every turret's front sweeps back from the mantlet (R12),
with the coordinator's floor of 0.5 m back per metre outboard (decisions
F2–F4).

- **BRM-3K**, BMP-3 edit 6 (`bmp/bmp3-argon-r2/README.md`): the octagon's
  front was flat across ±0.56 and now runs back ~32° from the mantlet.
  Sweep 0.45 → 0.61.
- **BMP-3**, edit 6: the dome already swept. Its two smoke clusters stood
  forward at the outer corners, pointing in toward the gun. They now sit on
  the dome's curve, pointing 5° out. Sweep 0.16 → 0.66.
- **BMP-2M**, edit 3 (`bmp/bmp2m-argon-r2/README.md`): the cheek plates were
  yawed with their outer ends forward and now yaw back. The cone draws back
  toward its sides, and the smoke clusters moved inboard.
  - The whole-turret number stays −0.21: the AG-30 grenade launcher's barrel
    (turret art) reaches forward at the right edge.
  - The body (`armor_paint` shell, the same formula) went 0.44 → 0.56.
- **Export:** lint 0 for all three, `asset validate` clean live and wreck,
  two exports byte-identical. Tiers:
  - BMP-2M 79,682 / 26,788 / 7,843 / 2,038;
  - BMP-3 60,962 / 25,174 / 7,664 / 1,752;
  - BRM-3K 59,450 / 25,062 / 7,448 / 1,770.
- **Sheets:** [`bmp2m-compare.png`](assets/after/bmp/bmp2m-compare.png),
  [`bmp3-compare.png`](assets/after/bmp/bmp3-compare.png),
  [`brm3k-compare.png`](assets/after/brm/brm3k-compare.png).

| Critique (silhouette, surface, fittings, materials, cohesion) | after the R12 edits |
|---|---|
| BMP-2M | 6 4 5 5 6 |
| BMP-3 | 6 5 5 5 6 |
| BRM-3K | 6 5 6 5 6 |

Every critic read the front as passing the rule from above. The BMP-2M and
BMP-3 still read as round turrets (they are cone and dome castings), with a
blunt nose at the mantlet. The bright facet on the BMP-3's front-left was
there before. What is left is the shared look (mottled, grimy paint and tan
fittings, LC12) and turrets lower and plainer than the photos (LC9). Still
below the bar.

## SP howitzers

### The user's notes (the done list)

- M109A7, PzH 2000, 2S19M2, PLZ-05 (each): "turret shape looks too big, or body too small. front side panel sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

### Lane D record

**Sessions and Argon cost** (tier `google-recommended`; every turn ran
`google/gemini-4-argon`, plus a cent or less of `openai/gpt-6-luna` for the
title; A5 holds):

| Vehicle | Session | Round 1 | Round 2 | Total |
|---|---|---|---|---|
| M109A7 | `jn7689cr9ja542wdqp81ff712h8j2nd9` | $2.43 | $2.17 | $4.61 |
| PzH 2000 | `jn75evf0d314g4damq9m6z75es8j3e3v` | $3.82 (stopped before its reply) | $1.73 | $5.56 |
| 2S19M2 | `jn7073hv5r3ywdqdm1yh6m0s4h8j295s` | $2.11 | $1.51 | $3.63 |
| PLZ-05 | `jn790akgaj0cn20b010s9hvp798j3z2e` | $0.59 | $0.07 | $0.67 |

**The note, as numbers in each brief** (frame and survey, question 3):

| | body length | turret (x from, to) | deck | roof | turret over deck | trunnion | world muzzle |
|---|---|---|---|---|---|---|---|
| M109A7 | 6.80 (bustle overhangs the rear plate ~0.6) | -3.40 to +1.10 | 1.64 | 2.80 | 1.16 | (0.70, 2.27) | x 6.70 |
| PzH 2000 | 8.33 (overhang ~0.4) | -4.165 to +0.60 | 1.80 | 3.06 | 1.26 | (0.60, 2.70) | x 7.70 |
| 2S19M2 | 7.15 | -2.46 to +1.53 | 1.29 | 2.82 | 1.53 | (1.35, 2.19) | x 8.34 |
| PLZ-05 | 7.30 | about -3.40 to +0.70 (unmeasured, from the quarters) | ~1.55 | ~3.0-3.1 | ~1.5 | (0.50, 2.32) | x 7.95 |

Each brief also carries the flush-skirt rule and asks for the turret against
the hull to be checked in the side render against the side photos. The guns
and turrets are hull art (no mount role): `turret_body` and `gun_body` under
`hull`, the roof machine guns `dressing_*`.

**State.** Every gun is drawn by the gun rig (D6): the turret never turns
(the mount is fixed in the hull) and the barrel pitches on the frame's pitch
node. All four: lint 0, validate clean (live and wreck), byte-identical,
tiers inside the caps, the note visibly fixed in the compare sheet (the
hull's deck and the turret's length and height now follow the numbers
above; the front skirt runs flush into the fender). `sp_howitzers.py` lost
its procedural bodies. Each folder's README numbers its local edits.

| Vehicle | Critique rounds (silhouette / surface / fittings / materials / cohesion) | Bar | Still below |
|---|---|---|---|
| [M109A7](assets/after/sp_howitzers/m109a7-compare.png) | 6.5/7/6/5/6; 6/7/5/6/6; 7/7/6/6/6 | below (avg 6.4) | the travel lock reads as sticks, a plain brake, flat roof hatches; the paint was fixed after round 3 (8a3995b96) |
| [PzH 2000](assets/after/sp_howitzers/pzh2000-compare.png) | 7/7/5/5/6; 7/8/6/7/7 | **meets** | the turret's angled front cheeks, a bare rear |
| [2S19M2](assets/after/sp_howitzers/2s19m2-compare.png) | 4/6/5/5/5; 6/7/5/6/6; 5/7/5/6/5 | below | the turret's inward-leaning sides and rear overhang with the conveyor arm, the bow's upturn, the brake |
| [PLZ-05](assets/after/sp_howitzers/plz05-compare.png) | 7/7/7/6/7; 7/7/6/6/7 | below (avg 6.6) | the turret front reads as a tank's wedge, a box-shaped brake |

The paler front decks the critics flagged on the M109A7 and PzH 2000 are
the shared paint's dust film on low upward faces, not these bodies.

**R12, swept turret fronts (2026-11-12).** Measured from above on each
turret's armour shell (gun excluded), the foremost x per |y| band:
- **M109A7: changed** (its edit 9). It was flat across ±0.82 m (x 1.01); now
  a ±0.35 m mantlet band, then faces back to x 0.76-0.86 at |y| 0.8.
- **PzH 2000:** already swept: 0.47 at the mantlet, 0.06 at |y| 0.8, -0.17 at 1.0.
- **2S19M2:** already swept: 1.28 at the mantlet, 0.88 at |y| 1.0, 0.49 at 1.6.
- **PLZ-05:** already swept: 0.71 at the mantlet, 0.28 at |y| 0.8, -0.04 at 1.0.
- The Gepard (slice 24) is already swept too: 1.41 at the tracking radar, 1.09 at |y| 0.6.

**R12 floor, measured (2026-11-13).** Rule R12's floor is 0.5 m back per metre
outboard, scored in plan from the cheek's foremost point beside the mantlet to the
turret's edge (whole-turret probe, gun and dressing excluded).
- **M109A7: 0.45 to 0.58** (edit 10): the cheek corner moves back to x 0.30
  (base) and 0.20 (roof), its edges parallel; the smoke launchers, periscope
  hoods and lower trim follow. Lint 0, validate clean live and wreck,
  byte-identical twice. Critique of the after shots: 7/6/6/6/7 (6.4), below the bar. The critic
  named painted-on roof and glacis hatches, a small gun cradle and olive-tinted
  shaded sides (the shared lighting). It also called the front narrow and pointed
  from above, against the front three-quarter photo's broader face.

- **PzH 2000: front plates swept** (edit 4): the nose plates beside the
  mantlet ran flat across ±0.64 m. They now sweep back 0.6 per metre, and
  the front's plan line falls from x 0.58 at the mantlet to 0.18 at the
  cheek corner (|y| 1.04), 0.57 per metre. The whole-turret probe reads
  **0.38 before, 0.27 after**, because the smoke dischargers (Argon's,
  unchanged) stand about 0.15 m proud of the cheek's outer corner at |y|
  0.93–1.32, so they are the foremost thing outboard once the plates sweep.
  The photos put the dischargers on the front plates beside the mantlet;
  moving them is outside this edit and is named here. Lint 0, validate
  clean live and wreck, byte-identical twice. Critique, one unprimed critic
  each with the same prompt: after 7/6/6/5/6 (6.0), before (the committed
  model) 6/5/5/5/6 (5.4). So the drop from lane D's last 7/8/6/7/7 is critic variance and the
  shared changes since, not the sweep. Neither critic flagged the plates. The
  findings both named were camouflage that stops at panel seams, a near-black
  shaded flank and rear, a roof tile grid that reads as painted, and a small
  mantlet. The after critic also called the front upright in the side view,
  and the before critic called it tapered in plan.

## The Argon pilot: T-72B3 and Abrams SEP v2

### Contracts

- **A unit's build script is a source.** `build.py` lives with its rounds in
  `assets/source/skins/<family>/argon-r2/`. Run alone
  (`OUT_DIR=<dir> blender -b -P build.py`), it renders and exports. Imported,
  its `build()` builds the vehicle into the caller's scene. It names each
  part by what moves it: `hull`, `turret`, `gun`, `hmg`, `hmg_gun`, `wheel_*`,
  `track_L`/`track_R` and `dressing_*`. It paints with the seven slots: `armor_paint`,
  `steel`, `rubber`, `optics`, `fittings`, `lamps` and `recesses`.
- **`scripted_body.wear(v, path, link_pitch, variant=None)`** (new; `variant`, the id a
  several-variant script's `build(variant)` takes, added for the rollout,
  `packages/scene-assets/blender/scripted_body.py`) runs the script and fits
  it to the export contract every procedural build keeps. `scripted_body.wreck`
  is a worn body's damage.
- **The family receipt** hashes every repository source the export loaded,
  the build script included (`vehicle_export._receipt`). Its `authoring`
  line names scripted bodies.
- **`vehicle_export.Vehicle.looks`** carries the family's `materials`
  keywords, and `vehicle_export.CHIP` names the default edge wear.
- **Unchanged:**
  - the GLB node contract (`turret`, `gun`, `muzzle`, `hmg`, `hmg_gun`,
    `hmg_muzzle`, `wheel_*` with `radius_m`, `track_L/R` with `track_length_m`
    and `link_pitch_m`, `dressing_*`, `_LOD0..3`);
  - frames, fixtures and the simulation, so no battle digest can move;
  - the lint baseline, except that its walk drops the two procedural models'
    entries (P13).

### What shipped

| | T-72B3 | Abrams SEPv2 |
|---|---|---|
| Tiers (live) | 78,364 / 31,656 / 7,722 / 3,078 | 57,682 / 32,358 / 10,240 / 3,720 |
| Tiers (wreck) | 97,468 / 34,228 / 7,358 / 2,878 | 77,352 / 35,502 / 9,362 / 3,392 |
| Caps (`tracked_heavy`) | 120k / 45k / 13k / 4k | same |
| Lint | 0 issues | 0 issues |
| `asset validate` | 0 findings | 0 findings |
| Local `build.py` edits | 13 (folder README) | 8 (folder README) |

- `asset bake` and `asset check` pass, and so do the Blender-side Python
  tests. All 276 tests in `web/tests/sceneAssets` pass, `unitFit` among them.
- **Catalog load:** 598.1 MiB for the game page, and 636.8 MiB for every
  unit's art against the 640 MiB limit. Slice 16 owns raising the limit (Q3).
- **To make the bake green,** the families whose slice-15 frames were never
  re-exported were exported (P14). The Abrams family's two procedural
  siblings were also re-exported, because the family exports as one (P12).

**Shots.** The game renderer, through the model workbench (`asset sheet`):
- [`assets/after/argon-pilot/t72-compare.png`](assets/after/argon-pilot/t72-compare.png)
- [`assets/after/argon-pilot/abrams-compare.png`](assets/after/argon-pilot/abrams-compare.png)

Each sheet shows the before (procedural), the after and the reference, from
the three-quarter front, the side, the front and the three-quarter rear.

### Critique (unprimed, after the fixes below)

| | silhouette | surface | fittings | materials | cohesion |
|---|---|---|---|---|---|
| T-72B3 | 6 | 6 | 6 | 5 | 5 |
| Abrams SEPv2 | 7 | 6 | 6 | 4 | 5 |

Neither model reaches the bar yet (no score under 6, an average of at least
7). What the critique found, and what came of each:

- **Fixed, ours:**
  - the T-72's V splash board lay pitched by the whole glacis slope, like a
    loose plank (build.py edit);
  - the Abrams' gun bands read as bare black rings (now paint).
- **Fixed before the critique, found in the game shots:**
  - Argon's convex edges chipped whole small blocks pale. The edge wear is
    now scaled to the procedural parts' (P8).
  - Painted hardware read as bare steel (P6).
- **Not ours here, the shared look:**
  - The paint's mottle and dust come from the scheme's texture recipe, the
    same as on every procedural tank. "Clean factory paint" is a recipe
    question for the whole roster.
  - The olive cast of shaded tan faces is the workbench's lighting and
    grading. The procedural Abrams shows the same.
- **The belt looks plain:** a smooth band, with links only in the game's
  scrolling texture. Every tracked unit has this, and it is the cost of P4.
- **The Abrams "see-through gap" over the wheels** shows only in the Blender
  closeup renderer, which draws the `recesses` slot white. The game shows a
  dark wall. It also closed a real gap: the procedural Abrams has one under
  its skirts.
- **Open, low confidence:**
  - the T-72's smoke-tube cluster may sit wrong against photos;
  - a soft "saucer" foot round its dome;
  - a shading smear on the Abrams' rear turret flank, seen in matcap only.

  Each is a `build.py` edit for the family lane.

### The recipe for every other tracked unit

### 1. Brief Argon (one Bento session per unit, `--tier google-recommended`)

Check the first turn's `usageByModel` (A5). The brief carries:

- **The references:** the unit's photos from `assets/references/<family>/`,
  and our current GLB as "our current model, for proportions only".
- **The measurements, from the frame** (`fixtures/catalog.json` and
  `specs/tracked-armour-models/assets/frames/<family>.json`):
  - the hull box (length, width, height), centred on x = 0 and y = 0 and
    standing on z = 0;
  - each mount's pivot and its world muzzle (pivot + `muzzle_m`);
  - the roof height.
  Give the current, post-slice-15 numbers (A3's lesson).
- **The structure:**
  - module-level helpers;
  - `def build():` builds the vehicle into the current scene with no reset,
    render or export, and returns its root;
  - `def main():` resets, builds, renders five views (three-quarter front,
    side, front, top, three-quarter rear), saves `<unit>.blend` and exports
    `<unit>.glb` to `os.environ["OUT_DIR"]`;
  - `if __name__ == "__main__": main()`;
  - no randomness without a fixed seed.
- **The part names:**
  - `hull`, with everything fixed to it;
  - `turret` (origin at the ring);
  - `gun` (origin at the trunnion);
  - `hmg`, the roof gun's turning base, and `hmg_gun`, the part that pitches;
  - one `wheel_<side>_<name>` object per wheel that turns: every road wheel,
    sprocket, idler and return roller, each with its origin on its axle;
  - `track_L` and `track_R`;
  - `dressing_<name>` for sights and sensors standing over the roof line.
- **The slots,** and only these seven:
  - `armor_paint`: every painted surface, bolts and hinges included;
  - `fittings`: dark painted hardware;
  - `steel`: bare metal only (track pins, tow cables, sprocket teeth, MG
    barrels, the muzzle);
  - `rubber`, `optics`, `lamps` and `recesses`.
- **What to leave out:** antenna whips (we add ours), crew, open hatches,
  markings and weathering.
- **The clearances:**
  - nothing of the hull, skirts or stowage inside the track run's box
    (between the hull's side and the skirts, from the ground to the run's
    top);
  - nothing of the hull within the turret's swept radius above the ring;
  - axle stubs end at the hull's side;
  - road wheels, return rollers, sprocket and idler don't overlap side-on;
  - the run's top stays under the sponsons.
  - the skirt's front panel is flush with the front fender: one outer plane
    and a continuous top edge, nothing of the skirt outboard of, above or
    ahead of the fender, and appliqué stopping where the glacis takes over
    (the user's "front side panels stick out" note; the T-72B3's edit 14).
- **The limits:**
  - under 80k triangles;
  - edges as two-segment bevels or sharp, not single-segment bevels, which
    read as full edges to the paint's wear (P8).

### 2. Two rounds (A6)

- **Round 1** builds it.
- **Measure the GLB** against the frame: the hull box, turret extents, each
  pivot and world muzzle, the roof. Run the pilot's integration once (step 4)
  and send what it flags: lint pairs, `asset validate` findings, tier counts.
- **Round 2** takes those as numbered fixes.
- **Archive both rounds** in `assets/source/skins/<family>/argon-r1/` and
  `argon-r2/`: `build.py`, `model.glb` (renamed from the exported
  `<unit>.glb`), `notes.md`, `renders/`, and the comparison sheet, if the
  session made one.

### 3. Local edits (no more Argon)

Patch `build.py` until it keeps the contract above. The pilot's edits are
the catalogue of what to expect:
- splitting fused wheels and roof guns out of the parts they were built into;
- moving the roof gun onto the frame's pivot;
- running gear and stowage clear of each other;
- painted hardware off `steel`;
- sights into dressing.

Number each edit in the folder README. Rebuild it standalone into
`throwaway/` and copy back `model.glb` and the renders.

### 4. Integrate

- **The family script** (`roster/<family>.py`):
  - add `BODY` (or `BODIES` by variant id) and the track's `LINK_PITCH`;
  - `build` makes the rig (`rig(v.frame, v.root, trunnion=...)`), calls
    `SB.wear(v, BODY, LINK_PITCH)` and places the whip antennas
    (`RF.antenna` under `dressing_antenna_*`);
  - `wreck` calls `SB.wreck(v, side, wheels)`.
- **Delete the procedural code** only that unit used. Keep what other
  families import (the T-72's hull for the T-80, T-90 and Type 99, the
  Abrams' for the M1E3) until they wear their own.
- **Export:**
  `bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/<family>.py`,
  then the same with `-- --wreck`. Fix every lint issue in `build.py`. To
  find where a pair meets, build the variant in Blender and test each part's
  vertices against the other with `vehicle_lint._is_point_inside_mesh`.
- **Check:**
  - `asset validate` on the GLB: fit and dressing findings are `build.py`
    edits; frames never move;
  - `asset sheet <glb> --references` for game-renderer shots beside the
    photos;
  - an unprimed critique, up to three rounds.
- **Finish:** `asset bake`, `asset icons`, `asset check`, then
  `web/tests/sceneAssets`.

### What a unit lane touches

- `assets/source/skins/<family>/argon-r{1,2}/`
- `packages/scene-assets/blender/roster/<family>.py`, deleting its now-dead
  procedural code
- that family's `assets/source/roster/<family>/` GLBs and receipt
- its icon
- the spec's `assets/after/<family>/` and its slice notes

The coordinator owns `scripted_body.py`, `vehicle_export.py`, the lint
baseline, the bake and the runtime.

### Cost and time per unit

- **Argon:** the pilot's two rounds cost $5.50 (T-72B3) and $5.31 (Abrams).
  The second round is about a third of the first. Expect $5–6 a unit, or
  $6–8 where a round must also build a sibling's variant (P12). For the 35
  remaining units, that is roughly $200–250.
- **Wall-clock:** one to two hours of Bento session per unit for two rounds.
- **Local edits and integration:** two to four hours of agent time per unit.
  The pilot took longer, because it built the integration once.
  - Lint clean-up is the bulk: the T-72 needed 12 geometry edits, the
    Abrams 8.
  - A brief that carries the clearances and slot rules above should halve
    that.
