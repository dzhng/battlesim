# Choices ledger: courtyards

Decisions made where the plan was silent. Add each as it is made.

#### A new court or garden piece is capped at 1 MiB raw
**When:** slice 00. **Choice:** Cap each new piece's bundle at 1 MiB raw, half a street prop's, rather than gzipping scenery or loading it lazily. **Gap:** The plan left the cap to the measurement. **Reach:** About 13 MiB raw added a map, against 267 MiB fetched today; slice 07 still filters regional art to the map's region. **Verdict:** sound. **Confidence:** medium (the art may need more texture to read up close).

#### A regional look travels gzipped and is fetched only for its family's maps
**When:** slice 07. **Choice:** An appearance tagged with a family is "fetched on request" like a building kit: gzipped, counted in the map's 50 MiB download, and fetched only for that family's maps. Shared scenery still loads raw for every map and is not counted, because it is 267 MiB today and would fail the gate as it stands. This refines slice 00's call (which declined to gzip scenery) for regional looks only. **Gap:** The plan said "load only the map's region" without saying how the gate counts it. **Reach:** `MAP_DOWNLOAD_MAX_BYTES` (was the shared kit cap) now covers kits plus the family's looks. **Verdict:** sound. **Confidence:** medium (shared scenery stays an ungated eager download).

#### Where a family has its own look for a kind, only its looks are candidates
**When:** slice 07. **Choice:** A Paris map draws a Paris bench even when a shared bench fits the box better; a kind whose only looks belong to other families draws the tinted stand-in. **Gap:** The plan said "prefer" without ranking against footprint fit. **Reach:** `PropAppearances`. **Verdict:** sound. **Confidence:** high.
