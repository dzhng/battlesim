# Forest shape candidate (historical)

C72 is now implemented; see [C72 one forest rule](../../slices/C72-one-forest-rule.md). This note records what the earlier unactivated candidate proved. Its patch and artifacts are only in tag `city-maps-evidence-2026-09-30`.

The candidate moved paving and woodland to one contract-owned ground shape, with native generation supplying physical primitives, explicit rectangle IDs and immutable trunk ranges, and rendering consuming those owners instead of assigning trees by bounds overlap. Its findings:

- All 57 native records (five fixtures, normal and late Endurance for two seeds, the literal forest producer under three density rows) matched byte for byte. That is rectangular source parity, not recovery of historical fractional rectangle widths.
- The bounds-overlap path assigned 130 placements to 84 generated trunks. Immutable source ranges and original IDs gave exactly the expected 84 ordered positions.
- The seven-query GPU check stayed red by one f32 ULP: the outside capsule endpoint returned `-1.0000001192092896` against `-1`. The other six distances and all seven membership flags matched.
