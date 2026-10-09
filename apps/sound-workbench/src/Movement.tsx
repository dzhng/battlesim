import { useEffect, useRef, useState } from "react";
import { resolveEffect, type SoundCatalog } from "../../../packages/battle-audio/src/catalog";
import { vehicleLoops } from "../../../packages/battle-audio/src/soundFrame";
import type { VehicleSound } from "../../../packages/battle-audio/src/audioPresentation";
import type { LiveLoop, LiveMix } from "./audition";
import type { EditorProps } from "./editorProps";

export interface MovementProps extends EditorProps {
  /** The audition sounding now, by key; empty when none. */
  playing: string;
  /** The live audition's mix, while one sounds. */
  mix: LiveMix | null;
  /** Starts a live audition of `sounds` under `key`; null if it was superseded. */
  live(key: string, sounds: readonly string[]): Promise<LiveMix | null>;
  stop(): void;
}

/** Footsteps are scheduled this far ahead, and topped up this often. */
const LOOKAHEAD_S = 0.5;
const TICK_MS = 100;

interface Drive {
  speed: number;
  traverse: boolean;
  reverse: boolean;
}

/** The loops `row` sounds when driven so, as the battle mixes them, with
 *  replaced slots resolved. */
function driveLoops(catalog: SoundCatalog, row: VehicleSound, drive: Drive): LiveLoop[] {
  const load = Math.min(1, drive.speed / row.full_speed_mps);
  return vehicleLoops(row, load, drive.traverse ? 1 : 0, drive.reverse).map((l) => ({
    key: l.slot,
    sound: resolveEffect(catalog, l.sound),
    gain: l.gain,
    rate: l.rate,
  }));
}

/** How vehicles and soldiers sound on the move: each vehicle class driven at a
 *  chosen speed, and a squad's footsteps at a chosen pace. The levels and
 *  curves are read only here; they live in `presentation.audio`. */
