# 02 · App resources, warm-up and failure

Contract: one admitted GPU survives client screen changes; viewport owns canvas/scene/listeners, disposing them on exit. Retain existing WASM and appearance caches. After menu paint warm reusable resources, no battle/map and no unused kit. Required appearance/GPU failure reaches existing refusal instead of indefinite loading. Menu remains usable. GPU loss/manual real-document recovery are terminal until Reload, no automatic retry.

Seam: one app GPU/resource owner around existing requestGpuDevice; viewport borrows it. Keep trackGpuAllocations and GpuRegistry release. Existing appearance hooks report readiness/failure through LabLoading without a parallel loader. Own LabViewport, gameAppearances/resource owner and focused tests. Coordinate app-shell mount with navigation owner.

First proof: repeat acquisitions retain the same admitted device; a viewport leaving during admission cannot destroy it. Then verify real menu/battle cycles return device allocations to retained baseline and remove abandoned canvas/listeners. Test required base/kit rejection and lost-device refusal. Playable artifact: menu warming and fresh battles on a reused device. Renderer instructions mandatory; no rendering algorithm changes.

Delegated: small owner/module shape and exact warm trigger after first paint. Fixed policy: no retries; lost device cannot be reused; pagehide tears down app scope, BFCache restore offers manual Reload. Full browser verification waits for integration, narrow cycle probe first.

Verification: write-tests red/green at the consumer seam, narrow types/lint/format, review and independent code review before commit, then audit choices. Preserve exact battle identities, deterministic replay outcomes, side-visible authority and manual mechanics reload. Scratch evidence remains ignored.
