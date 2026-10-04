# 04 · Army cards and compact bottom controls

Contract: all observed own units occupy one horizontal row of vertical cards
with role/silhouette, health and selection state. Cards appear above a compact
icon command row, centered. No selection-count label. Full shared InfoPanel
facts appear above the owning card on hover and keyboard focus. Plain click
selects; Shift-click toggles additive selection. No selection keeps the roster
and hides commands; replay retains roster/facts without commands. Large armies
scroll horizontally and remain reachable by Tab.

Use existing own observations, input selection, InfoPanel, generated icons,
command bindings/capability reach and caption owners. No duplicate panel renderer,
unit cache or selection authority. Remove the old SelectionDeck owner; developer
labs may keep their existing SelectionCard specimens. No visible Move/Garrison;
ordinary right-click retains both. Deploy stays visible but disabled when no selected unit can deploy.
Attack-move reaches the whole selection; armed units engage, unarmed units move.
Contextual Deploy/Pack/Leave building remain available. Escape dismisses details without claiming the
battle event. Captions and command hints remain above the actual deck; readout
occlusion hooks remain intact.

The user selected the all-unit B concept, then moved commands from its right side
to a small bottom row. This supersedes the prior selected-facts grid and its
vertical-scroll gates. Preserve their factual/capability coverage through the
new hover/focus surfaces, and verify horizontal overflow at 1600/1024 px.

Current proof: army roster tracer red on old SelectionDeck, green on ArmyDeck.
Shift-toggle falsification red as expected. Focused tests cover live facts,
health/casualty removal, Escape, selection and replay command availability.
Production/component screenshots, comparison, fresh critique and review pending.
Historical deck evidence remains in ignored throwaway/hud-deck and deck-final;
its accepted look is no longer the target.
