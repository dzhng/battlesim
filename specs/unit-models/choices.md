# Choices

- **Deferred mechanics stay disabled.** Source art is tracked separately from `assets/catalog.json`; a model file never makes a unit selectable. This preserves the existing card/catalog contract.
- **Every disabled card receives source coverage.** The first art pass uses one explicit GLB per card, including deferred aircraft, rotorcraft, support, drones, infantry kits, and ground cards. This gives UI and future mechanics work a stable asset identity.
- **Deferred-family silhouettes are provisional.** The disabled-family generator provides faction-colored, role-readable source placeholders. Family-specific meshes replace them as movement, weapons, mounts, and animation contracts are implemented.
- **Family branches are authored in one deterministic exporter.** Aircraft, rotorcraft, support, drones, ground vehicles, and infantry kits use ID-driven silhouette branches so regeneration is repeatable and does not create a second disabled-card asset catalog.
- **Challenger 3 keeps a family-authored model.** It is stored beside the Challenger family source and remains unbound while its mechanics are deferred.
