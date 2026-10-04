# Live map workbench

Open `/map-workbench` with `bun run dev:maps`. The workbench is a local developer tool. A draft changes generation settings and
report policy; the native Rust owners decide whether those settings are valid and
whether a particular seed can produce a map. A refused seed remains useful tuning
evidence, and valid settings can still be saved.

The [server](server.ts) captures the editable sources and their read-only physical
inputs together. A result belongs to those exact bytes and the requested type,
size and seed. Export preserves those inputs and their receipts so a later source
edit cannot silently change what an earlier report means. Source receipts are
separate from the generator's physical map and configuration hashes.

Plan inspection and sight analysis use retained native artifacts. Changing a crop
never regenerates the map. Each preview names the admitted maps the browser still
displays: its accepted draft and saved baseline. The store keeps those maps plus the current result,
and retires undisplayed work on the next preview. A refusal leaves the listed
older plans inspectable and retains its own exact input receipt. Sample runs own
a separate temporary artifact and never replace either preview.
Export releases sample geometry while preserving its input receipt; the next sample
expires that receipt. Cancelling a sample leaves the displayed preview inspectable.
Leaving or disconnecting cancels owned native work; server disposal removes its
retained artifacts.

Preview makes no source changes. Save checks Rust validity again and publishes
the server-held reviewed candidate through [shared fixture publication](../fixture-publication/README.md),
which owns stale receipts, byte-preserving no-ops and interrupted-write recovery.
Preset revision follows changed preset contents; analysis-only edits preserve it.

The [native executor](native.ts) owns process, output and job bounds; the
[HTTP boundary](httpServer.ts) owns local-origin and request-size bounds. Browser
requests name logical documents and opaque result IDs, never filesystem paths.
Heavy work runs in one active slot with one pending request. These limits contain
accidental work; they do not weaken generation correctness.

The [protocol](src/protocol.ts) defines the browser and native boundaries. The
repository task runner owns native build and development startup. Sampled sight
remains on demand until its complete additional browser-visible cost is proven
below the budget recorded in the [feature spec](../../specs/done/live-map-workbench/README.md).
