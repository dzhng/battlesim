# C01: building aggregates

**Depends on:** C00 and any required prerequisite scale architecture. **Kind:** slice.

## Question
Can one placed building own physical parts and template geometry with one integrity, garrison and terminal state?

## Contract it unlocks
`contract::map` defines `BuildingDefinition` from C00: parts, exposed edges/bays, height/floors/floor heights, entrances, local frame and `template_id`. The map pins `template_catalog_hash` once; appearance identity stays outside simulation data. The sim reads only final placed geometry.

- At world load, parts become props first; an owner prop identifies one building. Keep existing village house PropIds for this aggregate-only cutover.
- `building_of(PropId)` resolves all parts to their owner; structures stores one integrity and garrison admission remains per building.
- Hits wear the owner; destruction replaces every part atomically, including side-known replacement identities.
- Slots use exposed geometry only; band/cap changes wait for C40.
- Prop export gains owner/template reference data, and the browser decoder cuts over in the same commit.
- Existing map fixtures migrate to aggregates in this commit. Their original house geometry/appearance gets a legal descriptor; C13 later bakes reusable placements and C32/C22 resolve them. No per-building GLB or duplicate fact table is introduced.

## API seam
`contract::map`, `sim::world`, `structures`, `garrison`, world export and observation decoder. C04 consumes C00 descriptor types without importing scene-assets implementation.

## What the human can run or see
A part-B hit wears one integrity; a compound collapses atomically; a geometry/appearance overlay shows the descriptor's original house fit.

## Verification
- Hit ownership, atomic replacement and no slots on interior/non-exposed edges.
- Existing village digest/replay parity and lab scenes; C56's new surroundings are a later named geometry change.
- Descriptor geometry is transformed/materialized once with correct units/frame and immutable reference identity.
- Visual evidence: compare ownership/fit overlays against original geometry with compare-screenshots; run unprimed screenshot-critique last; preview-shots non-blocking.

## Delegated to the implementer
Owner-ID/export encoding and internal descriptor representation within G0's contract. New geometry owners or rule changes are spec gaps.

## Must stay green
Original aggregate-only outcomes and appearance; final building facts have one owner.

## Feedback that would change this slice
A descriptor/observation seam mismatch must be fixed before mapgen or placement consumers depend on it.

## Systems contract being implemented

A building owns the final C00 materialized geometry, category and regional family, its prop type, one owner PropId, and named part-to-PropId references. `MapDefinition.buildings` is the only placed-building fact table. The map pins one physical template catalogue hash; materials, source readiness and appearance stay outside that identity. The simulation does not load a template catalogue or appearance bundle. It validates internal placed-geometry and ID invariants; verification against the pinned source catalogue belongs to the common preparation/physical-library boundary in C09/C13/C32 and remains pending here.

Ordinary authored props wrap geometry with an optional explicit ID. Buildings reserve their explicit part IDs; omitted ordinary IDs fill the remaining namespace deterministically. All authored IDs form one dense `[0, count)` namespace, checked for collisions, range and completeness before allocating prop storage. Dynamic additions carry geometry only and allocate monotonically after that namespace. Every part resolves to exactly one building owner. Current source fixtures carry explicit IDs so the aggregate cutover preserves original body identities and ordering.

Generic exposed spans supply the existing capacity-driven seats; partial/interior faces receive no seats. No singleton compatibility seating branch or inferred ~3 m bay pattern is introduced. Existing source floors, entrances and bay patterns remain absent. Consumers requiring those facts must reject unresolved descriptors through the completeness boundary; physical completeness does not accept source/art fit.

Damage is per owner. A direct hit wears that owner; one blast uses its strongest (nearest-part) contribution once, rather than multiplying an explosion by part count. A terminal transition replaces all physical parts in one tick and releases a garrison once. Seeing any part reveals the owner's terminal state and all its replacements in one observation; a wholly hidden transition retains every previous part until revealed. These aggregate rules extend singleton behavior without changing the original house outcomes.

