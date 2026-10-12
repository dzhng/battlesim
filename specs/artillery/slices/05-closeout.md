# Slice 05 — Hard-cutover closeout and full verification

## Contract unlocked

The artillery feature is a coherent shipped change: authored sources, resolved catalog, runtime assets, AI commands, browser presentation, replays and parity evidence all identify the same behavior. Existing content that should remain unchanged retains its prior digest; artillery-enabled scenarios use the named new catalog/replay identity.

## Closeout work

- Regenerate and review `fixtures/catalog.json` and any browser/WASM packed publication derived from the source fixtures.
- Bless only intentionally changed parity/replay records, with the behavior change named in the choices/receipt. Do not add compatibility readers or migration branches.
- Run the artillery lab and affected existing weapon/skirmish scenes, then the full closeout checks once the implementation slices are complete.
- Re-run catalog, flight, weapons, AI, browser unit and visual suites. Retain evidence in ignored `throwaway/evidence/`.
- Update this README's status, checklist, next-agent section and any slice choices. If complete, invoke `close-spec` to archive the rationale under `specs/done/`.

## Verification

Use the repository-owned commands from `README.md`, `crates/sim/README.md` and `web/README.md`. At minimum, the closeout must include:

```bash
bun run check
bun run verify
```

Run these only after the focused slice checks are green. The full run answers the question that the new physical roster and AI/UI paths have not broken other battles; it is not an iteration gate.

## Firewalls and delegated choices

Do not broaden this closeout into balance tuning, new maps, sound production, renderer architecture or migration support. Those are separate specs if the evidence demands them. A replay/catalog digest change caused by enabling artillery is expected and must be named; an unrelated digest change is a failure to investigate.

## Acceptance and feedback

The feature is ready for close-spec only when every checklist item is complete, all required visual shots have been critiqued and compared, full checks are green, and the hard-cutover identity change is recorded. Any remaining open item stays explicitly on the README rather than being silently delegated to a future implementer.

