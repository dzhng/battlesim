# Choices

These are the final implementation choices made where the request left room.
All are judged sound, ranked from medium to high confidence. Source selection,
extraction, timbre, initial assignments and reversible UI design were explicitly
delegated by the user and are not separate approval items.

## Sound — medium confidence

### Calibrate prepared source levels using the strongest short loudness window

When a clip or recipe is prepared, its strongest 100 ms K-weighted window supplies
a fixed playback correction toward the library reference of -23 dB. K-weighting
accounts for frequency when estimating loudness. A quieter rifle can then be
compared with a dense HMG without file levels deciding which one wins the mix.
Recipe gains and game controls still decide the intended difference. Measuring
the entire file would let a long quiet decay make a sharp shot appear too quiet.

**Gap:** the user requested a common loudness reference without choosing its meter,
target or preparation stage. **Reach:** the same prepared-bank calibration serves
audition and battle playback, while recorded bytes and original synthetic samples
remain intact. **Verdict:** sound for short effects; final artistic balance remains
editable rather than encoded by source recording levels. **Confidence:** medium.
**Landed:** source calibration follow-up.

### Prepare the complete recipe library for each audio context

When a battle first enables sound, its audio context prepares every catalog recipe
before playing observed events. A context is the browser's independent audio engine;
buffers belong to that engine. This keeps later shots free of download or decoding
work. Loading only currently reachable recipes would save some preparation, but
would add a second selection walk and more readiness states.

**Gap:** the plan did not specify how broadly to prepare the bank. **Reach:** a
much larger future library should revisit this choice; the present runtime clips
are about 2 MiB. **Verdict:** sound at the current scale; use the available margin
for a simple readiness contract. **Confidence:** medium. **Landed:** playback pass,
`d4830cac`.

### Keep the extraction tool independent of browser dependencies

To rebuild a clip, run the Python asset tool with ffmpeg and ffprobe available.
The catalog supplies the pinned source hash, frame crop and processing profile;
the tool recreates the WAV and verifies its bytes. A Node wrapper could launch the
same media tools, but would couple offline preparation to the web dependency tree.

**Gap:** the request specified extraction, not its toolchain. **Reach:** media
contributors need Python 3 and the two ffmpeg tools; no Python packages were added.
**Verdict:** sound because the media operation already depends on ffmpeg, while
the small wrapper uses only the standard library. **Confidence:** medium.
**Landed:** catalog/media pass, `dac589ae`.

### Bound reviewed candidates in server memory

Each preview retains the exact proposed save on the local server. Creating a ninth
preview expires the oldest review; pressing Save on that old review asks for a
new preview. Restarting the server also expires reviews. Storing them permanently
would preserve old review buttons across restarts but add a second durable store.

**Gap:** the plan required server-held candidates but did not choose retention.
**Reach:** a workbench session should preview again after a server restart or many
later previews. **Verdict:** sound for a local editing tool; drafts remain in the
browser and previews do not write repository files. **Confidence:** medium.
**Landed:** workbench pass, `e3c71d9d`.

### Compose unequal loops without changing either layer's pitch

If a recording loops every 1.35 seconds and its quiet synthetic support loops every
1.5 seconds, both repeat at their own period. The resulting buffer uses the longer
period and a short crossfade at its boundary. Padding the short recording with
silence loses its body; stretching it to fit would change its character or require
another audio transform. Original synthetic baselines retain their exact samples.

**Gap:** the plan required loop support without specifying unequal layer lengths.
**Reach:** later loop blends follow the same rule rather than needing aligned crops.
**Verdict:** sound; continuous body and a softened seam matter more than preserving
an arbitrary composite phase. **Confidence:** medium. **Landed:** final review pass.

### Release the audio engine when leaving a page

When a player navigates away, pending downloads and decoding lose admission and
the audio context closes. If the browser keeps that document in its back-button
cache, returning prepares a new context from the page's captured catalog and
waits for fresh observed evidence. Keeping the old context could save preparation
but would also retain its loading work and presentation history across departure.

**Gap:** cancellation was required, but browser page departure and cached return
were unspecified. **Reach:** cached returns take the same short preparation path
as first enablement; they keep the accepted catalog generation. **Verdict:** sound;
the browser lifecycle has explicit resource ownership and never revives a departed
page's pending bank. **Confidence:** medium. **Landed:** final review pass.

## Sound — high confidence

### Keep safe audition attacks clear of compressor startup attenuation

Clicking Play creates a fresh audio graph. A newly created compressor can reduce
the entire attack of a brief rifle report even when its peak is already safe.
Audition uses a memoryless safety ceiling instead: ordinary samples pass unchanged,
and excessive peaks are capped. Centering mono material in stereo also makes its
reference comparable with stereo sources. The battle's shared mix keeps its own
compressor because it combines concurrent voices rather than isolated auditions.

**Gap:** the original audition safety graph did not account for compressor startup.
**Reach:** audition level no longer depends on how long the selected recording lasts.
**Verdict:** sound; a single audition has a known prepared level and needs peak safety,
while battle dynamics remain owned by the mix. **Confidence:** high. **Landed:**
source calibration follow-up.

### Stage browser evidence through real orders and observations

The contact scene first moves a scout into sight of the tank, then withdraws it
and waits for a newer hidden firing report about that same tank. The muzzle scene
uses separate fights for road fire and moving tank machine-gun fire. Changing
simulation rules to force these pictures would change the game to suit the test;
waiting longer without establishing the prerequisite would leave the failure
dependent on an unrelated battle unfolding by chance.

**Gap:** the existing scenes expected evidence their starting state no longer
established. **Reach:** these checks own deliberate staging through accepted
commands, while retaining their original visibility, range and pixel requirements.
**Verdict:** sound; the test proves the same player-visible contract with a causal
setup. **Confidence:** high. **Landed:** staging repairs, `2f4bf6d8` and `f812814e`.

### Use short PCM WAVs as runtime assets

A rifle recipe downloads a small cleaned WAV rather than the entire source film.
PCM is uncompressed sample data. A compressed delivery format would reduce network
bytes but add another bake and another source of lossy differences. The current
library fits comfortably as reproducible mono WAVs; masters and runtime assets
follow the repository's Git LFS storage.

**Gap:** transport and compression were unspecified. **Reach:** new clips use the
same ordinary browser decoder and reproducibility check. **Verdict:** sound for
the present library size. **Confidence:** high. **Landed:** catalog pass, `8f4a3e09`.

### Give editable audio its own document

Changing a tank mount's sound writes only `fixtures/sounds.json`. The game's rule
document still owns its base mix, distance curves and anonymous hearing cues.
Putting sound selections into that rule file would entangle an audio save with
mechanics editing and make the audio library harder to reuse.

**Gap:** the request did not choose the file boundary. **Reach:** future audio tools
must use this catalog rather than invent another assignment map. **Verdict:** sound;
sound authoring has one owner and does not rewrite game rules. **Confidence:** high.
**Landed:** catalog pass, `8f4a3e09`.

### Preserve the meaning of original recipe IDs

Selecting `rifle` explicitly always selects the original synthesized rifle. A
recording or blend gets its own ID, and cloning makes a baseline editable under a
new ID. Implicit presentation slots can follow shared replacements, while an
explicit selection keeps its identity. Reusing `rifle` for a recording would make
restoring the baseline unreliable after other assignments changed.

**Gap:** keeping baselines did not itself choose recipe identity rules. **Reach:**
saved assignments remain understandable, and baseline comparisons stay possible.
**Verdict:** sound; workbench saves enforce the distinction. **Confidence:**
high. **Landed:** catalog pass, `8f4a3e09`.
