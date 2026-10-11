// @vitest-environment node
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { expect, test } from "vitest";
import { ObservationDecoder, type ObservationLayout } from "@web/battle/sim/observation";
import { nodeCatalogSet } from "@web/battle/catalog/node";
import { createEffectFrame, effectPublication, gameEffects } from "@apps/battle-lab/src/effectFeed";
import {
  createEffectBatch,
  EFFECT_FLOATS,
  type EffectPublication,
} from "@packages/battle-renderer/src/effects/effectFrame";
import { offeredCastLights } from "@packages/battle-renderer/src/light/castLights";
import type { DrawnRotor } from "@packages/battle-renderer/src/models/drawnModels";
import type { MountMuzzle } from "@packages/scene-assets/src/mountMuzzle";

// Writes the browser's effects for fixed causes, the vectors native's port of
// the effects (`native/godot/effects/effect_frame.gd`) is compared against
// (`effect_vectors_test.gd`): a few publications that exercise every kind of
// effect (a squad's tracers, chosen by chance, and their flashes; a tank's
// shot off its muzzle; a ricochet; impacts; a blast; a cook-off whose turret
// lands; dust behind a moving hull; smoke behind a smoking airframe; a
// burning wreck; a grenade's smoke trail; a rotor), and every instance and
// cast light the browser draws at fixed clocks after them, seeded as it seeds
// them. It also writes each fixture-run plate's effect counts, replayed from
// the fixture capture as the page replays it, into the fixture run's
// manifest beside the `effects` section of the control it was cut from.
// Rerun it when the effects or their tuning change.
const fixtures = new URL("../../native/godot/tests/fixtures/", import.meta.url);
const TICK_HZ = 30;

const round = (v: number) => Math.round(v * 1e6) / 1e6;

/** A squad, a tank, a smoking airframe and a wreck, over five ticks. Paths
 *  are float32s, as a decoded publication's are: a tracer's chance is
 *  seeded by their text. */
function publication(tick: number): EffectPublication {
  const pub = publicationOf(tick);
  return {
    ...pub,
    segments: pub.segments.map((s) => ({ ...s, path: s.path.map((p) => p.map(Math.fround)) })),
  };
}

function publicationOf(tick: number): EffectPublication {
  const fired = tick >= 2 ? 1 : 0;
  const muzzle: MountMuzzle = {
    id: "gun",
    on: null,
    pivot: [0.5, 0, 2.2],
    ring: [0, 0, 0],
    muzzle: [5.2, 0, 0],
  };
  return {
    tick,
    segments: [
      ...(tick === 3
        ? [
            ...[20, 21, 22, 23].map((shooter, i) => ({
              path: [
                [10 + i * 0.37, 20.25, 1.4],
                [60 + i, 25.5, 1.7],
              ],
              ricochets: [],
              kind: "rifle",
              shooter,
              hit: "none",
              normal: null,
            })),
            {
              path: [
                [100, 50, 2.5],
                [140, 52, 1.0],
                [150, 60, 4.0],
              ],
              ricochets: [{ point: 1, normal: [0, 0.6, 0.8] }],
              kind: "tank_ap",
              shooter: null,
              hit: "ground",
              normal: [0, 0, 1],
            },
            {
              path: [
                [30, 30, 1.2],
                [42, 31, 3.1],
                [55, 33, 0.4],
              ],
              ricochets: [],
              kind: "grenade",
              shooter: 21,
              hit: "soldier",
              normal: [0.2, -0.3, 0.93],
            },
          ]
        : []),
      ...(tick === 4
        ? [
            {
              path: [
                [60, 25.5, 1.7],
                [90, 27, 1.1],
              ],
              ricochets: [],
              kind: "rifle",
              shooter: 20,
              hit: "hull",
              normal: [-1, 0, 0],
            },
          ]
        : []),
    ],
    blasts: tick === 4 ? [{ point: [70, 80, 0.5], radius: 6, kind: "tank_he" }] : [],
    shooters: [
      {
        key: 2,
        position: [10, 20, 0],
        half: null,
        yaw: 0,
        members: [20, 21, 22, 23],
        mounts: [
          { bearing: 0.1, elevation: 0, shots: tick >= 2 ? 4 : 0, kind: "rifle", muzzle: null },
        ],
      },
      {
        key: 4,
        position: [100 + tick * 0.6, 50, 0],
        half: [3.5, 1.8, 1.2],
        yaw: 0.3,
        members: [],
        mounts: [
          { bearing: 0.35, elevation: 0.02, shots: fired, kind: "tank_ap", muzzle },
          { bearing: 0.35, elevation: 0, shots: 0, kind: "hmg", muzzle: null },
        ],
      },
      {
        key: 7,
        position: [200 + tick * 2, 120, 90],
        half: [6, 1.5, 1.8],
        airborne: true,
        smoking: true,
        yaw: 1.2,
        members: [],
        mounts: [],
      },
    ],
    smokes:
      tick >= 2
        ? [
            {
              key: "heavy_wreck:2562.5,1396.25",
              kind: "wreck",
              center: [250, 140, 0.2],
              yaw: -2.4,
              half: [3.5, 1.8, 1.2],
            },
          ]
        : [],
    cookOffs:
      tick === 2 ? [{ center: [250, 140, 0.2], height: 2.4, landing: [253, 141.5, 1.9] }] : [],
  };
}

