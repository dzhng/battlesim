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
never regenerates the map. The store keeps the admitted saved baseline and admitted
draft, plus the latest refusal's input receipt; a refusal leaves the older admitted
plan inspectable. Sample runs own a separate temporary artifact and never replace either preview.
Export releases sample geometry while preserving its input receipt; the next sample
expires that receipt. Cancelling a sample leaves the displayed preview inspectable.
Leaving or disconnecting cancels owned native work; server disposal removes its
retained artifacts.

Preview makes no source changes. Save publishes the server-held, reviewed candidate
after checking all source receipts and Rust validity again. Preset revision is
managed from changed preset contents; analysis-only changes preserve it. No-op and
repeated saves preserve source bytes. Outside edits reject stale publication and
survive rollback. The shared [fixture publication owner](../fixture-publication/publication.ts)
recovers interrupted multi-file saves before either editor exposes a snapshot.

The [native executor](native.ts) owns process, output and job bounds; the
[HTTP boundary](httpServer.ts) owns local-origin and request-size bounds. Browser
requests name logical documents and opaque result IDs, never filesystem paths.
Heavy work runs in one active slot with one pending request. These limits contain
accidental work; they do not weaken generation correctness.

The [protocol](src/protocol.ts) defines the browser and native boundaries. The
repository task runner owns native build and development startup. Sampled sight
remains on demand until its complete additional browser-visible cost is proven
below the budget recorded in the [feature spec](../../specs/live-map-workbench/README.md).
