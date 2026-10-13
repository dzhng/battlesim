# 02 — Offline bundle admission and prepared-world startup

## Contract unlocked

An uploaded bundle constructs the normal prepared world and one rendered frame using only its embedded/resolved physical inputs plus explicitly referenced built-in content hashes.

## Seam

Extend `web/src/battle/prepare/protocol.ts` and the worker/client boundary with a bundle-load operation. Validate bytes through slice 01, call `PreparedWorld::from_scenario`/`into_battle`, and return the same preparation report and staged diagnostics used by normal play. The path must not call generation, `resolveSavedMap`, current fixture maps, or the map workbench.

Map snapshots must preserve the resolver's exact JSON and every template/catalog dependency required to materialize buildings. Built-in appearance data is admitted by content hash. User audio bytes are verified and handed to the existing battle-audio prepared bank; missing or corrupt media refuses before navigation.

## Runnable artifact

An upload smoke route loads a minimal exported bundle with fixture/network fetches disabled, reports stages, and starts one battle tick. A corrupted map, catalog reference, or audio record shows a named refusal while leaving the previous draft/session untouched.

## Verification

- native/WASM prepared-world parity;
- browser worker cancellation/disposal and atomic refusal tests;
- no fixture/generator fetches during bundle admission;
- resolver bytes and physical map identity remain unchanged;
- audio object URLs are disposed on refusal/navigation;
- built-in content hash mismatch is actionable.

## Visual gate

The only judged variable is upload status and refusal legibility in the loading surface. Run `game-ui`; capture the loading/refusal crop; use `screenshot-critique` last and `compare-screenshots` only if an existing loading target is changed. Open shots with `preview-shots` for non-blocking review.

