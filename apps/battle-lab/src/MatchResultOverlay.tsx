// The end-of-match report: the result as the viewer's side reads it, every
// player's totals as a scoreboard under their side, and the battle's story,
// each line naming the player behind it.
import { cardIcon, hudIcon, stateIcon } from "@packages/scene-assets/src/icons";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { Icon } from "@web/battle/present/icons";
import { playerName, type Seat } from "@web/battle/present/playerName";
import type { MatchEventView, SkirmishView } from "@web/battle/sim/observationView";
import type { SideName } from "@web/battle/sim/protocol";
import { clock, outcome } from "./battleStatus";

type Report = NonNullable<SkirmishView["report"]>;
type Side = MatchEventView["side"];
interface Unit {
  kind: string;
  id: number;
}

/** A spawn or refund line holds every unit one player bought (or sent home)
 *  within one minute of the match: at 5v5 the AIs buy in waves, and a line
 *  per unit would bury the fighting. A kill is always its own line. */
type Line =
  | {
      kind: "spawn" | "refund";
      tick: number;
      side: Side;
      owner: number | null;
      units: Unit[];
      amount: number;
    }
  | { kind: "kill"; tick: number; event: Extract<MatchEventView, { kind: "kill" }> };

function timeline(events: readonly MatchEventView[], tickHz: number): Line[] {
  const lines: Line[] = [];
  const open = new Map<string, Extract<Line, { units: Unit[] }>>();
  for (const event of events) {
    if (event.kind === "kill") {
      lines.push({ kind: "kill", tick: event.tick, event });
      continue;
    }
    const owner = event.owner ?? null;
    const key = `${event.kind} ${owner ?? event.side} ${Math.floor(event.tick / (60 * tickHz))}`;
    const unit = { kind: event.unitKind, id: event.unitId };
    const line = open.get(key);
    if (line) {
      line.units.push(unit);
      line.amount += event.amount;
      continue;
    }
    const fresh = {
      kind: event.kind,
      tick: event.tick,
      side: event.side,
      owner,
      units: [unit],
      amount: event.amount,
    };
    open.set(key, fresh);
    lines.push(fresh);
  }
  return lines;
}

/** Who the report is read by: the battle's players in scenario order, the
 *  human the client plays as, and their side (null when it watches). */
interface Names {
  seats: readonly Seat[];
  viewer: string | null;
  viewerSide: SideName | null;
}

/** The colour a player reads in, as the battle draws them: yours, an
 *  ally's, an enemy's; a watched battle's by side. */
type Tone = "own" | "ally" | "enemy" | SideName;
function toneOf(names: Names, side: SideName, seat?: Seat): Tone {
  if (names.viewerSide === null) return side;
  if (side !== names.viewerSide) return "enemy";
  return !seat || seat.id === names.viewer ? "own" : "ally";
}

/** The player behind a line, or their side where the event carries no owner. */
function nameOf(names: Names, owner: number | null | undefined, side: Side) {
  const seat = owner == null ? undefined : names.seats[owner];
  return {
    name: seat ? playerName(seat, seat.id === names.viewer) : side.toUpperCase(),
    tone: toneOf(names, side, seat),
    you: !!seat && seat.id === names.viewer,
  };
}

