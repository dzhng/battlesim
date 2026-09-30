# C00: physical template descriptors

**Depends on:** G0 and required geometry prerequisites. **Kind:** slice.

## Question
Can mapgen and asset tools share one physical template contract without placing art identity in simulation data?

## Contract it unlocks
`contract::templates` owns `BuildingTemplateDescriptor` and `TemplateGeometryCatalog { hash, templates }`. Each immutable descriptor declares category, regional family, local-metre parts, height/floors/floor heights, entrances, exposed edges, legal facade bay positions/phase and supported joins. The catalogue hash covers physical metadata only. Materials, textures, module placements and LODs have a separate appearance hash.

A placed building stores `template_id`, a translation/rotation frame and materialized physical geometry; `MapDefinition.template_catalog_hash` pins the catalogue once per map. No art hash belongs in the map, scenario digest or battle digest. Freeze Q3's ~3 m facade bay policy at G0; descriptor bay positions follow that policy and art aligns to them. Sim still owns floor-band admission, the 32-seat cap and occupancy. Neither art nor garrison independently invents a different facade phase.

## API seam
Pure contract data/encoding → mapgen materialization and scene-assets fit validation. C01 owns the simulation aggregate cutover; C13 exports catalogue data and C32 binds appearance to it.

## What the human can run or see
A small asymmetric descriptor overlay and canonical catalogue report, including the original village house shape and one compound.

## Verification
- Canonical physical identity is unchanged by materials, LODs or replacing prototype art.
- Transform/materialization, bay/entrance frames and unsupported joins have deterministic seam fixtures.
- Invalid source descriptors fail before downstream consumers receive them.
- Compare physical overlays against S2's fit evidence with compare-screenshots; run unprimed screenshot-critique last; preview-shots is non-blocking.

## Delegated to the implementer
Internal encoding and names within G0's frozen descriptor/identity choices. New physical dimensions and facade policy are not delegated.

## Must stay green
One physical geometry contract and lossless native/wasm encoding; no art-driven digest change.

## Feedback that would change this slice
A source unable to fit the accepted bay/floor policy reopens its legal recipe at G0.
