// A scripted firefight for the sound lab: eight seconds of publications,
// drawn motion and hearing cues, as a battle would feed them, rendered
// offline through the battle's real audio graph (`OfflineAudioContext`).
// The same publications give the visual events (launches, as the muzzle
// flashes read them, and blasts) the audio's onsets are measured against.
//
// Blue (the listener's side) holds a hedge south of a field: a rifle squad
// fires across it and walks, a tank idles, drives and traverses, and fires
// a guided missile late on. Red's identified tank fires AP, HE and HMG
// bursts from the far side; a round glances off it. A wreck burns. An
// unseen red truck and unseen riflemen are heard only as cues.
import game from "@fixtures/game.json";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { vec3 } from "math";
import type { AudioPresentation, Bus } from "@packages/battle-audio/src/audioPresentation";
import { cameraListener } from "@packages/battle-audio/src/battleAudio";
import {
  SoundFrame,
  type Listener,
  type SoundCue,
  type SoundMotion,
} from "@packages/battle-audio/src/soundFrame";
import { WebAudioSink } from "@packages/battle-audio/src/webAudioSink";
import type {
  EffectBlast,
  EffectPublication,
  EffectSegment,
  EffectShooter,
} from "@packages/battle-renderer/src/effects/effectFrame";
import { LaunchTracker } from "@packages/battle-renderer/src/effects/launches";
import { sideKey } from "@packages/battle-renderer/src/sideKey";
import { mountMuzzles } from "@packages/scene-assets/src/mountMuzzle";
import { gameEffects } from "./effectFeed";
import { gameAudio } from "./soundFeed";
import { gameSounds } from "@packages/battle-audio/src/shippedSounds";
import { battleSounds, type SoundCatalog } from "@packages/battle-audio/src/catalog";

export const FIREFIGHT_S = 8;
const HZ = game.tick_hz;
const DT = 1 / HZ;
/** Frames a second the offline render schedules at, like a display. */
const FPS = 60;

export const LISTENER: Listener = { position: [0, -40, 25], forward: [0, 1, 0] };

// Heard as blue: each unit's key is its `sideKey`, blue's ids even.
const BLUE_SQUAD = { key: sideKey(1, "blue", "blue"), members: [20, 21, 22, 23, 24, 25] };
const BLUE_TANK = { key: sideKey(2, "blue", "blue"), at: [-35, -10, 0] as number[] };
const RED_TANK = { key: sideKey(1, "red", "blue"), at: [150, 90, 0] as number[] };
const WRECK = {
  key: "heavy_wreck:40,30",
  kind: "wreck",
  center: [40, 30, 0],
  yaw: 0.4,
  half: [3.5, 1.8, 1.2],
};

/** The loud events' times, seconds: launches and blasts. */
const LOUD_S = [1.5, 3, 3.4, 4.5, 6, 7.2];

/** Seconds → tick. */
const tickAt = (s: number) => Math.round(s * HZ);

/** Whether a gun firing bursts from `starts` (ticks) fires a round at `tick`:
 *  its weapon's burst of rounds, its shot interval apart. */
function inBurst(kind: "rifle" | "hmg", starts: (start: number) => boolean, tick: number) {
  const { shot_interval_s, burst } = game.weapons[kind].magazine;
  const step = tickAt(shot_interval_s);
  for (let round = 0; round < burst.rounds; round++) if (starts(tick - round * step)) return true;
  return false;
}

/** Rifleman `k` starts a burst every 1.2 s, staggered, holding fire around
 *  the loud events so each is heard (and measured) alone. */
const rifleStarts = (k: number) => (tick: number) =>
  tick > 0 &&
  (tick + k * 5) % tickAt(1.2) === 0 &&
  tick * DT <= 7 &&
  !LOUD_S.some((t) => t - 0.4 < tick * DT && tick * DT < t + 0.15);
/** Red's roof HMG: two bursts. */
const hmgStarts = (tick: number) => tick === tickAt(4.9) || tick === tickAt(5.4);

interface Script {
  pubs: EffectPublication[];
  audible: SoundCue[][];
}

/** Every tick's publication and cues; the tanks fire from `units`' tank's
 *  muzzles, the cannon's and the roof HMG's. */
