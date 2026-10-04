# Choices ledger: courtyards

Decisions made where the plan was silent. Add each as it is made.

#### A new court or garden piece is capped at 1 MiB raw
**When:** slice 00. **Choice:** Cap each new piece's bundle at 1 MiB raw, half a street prop's, rather than gzipping scenery or loading it lazily. **Gap:** The plan left the cap to the measurement. **Reach:** About 13 MiB raw added a map, against 267 MiB fetched today; slice 07 still filters regional art to the map's region. **Verdict:** sound. **Confidence:** medium (the art may need more texture to read up close).

#### A regional look travels gzipped and is fetched only for its family's maps
**When:** slice 07. **Choice:** An appearance tagged with a family is "fetched on request" like a building kit: gzipped, counted in the map's 50 MiB download, and fetched only for that family's maps. Shared scenery still loads raw for every map and is not counted, because it is 267 MiB today and would fail the gate as it stands. This refines slice 00's call (which declined to gzip scenery) for regional looks only. **Gap:** The plan said "load only the map's region" without saying how the gate counts it. **Reach:** `MAP_DOWNLOAD_MAX_BYTES` (was the shared kit cap) now covers kits plus the family's looks. **Verdict:** sound. **Confidence:** medium (shared scenery stays an ungated eager download).

#### Where a family has its own look for a kind, only its looks are candidates
**When:** slice 07. **Choice:** A Paris map draws a Paris bench even when a shared bench fits the box better; a kind whose only looks belong to other families draws the tinted stand-in. **Gap:** The plan said "prefer" without ranking against footprint fit. **Reach:** `PropAppearances`. **Verdict:** sound. **Confidence:** high.

#### Gardens are placed after the open country's cover pass
**When:** slice 06. **Choice:** Garden dressing runs after `open_country::cover`, kept a trunk's clearance off every forest, instead of inside the street-furniture pass. Inside it, 3 of 81 maps were refused, because the sight certificate's copses lost their ground to gardens; after it, 81 of 81 build. **Gap:** The plan put gardens beside the other furniture consumers. **Reach:** `lib.rs` generation order; courts (slice 05) must decide the same question. **Verdict:** sound. **Confidence:** high.

#### Gardens take whatever authored parts the map has left
**When:** slice 06. **Choice:** Lots are dressed in an order drawn from each lot's stream until the authored-part limit is reached, so on the biggest maps some gardens stay bare, spread out rather than clustered. Metro Large seed 2 had 48.6k of its 60k parts before gardens. **Gap:** The plan named the limit as a test, not what to do at it. **Reach:** Bodies per Mixed Small town rise from about 4.3k to 14k; the closeout benchmark judges the cost. **Verdict:** sound, pending the benchmark. **Confidence:** medium.

#### Hedges, washing lines, sheds and garden fences hide; only sheds and fences block
**When:** slice 06, with tweak-mechanics. **Choice:** A hedge and a washing line hide what is behind them but stop neither men nor rounds (the catalog's smoke-screen combination). A shed blocks movers (a tank shoves it), hides, and gives light cover with rounds passing through; a garden fence is the same but any vehicle pushes it; a garden table is a bench. A squad walks round a fence rather than climbing it, as with walls. **Gap:** The plan named the pieces, not their physics. **Reach:** Every suburb and village fight. **Verdict:** sound. **Confidence:** medium (fences that squads cannot climb may channel suburb fights more than a viewer expects).

#### One boundary kind per lot, the same mix in every region
**When:** slice 06. **Choice:** A lot's boundary is all hedge or all fence (village 3:1 hedge, suburb 1:1), inset half its thickness; a neighbour's parallel run is refused by clearance, so shared edges are not doubled where the lots line up. Regional mixes wait for regional art. **Gap:** The plan left the mix open. **Reach:** Presets `props.gardens`. **Verdict:** sound. **Confidence:** medium (back-to-back lots of different depths still show doubled rear fences).
