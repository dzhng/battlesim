# Decisions

Preserve existing mechanics and public behavior while correcting audited violations. No compatibility or migrations for this unshipped implementation. Escalate unnecessary complexity to the user before implementing that issue.

- Selection state is reconciled when published own units disappear, before handlers commit. The same selected-unit lookup feeds HUD and capabilities. Click recognition owns click-driven selection changes; external selection explicitly resets it, replacing inferred selection-key tracking.
- Appearance replacement may wait for an earlier replacement. There is no product promise to cancel an earlier load. Serializing the whole install/impostor transaction at the frame owner avoids cancellation callbacks through multiple GPU layers and does not add work to rendering frames.
