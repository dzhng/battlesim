# 05 — Catalog and presentation closeout

## Contract unlocked

All derived catalogs, parity fixtures, developer air-contract scenes, and player-facing weapon readouts reflect the hard-cut contract.

## Work

- Regenerate `fixtures/catalog.json` and any browser-derived catalogs.
- Remove `aa_tier`, unit `air_defense`, and old interception displays from `apps/battle-lab/src/routes/airWar.tsx` and `web/scenes/air-contracts.mjs`.
- Verify target classes/can-intercept only appear where the existing weapon panel contract needs them; do not add a dashboard-style panel.
- Verify full and compact cards show blocked-launch rows with the existing warning-color icon. (Landed and checked in slice 3: the `panels` scene and `web/tests/blockedTrajectory.test.tsx`.) An unprimed critique of the existing `blocked_shot` icon (2026-10-15) found it unreadable at panel size: its dashed line of fire fades out, its hatched bar blurs, its weight sits off-centre, and in amber it is easily mistaken for `no_ammo`. Redraw it under `game-ui` before calling the readout done.

## Gates

Run focused web/readout tests, then the full Rust, web, browser, catalog, and parity checks once. Use `screenshot-critique` for changed visual shots; use `preview-shots` for human review when captures are produced. Run `review` before calling the feature done.