Digest ownership records nonidentity part-to-owner associations and outcome-relevant physical facts. Identity singleton associations add no state beyond the original owner. Future floors/edges/bays that change rule outcomes must be included rather than silently omitted. The original production oracle freezes complete observations, exact queries/seats and battle digests before this pass; representation changes may normalize only explicitly added public identity metadata.

`building_of(part)` names the immutable placed-building/fact identity. `structure_owner(part)` names the current state's one live integrity/garrison prop. An atomic replacement gives all its new parts one fresh active owner; immutable template/category association stays at the original fact owner. Removed original IDs still have no HP. This distinction preserves ordinary destroyable-remains chains rather than making a replacement inherit an already-destroyed integrity key.

Materialized exposed-edge records retain their exact authoritative part-local `span_m` alongside owner-emitted world span/normal. The generic garrison path uses local bounds and the part frame, preserving original rotated-seat arithmetic without inverse projection. The placed-geometry validator must reject contradictory local/world spans before a simulation consumer reads them. The original world values remain the preservation oracle; the new local-span field is an explicit metadata addition.

The public prop record has one exact ID representation: `idLo`, `idHi` (16-bit limbs), followed by the unchanged physical columns `kind,x,y,yaw,hx,hy,hz,baseZ`; its stride is 10. `WorldView.buildings()` publishes a JSON record with `catalogueHash` and immutable building rows (`owner`, `kind`, `templateId`, `category`, `regionalFamily`, named `parts` with exact integer `prop` IDs). This metadata references the existing exported prop geometry and does not duplicate it. The static picker resolves a physical hit to that public owner and admits its prop type by the catalog's garrison capability.

A `KnownProp` publication adds its exact current `id`, optional immutable `building` ID, optional current `structure_owner`, and optional immutable `authored_prop`; these and the existing optional `replaces` use the established 16-bit limb encoding. `replaces` retains its immediate replacement meaning; the building association survives a replacement chain independently. The source association is owned by the world for every public-map body and copied once into remains at creation; genuinely dynamic bodies retain no authored source. The garrison owner field also uses exact limbs. These fields share the existing atomic observation record, stream epoch and credit lifecycle. No ground/fog layout or tick coalescing changes are introduced here.

Each seat retains its source exposed-edge ordinal. Sensing averages seats per held physical edge, in directional-group then edge order: separate stepped exterior faces cannot average into an interior body. A box's four original facade eyes and arithmetic stay exact. Approach and entry inspect physical member parts; a shot fired past the occupied owner skips every physical part of that active state. All other sight and round queries still meet those bodies.

## Current systems evidence and deferred gates

[The aggregate proof](../assets/building-aggregate/README.md) keeps the original observations, queries, seats, animation words and singleton digest traces frozen. The additional physical API fixtures prove ownership and knowledge across parts without accepting new production models or dimensions. The existing box constructor is preparation code shared by fixture/endurance and analytic test producers; the world has no fallback that silently turns ordinary props into buildings.

Placed parts require immovable body rows until composite motion exists. A garrison-capable aggregate requires an exposed physical span. Missing source floors, entrances and bays remain unresolved rather than being inferred from appearance trim. Catalogue-versus-placement provenance verification and real template/source/appearance fit remain pending in their owning slices; complete visual/source G0 stays open.

`Battle::digest` keeps its state boundary: complete immutable map/classification/catalogue identity belongs to `scenario_digest`. The one original full-face garrison box is physically implicit in its collider and preserves its original digest. Other placed geometry, including its grouping frame and exposed intervals, has a cached physical fingerprint because current seats/eyes read it. Nonidentity member associations are hashed; source ancestry and replacement membership/history derive from the original authored namespace, hashed immediate replacement links and live prop identities. No category/family-specific compatibility branch defines this boundary.

Holdable replacement states resolve their current member props through the existing immutable authored-source association before seating. Their published footprint is the current-part envelope in the building frame, with a world centre and local half extents; the existing ring/ruler consume its circumscribed radius. An original singleton retains its exact fields. Static public-map picking still returns the immutable authored owner; interaction with fresh current replacements through side-known geometry belongs to C43. Direct commands may address the delivered current `structure_owner` already.
