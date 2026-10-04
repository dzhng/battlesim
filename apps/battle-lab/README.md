# Battle lab and player pages

The app composes the same simulation adapters, renderer, input and sound used by
player battles. Labs isolate a question with a declared fixture; a private visual
review setup must not change the game's shared rules.

## Routes and fixtures

[The router](src/router.tsx) owns player pages, developer tools and the lab index.
[The fixture registry](src/fixtures.json) declares focused routes and verification
identities; [route modules](src/routes/) implement them. The browser [scene runner](../../web/README.md#checks-and-evidence)
holds the registry and scene files to each other, so adding a lab needs its matching
check. The registry is the route inventory; docs do not maintain a parallel list.

The main menu starts generated or released saved battles and opens replays. A
seeded address names exact preparation inputs on the same build. Ordinary Play
may try bounded fresh candidates and a released saved fallback, then publishes the
actual admitted identity. A refused explicit seed cannot silently become another
battle. [Saved-map policy](../../fixtures/README.md#saved-maps) owns what the menu lists.

[The shared battle view](src/BattleView.tsx) composes page resources and observation
feeds. [Session ownership](src/useBattleSession.ts) keeps worker preparation and
cleanup tied to that page. [Lab loading](src/LabLoading.tsx) uses the player
loading screen across preparation, the first observation and the first drawn
frame; routes keep preparing beneath the cover so it never delays readiness.
Art uses [scene-assets](../../packages/scene-assets/README.md),
and sound uses [battle-audio](../../packages/battle-audio/README.md). [Model review](src/workbench/)
installs previews through the same appearance loader used by battles.

## Observation feeds

[Stable feeds](src/feed.ts) carry large, changing presentation data to the
viewport without making it a new React prop tree on every publication. React's
development instrumentation traverses changed typed-array props, so an apparently
ordinary prop update can retain enormous diagnostic payloads. Keep live geometry,
fog and effects behind the existing feed boundary rather than copying them into
component state.

[The viewport](src/LabViewport.tsx) subscribes and updates [the renderer](../../packages/battle-renderer/README.md)
through that boundary. Published cause and identity remain intact; feed interpolation
cannot grant visibility or invent firing. [The scene runner](../../web/scene.mjs)
checks oversized development performance details as well as ordinary page errors.

## Presentation contracts

[Browser presentation](../../web/src/battle/present/) turns observed evidence into
panels, captions, contacts and cursors. Hidden contact reports retain uncertainty;
retiring visuals are remembered evidence, never live targets. Panels prioritize
legibility and camera depth while preserving hover access to detail. Sensing rules
own concealment; the UI cannot infer a hidden observer from private state.

The shipped rationales explain the non-obvious decisions behind [readout backgrounds](../../specs/done/readout-backgrounds/README.md),
[group movement previews](../../specs/done/group-move-preview/README.md),
[physical move validity](../../specs/done/move-validity/README.md) and
[contextual cursors](../../specs/done/game-cursor/README.md). Weapon behavior and its
visible readiness follow [fire cadence](../../specs/done/fire-cadence/README.md) and
[infantry weapon use](../../specs/done/infantry-weapon-use/README.md).

[Projectile review](src/projectileReview.ts) uses gameplay flight and weapon cycles
with explicitly private nonlethal review settings. [Guided fire](../../specs/battle-foundation/contracts.md#guided-flight)
separates shared spotting from the launcher's physical sight. [Benchmarking](../../web/src/battle/benchmark/README.md)
retains exact workload identity so a convenient lab cannot replace a performance control.
