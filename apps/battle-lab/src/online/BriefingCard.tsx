import type { Briefing } from "@packages/multiplayer/src/wire";
import { spoken } from "../battleLinks";
import { useOnlineMap, type OnlineMap } from "./useOnlineMap";

export function BriefingCard({
  briefing,
  busy,
  error,
  onBack,
  onDeploy,
}: {
  briefing: Briefing;
  busy: boolean;
  error: string | null;
  onBack(): void;
  onDeploy(prepared: OnlineMap): void;
}) {
  const map = useOnlineMap(briefing.setup, briefing.mapIdentity);
  const enemy: Briefing["side"] = briefing.side === "blue" ? "red" : "blue";
  const ownScore = briefing.scores[briefing.side === "blue" ? 0 : 1];
  const enemyScore = briefing.scores[enemy === "blue" ? 0 : 1];
  const number = (value: number) => value.toLocaleString("en-US");
  return (
    <div className="frontline-veil">
      <section className="frontline-briefing" aria-label="Battle briefing" aria-busy={busy}>
        <button type="button" className="frontline-back" onClick={onBack} disabled={busy}>
          Back to Multiplayer
        </button>
        <p className="lobby-label">
          Frontline ·{" "}
          {briefing.minutesIn < 1
            ? "Battle just begun"
            : `${Math.floor(briefing.minutesIn)} ${Math.floor(briefing.minutesIn) === 1 ? "minute" : "minutes"} in`}
        </p>
        <h3>Your side needs a commander</h3>
        <p className="frontline-assignment" aria-label="Assignment">
          {spoken(briefing.setup.factions[briefing.side])} · Seat {briefing.seat} ·{" "}
          <span>{number(briefing.credits)} credits</span>
        </p>
        <div className="frontline-score" aria-label="Battle score">
          <div>
            <small>Your side</small>
            <strong>{number(ownScore)}</strong>
          </div>
          <div className="frontline-progress">
            <span className="frontline-target">of {number(briefing.target)}</span>
            <div className="frontline-score-track" aria-hidden="true">
              <i style={{ width: `${Math.min(1, ownScore / briefing.target) * 50}%` }} />
              <i
                data-enemy
                style={{ width: `${Math.min(1, enemyScore / briefing.target) * 50}%` }}
              />
            </div>
          </div>
          <div data-enemy>
            <small>Enemy</small>
            <strong>{number(enemyScore)}</strong>
          </div>
        </div>
        <div className="frontline-rosters" aria-label="Commanders">
          {[briefing.side, enemy].map((side, index) => {
            const roster = briefing.roster.filter((seat) => seat.side === side);
            const ais = roster.filter((seat) => seat.callsign === null).length;
            return (
              <div key={side} data-enemy={index === 1 || undefined}>
                <h4>
                  {index === 0 ? "Your side" : "Enemy"} · {spoken(briefing.setup.factions[side])}
                </h4>
                <ul aria-label={index === 0 ? "Allied commanders" : "Enemy commanders"}>
                  {roster
                    .filter((seat) => seat.callsign !== null)
                    .map((seat, i) => (
                      <li key={i}>{seat.callsign}</li>
                    ))}
                  {ais > 0 && <li className="frontline-ai">{ais} AI</li>}
                </ul>
              </div>
            );
          })}
        </div>
        <div className="frontline-deploy" role="group" aria-label="Deployment">
          {map.status === "failed" ? (
            <p role="alert">{map.error}</p>
          ) : (
            <p role="status">
              {busy
                ? "Deploying…"
                : map.status === "ready"
                  ? "Battlefield ready"
                  : "Preparing the battlefield…"}
            </p>
          )}
          {error && <p role="alert">{error}</p>}
          <button
            type="button"
            className="menu-deploy"
            disabled={busy || map.status !== "ready"}
            onClick={() => {
              if (map.status === "ready") onDeploy(map.prepared);
            }}
          >
            Deploy
          </button>
        </div>
      </section>
    </div>
  );
}
