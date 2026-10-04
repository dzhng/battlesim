# Fixture publication

Developer editors submit logical edits and reviewed candidate identities, never
filesystem destinations. [The publication owner](publication.ts) provides the shared
serialized write path; [change ownership](changes.ts) defines the documents that
may participate. Each editor still owns its validation and candidate lifetime.

A snapshot is exposed only after recovery of an interrupted publication. Save
checks captured source receipts again before replacing files. A stale preview is
refused rather than overwriting newer work. Repeated or empty saves preserve bytes.

File replacement is individually atomic. A journal provides recovery across files,
not instantaneous visibility of the entire set to arbitrary filesystem readers.
Readers using this owner wait for recovery. Rollback preserves detected outside
edits and reverses only replacements owned by the failed publication.

[Mechanics](../mechanics-editor/README.md), [map](../map-workbench/README.md) and
[sound](../sound-workbench/README.md) editors use this same contract. Running battle
pages retain their accepted generation; a new page or explicit restart admits a
new one. Editors' HTTP boundaries restrict writes to local same-origin development
requests; publication itself owns consistency rather than browser authentication.
