# Plans and rationale

[Feature directories](./) hold active plans. Each plan's README owns its current
handoff, verification checkpoints and unresolved work; its choices ledger records
decisions made where the plan was silent. Read that handoff before implementing a
slice. A planning document can describe future behavior, so it is not proof that
a capability has shipped.

[Unit roster and skirmish requirements](done/unit-roster/README.md) capture factions, unit
variants, the skirmish economy/objectives, basic AI and a phased implementation
ladder with parallel named-model production.

[Helicopters](done/helicopters/README.md) (shipped) made the roster's 19 helicopters playable in a low-air layer. [Transport](transport/README.md) is a placeholder for passenger carriage by trucks and helicopters.

[Air-war flight](air-war-flight/README.md) is a placeholder for the fixed-wing flight the [air war](done/air-war/README.md) specified but the simulation doesn't yet deliver: jets at their own altitude, circling, attack runs, and dogfight and afterburner tests.

[Battle voices and music](done/battle-voices-music/README.md) (shipped) gave units RTS-style voice lines (answers to their owner and simulation-emitted barks) and battles an adaptive background score, generated with ElevenLabs and reviewed in the sound workbench.

[Native rendering parity](native-rendering-parity/README.md) (active) builds a
native `wgpu` client that draws what the browser draws, over one Rust
presentation crate both clients share.

[Performance](performance/README.md) is a placeholder for measured, deferred
optimizations (GPU memory, model packing, loading), kept until a player-visible
problem calls for one.

[Finished features](done/) retain the rationale and accepted scope after shipping.
Their README explains why the design works and points to current owners. Choices,
measurements, references and rejected experiments beneath each feature retain their
original workload and date context; frozen numbers are evidence, not today's defaults.
Production commands and settings belong to their manifests, registries and source.

Historical source artifacts may live at the Git revision or tag named by their
receipt rather than in the current checkout. Follow that provenance explicitly;
do not substitute a current script for a frozen comparison baseline.
