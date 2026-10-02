# Admission and publication

Implement a supplied-document Rust admission API and native CLI. Input is the
game object and authored catalog documents. Output is the existing resolved
catalog view including weapons. Reuse Rules, catalog and FlightConfig validation;
return named errors instead of panicking. Add missing basic weapon-value checks
at the owning Rust validator, with red/green tests. Preserve all shipped values.

The Vite plugin exposes snapshot, preview and save at `/__mechanics`. Snapshot
contains authored documents, a revision and the resolved catalog. Changes address
section, entry, field path and value/restore intent; no client-supplied disk path.
Preview returns exact before/after file text and the validated catalog. Save
rechecks the revision, applies the same proposal, and returns the new snapshot.
Unknown/art fields and malformed operations are rejected. Handle inherited named
mounts and selected-unit soldier variants. Report part-masked edits.

Prove preview is read-only, invalid and stale drafts never write, inheritance
restores correctly, shared weapon changes include all affected units, source and
generated catalog match, publication failure rolls back, and interrupted writes
recover. Tests use scratch directories, never the developer's fixtures. Review,
audit choices and commit the pass. Backend and frontend may proceed independently.
