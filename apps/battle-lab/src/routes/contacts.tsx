import { useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { CaptionList, useCaptions } from "@web/battle/present/captions";
import { SoundControls } from "../SoundControls";
import sensorsMap from "@fixtures/sensors-lab.json";
import { contactLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario, type LabEvent } from "../scenarios";
import { useFeed } from "../feed";
import { villageCamera } from "../villageCamera";
import { TickStatus } from "../TickStatus";

// Blue watches the ridge. Red's rifle squad hides behind it and fires every
// three seconds; red's tank drives out into view and back behind the hill.
// Nobody opens fire for real: the squad's shots are the lab's firing events.
const FIRING: LabEvent[] = Array.from({ length: 40 }, (_, k) => ({
  tick: 60 + k * 90,
  fire: { unit: 3 },
}));
const SCENARIO = labScenario(
  sensorsMap,
  [
    { side: "blue", kind: "recon", position: [560, 400], engagement: "return_fire_only" },
    { side: "blue", kind: "rifle", position: [560, 560], engagement: "return_fire_only" },
    {
      side: "red",
      kind: "tank",
      position: [840, 470],
      yaw: Math.PI,
      engagement: "return_fire_only",
    },
    { side: "red", kind: "rifle", position: [860, 500], engagement: "return_fire_only" },
  ],
  FIRING,
  [
    {
      tick: 30,
      side: "red",
      order: { kind: "move", units: [2], gesture: 1, goal: [820, 330], route: "shortest" },
    },
    {
      tick: 30,
      side: "red",
      queued: true,
      order: { kind: "move", units: [2], gesture: 1, goal: [820, 470], route: "shortest" },
    },
    {
      tick: 30,
      side: "red",
      queued: true,
      order: { kind: "move", units: [2], gesture: 1, goal: [900, 560], route: "shortest" },
    },
  ],
);
const SEED = 6;

const CONTACTS_CAMERA: Camera3DParams = {
  target: [720, 420, 0],
  distance: 520,
  pitch: 0.95,
  yaw: -1.57,
  ...villageCamera.lens,
};

export default function Contacts() {
  const cues = useCaptions();
  const onDecoded = cues.note;
  const session = useBattleSession({
    map: sensorsMap,
    scenario: SCENARIO,
    seed: SEED,
    onDecoded,
    sound: true,
  });
  const { world, meshes, sim, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    return contactLayer(observation, surfaceZ);
  }, [world, observation, surfaceZ]);
  const overlayFeed = useFeed(overlay);

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    ...session.probes,
    transcript: () => cues.transcript.current,
  };

  if (!meshes) return null;
  const contacts = observation?.contacts ?? [];
  return (
    <>
      <LabViewport
        fixture="contacts"
        world={worldFeed}
        structures={session.structures}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={CONTACTS_CAMERA}
        onReady={session.onReady}
        onFrame={(_, camera) => session.hear(camera)}
        diagnostics={diagnostics}
      />
      <aside className="hud-panel lab-panel" data-testid="contacts-panel">
        <strong>Contacts and sound</strong>
        <TickStatus tick={observation?.tick} status={sim.status.status} />
        <SoundControls />
        <div className="lab-hint">Approximate contacts: an area, never a unit or exact spot</div>
        <div className="lab-legend">
          <span className="lab-swatch lab-swatch-firing" /> firing somewhere in the area{" "}
          <span className="lab-swatch lab-swatch-last-seen" /> last seen somewhere in the ringed
          area
        </div>
        <ul className="lab-log" data-testid="contact-list">
          {contacts.length === 0 && <li>None</li>}
          {contacts.map((c) => (
            <li key={c.id}>
              {c.source === "firing" ? "Firing" : "Last seen"} area {c.id} · {c.radius.toFixed(0)} m
              radius · fades in {Math.ceil((c.expiresTick - (observation?.tick ?? 0)) / 30)} s
            </li>
          ))}
        </ul>
        <div className="lab-hint">Heard</div>
        <CaptionList captions={cues} className="lab-log" />
      </aside>
    </>
  );
}
