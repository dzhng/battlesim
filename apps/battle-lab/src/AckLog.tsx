import type { AckEntry } from "@web/battle/input/useUnitControl";

/** One command outcome, verdict first. */
function AckLine({ entry }: { entry: AckEntry }) {
  const { ack } = entry;
  return (
    <li className={ack.error ? "lab-rejected" : "lab-accepted"}>
      {ack.error
        ? `✕ rejected (${ack.error.reason.replaceAll("_", " ")})`
        : `✓ accepted, applied at tick ${ack.applied_tick}`}{" "}
      — #{entry.seq} {entry.label}
    </li>
  );
}

/** The command acknowledgement log, newest first. */
export function AckLog({ acks }: { acks: readonly AckEntry[] }) {
  return (
    <>
      <div className="lab-hint">Commands, newest first</div>
      <ul className="lab-log" data-testid="ack-log">
        {acks.length === 0 && <li>None yet</li>}
        {acks.map((a) => (
          <AckLine key={a.seq} entry={a} />
        ))}
      </ul>
    </>
  );
}
