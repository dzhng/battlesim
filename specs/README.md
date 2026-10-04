# Plans and rationale

[Feature directories](./) hold active plans. Each plan's README owns its current
handoff, verification checkpoints and unresolved work; its choices ledger records
decisions made where the plan was silent. Read that handoff before implementing a
slice. A planning document can describe future behavior, so it is not proof that
a capability has shipped.

[Finished features](done/) retain the rationale and accepted scope after shipping.
Their README explains why the design works and points to current owners. Choices,
measurements, references and rejected experiments beneath each feature retain their
original workload and date context; frozen numbers are evidence, not today's defaults.
Production commands and settings belong to their manifests, registries and source.

Historical source artifacts may live at the Git revision or tag named by their
receipt rather than in the current checkout. Follow that provenance explicitly;
do not substitute a current script for a frozen comparison baseline.