export function firefightScript(units: UnitCatalog): Script {
  const [CANNON, HMG] = mountMuzzles(units.type("tank").mounts);
  const ticks = tickAt(FIREFIGHT_S);
  const pubs: EffectPublication[] = [];
  const audible: SoundCue[][] = [];
  const rifleShots = [0, 0];
  let tankShots = [0, 0];
  let blueTankShots = [0];
  const rng = (i: number) => (((Math.sin(i * 12.9898) * 43758.5453) % 1) + 1) % 1;
  let missile: number[] | null = null;
  for (let tick = 1; tick <= ticks; tick++) {
    const s = tick * DT;
    const segments: EffectSegment[] = [];
    const blasts: EffectBlast[] = [];
    const cues: SoundCue[] = [];
    // The squad: each rifleman fires bursts across the field.
    const squadAt = soldierPositions(s);
    BLUE_SQUAD.members.forEach((id, k) => {
      if (inBurst("rifle", rifleStarts(k), tick)) rifleShots[0]++;
      // A squad's round leaves its muzzle on the tick after the one that fired it.
      if (!inBurst("rifle", rifleStarts(k), tick - 1)) return;
      const from = squadAt[k];
      const to = [RED_TANK.at[0] + (rng(tick + k) - 0.5) * 20, RED_TANK.at[1] - 8, 0.2];
      segments.push(seg([from, to], "rifle", id, "ground"));
    });
    // Red's tank: AP at 1.5 s and 4.5 s at the blue tank, HE at 3 s that
    // lands short of the hedge at 3.4 s, HMG bursts from 4.9 s.
    if (tick === tickAt(1.5) || tick === tickAt(4.5)) {
      tankShots = [tankShots[0] + 1, tankShots[1]];
      segments.push(
        seg(
          [
            [RED_TANK.at[0] - 5, RED_TANK.at[1] - 3, 2],
            [BLUE_TANK.at[0] + 2, BLUE_TANK.at[1] + 1, 1.2],
          ],
          "tank_ap",
          null,
          "hull",
        ),
      );
    }
    if (tick === tickAt(3)) {
      tankShots = [tankShots[0] + 1, tankShots[1]];
    }
    if (tick === tickAt(3.4)) {
      segments.push(
        seg(
          [
            [60, 50, 12],
            [5, 12, 0],
          ],
          "tank_he",
          null,
          "ground",
        ),
      );
      blasts.push({ point: [5, 12, 0], radius: 6, kind: "tank_he" });
    }
    if (inBurst("hmg", hmgStarts, tick)) {
      tankShots = [tankShots[0], tankShots[1] + 1];
      segments.push(
        seg(
          [
            [RED_TANK.at[0] - 3, RED_TANK.at[1] - 2, 2.6],
            [(rng(tick) - 0.5) * 30, 5, 0],
          ],
          "hmg",
          null,
          "ground",
        ),
      );
    }
    // A blue round glances off red's tank at 2.2 s.
    if (tick === tickAt(2.2)) {
      const hit = [RED_TANK.at[0] - 3, RED_TANK.at[1] - 1, 1.5];
      segments.push({
        ...seg(
          [[-5, 2, 1.6], hit, [RED_TANK.at[0] + 10, RED_TANK.at[1] + 40, 8]],
          "rifle",
          22,
          "none",
        ),
        ricochets: [{ point: 1, normal: [-1, 0, 0] }],
      });
      rifleShots[0]++;
    }
    // The blue tank's guided missile: launched at 6 s, it flies for 1.2 s,
    // strikes red's tank and bursts.
    if (tick === tickAt(6)) {
      blueTankShots = [blueTankShots[0] + 1];
      missile = [BLUE_TANK.at[0] + 3, BLUE_TANK.at[1] + 2, 2.2];
    }
    if (missile) {
      const u = (s - 6) / 1.2;
      const next = [
        BLUE_TANK.at[0] + (RED_TANK.at[0] - BLUE_TANK.at[0]) * Math.min(1, u),
        BLUE_TANK.at[1] + (RED_TANK.at[1] - BLUE_TANK.at[1]) * Math.min(1, u),
        2.2 + 6 * Math.sin(Math.PI * Math.min(1, u)),
      ];
      const done = u >= 1;
      segments.push(seg([missile, next], "atgm", null, done ? "hull" : "none"));
      if (done) {
        blasts.push({ point: next, radius: 3, kind: "atgm" });
        missile = null;
      } else missile = next;
    }
    // Unseen: a red truck far to the north-east at 1 s, riflemen near to the north at 5 s.
    if (tick === tickAt(1))
      cues.push({ category: "vehicle", sector: 1, band: "far", moving: true });
    if (tick === tickAt(5)) cues.push({ category: "shot", sector: 2, band: "near", moving: false });
    const shooters: EffectShooter[] = [
      {
        key: BLUE_SQUAD.key,
        position: squadAt[0],
        half: null,
        yaw: 0,
        members: BLUE_SQUAD.members,
        mounts: [
          {
            bearing: 0.5,
            elevation: 0,
            shots: rifleShots[0],
            kind: "rifle",
            muzzle: null,
          },
        ],
      },
      {
        key: RED_TANK.key,
        position: RED_TANK.at,
        half: [3.5, 1.8, 1.2],
        yaw: Math.PI,
        members: [],
        mounts: [
          {
            bearing: Math.PI + 0.55,
            elevation: 0,
            shots: tankShots[0],
            kind: tick <= tickAt(2) || tick > tickAt(4) ? "tank_ap" : "tank_he",
            muzzle: CANNON,
          },
          {
            bearing: Math.PI + 0.6,
            elevation: 0,
            shots: tankShots[1],
            kind: "hmg",
            muzzle: HMG,
          },
        ],
      },
      {
        key: BLUE_TANK.key,
        position: blueTankAt(s),
        half: [3.5, 1.8, 1.2],
        yaw: 0,
        members: [],
        mounts: [
          {
            bearing: 0.55,
            elevation: 0.05,
            shots: blueTankShots[0],
            kind: "atgm",
            muzzle: CANNON,
          },
        ],
      },
    ];
    pubs.push({ tick, segments, blasts, shooters, smokes: [WRECK] });
    audible.push(cues);
  }
  return { pubs, audible };
}

