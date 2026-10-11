# Choices

These are agent decisions in the finished contextual cursor feature, checked against its final code. The user chose Arrow + action, plain movement, one reachable entrant, nearby gathering for every companion, and useful partial success. Review the finite planning allowance, retained armed holding work, and screen-space overlay first: those have the most judgment behind them.

## Sound — medium confidence

### Divide finite planning work between earlier orders, entry and gathering

A far squad can require a long search while a nearer squad could enter and a tank could gather. The planner shares the existing movement-validation allowance across earlier queued work, entry-route search, physical proof and fallback gathering. Route searches take turns rather than allowing the first squad to consume everything. An unfinished proof cannot justify choosing a worse entrant; useful gathering may still succeed. The plan required bounded fair work but did not choose the allocation. Future large selections and complex maps inherit this completeness tradeoff. **Sound, medium:** it keeps work finite and preserves uncertainty, but no allocation proves every possible large group.

### Keep compatible armed work on a squad already holding the building

A selected squad inside the house may have an attack order that fires from its position. Clicking that house again gives the squad's existing hold priority and gathers its companions. An unfinished physical entry still requires bounded proof before it earns an entry promise. Shift retains compatible holding work; a replacement cancels a queued departure that would undo the new request. Compatibility scans beyond attacks because an expiring contact can expose a later exit. An outside squad with only a future reservation instead competes afresh on replacement. The plan left the boundary of “already entering” and compatibility open. Later queue changes must preserve the distinction between a physical position and a deferred promise. **Sound, medium:** useful holding fire survives, though clearing every previous order would be another coherent control policy.

### Let the system draw the cursor

When the player points at a house, the arrow tip sits at the actual pointer and the badge sits below/right. Each action is one image ([cursorImage](../../../web/src/battle/present/gameCursor.tsx)), composed from the generated arrow and badge icons in the HUD's colours, which the app hands to the system as the CSS cursor. The system draws it apart from the page, so it moves with the pointer however long the battle takes to draw a frame. The first version positioned a DOM element instead, because headless screenshots omit a system cursor and could not prove the result. But a page element moves only when the page presents a frame, and players felt the lag. The cursor lab now draws each action's image into the page, where screenshots pin it; scenes check which image the system is given. The image stays within 32 CSS pixels, the largest a browser draws everywhere; larger cursors fall back to the plain arrow near the window's edge. **Sound, high:** this is how games in a browser get a lag-free pointer, and the art and actions are unchanged.

### Leave older movement uncertainty plain

A building query reports whether proof stopped early. The older movement query returns only placed or unplaced destinations, so all-unplaced movement cannot distinguish a known refusal from unfinished proof. Hover keeps the plain arrow instead of asserting a rejection; an actual admission refusal can still show blocked confirmation. The spec did not widen that older query. A future definitive blocked-movement badge needs a real certainty result from its owner. **Sound, medium:** conservative feedback avoids a false promise without changing existing movement admission.

### Reuse the existing action symbols

Pointing at an enemy adds the game's existing aim mark; a known refusal adds its rejection cross. The approved illustration used another icon family, but the prompt allowed production symbols and did not choose every exact glyph. A second cursor-only family would teach two symbols for one action. Future actions extend the generated icon owner rather than copying SVG paths into the cursor. **Sound, medium:** the shapes share the player's established visual language; the precise symbol choice still involves taste.

## Sound — high confidence

### Admit entry and companions as one intent

Two squads and a tank click a house. The browser sends one combined building order; the simulation chooses an entrant and gathering destinations together. Separate commands could let entry change the conditions used to admit movement and make the hover preview disagree with release. The requested outcome did not specify the command boundary. Future clients inherit one admission and acknowledgement, while authored single-squad garrison commands retain their distinct operation. **Sound, high:** one authority can prove the complete useful action together.

### Enforce the proved approach using existing orders

A squad's reachable approach may be around the corner. Its first movement order targets that exact point, using ordinary squad spread, cover and arrival tolerance, and then executes the existing garrison operation; a direct garrison command alone could start entry at another nearby wall. The plan promised a route but left its execution representation open. The prepared entry decision and placement remain together until application, which does not reinterpret them. Future consumers need no new persistent unit state. **Sound, high:** the action follows the route that earned its promise.

### Recheck surviving moves with failed movers held

A tank cannot reach its gathering spot. For a replacement order, the squad's entry and every surviving gather move are rechecked with that tank still occupying its real position. For Shift, its earlier finite work continues in the proof and only the failed new move is omitted. Further failures can require another pass, and no unfinished repair becomes an accepted promise. The plan required joint certification but left exhaustion during repair open. The strict stabilized proof applies to combined building plans; ordinary movement admission retains its prior behavior. **Sound, high:** entry cannot depend on a failed companion moving away.

### Carry uncertainty through the building result

