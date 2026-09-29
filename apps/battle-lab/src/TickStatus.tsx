/** A lab panel's first line: the published tick and the battle's status. */
export function TickStatus({ tick, status }: { tick: number | undefined; status: string }) {
  return (
    <div>
      Tick {tick ?? "—"} · {status}
    </div>
  );
}