function seg(path: number[][], kind: string, shooter: number | null, hit: string): EffectSegment {
  return { path, ricochets: [], kind, shooter, hit, normal: hit === "none" ? null : [0, 0, 1] };
}

/** The squad walks east along the hedge for the first two seconds. */
function soldierPositions(s: number): number[][] {
  const walked = Math.min(s, 2) * 1.4;
  return BLUE_SQUAD.members.map((_, k) => [-12 + k * 2.5 + walked, (k % 2) * 1.5, 1]);
}

/** The blue tank idles, drives east from 1 s to 3.5 s at 6 m/s, then stops. */
function blueTankAt(s: number): number[] {
  const moved = Math.max(0, Math.min(s, 3.5) - 1) * 6;
  return [BLUE_TANK.at[0] + moved, BLUE_TANK.at[1], 0];
}

/** What is drawn moving at presentation time `s`. */
export function firefightMotion(s: number): SoundMotion {
  const at = blueTankAt(s);
  const rolled = at[0] - BLUE_TANK.at[0];
  // The turret traverses 1 rad over 2 s from 3.8 s.
  const turret = Math.max(0, Math.min(1, (s - 3.8) / 2));
  return {
    vehicles: [
      {
        key: BLUE_TANK.key,
        vehicleClass: "tracked_heavy",
        position: at,
        travelL: rolled,
        travelR: rolled,
        turret,
      },
      {
        key: RED_TANK.key,
        vehicleClass: "tracked_heavy",
        position: RED_TANK.at,
        travelL: 0,
        travelR: 0,
        turret: 0,
      },
    ],
    soldiers: soldierPositions(s).map((position, k) => ({ id: BLUE_SQUAD.members[k], position })),
  };
}

/** The visual events a sound should line up with: each loud launch (as the
 *  muzzle flashes place it, at its tick's start) and each blast (at its
 *  tick's end), seconds. */
export function visualEvents(script: Script): { t: number; what: string }[] {
  const launches = new LaunchTracker();
  const out: { t: number; what: string }[] = [];
  for (const pub of script.pubs) {
    for (const l of launches.note(pub, false))
      if (l.kind !== "rifle" && l.kind !== "hmg")
        out.push({ t: (pub.tick - 1) * DT, what: `launch ${l.kind}` });
    for (const b of pub.blasts) out.push({ t: pub.tick * DT, what: `blast ${b.kind}` });
  }
  return out;
}

