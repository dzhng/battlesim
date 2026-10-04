# 02 · App resources, warm-up and failure

Contract: one admitted GPU survives client screen changes; viewport owns canvas/scene/listeners, disposing them on exit. Retain existing WASM and appearance caches. After menu paint warm reusable resources, no battle/map and no unused kit. Required appearance/GPU failure reaches existing refusal instead of indefinite loading. Menu remains usable. GPU loss/manual real-document recovery are terminal until Reload, no automatic retry.

Seam: one app GPU/resource owner around existing requestGpuDevice; viewport borrows it. Keep trackGpuAllocations and GpuRegistry release. Existing appearance hooks report readiness/failure through LabLoading without a parallel loader. Own LabViewport, gameAppearances/resource owner and focused tests. Coordinate app-shell mount with navigation owner.

First proof: repeat acquisitions retain the same admitted device; a viewport leaving during admission cannot destroy it. Then verify real menu/battle cycles return device allocations to retained baseline and remove abandoned canvas/listeners. Test required base/kit rejection and lost-device refusal. Playable artifact: menu warming and fresh battles on a reused device. Renderer instructions mandatory; no rendering algorithm changes.

Delegated: small owner/module shape and exact warm trigger after first paint. Fixed policy: no retries; lost device cannot be reused; pagehide tears down app scope, BFCache restore offers manual Reload. Full browser verification waits for integration, narrow cycle probe first.

Verification: write-tests red/green at the consumer seam, narrow types/lint/format, review and independent code review before commit, then audit choices. Preserve exact battle identities, deterministic replay outcomes, side-visible authority and manual mechanics reload. Scratch evidence remains ignored.


## Verified implementation

`AppResources` borrows the existing device admission owner and caches its result;
WASM and appearances retain their established page caches. `AppShell` owns the
resource refusal and audio provider above route visits. The viewport releases
its scene and canvas configuration; actual pagehide alone destroys the GPU.
Required appearance failures flow to the same refusal with menu/Reload recovery.

Focused owner/canvas/appearance proofs pass, including required-kit refusal and
late admission/frame cleanup. Independent review found that effect-refresh cleanup
could destroy the retained device; a red/green regression now protects development
refresh, while pagehide stays terminal. Final integration review found no further
defect. Real foundation visits prove one admitted device, no device destruction,
zero scene allocations after exits, no menu battle worker and the same document.
Removing the acquisition cache makes those visits fail; restoring it passes.

Pause/loading owns command and camera input through existing owners. Focused
keyboard/pointer regressions pass, including held Space and armed Escape ordering.
Editor return links keep navigation in the document instead of reloading page owners.
The complete app journey and full-spec checks follow deck/screen integration.
