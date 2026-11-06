# Evidence and research

Planning baseline: `0a156e668e6077c2c3f4d8b17158f60fb3724b2e`, with a clean tracked
worktree before this spec. The audit IDs refer to the 2026-11-06 source audit; the
[scope ledger](scope.md) preserves the decisions needed after scratch evidence is
removed. It does not require the original chat or report to resume.

Repository evidence is the primary authority: source transition owners, subsystem
READMEs and existing consumer tests. Discovery began with `jevgrep`; the original
wide audit reported incomplete discovery, so exact source inventories and direct
reads supplemented it. Independent drafters used narrow behavioral searches and
then inspected actual transition code. This plan is not a formal enumeration of
all possible states or proof of current correctness.

## Primary external references

Fetched and inspected during planning on 2026-11-06:

- [React: extracting state logic into a reducer](https://react.dev/learn/extracting-state-logic-into-a-reducer).
  Consolidating related state updates into one reducer fits the editors. Keep
  reducers pure; request execution and resource cleanup belong to the operation
  controller/effect, not reducer execution.
- [React: useEffect](https://react.dev/reference/react/useEffect).
  Cleanup belongs to its setup lifetime; async responses may complete out of
  order. Keep identity/cleanup gates, and preserve development setup/cleanup/setup
  behavior. Replacing many booleans does not remove this requirement.
- [TypeScript: narrowing and discriminated unions](https://www.typescriptlang.org/docs/handbook/2/narrowing.html).
  Use payload-bearing variants for mutually exclusive phases and exhaustive
  handling. Independent operations are separate records, not a cross-product enum.
- [MDN: ReadableStream.cancel](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStream/cancel).
  Cancellation is its own stream operation and propagates to the underlying
  source; it must not be conflated with normal completion. This supports keeping
  download abnormal-completion investigation separate from the current cleanup.

No external statechart library or reference implementation is being ported.
No reproduction spike, assets, API credentials, paid service calls or dependency
installation are needed. Library selection is closed: use the repository's
existing React/TypeScript and resource owners.
