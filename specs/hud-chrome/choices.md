# Final implementation choices

Review these first: synchronous router transitions and history metadata, the full
recording through the existing mono pipeline, and the maximum width of a very
large army row. These carry the most maintenance or taste uncertainty.

The rationale records the final user choices; the
[historical exploration map](unknowns-map.html) attributes earlier compact-deck
preferences. This ledger records the
remaining decisions made where the plan left a gap, checked against the final
implementation. No unsound or unresolved user-only choice remains.

## Sound · medium confidence

### Commit departures synchronously and reject retired continuations

**When:** navigation pass, `63bdd72a`.

**Choice:** A player leaves for the menu and immediately presses Back. The
router commits both changes synchronously, so the first battle is discarded
before the returned address starts a fresh one. A queued map result also checks
the current browser history entry: if Cancel already changed that entry, the
result cannot replace the menu's address even while React is finishing cleanup.
Once its component is gone, that old callback stays retired. In decision terms:
`depart → discard; later return → new visit; old result → ignore`.

**Gap:** The plan required fresh visits and isolated late work but did not choose
the boundary during concurrent rendering. The alternative of waiting only for
unmount allowed an observable wrong address after Cancel; concurrent transitions
could skip the intermediate departure.

**Reach:** The entry and isolated route tests share the router's
`unstable_useTransitions={false}` policy. `navigation.tsx` reads the pinned
router's history `key` and `usr` (its saved user-state field). Router upgrades
must preserve the departure/publication proofs and those metadata meanings.

**Verdict:** sound; it enforces the requested visit boundary without another
history manager. **Confidence:** medium because the behavior is proved but the
unstable router option and metadata layout remain an upgrade obligation.

### Prepare the full recording through the existing mono sound bank

**When:** recording pass, `e669feca`.

**Choice:** Opening the menu prepares the entire supplied recording using the
same mono WAV pipeline and decoded buffer bank as other sounds. A new menu
voice begins at the introduction; a short crossfade softens the repeat. Music
is raised above quiet countryside ambience, following the requested increase.
The alternative is a separate compressed/stereo music player with another
loading and playback lifetime.

**Gap:** The user chose the source and stronger music, but did not prescribe
duration, runtime encoding, repeat preparation or the final relative level.

**Reach:** Reusing the bank keeps one playback owner and preserves tone without
the effects filter used by other recordings. It costs a larger runtime transfer
and decoded buffer than a compressed streaming player. Tone/source are settled;
the mix level remains a reversible listening preference in `MENU_BED`.

**Verdict:** sound; one preparation path is simpler and its ownership already
serves menu-to-battle continuity. **Confidence:** medium because full-buffer cost
and the raised mix are tradeoffs the user may later tune.

### Keep very large armies in one horizontal scrolling row

**When:** army deck replacement.

**Choice:** Every owned unit gets a card. When the army exceeds the available
width, the same row scrolls sideways; keyboard focus scrolls the whole focused
card into view. The alternative of wrapping cards would grow the HUD upward
and cover units on the battlefield.

**Gap:** The user chose one Total War-style row without specifying overflow.

**Reach:** Later roster controls should preserve horizontal reachability and keep
commands below the row. There is no pagination or hidden-unit cap.

**Verdict:** sound; native scrolling preserves the chosen layout and every unit.
**Confidence:** medium because its maximum width is reversible layout discretion.

### A newer replay selection supersedes an older pending read

**When:** navigation pass, `63bdd72a`.

**Choice:** A player selects one replay and then another before the first file
finishes reading. Only the newer choice may open a viewer or show an error. A
small request counter retires the older callback and callbacks after departure.
An already-started storage write may finish, but cannot later take over the
page. Letting reads race to navigate would make the slower old selection win.

**Gap:** Departure isolation was specified; multiple file choices within one
visit were not.

**Reach:** Replay import follows the latest user intent without a queue or a
second storage mechanism.

