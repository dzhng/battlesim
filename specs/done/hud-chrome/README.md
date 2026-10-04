# Army HUD and app lifetime

The desktop battle interface keeps all owned units in vertical army cards above
a compact, centered command row. The menu, loading, refusal and pause surfaces share
its restrained tactical styling. Navigation preserves the document's expensive
resources while each departed battle is discarded.

## Why this shape

The battlefield deserves most of the screen. The user selected a Total War-style
row of every owned unit, then placed compact icon controls below it. Cards carry
unit identity, health and selection; full existing facts appear only when the
player hovers or focuses a card. Ordinary right-click already communicates
movement and entering buildings, so dedicated Move and Garrison buttons do not
help that decision. Command names and shortcuts remain on hover and focus.

Mixed selections are normal RTS play. Attack-move sends every selected unit:
armed units can stop to engage, while unarmed units simply advance. Deployment
applies to capable units and remains visible but disabled when none can deploy.
Capability fractions add arithmetic where familiar behavior already explains the
action, so the controls omit them.

Large armies scroll horizontally instead of covering more of the battlefield.
Keyboard focus reveals the whole focused card. The existing facts panel serves
both card details and battlefield callouts; no second unit-information vocabulary
or selection authority is needed. Captions and hints share normal layout with
the deck because guessed footer heights become wrong as content changes.

Leaving for the menu means leaving the battle. Keeping a discarded battle would
create a second Resume mechanism and retain workers, maps and combat evidence.
Pause is the sole route back to that running battle. History return instead
starts exact inputs afresh. Publishing an admitted share address is different:
it names the battle already running and must not restart it.

Resource reuse follows that distinction. GPU admission, WASM, prepared sound
buffers and loaded appearances are useful to the next visit. Scene allocations,
canvas configuration, workers, worlds and combat audio graphs belong only to
the current battle. Menu warm-up begins after paint and prepares reusable
resources; map preparation stays behind Play and building kits stay demanded.
There is no preload scheduler or automatic recovery loop.

The user rejected two synthesized menu cues and supplied the Battlefield 2
recording. It uses the existing sound catalog and preparation path, starts at
its introduction and sits above quiet outdoor ambience. Retaining one audio
context lets the first browser-permitted activation also serve the later
battle. Permission can still block first-paint audio; the next qualifying input
retries activation. Music continues through loading and fades when combat audio
actually starts, rather than when a route merely changes.

## Invariants

- A navigation owns a fresh visit. Exact-address publication preserves that
  visit; Back, Forward and later same-path navigation never resurrect discarded
  progress. Late preparation or replay reads cannot navigate for a retired visit.
- Pause controls own keyboard, pointer and camera input. An armed command gets
  Escape before pause does; closing pause resumes only a pause it created.
- A departing viewport releases its scene and canvas configuration without
  destroying the page GPU. A departing battle releases its workers and audio
  graph without closing the page context or dropping the prepared bank.
- Required-resource failure refuses play with the menu and manual Reload still
  available. Real `pagehide` is terminal for retained resources; restoring that
  document from the browser's back/forward cache requires Reload.
- No selection preserves the army roster and hides commands. Replays expose
  cards and facts without command input. No selection count or capability
  fractions appear; unavailable Deploy remains disabled.
- Army details and floating readouts share `InfoPanel`. Own living personnel
  and health are different facts; enemy personnel remains unknown. UI and sound
  use admitted observations, never hidden battle state.
- Captions clear the actual deck, hints clear captions, and the deck keeps the
  existing readout-occlusion hooks. Overflow preserves complete facts and
  keyboard/pointer reachability.
- Existing screens and developer links remain available. Labs keep their
  developer interfaces. Desktop mouse/keyboard is the target; no touch controls,
  lobby, results screen, battle cache or departure confirmation is introduced.
- Saved mechanics are captured at page startup. Page reload is
  the refresh boundary for an already mounted page; new and restarted battles add no refresh machinery.

## Owners and proofs

