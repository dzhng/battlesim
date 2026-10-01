import { useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { CaptionList, useCaptions } from "@web/battle/present/captions";
import { SoundControls } from "../SoundControls";
import { contactLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

// Blue watches the ridge. Red's rifle squad hides behind it and fires every
// three seconds; red's tank drives out into view and back behind the hill.
// Nobody opens fire for real: the squad's shots are the lab's firing events.
const SEED = 6;

const CONTACTS_CAMERA: Camera3DParams = {
  target: [720, 420, 0],
  distance: 520,
  pitch: 0.95,
  yaw: -1.57,
  ...gameCamera.lens,
};

export default function Contacts() {
  return (
    <SavedEncounter map="sensors" encounter="contacts">
      {(battle) => <ContactsLab battle={battle} />}
    </SavedEncounter>
  );
}

function ContactsLab({ battle }: { battle: SavedBattle }) {
  const cues = useCaptions();
  const onDecoded = cues.note;
  const session = useBattleSession({
    ...battle,
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

  if (!meshes) return null;
  const contacts = observation?.contacts ?? [];
  return (
    <>
      <LabViewport
        fixture="contacts"
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        massing={session.massingFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={CONTACTS_CAMERA}
        onReady={session.onReady}
        onFrame={(_, camera) => session.hear(camera)}
        diagnostics={session.probes}
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
