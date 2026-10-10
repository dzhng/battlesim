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
