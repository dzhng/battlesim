# Ground representation seams

Native ground and the browser receiver keep exact cell values while storing each tile as short runs or a dense page, whichever its contents need. Native learned pages share truth only after all 256 cells are equal; the first later truth edit copies the page, so hidden edits cannot change learned values. Learning stamps and fog synchronization hints use the same lossless pages. Truth, learned cells and their order stay authoritative; GPU residency owns none of that knowledge.

The first sparse wasm candidate matched the [frozen oracle](README.md); the integrated candidate still passes it.

What the red/green cuts showed:

- Untouched 18 km native ground indexes fell from 30,375,000 payload bytes to under 4,096; untouched browser ground fell from 1,620,000,000 bytes to only the touched pages.
- Disabling page compression fails the uniform learned-page memory test; reintroducing dense writes before the seal fails the transient-memory test on the first edited cell.
- A full uniform browser tile plus the opposite map corner stays below 128 payload bytes. A changed cell splits a run without touching its neighbors.
- Production has one allocation-free run iterator and no cell-list delivery path; only small oracle tests expand runs back to cell order.

Open: high-entropy pages still need dense exact bytes, so compression does not waive their bound. The old full snapshot at 18 km would have gathered about 3.9 GB of native cell patches and written 5.2 GB of wire floats; the run transport must remove both. Full-size heap peaks, churn, publication admission, overview sampling and frame cost were left to later evidence, as were G0 and artwork acceptance.
