# 12: closeout

## Contract

Smoke is shipped and playable wherever units fight (D18). The whole repository
check ladder has passed once, and the spec is closed.

## Steps

1. **Pull LFS:** runtime and third-party audio, roster and infantry sources,
   scene baselines, and the city (memory: full run needs LFS).
2. **Run the full repository closeout checks once** (development.md, check
   ladder step 3), including every browser scene and the native checks.
3. **Run the balance report once.** Record the paired before and after in
   `choices.md`. Don't retune in this pass (`tweak-mechanics`, step 8). Retuning
   is a separate pass if the user asks for one.
4. **Run `/review`:** shape, diff and docs. Update the READMEs that own what
   changed:
   - `crates/contract/README.md`: the smoke module and capability;
   - `crates/sim/README.md`: clouds and sight depth;
   - `crates/presentation/README.md`: the smoke-screen effect and fog smoke;
   - `fixtures/README.md`: `smoke_looks` and sounds.

   Make sure no doc still calls a transient prop a smoke screen.
5. **Audit choices:** run `audit-choices` over `choices.md`, then `close-spec`
   to archive this plan to `specs/done/smoke-screen/`.

## Must stay green

Everything.