Visit lifetime and exact URL publication live in
[navigation](../../../apps/battle-lab/src/navigation.tsx), with the existing
[route registry](../../../apps/battle-lab/src/router.tsx) and
[typed battle links](../../../apps/battle-lab/src/battleLinks.ts).
[Navigation tests](../../../web/tests/navigation.test.tsx),
[battle visits](../../../web/tests/battleVisit.test.tsx) and
[replay navigation](../../../web/tests/replayNavigation.test.tsx) pin departure,
publication and stale completions; the synchronous `BrowserRouter` policy in
[the entry](../../../web/src/main.tsx) is part of that contract. Router upgrades
must preserve those proofs and the history metadata consumed by the visit owner.

[The app shell](../../../apps/battle-lab/src/AppShell.tsx) composes the page
resource and audio owners. [Resource tests](../../../web/tests/appResources.test.ts)
and [viewport tests](../../../web/tests/viewport.test.tsx) and
[appearance lifecycle tests](../../../web/tests/gameAppearances.test.tsx) pin admission, failure
and scene/canvas release. [App audio](../../../packages/battle-audio/src/appAudio.ts)
and [its tests](../../../web/tests/appAudio.test.ts) own permission, shared bank,
transition and graph cleanup. Source attribution and preparation remain in
[the sound catalog](../../../fixtures/sounds.json); personal-project use does
not change the recording's rights receipt.

[ArmyDeck](../../../web/src/battle/present/armyDeck.tsx) owns the roster layout;
[CommandBar](../../../web/src/battle/present/readouts.tsx) owns command interactions, using
[InfoPanel](../../../web/src/battle/present/infoPanel.tsx).
[Army tests](../../../web/tests/armyDeck.test.tsx),
[command tests](../../../web/tests/commandDeck.test.tsx) and
[input tests](../../../web/tests/unitControl.test.tsx) pin card access, command
availability and whole-selection attack-move;
[pause tests](../../../web/tests/pauseMenu.test.tsx) pin Escape precedence;
[mechanics startup tests](../../../web/tests/mechanicsStartup.test.ts) pin the reload boundary; the existing [panel workbench](../../../apps/battle-lab/src/routes/panels.tsx)
and [panels scene](../../../web/scenes/panels.mjs) exercise rich and oversized states.
[The player journey](../../../web/scenes/_appJourney.mjs) runs through actual
menu, play, pause, restart, history and replay, checking digest identity,
retained page owners and released battle allocations.

## Rejected approaches and visual provenance

Concurrent route transitions could coalesce a rapid menu/history round trip and
keep the old battle alive. The router's synchronous transition option preserves
the requested departure boundary. Checking component unmount alone also let a
queued admission publish a battle address after Cancel; current history and
retired-continuation checks close that boundary without a second history manager.

The synthesized ambient and brass cues did not meet the user's requested music
tone. Their synthesis was removed when the supplied recording became the source.
Automatic mechanics refresh on new/restart was separately rejected as unnecessary
development machinery. Required-resource failures and terminal browser departure
use explicit recovery instead of rebuilding destroyed handles automatically.

[The earlier compact deck](assets/compact-deck-reference.png) records the initial
joined direction. It was superseded by [the all-unit B concept](assets/army-cards-reference.png),
a code-native mockup with actual icons and a captured battlefield; its
[hover reference](assets/army-cards-hover-reference.png) records full facts on demand. The user then
placed commands below the cards and removed capability fractions;
[the revised composition](assets/army-bottom-controls-reference.png) freezes that
geometry. These are planning references, not production screenshots.
[The historical exploration map](unknowns-map.html) contains the earlier compact
hybrid illustration and attributed preferences; its questions are historical.
This rationale and the [final choices ledger](choices.md) record the settled
contracts.

Matched player-route captures and rich-state component screenshots were compared
against the selected layout, with native crops and an unprimed visual critique.
The final fresh critique found no concrete layout defect after actual-Tab
captures distinguished pressed commands from focus. The captured health bars are
full; tests prove changing health values, but damaged-health readability was not
visually judged. Scratch evidence remains ignored under `throwaway/evidence/`, `throwaway/army-visual/`,
`throwaway/screens-final/` and `throwaway/audio-review/`. Preview automation could
not establish a visible review window; no user approval through Preview is claimed.
