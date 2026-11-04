import { useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { LabViewport } from "../LabViewport";
import { SavedEncounters, type SavedBattle } from "../savedMaps";
import { useBattleSession } from "../useBattleSession";
import { useFeed } from "@web/feed";
import { useLabOverlay } from "../labOverlay";
import { TickStatus } from "../TickStatus";
import { ArmyDeck } from "@web/battle/present/armyDeck";
import { ReadoutLayer } from "@web/battle/present/readouts";
import { RejectedOrder } from "@web/battle/present/rejectedOrder";
import { gameCamera } from "../gameCamera";

/** Close on the carriers and squads, so their cards read. */
const CAMERA: Camera3DParams = {
  target: [235, 255, 0],
  distance: 150,
  pitch: 0.95,
  yaw: -1.57,
  ...gameCamera.lens,
};

/** `board` scripts boarding from both directions; `controls` scripts
 *  nothing, for the player's own controls; `heli` scripts a transport
 *  helicopter collecting a squad and unloading it further on. */
const VARIANTS = {
  board: "Board and Collect are scripted from both directions.",
  controls:
    "Right-click a carrier with squads selected to board, or a squad with a carrier selected to collect it. U then left-click unloads there; U twice unloads in place.",
  heli: "A transport helicopter collects a squad and unloads it further on, each at a low hover.",
} as const;
type Variant = keyof typeof VARIANTS;
const VARIANT_NAMES = Object.keys(VARIANTS) as Variant[];

export default function Transport() {
  return (
    <SavedEncounters fixture="transport" encounters={VARIANT_NAMES}>
      {(battles) => <TransportLab battles={battles} />}
    </SavedEncounters>
  );
}

function TransportLab({ battles }: { battles: Record<Variant, SavedBattle> }) {
  const [variant] = useState<Variant>(() => {
    const requested = new URLSearchParams(window.location.search).get("variant");
    return VARIANT_NAMES.find((name) => name === requested) ?? "board";
  });
  const session = useBattleSession({ ...battles[variant], seed: 21 });
  const { meshes, sim, control } = session;
  const { observation } = sim;
  const worldFeed = useFeed(meshes);
  const { overlayFeed, onFrame } = useLabOverlay(session, CAMERA);
  const armyState = useFeed({ own: observation?.own ?? [], control });

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="transport"
        world={worldFeed}
        {...session.known}
        overlay={overlayFeed}
        fog={session.fogFeed}
        {...session.viewport}
        initialCamera={CAMERA}
        {...session.input}
        onFrame={onFrame}
        diagnostics={session.probes}
      />
      <ReadoutLayer
        own={observation?.own ?? []}
        rules={session.rules}
        selected={control.selected}
        handle={session.readouts}
      />
      <aside className="hud-panel lab-panel" data-occludes-readouts data-testid="transport-panel">
        <strong>Passenger transport</strong>
        <div className="lab-hint">{VARIANTS[variant]} Passengers leave the field while aboard.</div>
        <TickStatus tick={observation?.tick} status={sim.status.status} />
      </aside>
      <div className="hud">
        <ArmyDeck
          state={armyState}
          selected={control.selected}
          onSelect={control.setSelected}
          rules={session.rules}
          captions={null}
        />
      </div>
      <RejectedOrder acks={control.acks} {...session.refusals} />
    </>
  );
}
