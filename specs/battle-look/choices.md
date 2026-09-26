# Choices ledger

Decisions made during the build where a slice was silent, per [audit-choices](../../.agents/skills/audit-choices/SKILL.md). Each entry: when, the choice, the gap, the reach, a verdict and a confidence.

## Slice 00

- **HE's armour fraction is 0.15 (12 damage per hit, against AP's 40).** *Gap:* the user said "partial, not nearly as effective as AP" but gave no number. *Reach:* how long a tank that has run out of AP takes to kill another tank. Tunable in `fixtures/village.json`. *Verdict:* sound, provisional. *Confidence:* medium.
- **The probe forces a redraw every frame.** *Gap:* the viewport only draws when something changes, so an idle measurement reports vsync, not render cost. *Reach:* row 0 is an upper bound on per-frame work. *Verdict:* sound. *Confidence:* high.
- **`village-perf` is its own fixture at `/battle/village-perf`, reusing the village page.** *Gap:* the router matches exact paths, and a production-build verdict must be its own fixture. *Reach:* one short-lived fixture, route and scene, all deleted by slice 10. *Verdict:* sound. *Confidence:* high.
- **Texture bytes are not sized in row 0.** *Gap:* the allocation tracker counts textures but does not size them, and the flat renderer has only two. *Reach:* one blank column until the resource registry lands in slice 12, which should size them. *Verdict:* sound, provisional. *Confidence:* medium.
