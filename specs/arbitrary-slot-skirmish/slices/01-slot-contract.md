# Slice 01 — slot contract and validation

Replace binary generated-skirmish setup with ordered slots. Reuse `CommandEnvelope.slot`, `UnitSetup.owner`, and existing controller transitions; do not reuse authored-bundle `PlayerSlot` directly. Validate one to five slots per side, unique IDs, controllers, deterministic order, per-slot wallet/roster ownership, and current one-faction-per-side preparation policy. Keep all-AI reusable simulation setups valid.

Write red Rust tests first; rewrite affected purchase/objective tests. No UI or visual work belongs here.
