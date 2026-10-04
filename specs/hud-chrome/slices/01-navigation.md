# 01 · Navigation and battle visits

Contract: ordinary links and programmatic navigation change screens without document reload. A fresh visit captures exact requested query; admitted exact URL publication preserves its session. Later same-path navigation and Back/Forward create fresh sessions, with no discarded battle cache. Exit cancels preparation and late replay import/admission cannot navigate after departure.

Seam: declarative React Router in existing router/main; retain battleLinks and fixture registry/lazy routes. Prove the pinned stable version's replacement behavior. Use one visit identity independent of URL publication; preserve history/router state. Own router.tsx, main.tsx, MainMenu link conversion, PauseMenu/LoadingScreen links, battle/replay navigation and route tests. Coordinate styling edits in later slices; do not add shared resource machinery here.

First proof: one integration test through actual route owner demonstrates main menu → battle and Back without document navigation. Then distinguish publication from a new visit, same-path fresh navigation, Forward, and late completion. Use controlled route/preparation edges with real router. Keep existing menu inventory and typed URL tests. Playable artifact: existing pages with working client history and cancellation.

Delegated: internal function names and exact visit-state mechanism after tested router proof. Unknown-path behavior is fixed to existing menu fallback. No migration or compatibility mode. Human feedback changes flow only if it revises the settled immediate-discard contract.

Verification: write-tests red/green at the consumer seam, narrow types/lint/format, review and independent code review before commit, then audit choices. Preserve exact battle identities, deterministic replay outcomes, side-visible authority and manual mechanics reload. Scratch evidence remains ignored.
