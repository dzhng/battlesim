# Sound workbench

`/sound-workbench` is an internal developer tool. The library lists all stored clips
and every synthetic baseline, including unused reload/mechanical assets. Search,
filter, play/stop, show provenance and audition composed recipes. Recipe editing
chooses clip variations, optional synthesis and level. Unit rows derive from the
resolved unit catalog; each authored mount has a near/far choice and gain. Global
firing defaults and round-specific material impacts are editable too.

Snapshot → draft → preview → save uses a server-held reviewed candidate and exact
source revision. Reject unknown types/mounts/media, stale/outside edits, incorrect
loop/shot use and oversized/nonlocal writes. Reuse `FixturePublication`, extending
its audio allowlist/journal. Preview does not write. No-op save preserves bytes.
Saving changes only fixtures/sounds.json; source/catalog metadata is immutable.
Retain drafts on errors, show exact replacements, and allow discard/restore baseline.
Suppress audio-catalog HMR so running battles keep their generation; reload captures
latest. Integrate a developer-menu entry using its existing look.

Test-first proof covers unused clips, independent unit edits, preview/save/reload,
staleness, no-op, invalid identity and outside edits. Browser testing never plays
audible output during automated tests: use offline decoding/rendering. Capture the
real tool at desktop and narrow widths, opened editors and review state. Compare
against the intended compact, readable developer workbench, get an unprimed
screenshot-critique as final visual gate, then show the user via preview-shots.
