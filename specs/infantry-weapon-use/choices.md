# Implementation choices

## Sound — planning decisions

### Physical aim rather than shared rifle aim (high confidence)

When one AT operator puts away his rifle, only his aiming work is abandoned; his guards keep their progress. Aim therefore belongs to each physical Cycle, while the shared Lock remains the targeting owner. The alternative would reset all riflemen or retain stale aim for the switcher. This constrains all future infantry weapon use, and gives vehicles the same physical timing owner without infantry exclusivity.

### Stable useful-weapon priority (high confidence)

A launcher aimed at a legal effective target remains useful while reloading. Selecting the currently ready rifle instead would pause the launcher every tick and prevent completion. Guidance takes priority, then the assigned special's valid engagement, then the rifle; idle time permits reload work. This is a game policy, not a rate-of-fire optimization. Existing spare assignment is retained because target-aware switching among recovered spares was explicitly outside the accepted scope.

### Whole-body equipment sets (high confidence)

The existing assets combine body, weapon and sockets. A carrier gets launcher-held or rifle-held/launcher-on-back variants, indexed consistently so his identity stays recognizable. A modular weapon attachment system would add a new asset/runtime concept just for this change. The active/carried object is a hard cutover because both authoring and consumers are owned here.

### Active mount alongside visible identity (high confidence)

Each soldier's selected weapon is published with his stable member ID and slot. Rendering does not reconstruct selection from squad shots, timers, or weapon names. The same filtering that publishes visible identities filters their activity. This exposes one new authoritative fact, including it in replay state, instead of creating a parallel renderer decision.
