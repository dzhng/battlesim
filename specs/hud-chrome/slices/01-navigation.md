# 01 · Navigation and battle visits

Contract: ordinary links and programmatic navigation change screens without document reload. A fresh visit captures exact requested query; admitted exact URL publication preserves its session. Later same-path navigation and Back/Forward create fresh sessions, with no discarded battle cache. Exit cancels preparation and late replay import/admission cannot navigate after departure.

Seam: declarative React Router in existing router/main; retain battleLinks and fixture registry/lazy routes. Prove the pinned stable version's replacement behavior. Use one visit identity independent of URL publication; preserve history/router state. Own router.tsx, main.tsx, MainMenu link conversion, PauseMenu/LoadingScreen links, battle/replay navigation and route tests. Coordinate styling edits in later slices; do not add shared resource machinery here.

First proof: one integration test through actual route owner demonstrates main menu → battle and Back without document navigation. Then distinguish publication from a new visit, same-path fresh navigation, Forward, and late completion. Use controlled route/preparation edges with real router. Keep existing menu inventory and typed URL tests. Playable artifact: existing pages with working client history and cancellation.

Delegated: internal function names and exact visit-state mechanism after tested router proof. Unknown-path behavior is fixed to existing menu fallback. No migration or compatibility mode. Human feedback changes flow only if it revises the settled immediate-discard contract.

Verification: write-tests red/green at the consumer seam, narrow types/lint/format, review and independent code review before commit, then audit choices. Preserve exact battle identities, deterministic replay outcomes, side-visible authority and manual mechanics reload. Scratch evidence remains ignored.


## Navigation pass handoff · 2026-10-04

Implemented in the navigation worktree. Entry composition is
`BrowserRouter unstable_useTransitions={false} → app providers (integration pass) → LabRouter`; the router
exports `screenForPath` from the same resolution as rendering, so menu warm-up
and menu audio need no second route inventory. `navigation.tsx` owns page visits,
exact-address publication and the immediate continuation guard. Resource,
audio and styling work remain with their owning slices.

Independent review found a rapid history round trip retaining the departed
component. A red/green regression now proves fresh progress and rejection of
its stale queued publisher. The router uses its synchronous navigation option;
continuation hooks remain permanently inactive after unmount.

Evidence: red/green Deploy and exact-publication tests; the actual battle-route
Cancel race was red with a mounted-only guard and green with the history-boundary
guard. Replay file-read and storage-completion departure tests were falsified
against removed guards and restored green. Browser entry navigation exercises
real worker termination and fresh Back/Forward visits without document
replacement. Existing worker replay/digest and storage checks retain their real
browser boundary. The 10 focused navigation/admission/replay tests, existing screen tests and
two real browser checks passed; types, owned lint/format and independent
follow-up review are clean. One combined run hit the existing benchmark
loading one-second polling limit under concurrent compile load; no threshold
was changed. Evidence logs live in the navigation worktree's ignored
`throwaway/navigation-*.log`. Full checks remain deferred to closeout.

Review scope: no screen appearance or gameplay change is claimed. Remaining
editor-app back links belong to integration; battle-lab links use client
navigation. The lost-device/document-departure contract belongs to resources.
The next agent should integrate this pass, preserve its visit/publication tests,
and update the global handoff rather than restore pathname-only navigation.