export function Movement({ snapshot, draft, edit, play, playing, mix, live, stop }: MovementProps) {
  const [drives, setDrives] = useState<Record<string, Drive>>(() =>
    Object.fromEntries(
      Object.entries(snapshot.vehicles).map(([cls, row]) => [
        cls,
        { speed: row.full_speed_mps / 2, traverse: false, reverse: false },
      ]),
    ),
  );
  const [soldiers, setSoldiers] = useState(1);
  const [pace, setPace] = useState(snapshot.runMps);
  const steps = useRef({ soldiers, pace });
  steps.current = { soldiers, pace };
  const f = snapshot.footsteps;
  const footstep = resolveEffect(draft, f.sound);
  // Recipes made only of footstep recordings are the footstep's candidates.
  const candidates = Object.entries(draft.sounds).filter(
    ([, sound]) =>
      sound.clips.length && sound.clips.every((c) => draft.clips[c]?.category === "footstep"),
  );

  const drive = async (cls: string) => {
    const key = `drive:${cls}`;
    if (playing === key) return stop();
    const row = snapshot.vehicles[cls];
    const sounds = [row.engine, row.running, row.turret, row.reverse]
      .filter((s): s is string => !!s)
      .map((s) => resolveEffect(draft, s));
    (await live(key, sounds))?.loops(driveLoops(draft, row, drives[cls]));
  };
  const setDrive = (cls: string, change: Partial<Drive>) => {
    const next = { ...drives[cls], ...change };
    setDrives({ ...drives, [cls]: next });
    if (playing === `drive:${cls}`) mix?.loops(driveLoops(draft, snapshot.vehicles[cls], next));
  };

  // A squad walking: each soldier steps once a stride, out of step with the rest.
  const walking = playing === "footsteps";
  useEffect(() => {
    const m = walking ? mix : null;
    if (!m) return;
    const start = performance.now() / 1000;
    const next: number[] = [];
    const tick = () => {
      const now = performance.now() / 1000 - start;
      const { soldiers, pace } = steps.current;
      const interval = f.stride_m / pace;
      while (next.length < soldiers) next.push(now + (next.length ? Math.random() * interval : 0));
      next.length = soldiers;
      for (let i = 0; i < soldiers; i++)
        for (; next[i] < now + LOOKAHEAD_S; next[i] += interval)
          m.hit(footstep, f.gain, next[i] - now);
    };
    tick();
    const timer = setInterval(tick, TICK_MS);
    return () => clearInterval(timer);
  }, [walking, mix, footstep, f.gain, f.stride_m]);
  const walk = () => (walking ? stop() : void live("footsteps", [footstep]));

  return (
    <section className="sw-detail sw-global">
      <h2>Footsteps</h2>
      <p>
        Each soldier steps once every {f.stride_m} m at level {f.gain}; the battle runs soldiers at{" "}
        {snapshot.runMps} m/s. The choice replaces the footstep for every soldier.
      </p>
      <div className="sw-assignment">
        <label>
          Footstep
          <select
            aria-label="Footstep sound"
            value={draft.effects[f.sound] ?? ""}
            onChange={(e) =>
              edit((c) => {
                if (e.target.value) c.effects[f.sound] = e.target.value;
                else delete c.effects[f.sound];
              })
            }
          >
            <option value="">Original synthesis</option>
            {candidates.map(([id, sound]) => (
              <option key={id} value={id}>
                {sound.label}
              </option>
            ))}
          </select>
        </label>
        <div className="sw-drive">
          <label className="sw-slider">
            <span>
              Soldiers · <output>{soldiers}</output>
            </span>
            <input
              type="range"
              aria-label="Soldiers"
              min={1}
              max={10}
              value={soldiers}
              onChange={(e) => setSoldiers(Number(e.target.value))}
            />
          </label>
          <label className="sw-slider">
            <span>
              Pace · <output>{pace.toFixed(1)} m/s</output>
            </span>
            <input
              type="range"
              aria-label="Pace"
              min={0.5}
              max={5}
              step={0.1}
              value={pace}
              onChange={(e) => setPace(Number(e.target.value))}
            />
          </label>
          <button onClick={walk}>{walking ? "Stop footsteps" : "Play footsteps"}</button>
          <button onClick={() => void play("sound", footstep, Math.floor(Math.random() * 1e6))}>
            Play one step
          </button>
        </div>
      </div>
      <h2>Vehicles</h2>
      <p>
        A vehicle sounds as its class: how it moves, its hull&apos;s weight and whether it hauls
        supply. Drive plays its loops mixed as the battle mixes them at that speed. Classes and
        their levels live in the game&apos;s presentation; replace a loop for every class under
        Defaults &amp; effects.
      </p>
      {Object.entries(snapshot.vehicles).map(([cls, row]) => {
        const d = drives[cls];
        const driving = playing === `drive:${cls}`;
        return (
          <div className="sw-assignment" key={cls}>
            <h3>{cls}</h3>
            <div className="sw-drive">
              <label className="sw-slider">
                <span>
                  Speed ·{" "}
                  <output>
                    {d.speed.toFixed(1)} m/s · {Math.round((100 * d.speed) / row.full_speed_mps)}%
                    load
                  </output>
                </span>
                <input
                  type="range"
                  aria-label={`${cls} speed`}
                  min={0}
                  max={row.full_speed_mps}
                  step={0.5}
                  value={d.speed}
                  onChange={(e) => setDrive(cls, { speed: Number(e.target.value) })}
                />
              </label>
              {row.turret && (
                <label className="sw-check">
                  <input
                    type="checkbox"
                    checked={d.traverse}
                    onChange={(e) => setDrive(cls, { traverse: e.target.checked })}
                  />{" "}
                  Turret traversing
                </label>
              )}
              {row.reverse && (
                <label className="sw-check">
                  <input
                    type="checkbox"
                    checked={d.reverse}
                    onChange={(e) => setDrive(cls, { reverse: e.target.checked })}
                  />{" "}
                  Reversing
                </label>
              )}
              <button aria-label={`Drive ${cls}`} onClick={() => void drive(cls)}>
                {driving ? "Stop" : "Drive"}
              </button>
            </div>
            {(
              [
                ["engine", row.engine],
                ["running gear", row.running],
                ["turret", row.turret],
                ["reverse", row.reverse],
              ] as const
            ).map(([slot, baseline]) => {
              if (!baseline) return null;
              const sound = resolveEffect(draft, baseline);
              return (
                <div className="sw-effect-row" key={slot}>
                  <span>
                    {slot} alone · {draft.sounds[sound]?.label ?? sound}
                  </span>
                  <button
                    aria-label={`Play ${cls} ${slot}`}
                    onClick={() => void play("sound", sound)}
                  >
                    Play
                  </button>
                </div>
              );
            })}
          </div>
        );
      })}
    </section>
  );
}
