# 05 — Catalog and presentation closeout

## Contract unlocked

The remaining player-facing part of the contract is a readable blocked-launch warning. The catalog regeneration, parity updates, AA-role deletion, and full/compact row assertions already landed with slices 1–3.

## Work

- Redraw the existing `blocked_shot` icon under `game-ui`. An unprimed critique (2026-10-15) found its dashed line of fire fades out, its hatched bar blurs, its weight sits off-centre, and its amber treatment is easily mistaken for `no_ammo`.
- Keep the existing full/compact row contract and `BlockedTrajectory` reason; do not add a new panel, label, or dashboard surface.

## Gates

Run the focused panel/readout test after the icon change, then run the full Rust, web, browser, catalog, and parity checks once. Use `screenshot-critique` as the last visual check and `preview-shots` for human review when captures are produced. Run `review` before calling the feature done.
