import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { CueAudio, describeCue, type Caption } from "@web/battle/present/audio";
import type { ObservationView } from "@web/battle/sim/observation";
import sensorsMap from "@fixtures/sensors-lab.json";
import { evidenceLayer } from "../battleOverlay";
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
  const audio = useRef(new CueAudio());
  const [captions, setCaptions] = useState<(Caption & { count?: number })[]>([]);
  const [soundOn, setSoundOn] = useState(false);
  const transcript = useRef<Caption[]>([]);

  const onDecoded = useCallback((o: ObservationView) => {
    if (!o.audible.length) return;
    const yaw = window.__lab?.camera?.().yaw ?? 0;
    const lines = o.audible.map((cue) => {
      audio.current.play(cue, yaw);
      const listener = o.own.find((u) => u.id === cue.listener);
      return {
        tick: o.tick,
        text: describeCue(cue, listener ? `${listener.kind} #${listener.id}` : "a unit"),
      };
    });
    transcript.current.push(...lines);
    // Repeats of the same sound collapse into one line with a count.
    setCaptions((current) => {
      const next = [...current];
      for (const line of lines) {
        if (next[0]?.text === line.text)
          next[0] = { ...next[0], tick: line.tick, count: (next[0].count ?? 1) + 1 };
        else next.unshift(line);
      }
      return next.slice(0, 6);
    });
  }, []);
  const session = useBattleSession({ map: sensorsMap, scenario: SCENARIO, seed: SEED, onDecoded });
  const { world, meshes, sim, surfaceZ } = session;
  const { observation } = sim;
  useEffect(() => () => audio.current.dispose(), []);

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    return evidenceLayer(observation, surfaceZ);
  }, [world, observation, surfaceZ]);

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    ...session.probes,
    transcript: () => transcript.current,
    scheduledSounds: () => audio.current.scheduled,
  };

  if (!meshes) return null;
  const contacts = observation?.contacts ?? [];
  return (
    <>
      <LabViewport
        fixture="contacts"
        world={meshes}
        overlay={overlay}
        fog={observation?.fog ?? null}
        instances={[]}
        frameInstances={session.frameInstances}
        initialCamera={CONTACTS_CAMERA}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <aside className="lab-panel" data-testid="contacts-panel">
        <strong>Contacts and sound</strong>
        <div>
          Tick {observation?.tick ?? "—"} · {sim.status.status}
        </div>
        <label>
          <input
            type="checkbox"
            checked={soundOn}
            onChange={(e) => {
              if (e.target.checked) audio.current.enable();
              setSoundOn(e.target.checked);
            }}
          />
          Play sounds (captions always shown)
        </label>
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
        <div className="lab-hint">Heard (newest first)</div>
        <ul className="lab-log" data-testid="captions">
          {captions.length === 0 && <li>Nothing heard</li>}
          {captions.map((c, k) => (
            <li key={`${c.tick}-${k}`}>
              {c.text}
              {c.count ? ` ×${c.count}` : ""}
            </li>
          ))}
        </ul>
      </aside>
    </>
  );
}