/** A `WebAudioSink` on an offline context, on a clock the renderer sets. */
class OfflineSink extends WebAudioSink {
  t = 0;
  now() {
    return this.t;
  }
}

export interface FirefightRender {
  sampleRate: number;
  /** Interleaved stereo. */
  samples: Float32Array;
  /** Wall milliseconds: scheduling every frame on the main thread, and the render. */
  scheduleMs: number;
  renderMs: number;
  started: number;
  dropped: number;
}

/** The game's sound rendered offline for `seconds` at 48 kHz stereo:
 *  the context, its sink with the bank loaded, and the frame that plays
 *  into it. `solo` keeps one bus and mutes the others. */
async function offlineSound(seconds: number, solo?: Bus, catalog: SoundCatalog = gameSounds) {
  const sampleRate = 48000;
  const ctx = new OfflineAudioContext(2, Math.round(seconds * sampleRate), sampleRate);
  const { buses } = gameAudio;
  const presentation: AudioPresentation = solo
    ? {
        ...gameAudio,
        buses: {
          master: buses.master,
          units: solo === "units" ? buses.units : 0,
          effects: solo === "effects" ? buses.effects : 0,
          ambience: solo === "ambience" ? buses.ambience : 0,
        },
      }
    : gameAudio;
  const sink = new OfflineSink(ctx, presentation, catalog);
  await sink.bank.prepare(battleSounds(catalog));
  const frame = new SoundFrame(
    {
      tickHz: HZ,
      presentation,
      catalog,
      smokeTimes: gameEffects.smoke,
      cookOff: gameEffects.cook_off,
    },
    sink,
  );
  return { sampleRate, ctx, sink, frame };
}

/** Render the firefight offline; `solo` keeps one bus and mutes the others. */
export async function renderFirefight(
  units: UnitCatalog,
  solo?: Bus,
  catalog: SoundCatalog = gameSounds,
): Promise<FirefightRender> {
  const { sampleRate, ctx, sink, frame } = await offlineSound(FIREFIGHT_S, solo, catalog);
  const script = firefightScript(units);
  const t0 = performance.now();
  let next = 0;
  for (let f = 0; f <= FIREFIGHT_S * FPS; f++) {
    const s = f / FPS;
    // Publications arrive as their ticks complete; the clock shows the
    // tick before the latest, as the interpolator does.
    while (next < script.pubs.length && script.pubs[next].tick * DT <= s + DT) {
      frame.note({ effects: script.pubs[next], audible: script.audible[next] });
      next++;
    }
    sink.t = s;
    frame.update(s, s, firefightMotion(s), LISTENER);
  }
  const scheduleMs = performance.now() - t0;
  const r0 = performance.now();
  const buffer = await ctx.startRendering();
  const renderMs = performance.now() - r0;
  const samples = new Float32Array(buffer.length * 2);
  const l = buffer.getChannelData(0);
  const r = buffer.getChannelData(1);
  for (let i = 0; i < buffer.length; i++) {
    samples[2 * i] = l[i];
    samples[2 * i + 1] = r[i];
  }
  const stats = frame.stats();
  return {
    sampleRate,
    samples,
    scheduleMs,
    renderMs,
    started: stats.started,
    dropped: stats.dropped,
  };
}

/** Main-thread cost of a frame's sound (its publications noted, and one
 *  `SoundFrame.update`) at 100 a side (a sink that
 *  builds real nodes on an offline context): 200 soldiers walking, 40
 *  vehicles driving, every rifleman firing twice a second. Milliseconds per
 *  frame, p50 and p95. */