function UnitGlyph({ kind, tone }: { kind: string; tone: Tone }) {
  return <Icon path={cardIcon(kind)} className={`match-result-unit match-result-tone-${tone}`} />;
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

type Who = ReturnType<typeof nameOf>;

/** One side of a kill: the unit's glyph, its player and the unit; the
 *  victim's glyph drawn broken (dashed and dimmed). */
function Actor({
  who,
  unit,
  cost,
  destroyed = false,
}: {
  who: Who;
  unit: Unit;
  cost?: number;
  destroyed?: boolean;
}) {
  return (
    <div className="match-result-actor" data-destroyed={destroyed || undefined}>
      <UnitGlyph kind={unit.kind} tone={who.tone} />
      <span className={`match-result-event-side match-result-tone-${who.tone}`}>
        <b>{who.name}</b>
        <UnitLabel kind={unit.kind} id={unit.id} cost={cost} />
      </span>
    </div>
  );
}

/** The units one player bought or sent home together, on one slim line:
 *  the player, then each unit's glyph (and its name when it is alone). */
function Purchases({ who, units }: { who: Who; units: Unit[] }) {
  return (
    <div className={`match-result-actor match-result-tone-${who.tone}`}>
      <b className="match-result-who">{who.name}</b>
      <span className="match-result-group">
        {units.map((unit) => (
          <Icon key={unit.id} path={cardIcon(unit.kind)} className="match-result-unit-small" />
        ))}
      </span>
      {units.length === 1 && (
        <span className="match-result-group-name">
          <UnitLabel kind={units[0].kind} id={units[0].id} />
        </span>
      )}
    </div>
  );
}

function LineRow({ line, names }: { line: Line; names: Names }) {
  const { rules } = useSessionCatalog();
  if (line.kind === "kill") {
    const { event } = line;
    const killer = event.killer;
    const victim = nameOf(names, event.owner, event.side);
    const you = victim.you || (!!killer && nameOf(names, killer.owner, killer.side).you);
    return (
      <li className="match-result-event" data-event="kill" data-you={you || undefined}>
        <time>{clock(line.tick, rules.tick_hz)}</time>
        {killer ? (
          <Actor
            who={nameOf(names, killer.owner, killer.side)}
            unit={{ kind: killer.unitKind, id: killer.unitId }}
            cost={killer.cost}
          />
        ) : (
          <div className="match-result-actor">
            <span className="match-result-unknown">?</span>
            <span className="match-result-event-side">UNKNOWN SOURCE</span>
          </div>
        )}
        <span className="match-result-verb">
          <Icon path={hudIcon("rejected")} className="match-result-action" />
          KILLED
        </span>
        <Actor
          who={victim}
          unit={{ kind: event.unitKind, id: event.unitId }}
          cost={event.victimCost}
          destroyed
        />
      </li>
    );
  }
  const spawn = line.kind === "spawn";
  const who = nameOf(names, line.owner, line.side);
  const one = line.units.length === 1;
  return (
    <li className="match-result-event" data-event={line.kind} data-you={who.you || undefined}>
      <time>{clock(line.tick, rules.tick_hz)}</time>
      <Purchases who={who} units={line.units} />
      <Icon
        path={stateIcon(spawn ? "deploy" : "withdrawing")}
        className="match-result-action"
        title={spawn ? "spawned" : "refunded"}
      />
      <span className="match-result-event-copy">
        <span>{`${spawn ? "SPAWNED" : "REFUNDED"}${one ? "" : ` · ${line.units.length} UNITS`}`}</span>
        <small>{`${Math.round(line.amount)} CREDITS`}</small>
      </span>
    </li>
  );
}

/** What a scoreboard column counts. */
const COLUMNS = [
  { icon: stateIcon("deploy"), label: "UNITS", title: "Units spawned" },
  { icon: stateIcon("stock"), label: "SPENT", title: "Credits spent" },
  { icon: hudIcon("rejected"), label: "KILLS", title: "Units killed" },
  { icon: stateIcon("withdrawing"), label: "REFUNDED", title: "Credits refunded" },
];

/** One side's players, each with their own totals; the viewer's row lit. */
function Scoreboard({ side, report, names }: { side: SideName; report: Report; names: Names }) {
  const players = names.seats.flatMap((seat, index) =>
    seat.side === side ? [{ seat, index }] : [],
  );
  const label = side.charAt(0).toUpperCase() + side.slice(1);
  return (
    <section
      role="group"
      aria-label={label}
      className={`match-result-side match-result-tone-${toneOf(names, side)}`}
    >
      <table>
        <thead>
          <tr>
            <th scope="col" className="match-result-side-name">
              {side.toUpperCase()}
            </th>
            {COLUMNS.map((column) => (
              <th scope="col" key={column.label} title={column.title}>
                <Icon path={column.icon} />
                {column.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {players.map(({ seat, index }) => {
            const totals = report.totals[index] ?? { spawned: 0, spent: 0, killed: 0, refunded: 0 };
            const you = seat.id === names.viewer;
            const name = playerName(seat, you);
            return (
              <tr key={seat.id} aria-label={name} data-you={you || undefined}>
                <th scope="row" className={`match-result-tone-${toneOf(names, side, seat)}`}>
                  {name}
                </th>
                <td>{totals.spawned}</td>
                <td className="match-result-credits">{totals.spent}</td>
                <td>{totals.killed}</td>
                <td className="match-result-credits">{Math.round(totals.refunded)}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </section>
  );
}

/** The report over a finished battle. `seats` are the battle's players in
 *  scenario order (the report's totals are indexed by it); `viewer` is the
 *  human the client plays as, null when it watches AI play. */
export function MatchResultOverlay({
  match,
  seats,
  viewer,
}: {
  match: SkirmishView;
  seats: readonly Seat[];
  viewer: string | null;
}) {
  const { rules } = useSessionCatalog();
  const report = match.report;
  if (match.phase !== "finished" || !report) return null;
  const viewerSide = seats.find((seat) => seat.id === viewer)?.side ?? null;
  const names = { seats, viewer, viewerSide };
  const sides = [...new Set(seats.map((seat) => seat.side))];
  const result = match.result ?? "draw";
  return (
    <div className="match-result-scrim" role="dialog" aria-label="Match result">
      <section className={`match-result-modal match-result-${result}`}>
        <header className="match-result-heading">
          <h1>{outcome(result, viewerSide)}</h1>
          <span className="match-result-score" aria-label="Final score">
            <small>SCORE</small>
            <b className="match-result-score-blue">{Math.floor(match.scores[0])}</b>
            <span>—</span>
            <b className="match-result-score-red">{Math.floor(match.scores[1])}</b>
          </span>
        </header>
        <div className="match-result-sides">
          {sides.map((side) => (
            <Scoreboard key={side} side={side} report={report} names={names} />
          ))}
        </div>
        <ol className="match-result-timeline" aria-label="Unit action timeline">
          {timeline(report.events, rules.tick_hz).map((line, index) => (
            <LineRow line={line} names={names} key={`${line.tick}-${index}`} />
          ))}
        </ol>
      </section>
    </div>
  );
}
