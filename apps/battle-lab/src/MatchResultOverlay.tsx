import { cardIcon } from "@packages/scene-assets/src/icons";
import { hudIcon, stateIcon } from "@packages/scene-assets/src/icons";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { Icon } from "@web/battle/present/icons";
import type { MatchEventView, ObservationView } from "@web/battle/sim/observationView";
import game from "@fixtures/game.json";

type Report = NonNullable<NonNullable<ObservationView["skirmish"]>["report"]>;

const sideName = (side: "blue" | "red") => (side === "blue" ? "BLUE PLAYER" : "RED PLAYER");

function UnitGlyph({ kind, side }: { kind: string; side: "blue" | "red" }) {
  return <Icon path={cardIcon(kind)} className={`match-result-unit match-result-unit-${side}`} />;
}

function UnitLabel({ kind, id, cost }: { kind: string; id: number; cost?: number }) {
  const { units } = useSessionCatalog();
  return (
    <span>
      {units.has(kind) ? units.type(kind).name : kind} #{id + 1}
      {cost === undefined ? null : <small className="match-result-unit-cost">{cost} CREDITS</small>}
    </span>
  );
}

function EventRow({ event }: { event: MatchEventView }) {
  const seconds = Math.floor(event.tick / game.tick_hz);
  if (event.kind === "kill") {
    return (
      <div className="match-result-event" data-event="kill">
        <time>{`${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`}</time>
        <div className="match-result-actor">
          {event.killer ? <UnitGlyph kind={event.killer.unitKind} side={event.killer.side} /> : <span className="match-result-unknown">?</span>}
          <span className={`match-result-event-side match-result-event-side-${event.killer?.side ?? event.side}`}>
            {event.killer ? <><b>{sideName(event.killer.side)}</b><UnitLabel kind={event.killer.unitKind} id={event.killer.unitId} cost={event.killer.cost} /></> : "UNKNOWN SOURCE"}
          </span>
        </div>
        <Icon path={hudIcon("rejected")} className="match-result-action match-result-kill" title="killed" />
        <div className="match-result-actor match-result-victim">
          <UnitGlyph kind={event.unitKind} side={event.side} />
          <span className={`match-result-event-side match-result-event-side-${event.side}`}>
            <b>{sideName(event.side)}</b><UnitLabel kind={event.unitKind} id={event.unitId} cost={event.victimCost} />
          </span>
        </div>
      </div>
    );
  }
  const spawn = event.kind === "spawn";
  return (
    <div className="match-result-event" data-event={event.kind}>
      <time>{`${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`}</time>
      <div className="match-result-actor">
        <UnitGlyph kind={event.unitKind} side={event.side} />
        <span className={`match-result-event-side match-result-event-side-${event.side}`}>
          <b>{sideName(event.side)}</b><UnitLabel kind={event.unitKind} id={event.unitId} />
        </span>
      </div>
      <Icon path={stateIcon(spawn ? "deploy" : "withdrawing")} className="match-result-action" title={spawn ? "spawned" : "refunded"} />
      <span className="match-result-event-copy">{spawn ? "SPAWNED" : "REFUNDED"}<small>{spawn ? `${event.amount} credits` : `${event.amount.toFixed(0)} credits`}</small></span>
    </div>
  );
}

function TotalsRail({ report }: { report: Report }) {
  return (
    <div className="match-result-totals" aria-label="Match totals">
      {["blue", "red"].map((side, index) => {
        const totals = report.totals[index] ?? { spawned: 0, spent: 0, killed: 0, refunded: 0 };
        return (
          <div className={`match-result-total-side match-result-total-${side}`} key={side}>
            <strong>{sideName(side as "blue" | "red")}</strong>
            <div className="match-result-total-items">
              <span title="Units spawned"><Icon path={stateIcon("deploy")} /><b>{totals.spawned}</b><small>SPAWNED</small></span>
              <span title="Credits spent"><Icon path={stateIcon("stock")} /><b>{totals.spent}</b><small>CREDITS SPENT</small></span>
              <span title="Units killed"><Icon path={hudIcon("rejected")} /><b>{totals.killed}</b><small>UNITS KILLED</small></span>
              <span title="Credits refunded"><Icon path={stateIcon("withdrawing")} /><b>{Math.round(totals.refunded)}</b><small>CREDITS REFUNDED</small></span>
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function MatchResultOverlay({ match }: { match: NonNullable<ObservationView["skirmish"]> }) {
  const report = match.report;
  if (match.phase !== "finished" || !report) return null;
  const title = match.result === "draw" ? "DRAW" : match.result === "blue" ? "VICTORY" : "DEFEAT";
  return (
    <div className="match-result-scrim" role="dialog" aria-label="Match result">
      <section className={`match-result-modal match-result-${match.result ?? "draw"}`}>
        <header className="match-result-heading">
          <h1>{title}</h1>
          <span className="match-result-score"><b>{Math.floor(match.scores[0])} — {Math.floor(match.scores[1])}</b></span>
        </header>
        <TotalsRail report={report} />
        <div className="match-result-timeline-wrap">
          <div className="match-result-timeline" aria-label="Unit action timeline">
            {report.events.map((event, index) => <EventRow event={event} key={`${event.tick}-${index}`} />)}
          </div>
        </div>
      </section>
    </div>
  );
}
