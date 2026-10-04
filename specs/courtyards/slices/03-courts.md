# 03 Dense blocks paved

**Unlocks:** the courts: where the amenities stand and what the ground looks like between a dense block's buildings.

**Seam:** new `crates/mapgen/src/parcels/courts.rs` owns court rings. For each district whose presets set `props.courts.paved`, the court is the district ring, drawn in to the last lot's rear along any edge that carries no carriageway (`space::clip` tells those edges apart). `parcels::fill` adds each court as a `Paving` polygon after the aprons and keeps the rings in the plan (`MapPlan.courts: Vec<CourtPlan { id: "<district>/court", district, ring }>`, plan-only) for slice 05.

**Run/see:** the in-game town shots of the before set (`town-120-low`, `court-a-30-low`, `junction-45`) per region, now with paved blocks; `mapgen inspect` crops of one settlement.

**Verify (tests first, `crates/mapgen/tests/parcels.rs` or a new `courts.rs`):** each court is a simple ring inside its district; no court in a garden suburb, village, farm or park; streets draw over courts; plans stay deterministic and within `max_ground_points`; `layout_sweep` admits as many maps as before. Visual: screenshot-critique; compare-screenshots against [the before shots](../assets/before/).

**Review checkpoint (non-blocking):** do the paved areas match "between the buildings"? Is the edge against fields right? Farm and industry calls (README, decision 1). Open with preview-shots, wait ~5 min, decide, record.

**Delegated:** the inset rule's exact distance; whether a block whose lots are all open is paved.
