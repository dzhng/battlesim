# Implementation choices

Settled user/delegated planning decisions remain in the exploration map. This ledger records implementation decisions where that plan left a gap.

## 03 audio audit · 2026-10-04

No additional non-delegated architecture choices. The slice delegated graph
ownership and initial synthesis/mix: the implementation retains the page context
and bank while disposing each battle's entire mixer. This cuts off its reverb
tails as well as its voices, without aborting shared preparation. The required
React provider, route policy and terminal document departure follow the
coordinator's accepted integration seam.

## Navigation pass · 2026-10-04

### Commit departures without concurrent route transitions

When: slice 01.

Choice: the router disables concurrent navigation transitions, so leaving and
immediately returning cannot be merged into one retained battle page. The
independent review found that the default transition could skip the intermediate
page; the regression retained battle progress after a rapid menu/Back round trip.
The platform's `unstable_useTransitions={false}` option resolved that failure
without a second history manager. Each continuation also becomes permanently
inactive when its component unmounts; a stale publisher must not reactivate just
because history returns to the same entry.

A page additionally checks the current browser history entry before admitting an
asynchronous result. For example, a map can finish just as the player clicks
Cancel. React may still have the battle page mounted while it schedules the
menu. Waiting for component cleanup lets the queued map result replace the
menu URL with a battle URL. The visit check stops that result as soon as the
history entry changes. The alternative of checking only a mounted flag failed
the actual battle route test with the menu visible at a battle address.

Gap: the plan required stale continuations to stop but did not prescribe the
boundary during concurrent rendering.

Reach: entry and isolated browser harnesses must share the synchronous router
policy. The navigation owner reads the pinned router history metadata (`key`
and `usr`, its user-state field) and its own retained-visit marker. Those reads
stay together in `navigation.tsx`; a router upgrade must preserve the focused
Cancel/publication/history proofs. No second history manager is added.

Verdict: sound. The test demonstrates an observable wrong URL that a
cleanup-only guard does not prevent.

Confidence: medium; the behavior is established, while the unstable router
option and pinned history layout are maintenance tradeoffs.

### A newer replay selection supersedes an older pending selection

When: slice 01.

Choice: selecting another file makes the earlier file's continuation inactive.
If the first file is still being read and the player chooses a second, the
first cannot later replace the viewer or show its error. The same small
request counter also makes callbacks inactive after component cleanup; it
does not cancel an IndexedDB write that has already begun. Without it, an old
file could take over the newly chosen viewer when its slower read finishes.

Gap: the plan specified stale work after page departure but not two file
selections within one visit.

Reach: the replay importer treats the latest selection as the current user
intent. Already-started storage transactions may complete, but their
continuations cannot navigate after departure or a newer selection.

Verdict: sound. This extends the same current-intent rule without a queue or
new storage mechanism.

Confidence: high.

The exact React Router 7.13.2 pin, retained-visit marker and internal names were
explicitly delegated by the slice; they are implementation discretion rather
than new product decisions.

## Recorded menu music

The user chose and supplied the recording. Keeping the full track, with the
existing mono preparation and a short repeat crossfade, avoids a separate music
pipeline and preserves the requested tone. Playback begins at the introduction;
ambient loops retain staggered starts. Verdict: sound; confidence high.

The menu music gain is twice the rejected sample's last level; ambience remains
quiet. This is a reversible taste choice that makes the requested increase audible.
Verdict: sound; confidence medium, subject to the user's listening preference.
Settled user/delegated planning decisions remain in the exploration map.

## Slice 04 · Deck layout choices

### Bound very large selections with scrolling

- **When:** slice 04, before commit.
- **The choice:** Selecting a whole force can produce more facts than the screen can hold. The facts grow until the deck occupies about thirty percent of the viewport, then scroll within their own region. Commands remain visible and centered below them. Captions and command hints stay above the footer. The unbuilt alternative was a permanently short selection list with less battle coverage, or shortened facts; those change the requested rich-selection layout.
- **The gap:** The plan required growth/wrapping but did not say what happens once content exceeds the viewport.
- **The reach:** Later HUD styling inherits a scrollable selection region and must keep its keyboard/pointer content reachable. A 48-unit case verifies the final facts and actions at both reviewed widths.
- **Verdict:** sound; one viewport-relative bound handles arbitrary selection height without dropping information or covering most of the battlefield.
- **Confidence:** medium; the exact amount of battlefield a huge selection may cover is reversible layout discretion.

### Let one layout own captions and command hints

- **When:** slice 04, before commit.
- **The choice:** When a busy selection becomes taller, captions move above it automatically. A focused or hovered command puts its hint in the same stack, below captions and above the footer. The hint is rendered into that shared location through a React portal, which means the button still owns its accessible description while the layout chooses its physical position. Existing hover/focus events measure the owning button once and align the hint horizontally above it; no observer or timer runs. Positioning each hint above its button instead would let it cross captions when the deck changes height.
- **The gap:** The plan required captions above the actual deck but left the shared component boundary unspecified; exact tooltip implementation was delegated.
- **The reach:** Production battle and controlled panel fixtures compose the same `SelectionDeck`. Future lower-HUD content should use that shared layout rather than reconstruct fixed offsets.
- **Verdict:** sound; layout determines spacing from real content, and the button retains its interaction semantics.
- **Confidence:** high.

### Reuse the panel workbench for deck states

- **When:** slice 04, before commit.
- **The choice:** Opening the existing panel lab can now switch to the real battle deck over controlled observations. The existing unit specimen builder also supplies these cases, including mixed capabilities and very large selections. Creating a separate lab route and a second fixture builder would duplicate the same observation and panel setup.
- **The gap:** The plan named component fixtures as a review surface without choosing their route or data owner.
- **The reach:** Deck visual checks remain part of the existing panels scene and registry identity; no extra route, dependency or production fixture schema was introduced.
- **Verdict:** sound; fixtures exercise the production components through the existing review owner.
- **Confidence:** high.

### Keep exact soldier count beside health

- **When:** slice 04, before commit, after fresh visual critique.
- **The choice:** A wounded squad can have all its soldiers alive while its health pips fall, and a squad can lose soldiers while retaining similar pips. The shared panel now names the living soldiers next to those pips, counting each published member whose health remains positive. Enemy panels show no count because the side has no personnel observation for them. The alternative was to use five health pips as a rough proxy for headcount; that cannot preserve the reference's exact personnel fact.
- **The gap:** The concept showed exact soldiers but the existing shared vocabulary only exposed combined health. The coordinating agent authorized extending that owner after critique.
- **The reach:** Own floating readouts and selection facts use the same optional `Panel.personnel` field and InfoPanel output; no new simulation property or second panel renderer exists. Future consumers must preserve the difference between health and living personnel.
- **Verdict:** sound; exact own personnel comes from admitted observations, and enemy knowledge stays unchanged.
- **Confidence:** high.

Full multi-selection detail, omission of visible Move/Garrison, optional replay commands and binding-table labels follow explicit contracts. Reversible spacing, typography, icon size, background opacity and focus/hover implementation use delegated design discretion. Escape dismissal preserves the existing battle event owner and adds no new global input rule. No unsound or needs-user architectural choice remains in this pass; final visual acceptance is still pending.
