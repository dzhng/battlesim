import { useMemo, useRef } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { Captions, SoundSwitch, useSoundCues } from "@web/battle/present/captions";
import sensorsMap from "@fixtures/sensors-lab.json";
import { evidenceLayer, knownStructures } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { labScenario, type LabEvent } from "../scenarios";

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
    { side: "red", kind: "tank", position: [840, 470], engagement: "return_fire_only" },
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
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

export default function Contacts() {
  // Sounds pan by where the camera looks now.
  const yaw = useRef(0);
  const cues = useSoundCues(() => yaw.current);
  const onDecoded = cues.note;
  const session = useBattleSession({ map: sensorsMap, scenario: SCENARIO, seed: SEED, onDecoded });
  const { world, meshes, sim, surfaceZ } = session;
  const { observation } = sim;

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    return evidenceLayer(observation, surfaceZ);
  }, [world, observation, surfaceZ]);
  // The obstacles, ruins and wrecks blue has learned: world structures.
  const structures = useMemo(
    () => (observation ? knownStructures(observation) : undefined),
    [observation],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    ...session.probes,
    transcript: () => cues.transcript.current,
    scheduledSounds: () => cues.scheduled(),
  };

  if (!meshes) return null;
  const contacts = observation?.contacts ?? [];
  return (
    <>
      <LabViewport
        fixture="contacts"
        world={meshes}
        structures={structures}
        overlay={overlay}
        fog={observation?.fog ?? null}
        instances={[]}
        frameInstances={session.frameInstances}
        initialCamera={CONTACTS_CAMERA}
        onReady={session.onReady}
        onFrame={(_, camera) => (yaw.current = camera.yaw)}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="contacts-panel">
        <strong>Contacts and sound</strong>
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status.status}
        </div>
        <SoundSwitch cues={cues} />
        <div className="lab-hint">Approximate contacts: an area, never a unit or exact spot</div>
        <div className="lab-legend">
          <span className="lab-swatch lab-swatch-firing" /> firing somewhere in the area{" "}
          <span className="lab-swatch lab-swatch-last-seen" /> last seen somewhere in the ring
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
        <Captions cues={cues} />
      </aside>
    </>
  );
}
