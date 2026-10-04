# 04 · Icon commands and joined deck

Contract: bottom center joins existing SelectionCard/InfoPanel with icon-only command buttons. Remove visible Move/Garrison only; retain ordinary right-click and all bindings/capability actions, including Deploy/Pack/Leave building. Show name+shortcut tooltip on hover and keyboard focus, accessible name/pressed/disabled/partial reach. No selection hides deck; replay shows facts without command input. Captions layout above actual deck; retain readout occlusion hooks.

Seam: readouts.tsx, InfoPanel, captions/BattleView layout and hud.css. One InfoPanel vocabulary, no duplicate compact panel renderer. First behavioral test establishes omitted redundant buttons and retained capability action through actual CommandBar. Then tooltip keyboard interaction and contextual availability. Retain existing command binding, panel row and readout contracts.

Visual variable: density/legibility only. Bottom deck+caption boundary crop versus frozen compact-deck-reference and matched baseline. Preserve world styling. Test real long names, mixed/multi-selection, multiple ammo/mounts, suppressed, deploying, resupplying and garrison/exit states. Grow/wrap rather than shrinking text; concept dimensions illustrative. Component fixtures and focused real village/readouts/panels captures are review surfaces.

Delegated: reversible spacing/typography within readable tactical style and exact tooltip implementation. Existing capability semantics fixed. Report any real-state expansion beyond concept instead of hiding facts to match its box.

Verification: write-tests red/green at the consumer seam, narrow types/lint/format, review and independent code review before commit, then audit choices. Preserve exact battle identities, deterministic replay outcomes, side-visible authority and manual mechanics reload. Scratch evidence remains ignored.

Visual gate: use matched production-route before/after screenshots; compare-screenshots with the specified crop and target. Inspect full shots and native/enlarged crops. Run an unprimed screenshot-critique as the LAST visual acceptance step. Show a minimal Preview set, give a non-blocking reaction window while continuing independent work, record reversible calls and close Preview if proceeding unattended.
