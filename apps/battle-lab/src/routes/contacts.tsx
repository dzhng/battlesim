import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { buildWorldMeshes } from "@packages/battle-renderer/src/worldMesh";
import { buildEvidenceOverlay } from "@packages/battle-renderer/src/evidenceOverlay";
import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import { CueAudio, describeCue, type Caption } from "@web/battle/present/audio";
import type { ObservationView } from "@web/battle/sim/observation";
import sensorsMap from "@fixtures/sensors-lab.json";
import { LabViewport } from "../LabViewport";
import { labScenario, type LabEvent } from "../scenarios";
import { sideInstances } from "../sideInstances";
import { useSimSession } from "../useSimSession";
import { useStaticWorld } from "../useStaticWorld";

// Blue watches the ridge. Red's rifle squad hides behind it and fires every
// three seconds; red's tank drives out into view and back behind the hill.
const FIRING: LabEvent[] = Array.from({ length: 40 }, (_, k) => ({
  tick: 60 + k * 90,
  fire: { unit: 3 },
}));
const SCENARIO = labScenario(
  sensorsMap,
  [
    { side: "blue", kind: "recon", position: [560, 400] },
    { side: "blue", kind: "rifle", position: [560, 560] },
    { side: "red", kind: "tank", position: [840, 470] },
    { side: "red", kind: "rifle", position: [860, 500] },
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

export const CONTACTS_CAMERA: Camera3DParams = {
  target: [720, 420, 0],
  distance: 520,
  pitch: 0.95,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 1,
  near: 1,
};

export default function Contacts() {
  const world = useStaticWorld(sensorsMap);
  const meshes = useMemo(
    () => world && buildWorldMeshes(world.exports, world.layout, "surface"),
    [world],
  );
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
  const sim = useSimSession({ scenario: SCENARIO, seed: SEED, onDecoded });
  const { observation } = sim;
  useEffect(() => () => audio.current.dispose(), []);

  const frameInstances = useCallback(
    (now: number): SceneInstance[] | null => {
      const poses = sim.interpolator.current?.sample(now);
      if (!poses || !observation) return null;
      return sideInstances("blue", poses, observation, []).instances;
    },
    [observation, sim.interpolator],
  );

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const z = (x: number, y: number) => world.view.surface_at(x, y)[0] ?? 0;
    return buildEvidenceOverlay(
      observation.contacts.map((c) => ({
        center: c.center,
        radius: c.radius,
        source: c.source,
        freshness: Math.max(
          0,
          (c.expiresTick - observation.tick) / Math.max(1, c.expiresTick - c.evidenceTick),
        ),
      })),
      observation.knownProps,
      z,
    );
  }, [world, observation]);

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    tick: () => sim.latest.current?.tick ?? 0,
    observation: () => sim.latest.current,
    transcript: () => transcript.current,
    scheduledSounds: () => audio.current.scheduled,
    pause: () => sim.client?.pause(),
    resume: () => sim.client?.resume(),
    advance: (n: number) => sim.client!.advance(n),
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
        frameInstances={frameInstances}
        initialCamera={CONTACTS_CAMERA}
        onReady={sim.onViewportReady}
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
