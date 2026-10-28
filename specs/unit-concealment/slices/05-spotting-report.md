# 05 Spotting report

**Unlocks:** the balance tool. One command prints how far each observer group identifies each target group, from the real catalog and rules, through the simulation's own sensing functions, so it can't drift.

## Seam

- `crates/sim/examples/spotting_report.rs`, following `crates/sim/examples/README.md`.
- Load through `sim::fixtures::game()` (one parser).
- **Rows** group target units by identical `concealment`, listing their names. Each row has open, forest and garrisoned cells, each still and moving.
- **Columns** group observers by identical (`ground_m × sight_shape.front`, `spotting`).
- **Cells** are `sight × effective(target(unit, ground(...), moving))`, in metres.
- Skip airborne targets. Grouping lives only in the example.
- **Output:** writes `throwaway/spotting/report.html`, a matrix like the walk's mock (heat-shaded, units listed per row), and prints a text table.
- **The header states its assumptions:** front arc, clear line of sight, no foliage along the line, and garrisoned at full shelter.

## Human can see

Run `cargo run -p sim --release --example spotting_report` and open the HTML.

## Verification

- Run it on the slice 04 data. Every cell must match today's behaviour, for example forest infantry at a quarter of sight and recon reaching tanks in forest at full sight.
- Then run the browser parity tests once (`exactNumbers`, `skirmishObjectives`, `skirmishPurchases`, `mapLayout`). This closes the neutral half of the spec.

## Delegated to you

HTML layout and styling (keep it self-contained), and argument flags. Keep it fast to rerun.
