# Test units

These are test units, never game content. Tests, browser scenes, the labs that
serve them, the benchmark, the endurance lab and the map workbench run on them,
so a roster change moves none of their verdicts or baselines. Every id here is
`test_*`.

They are the test set's own documents (`sim::fixtures::CatalogSet::Test`,
`SET_FOLDERS` in `web/src/battle/catalog/compose.ts`), resolved at run time
on top of the game's. The game's committed catalog (`fixtures/catalog.json`)
never holds them, and a test holds it to that. Their art is labelled test art
(`assets/source/test/`), not remodelled.
