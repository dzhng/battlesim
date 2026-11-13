// /labs/smoke: smoke screens on the smoke test map's encounter
// (`fixtures/maps/smoke/encounters/smoke.json`), through the production
// controls and readouts. Two blue smoke tanks (one on the road, one on grass)
// and a smoke squad at the wood's edge face a red tank and AT team, who see
// the tanks across open ground. Blue's second tank shells a red jeep into a
// burning wreck, and the road tank pops smoke at four seconds. The side
// switch watches the same battle as red: its fog closes over the cloud and
// keeps the last-contact marker where the tank vanished. `?cost` plays the
// frame-cost workload instead: a hundred blue tanks, twelve of which pop
// smoke together at two seconds.
import { useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { SideName } from "@web/battle/sim/protocol";
import { SavedEncounter } from "../savedMaps";
import { BattleView } from "../BattleView";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";
import type { BattleSession } from "../useBattleSession";

const SEED = 1;

/** The stage from the south-west: blue's three carriers near, red's two far. */
const SMOKE_CAMERA: Camera3DParams = {
  target: [215, 125, 0],
  distance: 230,
  pitch: 0.9,
  yaw: -1.57,
  ...gameCamera.lens,
};

export default function Smoke() {
  const [encounter] = useState(() =>
    new URLSearchParams(window.location.search).has("cost") ? "cost" : "smoke",
  );
  const [side, setSide] = useState<SideName>("blue");
  // Sent now, so a following advance already publishes the new side.
  const observeAs = (session: BattleSession, next: SideName) => {
    session.sim.client?.observeAs(next);
    setSide(next);
  };
  return (
    <SavedEncounter fixture="smoke" encounter={encounter}>
      {(battle) => (
        <BattleView
          fixture="smoke"
          scenario={battle.scenario}
          seed={SEED}
          side={side}
          camera={SMOKE_CAMERA}
          status={(session) => (
            <>
              <strong>Smoke screen</strong>
              <span className="lab-warning" data-testid="viewing-as">
                Viewing as {side.toUpperCase()} — diagnostic side switch
              </span>
              {(["blue", "red"] as const).map((s) => (
                <button
                  key={s}
                  type="button"
                  aria-pressed={side === s}
                  onClick={() => observeAs(session, s)}
                >
                  {s === "blue" ? "Blue view" : "Red view"}
                </button>
              ))}
              <TickStatus tick={session.sim.observation?.tick} status={session.sim.status.status} />
            </>
          )}
          diagnostics={(session) => ({
            side: () => side,
            setSide: (next: SideName) => observeAs(session, next),
          })}
        />
      )}
    </SavedEncounter>
  );
}