A crowded route can consume the finite proof allowance before the planner decides whether entry is possible. The building result carries an optional `unproven` flag, meaning its unsuccessful portions are undecided. Successful entry or gathering still determines the useful action; a wholly unsuccessful undecided result stays plain. The initial result shape lacked that distinction. Future preview consumers can separate unfinished work from a known restriction. **Sound, high:** a bounded search does not pretend to prove impossibility.

### Define shortest at the game's navigation resolution

The closest squad in straight-line distance can face a long detour. Completed shortest-policy paths to exposed facade samples are compared by path length, with unit ID and stable facade order breaking ties. An unfinished competing route whose lower bound could win vetoes a fresh entrant. The user chose reachable route distance, while the exact geometric precision and ties were left open. Future work inherits the game's existing route grid rather than a separate continuous-space solver. **Sound, high:** the result is deterministic and uses the existing movement owner.

### Use one grouped approach direction

A column arriving from the west gathers on the west side. The selected group's centre and the building define that direction; queued planning uses proved earlier endpoints. Degenerate directions fall back deterministically to entry geometry or stable unit order. The user chose an approach-side gathering policy but did not specify its calculation. Future formations inherit one front with room for the entrant rather than independent nearest walls. **Sound, high:** it keeps the group together without inventing a tactical flank.

### Plan combined orders at the same deterministic tick

A queued garrison exit can use tick-seeded placement. Live admission, authored combined commands and replay therefore plan against the tick before application; a read-only hover query uses the current tick. Otherwise the same recorded intent could choose a different exit during replay. The plan required replay equality but left this timing convention open. Future combined-order callers must use the same planning time. **Sound, high:** the recorded order has one deterministic meaning across entry points.

### Make hover describe the command without consuming it

With attack-ground armed, moving the pointer describes the next click but neither disarms the mode nor issues an order. One pure resolver supplies precedence to hover, held preview and dispatch; only dispatch consumes a gesture and applies existing mode-reset behavior. The old input path mixed description and mutation. Future precedence changes have one policy owner. **Sound, high:** mouse motion cannot change gameplay orders.

### Capture the press and prune only what becomes invalid

The player presses over a house, then drags away or changes the live selection. The order keeps the pressed target, unit IDs and modifiers; dragging changes facing. Observed casualties are removed, an expired contact is refused, and a battle reset cancels the old press. No newly selected unit is silently added. The plan required a captured intent but left invalidation details open. Future input modes inherit a stable press with current validity. **Sound, high:** release follows the gesture the player began without commanding dead or unrelated units.

### Refresh a moving battle without starving feedback

A hover query can finish after another publication arrives. If its semantic action is unchanged, its result stays visible while a newer query refreshes it. A change to target, side, client, selection, eligibility or known geometry immediately clears obsolete feedback, and at most one query runs at once. Geometry identity includes the separate clearing count and epoch: a felled tree need not alter remembered props, and a new epoch can replace cleared cells at the same count. The plan did not define publication-versus-intent identity. Future preview owners inherit coalesced refresh rather than one request per frame. **Sound, high:** continuous ticks do not keep the cursor permanently pending.

### Let admission own release confirmation

The player releases a faced gathering drag while the worker is still answering. The held request retains its marks until admission responds; that authoritative result then owns confirmation until its applied publication. A fresh hover over the same target has no facing gesture and can show that result, but a different target, Shift state or command kind cannot inherit it. The plan required current-state race handling but left the display handoff open. Future commands share a clear request-to-acknowledgement lifetime. **Sound, high:** the displayed result follows what the worker actually admitted.

### Treat a building-preview failure as local to that query

A selected unit may die while its building-preview request is in flight. That query settles with an error and conservative plain feedback; surviving units can still receive commands. A reset also prevents the old acknowledgement from repopulating the new battle's log. The plan did not specify these asynchronous error boundaries. Future building-preview consumers inherit query-local failure while terminal worker errors remain terminal. The existing movement WASM query already turns normal validation refusals into empty destinations. **Sound, high:** a routine input race does not stop the battle or contaminate its replacement.

### Keep refusal text clear of the whole cursor

A failed companion can produce a long callout beside a successful garrison badge. The callout uses its measured size to flip and clamp at viewport edges and sits beyond the complete arrow-plus-badge bounds. A fixed label estimate would clip longer text at narrow widths. The approved concept did not specify refusal placement. Future refusal text inherits measured bounds without hiding or shortening the actual message. **Sound, high:** the player can read both the useful action and its failed portion.

### Give partial refusal its own real diagnostic encounter

To exercise a squad entering while a selected tank cannot move, a diagnostic encounter places that tank inside ordinary unpushable bodies on the existing garrison map. A third unselected squad supports the separate direct-garrison refusal. The baseline encounter does not change, and no query response or rule is faked. The plan required an observable partial failure but did not choose a fixture. Future browser checks inherit a small controlled encounter that uses the real catalog and authority. **Sound, high:** the difficult state is reproducible without changing the game to make the example happen.
