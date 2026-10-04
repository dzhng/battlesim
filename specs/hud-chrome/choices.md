# Implementation choices

No implementation choices banked yet. Settled user/delegated planning decisions remain in the exploration map.

## 03 audio audit · 2026-10-04

No additional non-delegated architecture choices. The slice delegated graph
ownership and initial synthesis/mix: the implementation retains the page context
and bank while disposing each battle's entire mixer. This cuts off its reverb
tails as well as its voices, without aborting shared preparation. The required
React provider, route policy and terminal document departure follow the
coordinator's accepted integration seam.

Music remains provisional: after listening, the user requested a louder mix and
a different musical tone, and asked about extracting Battlefield 2 menu music.
The current authored fallback is raised about 3 dB; no synthesis redesign or
external asset is admitted while the source path is pending. The listening
checkpoint is open. The lifecycle implementation is independent of that asset.
Settled user/delegated planning decisions remain in the exploration map.

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
