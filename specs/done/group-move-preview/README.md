# Group move preview and placement

A right-button press shows each selected unit's destination marker. Holding and dragging rotates the layout about the clicked front center; release commits it and hands the markers to the usual confirmation fade. The purpose is to let the player judge an order before issuing it, using the visual language they already know.

## One placement authority

Preview and command admission use the simulation's [placement owner](../../../crates/sim/src/formation.rs), through [Battle](../../../crates/sim/src/battle.rs). The browser receives destinations and validity through the [command contract](../../../crates/contract/src/command.rs); it does not reconstruct navigation. Admission retains the accepted placement until application, while replay records intent and reconstructs admission from the same state. An earlier preview may be corrected if units or known geometry changed before release.

Current relative arrangement is retained where it fits. The nominal front center comes from settled footprints in the first selected unit's frame. This keeps rotation anchored without inventing rows or tactical roles. Obstacles may displace individual slots without moving the anchor.

## Space and finite work

Footprint demand sets the initial extent; obstacles can expand the search. Conservative hull discs and squad settled-area discs reserve the selected group's successful destinations. Partial placement follows the user's choice: for replacement orders, units with slots move and the others hold; a failed queued waypoint preserves its queue. Moving destinations also avoid the held units' current footprints. Repacking keeps unaffected slots and shares the original search allowance.

No selection cap is imposed. Every member gets an initial intended-point check and all alternative checks share a finite allowance. Failure means the bounded search found no slot, rather than proving no arrangement exists. These reservations do not include unrelated orders, and known-map standing room does not certify an entire future route or every soldier's cover position.

Failed unqueued moves cancel movement; failed queued waypoints preserve the existing queue. An explicit facing sets the arrival-facing target for pivot-capable units; an obstructed tracked pivot can stop short of that heading. Otherwise, and for wheels, heading is an approach estimate refined by routing; normal combat-facing rules still apply.

## Presentation and replay invariants

[Pointer paint](../../../apps/battle-lab/src/pointerPaint.ts) keeps one query in flight and displays the latest available result while requesting the latest cursor state. Gesture generations prevent cancelled replies from painting a new press, even at the same anchor. The renderer uses the existing [destination marker](../../../packages/battle-renderer/src/orderOverlay.ts), and confirmation uses the existing [order fade](../../../web/src/battle/present/orderReveal.ts).

[Navigation placement probes](../../../crates/sim/src/navigation.rs) may warm clearance values but cannot pay route-work charges. Otherwise merely holding the mouse could change scheduling and replay outcomes. Native [formation](../../../crates/sim/tests/formation.rs) and [navigation](../../../crates/sim/tests/navigation.rs) checks pin spacing, partial fallback, bounded work and future/replay parity. The [village browser scene](../../../web/scenes/village.mjs) pins the press, drag and confirmation flow.

## Visual provenance

The requirement came from the user's Total War-style interaction description; no external reference image defined the style. Project-owned browser captures preserve the historical [one-marker baseline](assets/reference/before.png), its matched [per-unit comparison](assets/reference/per-unit.png), and the [front-center result](assets/reference/front-center.png). The baseline is historical behavior, not the desired design. The final capture is wider so all rings and arrowheads remain visible.

An unprimed visual review checked complete geometry, ground perspective and unit layering across held, released and committed states. The existing palette and callout layer are retained: marker contrast is weaker on tan roads, and a floating callout can cross a destination ring. Committed order lines resolve which unit owns the destination.
