# C61: map catalogue listings

**Depends on:** C60. **Kind:** slice.

## Question
Do saved-map listings come from the catalogue while runtime acquisition uses the same map resolver?

## Contract it unlocks
- Player battles use released playable catalogue maps with the configured encounter; runtime generation and replay keep their existing front-door actions. Under M29, developer arenas remain behind the developer link. `/labs` inspects every active catalogue map, including drafts, alongside fixture routes.
- **Routes stay separate from maps:** `apps/battle-lab/src/fixtures.json` owns routes, scenes and their fixed saved-map references. `map: null` means no fixed saved catalogue source: a tool, runtime-generated battle or synthetic world may still draw geometry. Acquisition remains the existing map resolver and MapSource contract.
- One generic map-view route and scene covers any catalogued map without a bespoke route. C55 adds the primary generated-map type/size controls and preparation status; transient seeds do not join a hand-kept saved list.
- A cross-check test: every map that is not retired has an inspection link, every fixed route map exists, and a declared benchmark binds to the map that lists it.
- No router is built here (that's hud-chrome's firewall); hud-chrome consumes `listMaps` later.

## API seam
`MainMenu.tsx`, `apps/battle-lab` listing, `web/scene.mjs:56-67`.

## What the human can run or see
The player menu keeps its existing choices and appearance. `/labs` adds saved-map inspection under catalogue labels; fixture identities and their existing default scene coverage remain unchanged.

## Verification
- Cross-check test.
- A temp draft map appears only where its status allows.
- `scene -- --list` ids unchanged.

## Delegated to the implementer
Menu ordering; the generic route's path. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Scene ids; the menu scene.

## Feedback that would change this slice
A listing that makes maps hard to find changes metadata presentation while keeping the catalogue as the sole source.

## Outcome

**Implemented.** The main menu already follows M29 and the catalogue's released playable encounter filter. The developer index now lists active saved maps by metadata and keeps every registered fixture and tool reachable. The registry's fixed map references feed saved-route acquisition rather than repeating map ids in each JSX route. Shared encounter and street factories receive the calling fixture id, so alias metadata also drives the world they acquire; the explicit village variant of the ground lab retains its selected village source. Generated and synthetic worlds declare no fixed catalogue source.

The existing geometry route is the generic inspector: `?map=<id>` loads that catalogue map through `SavedMap` and the same resolver, names it from metadata, and frames its extent with the shared camera settings. Without the query, its original fixture, camera and geometry checks remain unchanged. No scene identity is added or removed; the geometry scene also checks acquisition and rendering of another saved map.

Verification: the draft-name/link and player-filter regression, registry/catalogue cross-check and generic acquisition regression each failed before implementation, then passed. Focused router/start and registry scene tests pass; the geometry browser scene preserves its existing checks and verifies the generic river view. The player menu's captured DOM and PNG bytes match the baseline. A large saved-map probe failed at the original lab's fixed ray reach, and React's profiler-size guard caught the full map copied through component props. The viewer now scales ray reach with camera range and map extent, and passes a captured map reader across its component boundary; both checks then passed, including an actual overview click. A shared-factory source mutation also failed with only the map lookup pinned to the base village and the new API intact, then passed with caller-owned acquisition; the existing village-family and endurance scenario JSON stayed byte-identical through the actual WASM factory. Browser evidence stays in ignored scratch.

The independent visual review used the screenshot-critique skill's adversarial fallback because a fresh agent could not be created. It examined the complete desktop, narrow and full-content listing set and geometry frames/crops. The strongest visible countercases were: saved maps push fixtures below the first fold; bright links might merge with the dark field; a reduced full-page image could hide narrow clipping; the player menu might shift; the generic river could load a wrong or empty world; and the retained blocky geometry arena could look unfinished. Native crops showed readable labels, wrapped narrow descriptions and no clipped links; the menu's byte parity and the framed river/bridge confirmed the relevant boundaries. The developer arena's geometry is retained diagnostic content, not acceptance of a new landscape. The large-map desktop, narrow and panel crop show a readable hit readout; diagram-like fields and haze remain landscape questions outside C61. The complete C61 capture set is accepted within this inspection scope and was opened in one Preview window for the nonblocking human checkpoint.
