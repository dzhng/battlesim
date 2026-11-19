# 07 Published progress

**Status: shipped 2026-11-19.** `ObservationFrame.scenario`, the `beats`
group, `scenarioChapter` and `beatCount` header words, the `chapters` and
`beats` layout tables, the reader's `Scenario` view and the browser's
`ScenarioView`. Evidence: `director::a_beat_is_published_by_name_only_to_its_audience`;
publication tests after a re-bless; the web decoder tests. Deviations:
[decisions.md](../decisions.md#slice-07).

**Unlocks:** the page learns the open chapter and the beats its own side was told,
through the same visibility path as everything else.

**Contract (C7):**

- A header word, `scenarioChapter`, goes to every side.
- A group `beats` holds rows `[beat, firedTick]`, each row only to a side in that
  beat's audience.
- The layout gains `chapters` and `beats` id tables (the layout is the one owner
  of published names).
- `ObservationFrame.scenario: Option<ScenarioView { chapter, beats: Vec<(BeatIndex, Tick)> }>`.
- The reader decodes it into the page's observation view.

**Seam:** `crates/sim/src/publication/` (layout and rows) and the reader beside
it, which every client uses; `web/src/battle/sim/observationView.ts`.

**Playable:** the tutorial battle with chapters 1–2 directed (from 08's content
if it lands first, or a placeholder chapter). The developer `FeedInspector`
lists published beats as they fire.

**Gates:**

- Publication reader and wire tests (`crates/sim/tests/publication_reader.rs`,
  `publication_wire.rs`): a beat appears only in its audience's publication, and
  decodes to its id through the layout.
- A battle without a director publishes no scenario group: existing publication
  bytes are unchanged (or a single named re-bless, recorded in the ledger).

**Delegated:** the exact packing.

**Must stay green:** publication tests, browser decoder tests.