export async function battleScaleCost(
  catalog: SoundCatalog = gameSounds,
): Promise<{ p50: number; p95: number; notes: number }> {
  const { sink, frame } = await offlineSound(4, undefined, catalog);
  const soldiers = Array.from({ length: 200 }, (_, i) => ({
    id: i + 1,
    position: [(i % 20) * 10 - 100, Math.floor(i / 20) * 12 - 60, 1],
  }));
  const vehicles = Array.from({ length: 40 }, (_, i) => ({
    key: 1000 + i,
    vehicleClass: ["tracked_heavy", "wheeled_medium_logistics", "wheeled_light"][i % 3],
    position: [i * 10 - 200, 80, 0],
    travelL: 0,
    travelR: 0,
    turret: 0,
  }));
  const times: number[] = [];
  let tick = 0;
  let notes = 0;
  for (let f = 1; f <= 3 * FPS; f++) {
    const s = f / FPS;
    const t = performance.now();
    while ((tick + 1) * DT <= s + DT) {
      tick++;
      const firing = soldiers.filter((x) => (tick + x.id) % 15 === 0);
      const pub: EffectPublication = {
        tick,
        segments: firing.map((x) =>
          seg([x.position, [x.position[0], x.position[1] + 150, 0]], "rifle", x.id, "ground"),
        ),
        blasts: [],
        smokes: [],
        shooters: soldiers.map((x) => ({
          key: sideKey(x.id, "blue", "blue"),
          position: x.position,
          half: null,
          yaw: 0,
          members: [x.id],
          mounts: [
            {
              bearing: 0,
              elevation: 0,
              shots: Math.floor((tick + x.id) / 15),
              kind: "rifle",
              muzzle: null,
            },
          ],
        })),
      };
      frame.note({ effects: pub, audible: [] });
      notes++;
    }
    for (const x of soldiers) x.position = [x.position[0] + 0.02, x.position[1], 1];
    for (const v of vehicles) {
      v.travelL += 0.1;
      v.travelR += 0.1;
      v.turret += 0.005;
    }
    sink.t = s;
    frame.update(s, s, { vehicles, soldiers }, LISTENER);
    times.push(performance.now() - t);
  }
  times.sort((a, b) => a - b);
  return {
    p50: times[Math.floor(times.length / 2)],
    p95: times[Math.floor(times.length * 0.95)],
    notes,
  };
}

/** One blue rifleman's single shot, heard offline through the real graph
 *  (effects bus alone) from `listener`: mono, 2.5 s at 48 kHz. */
async function renderShot(
  listener: Listener,
  at: number[],
  catalog: SoundCatalog,
): Promise<Float32Array> {
  const seconds = 2.5;
  const { ctx, sink, frame } = await offlineSound(seconds, "effects", catalog);
  const key = sideKey(1, "blue", "blue");
  const shooter = (shots: number): EffectShooter => ({
    key,
    position: at,
    half: null,
    yaw: 0,
    members: [20],
    mounts: [{ bearing: 0, elevation: 0, shots, kind: "rifle", muzzle: null }],
  });
  const fireTick = tickAt(0.2);
  for (let tick = 1; tick <= tickAt(seconds); tick++) {
    frame.note({
      effects: {
        tick,
        // The round flies from the tick after its counter rises.
        segments:
          tick === fireTick + 1 ? [seg([at, [at[0] + 400, at[1], 0.2]], "rifle", 20, "none")] : [],
        blasts: [],
        shooters: [shooter(tick >= fireTick ? 1 : 0)],
        smokes: [],
      },
      audible: [],
    });
    for (let f = Math.ceil((tick - 1) * DT * FPS); f < tick * DT * FPS; f++) {
      sink.t = f / FPS;
      frame.update(f / FPS, f / FPS, { vehicles: [], soldiers: [] }, listener);
    }
  }
  const buffer = await ctx.startRendering();
  const l = buffer.getChannelData(0);
  const r = buffer.getChannelData(1);
  return l.map((v, i) => (v + r[i]) / 2);
}

/** The same single shot heard up close (inside `distance.ref_m`) and from
 *  the camera at its farthest zoom looking at it, with the fixture's floor
 *  and far cutoff the sound scene measures them against. */
export async function renderDistanceProbe(catalog: SoundCatalog = gameSounds) {
  const at = [0, 0, 1];
  const cam = game.presentation.camera;
  const far = cameraListener(
    {
      target: vec3.fromValues(at[0], at[1], 0),
      distance: cam.zoom_max,
      pitch: cam.pitch_curve[cam.pitch_curve.length - 1][1],
      yaw: cam.default.yaw,
    },
    gameAudio.listener_eye_share,
  );
  const close: Listener = { position: [0, -4, 3], forward: [0, 1, 0] };
  return {
    sampleRate: 48000,
    near: await renderShot(close, at, catalog),
    far: await renderShot(far, at, catalog),
    farDistanceM: Math.hypot(far.position[0], far.position[1], far.position[2] - at[2]),
    floor: gameAudio.distance.floor,
    farHz: gameAudio.air.far_hz,
  };
}
