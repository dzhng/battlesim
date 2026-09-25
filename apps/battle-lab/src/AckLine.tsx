import type { AckEntry } from "@web/battle/input/useUnitControl";

/** One command outcome, verdict first. */
export function AckLine({ entry }: { entry: AckEntry }) {
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
