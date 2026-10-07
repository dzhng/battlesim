# 16 Closeout of the playable game

1. `bun run check` and `bun run verify` once (the full runs AGENTS.md saves for
   the end); fix what they find.
2. Balance: the roster's numbers are untouched except the HMMWV's frame
   (decision 3), already sampled in slice 07; no full balance report.
3. Icons re-derived for every rebuilt appearance; `check` green.
4. No vehicle on an interim wreck (slice 09's seam ends here); every unit's
   tiers pass slice 09's reduction check; catalog-load bytes and texture
   layers within slice 08's limits.
5. Frame cost: a lab scene with a column of rebuilt roster vehicles and
   squads, against slice 07's baseline; within what slice 12 accepted.
6. The menu reel's exact-shot test passes (slice 04). One ordinary skirmish per faction pairing and the menu reel: screenshots at
   battle distance and close, compare-screenshots against the before sheets,
   screenshot-critique unprimed, last; show the user with preview-shots.
7. `git grep -i village` lists only the generator's settlement class;
   `git grep` for the bare old generic ids lists only the weapon `rifle` and
   the roles `at`, `recon`. No family script redefines a wheel or a track; the
   legacy family helpers are gone.
8. Update the README handoff to point at slice 17.
