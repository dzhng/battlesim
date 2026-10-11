# 02 — Recon and operations

This slice turns doctrine intent into battlefield movement that uses the game's
physical sensing and movement rules.

## Recon posts

For each recon unit, score reachable candidate posts around the selected
approach/objective. Use the unit's published `UnitSight.range`, sight shape,
eyes, concealment, facing, known obstacles, and side-scoped sight query. Prefer a
concealed post that observes the approach at the sensor envelope and remains
connected to the next mission. If no full-range post is legal, fall back in a
fixed order to the nearest legal standoff, then the objective/base behavior.

Recon must not be sent to the objective center by default. It remains an
objective-capable combat unit when it needs to hold ground; sensor value and
capture value are separate task bids.

## Operations

Assign task groups with stable ownership and hysteresis. Recon screens ahead of
assault groups, air-denial units protect objectives/logistics, AT units take
covered firing positions, infantry captures/holds, and logistics stays behind
the combat line and deploys. Use existing automatic weapon selection for routine
fire; issue explicit attack orders only for useful focus or pursuit.

## Verification

- Pure tests assert recon post distance, sight envelope, facing, concealment and
  blocked-route fallback.
- An analytic or saved battle proves recon identifies contacts before the main
  assault reaches the objective.
- Integration covers objective holding, helicopter non-capture, supply movement,
  retreat, and command-rate limits.
- A focused browser battle is optional evidence; if a screenshot is produced,
  run `compare-screenshots` against the declared baseline and finish with an
  unprimed `screenshot-critique` before accepting it.

