import type { AckEntry } from "@web/battle/input/useUnitControl";

/** One command outcome, as a terse log line: the order, then its verdict
 *  (the tick it took effect, or why it was refused), marked ✓ or ✕ so the
 *  verdict never rests on colour alone. */
function AckLine({ entry }: { entry: AckEntry }) {
  const { ack } = entry;
  return (
    <li className={ack.error ? "lab-rejected" : "lab-accepted"}>
      {ack.error ? "✕" : "✓"} #{entry.seq} {entry.label} ·{" "}
      {ack.error
        ? `rejected: ${ack.error.reason.replaceAll("_", " ")}`
        : `tick ${ack.applied_tick}`}
    </li>
  );
}

/** The command acknowledgement log, newest first. */
export function AckLog({ acks }: { acks: readonly AckEntry[] }) {
  return (
    <>
      <div className="lab-hint">Command log</div>
      <ul className="lab-log" data-testid="ack-log">
        {acks.length === 0 && <li>None yet</li>}
        {acks.map((a) => (
          <AckLine key={a.seq} entry={a} />
        ))}
      </ul>
    </>
  );
}
