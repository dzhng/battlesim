# Slice 03 — Shared Main Menu and Pause Settings UI

## Contract unlocked

The same graphics controls render in Main Menu Settings and an in-game Pause > Settings modal. The four choices, current effective tier, 45 FPS target, last auto-detect result, and rerun action have one shared React component and one store.

## API seam

Factor `GraphicsSettingsControls` from the existing Sound/Diagnostics composition. Main Menu uses it in its existing Settings page. PauseMenu adds a nested Settings modal alongside Tutorial, using existing inert/aria-modal/Escape behavior. Selecting a concrete tier persists it and invokes the paused rebuild transaction; controls are disabled with an Applying message until the replacement frame is ready. Closing the pause menu resumes only the pause owned by `usePauseMenu`.

## Verification

Write DOM tests for both surfaces, four choices only, shared selected value, persistence, rerun action, applying disabled state, Escape/close semantics, and a frozen simulation tick during rebuild. Add a browser UI fixture covering Main Menu Settings and Pause Settings.

Run `game-ui` before implementation and use its tactical HUD guidance. For captures, use `compare-screenshots` where existing menu/pause baselines exist, run `screenshot-critique` last, and show shots with `preview-shots`.

## Delegated freedom

Exact copy length and CSS class placement are delegated within the accepted mock. Do not introduce dashboard-style cards, individual graphics sliders, or a second modal/state store.
