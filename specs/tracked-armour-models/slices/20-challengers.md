# 20: Challengers

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane B, first. Branches from the rollout plan commit.

**Models:** Challenger 2 TES (`challenger.py`), Challenger 3 (`challenger_3.py`).

## The user's notes (the done list)

- Challenger 2 TES: "turret shape wrong".
- Challenger 3: "turret shape wrong".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- One session builds both as `build(variant)`: the Challenger 3 is a Challenger 2 hull under a new turret. If round 1 shows the hulls differ more than that (running gear, armour packs), split them, at no more than two rounds each.
- The half-width stays at the 2.1 m cap (`limits.rs:21-24`). Wider side armour packs ride as `dressing_*` within the allowance, or are named.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.

## Lane B log (2026-11-12)

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

## Lane B result (2026-11-13)

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