**Verdict:** sound; the same current-intent rule covers both cases.
**Confidence:** medium; the continuation guards were inspected, while existing
regressions directly exercise departure rather than competing file reads.

## Sound · high confidence

### One layout owns captions and command hints

**When:** deck pass, `ae5e87aa`.

**Choice:** Hovering an army card reveals its full facts. Captions move above the
real footer and detail through normal layout, and a command's hint sits between captions
and footer. A React portal places that hint in the shared stack while its
button retains the accessible description. Hover/focus measures the owning
button once to align the hint; no timer or observer runs. Independent floating
offsets would let hints and captions cross when the footer grows.

**Gap:** The plan required content-aware caption placement but left the component
boundary open.

**Reach:** Production battles and panel fixtures share `ArmyDeck`; later
bottom-HUD content should join that owner rather than reconstruct its offsets.

**Verdict:** sound; actual layout owns spacing and the command owns interaction.
**Confidence:** high.

### Living soldiers and health remain separate facts

**When:** deck pass, `ae5e87aa`.

**Choice:** An eight-person squad can be wounded without losing a soldier, while
another squad with similar total health can have fewer people alive. Own panels
therefore name living personnel beside health pips, using observed members with
positive health. Enemy personnel stays unknown. Treating pips as headcount would
erase that distinction.

**Gap:** The concept showed exact personnel; the shared panel vocabulary lacked
that fact. Extending the shared owner was authorized after visual critique.

**Reach:** Selection facts and floating readouts use the same optional
`Panel.personnel` field; no simulation property or second panel renderer exists.

**Verdict:** sound; admitted own observations provide the count without adding
enemy knowledge. **Confidence:** high.

### Reuse the panel workbench for rich deck states

**When:** deck pass, `ae5e87aa`.

**Choice:** The existing panel lab can display the real deck over controlled
observations, including mixed capabilities and large selections. Its existing
specimen builder supplies those states. A new route and fixture builder would
duplicate the same observation setup.

**Gap:** Component review was requested without choosing a route or fixture
owner.

**Reach:** Deck evidence remains under the panels scene and registry identity,
with no extra production route or fixture schema.

**Verdict:** sound; it exercises the shipped components through the established
review owner. **Confidence:** high.

### Expanded Developer content uses normal menu scrolling

**When:** screen pass, `336cdb13`.

**Choice:** Opening Developer can make the menu taller than the window. Its
existing scroll region exposes the remaining links; keyboard focus scrolls Labs
fully into view. Forcing every tool into one screen would shrink the readable
player controls or add another surface.

**Gap:** Compact screen composition did not require expanded developer content
to fit one viewport.

**Reach:** The disclosure may grow vertically while ordinary play keeps the
compact default plate. Scroll and keyboard access remain required.

**Verdict:** sound; technical content stays readable and reachable.
**Confidence:** high.


### Preserve each city source set's visual evidence

**When:** complete-scene closeout, `f8ff6a1a`.

**Choice:** Each source set writes its pictures beneath its own evidence directory.
Two sources can contain the same building category; flat filenames would overwrite
one source's proof. Readers descend the source directory to find the pictures.

**Gap:** Complete separate visits were required, but evidence organization was unspecified.

**Reach:** This changes ignored verification output only, without a runtime owner.

**Verdict:** sound; every source's evidence survives. **Confidence:** high.

### Judge narrowed damage-state availability across the selected catalogue

**When:** complete-scene closeout, `f8ff6a1a`.

**Choice:** A ruin-only visit omits towers that have no ruin state while still
checking the selected catalogue's ruins. A requested state absent from the entire
selected catalogue fails. Requiring every source to contain every state would
reject valid sources when the combined visit is split.

**Gap:** The plan required the existing narrowing contract without prescribing
how source-set visits preserve it.

**Reach:** The verification scene retains the combined catalogue's meaning;
runtime states, budgets and rules stay unchanged.

**Verdict:** sound; the selected catalogue remains the authority. **Confidence:** high.