/** Drawn muzzles: the squad's first soldier's and the tank's gun's; the rest
 *  as published. */
const muzzles = {
  muzzle(shooter: number, mount: number, soldier: number | null, at: number[]) {
    if (shooter === 2 && soldier === 20) [at[0], at[1], at[2]] = [10.3, 20.1, 1.6];
    else if (shooter === 4 && mount === 0) [at[0], at[1], at[2]] = [106.4, 51.9, 2.3];
    else return false;
    return true;
  },
};
const rotor: DrawnRotor = {
  hub: [210, 120, 92],
  axis: [0, 0.1, 0.995],
  blade: [1, 0, 0],
  radius: 7.3,
  blades: 4,
  chord: 0.53,
  turned: 0.9,
  mast: { base: [210, 120, 91.2], top: [210, 120, 92.4] },
  mastRadius: 0.12,
};

test("the browser's effects for fixed causes are written for native", () => {
  const frame = createEffectFrame(TICK_HZ);
  const batch = createEffectBatch(gameEffects.capacity);
  const clocks = [0.05, 0.09, 0.12, 0.2, 0.45, 1.3, 4.0];
  const publications = [1, 2, 3, 4, 5].map(publication);
  const frames: unknown[] = [];
  let next = 0;
  for (const clock of clocks) {
    while (next < publications.length && (publications[next].tick - 1) / TICK_HZ <= clock)
      frame.note(publications[next++]);
    frame.build(clock, batch, muzzles, { rotors: (visit) => visit(rotor) });
    frames.push({
      clock,
      noted: next,
      instances: Array.from(batch.data.subarray(0, batch.count * EFFECT_FLOATS), round),
      lights: offeredCastLights(batch.lights).map((l) => ({
        ...l,
        at: l.at.map(round),
        rgb: l.rgb.map(round),
        radius: round(l.radius),
      })),
      counts: frame.counts(clock, batch),
    });
  }
  const shapes = new Set(
    frames.flatMap((f) => {
      const data = (f as { instances: number[] }).instances;
      return Array.from(
        { length: data.length / EFFECT_FLOATS },
        (_, i) => data[i * EFFECT_FLOATS + 12],
      );
    }),
  );
  // Every shape drawn, every kind of effect running, and lights cast.
  expect([...shapes].sort()).toEqual([0, 1, 2, 3]);
  const kinds = new Set(
    frames.flatMap((f) =>
      Object.entries((f as { counts: { running: Record<string, number> } }).counts.running)
        .filter(([, n]) => n > 0)
        .map(([k]) => k),
    ),
  );
  expect([...kinds].sort()).toEqual(
    ["blast", "flame", "flash", "impact", "puff", "sparks", "tracer", "trail"].sort(),
  );
  mkdirSync(new URL("effects/", fixtures), { recursive: true });
  writeFileSync(
    new URL("effects/effect-vectors.json", fixtures),
    `${JSON.stringify({
      tickHz: TICK_HZ,
      presentation: gameEffects,
      publications,
      muzzles: [
        [2, 0, 20, [10.3, 20.1, 1.6]],
        [4, 0, null, [106.4, 51.9, 2.3]],
      ],
      rotor,
      frames,
    })}\n`,
  );
});

test("the fixture run's plates' effect counts are the page's replay of its capture", async () => {
  const { units } = await nodeCatalogSet("menu");
  const dir = new URL("run/paris-corner/", fixtures);
  const capture = JSON.parse(readFileSync(new URL("capture.json", dir), "utf8"));
  const manifestUrl = new URL("manifest.json", dir);
  const manifest = JSON.parse(readFileSync(manifestUrl, "utf8"));
  const decoder = new ObservationDecoder(JSON.parse(capture.layout) as ObservationLayout);
  const frame = createEffectFrame(capture.tickHz);
  const batch = createEffectBatch(gameEffects.capacity);
  const counts: unknown[] = [];
  let at = 0;
  for (const f of capture.frames) {
    while (at < capture.samples.length && capture.samples[at].tick <= f.tick) {
      const o = decoder.decode(
        new Float32Array(new Uint32Array(capture.samples[at++].publication).buffer),
      )!;
      frame.note(effectPublication(o, capture.side, units));
    }
    if (f.plate === null) continue;
    frame.build(f.simTime, batch);
    counts.push({ plate: f.plate, ...frame.counts(f.simTime, batch) });
  }
  expect(counts).toHaveLength(1);
  expect(manifest.effects.presentation).toEqual(JSON.parse(JSON.stringify(gameEffects)));
  writeFileSync(manifestUrl, `${JSON.stringify({ ...manifest, effectCounts: counts })}\n`);
});
