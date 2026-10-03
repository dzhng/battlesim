# Choices

Planning delegation: the user authorized applying useful intent, actual reachability and partial success to routine cases. User-selected behavior and visual decisions live in the README; this ledger holds agent decisions that fill gaps.

## Sound — medium confidence

### One group intent is admitted as a whole

When two squads and a tank click a house, the worker chooses the entrant and gathering positions together. Sending a separate entry command followed by a move would let the first command change what the second can do, so preview and execution could disagree. The prompt specified the result but not the command boundary. One `OccupyBuilding` intent records that result and lowers to the existing unit queues. Future callers inherit one admission/acknowledgement contract. Sound: the simulation remains the one authority.

### A screen-space cursor overlay makes production evidence inspectable

Moving over a house places the arrow tip exactly at the input pointer and adds the building symbol below/right. A native CSS image cursor would also work, but ordinary headless screenshots omit the system pointer. The overlay uses the current viewport pointer, renders once and changes its position directly; no GPU pass or dependency is introduced. The prompt fixed appearance, not rendering mechanism. Sound: the existing viewport already reports pointer position every frame, and actual gameplay captures can prove this same component. Medium confidence: pointer responsiveness must be judged in the real route. Native CSS hotspot/fallback alternative is documented by [MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/cursor).

## Sound — high confidence

### Queue and route ties preserve predictable intent

Shift-clicking a building queues entry for the chosen squad and gathering for companions after finite earlier orders; a failed queued destination keeps what the unit was already doing. Equally short entry routes choose the same unit ID even if selection order changes. The user specified queue behavior only indirectly through existing controls and asked the agent to settle routine cases. These choices preserve useful intent and deterministic replay rather than inventing order based on mouse-selection history.

### Gathering has one direction and does not surround the building

A column approaching from the west gathers on the west side, leaving the entrant's lane open. Sending each unit to its own nearest wall could place some around the east corner and expose them unexpectedly. The user chose the approach side but did not define its calculation. The group centroid and building define one direction, with physical footprint clearance and stable degeneracy handling. Sound: it matches the user's chosen grouped approach and existing rear-side formation placement.

### Already selected occupants stay inside

If the selected squad already holds the clicked house, it keeps that position while its selected companions gather outside. Moving it out merely because entry is unnecessary would undo the useful part of the click. The prompt did not specify repeat clicks on a selected occupant. Sound: reaching the requested state is idempotent and no extra capacity or eviction rule is introduced.
