# Choices

## Sound — high confidence

**An audio document owns editable selections.** When someone changes a tank's
sound, only `fixtures/sounds.json` changes. The existing game document still owns
mixing, distance and anonymous hearing cues. The request did not specify the file
boundary; a separate document makes an audio save independent of rule editing.
Future sound tools should use this document rather than add another assignment map.

**Original recipe IDs keep their baseline meaning.** Choosing `rifle` always means
the original synthesized rifle. A recording or blend has its own ID. This makes
restoring a baseline reliable after a shared default has changed; otherwise a
recipe named `rifle` could silently change what every old assignment means.

Initial artistic and extraction choices are delegated by the user. This ledger
records implementation choices outside that delegation as the feature lands.
