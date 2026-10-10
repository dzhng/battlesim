# 18 — Helicopters and layered observation

**Status:** superseded on 2026-10-10 by [specs/helicopters](../../helicopters/README.md).

The user replaced this slice's design. Don't implement it.
- **Was:** two flight modes, normal at 60 m and low at 15 m, with low flight trading sight for stealth.
- **Now:** helicopters always fly in one low-air layer. They cruise about 20 m above the ground, pop up over roofs, and route around anything taller than 30 m.

Requirements V01, V02 and V05 are carried there. A01 (a low-flight mode) and V06's "low helicopter flight" detection modifier are superseded, because there is no separate low-flight mode.
