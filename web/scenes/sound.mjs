// The battle's sound, heard only through measurements. The lab
// renders a scripted firefight offline through the real audio graph; this
// scene checks it is not silent, never clips, has every bus audible, and
// that each loud visual event (a gun launch, a blast) has its sound's onset
// at the same moment; and that one shot heard from the farthest zoom keeps
// at least the fixture's distance floor of its close level, low-passed and
// with a reverb tail. It writes the WAV and a spectrogram for review, and
// the main-thread cost of sound at 100 a side.
import { writeFile } from "node:fs/promises";
import { PNG } from "pngjs";
import { fileURLToPath } from "node:url";

const db = (v) => 20 * Math.log10(Math.max(v, 1e-9));

/** Interleaved 16-bit stereo WAV (the lab's encoding) → left, right, mono. */
function decodeWav(buf) {
  const n = (buf.length - 44) / 4;
  const l = new Float32Array(n);
  const r = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    l[i] = buf.readInt16LE(44 + i * 4) / 32767;
    r[i] = buf.readInt16LE(46 + i * 4) / 32767;
  }
  const mono = l.map((v, i) => (v + r[i]) / 2);
  return { l, r, mono };
}

function levels({ l, r }) {
  let peak = 0;
  let sum = 0;
  for (let i = 0; i < l.length; i++) {
    peak = Math.max(peak, Math.abs(l[i]), Math.abs(r[i]));
    sum += l[i] * l[i] + r[i] * r[i];
  }
  return { peakDb: db(peak), rmsDb: db(Math.sqrt(sum / (2 * l.length))), peak };
}

/** Energy in 5 ms hops, and each hop's rise over the 40 ms before it (dB). */
function onsetCurve(mono, sampleRate) {
  const hop = Math.round(sampleRate * 0.005);
  const energy = [];
  for (let i = 0; i + hop <= mono.length; i += hop) {
    let e = 0;
    for (let k = i; k < i + hop; k++) e += mono[k] * mono[k];
    energy.push(e / hop + 1e-12);
  }
  const rise = energy.map((e, i) => {
    const before = energy.slice(Math.max(0, i - 8), i);
    const mean = before.length ? before.reduce((a, b) => a + b, 0) / before.length : e;
    return 10 * Math.log10(e / mean);
  });
  return { hop: hop / sampleRate, rise };
}

/** Magnitude spectrogram (1024-point FFT, hop 512, 0–12 kHz, −100 to −15 dBFS), as a PNG. */
function spectrogram(mono, sampleRate) {
  const N = 1024;
  const hop = 512;
  const bins = Math.floor((12000 / sampleRate) * N);
  const frames = Math.floor((mono.length - N) / hop);
  const height = 256;
  const png = new PNG({ width: frames, height });
  const re = new Float64Array(N);
  const im = new Float64Array(N);
  const win = Float64Array.from(
    { length: N },
    (_, i) => 0.5 - 0.5 * Math.cos((2 * Math.PI * i) / N),
  );
  for (let f = 0; f < frames; f++) {
    for (let i = 0; i < N; i++) {
      re[i] = mono[f * hop + i] * win[i];
      im[i] = 0;
    }
    fft(re, im);
    for (let y = 0; y < height; y++) {
      const b = Math.min(bins - 1, Math.floor(((height - 1 - y) / height) * bins));
      const mag = (Math.hypot(re[b], im[b]) * 4) / N; // full scale = 1
      const v = Math.max(0, Math.min(1, (db(mag) + 100) / 85));
      const o = (y * frames + f) * 4;
      png.data[o] = Math.round(255 * Math.min(1, v * 1.6));
      png.data[o + 1] = Math.round(255 * Math.max(0, v * 1.6 - 0.6));
      png.data[o + 2] = Math.round(255 * Math.max(0, 0.5 - Math.abs(v - 0.3)) * 1.5);
      png.data[o + 3] = 255;
    }
  }
  return PNG.sync.write(png);
}

/** A one-shot render's loudness and colour: RMS over its sounding part
 *  (from onset to 1 s after), the share of spectral energy above 4 kHz,
 *  the spectral centroid, and the share of energy after 0.5 s past onset
 *  (its tail). */
function bands(mono, sampleRate) {
  let onset = mono.findIndex((v) => Math.abs(v) > 1e-4);
  if (onset < 0) onset = 0;
  const len = Math.round(sampleRate * 1.0);
  const x = mono.subarray(onset, Math.min(mono.length, onset + len));
  let sum = 0;
  for (const v of x) sum += v * v;
  let tailSum = 0;
  for (let i = Math.round(0.5 * sampleRate); i < mono.length - onset; i++)
    tailSum += mono[onset + i] ** 2;
  let all = 0;
  for (let i = onset; i < mono.length; i++) all += mono[i] ** 2;
  const N = 2048;
  let hi = 0;
  let total = 0;
  let moment = 0;
  const re = new Float64Array(N);
  const im = new Float64Array(N);
  for (let f = 0; f + N <= x.length; f += N / 2) {
    for (let i = 0; i < N; i++) {
      re[i] = x[f + i] * (0.5 - 0.5 * Math.cos((2 * Math.PI * i) / N));
      im[i] = 0;
    }
    fft(re, im);
    for (let b = 1; b < N / 2; b++) {
      const e = re[b] * re[b] + im[b] * im[b];
      const hz = (b * sampleRate) / N;
      total += e;
      moment += e * hz;
      if (hz > 4000) hi += e;
    }
  }
  let peak = 0;
  for (const v of x) peak = Math.max(peak, Math.abs(v));
  return {
    peak,
    rms: Math.sqrt(sum / x.length),
    highShare: hi / total,
    centroidHz: moment / total,
    tail: tailSum / all,
  };
}

/** In-place radix-2 FFT. */
function fft(re, im) {
  const n = re.length;
  for (let i = 1, j = 0; i < n; i++) {
    let bit = n >> 1;
    for (; j & bit; bit >>= 1) j ^= bit;
    j ^= bit;
    if (i < j) {
      [re[i], re[j]] = [re[j], re[i]];
      [im[i], im[j]] = [im[j], im[i]];
    }
  }
  for (let len = 2; len <= n; len <<= 1) {
    const a = (-2 * Math.PI) / len;
    for (let i = 0; i < n; i += len)
      for (let k = 0; k < len / 2; k++) {
        const c = Math.cos(a * k);
        const s = Math.sin(a * k);
        const xr = re[i + k + len / 2] * c - im[i + k + len / 2] * s;
        const xi = re[i + k + len / 2] * s + im[i + k + len / 2] * c;
        re[i + k + len / 2] = re[i + k] - xr;
        im[i + k + len / 2] = im[i + k] - xi;
        re[i + k] += xr;
        im[i + k] += xi;
      }
  }
}

export async function run(ctx) {
  const page = await ctx.newPage();
  await page.goto(ctx.url);
  await page.waitForFunction(() => window.__sound?.ready, undefined, { timeout: 30000 });

  const render = async (solo) => {
    const r = await page.evaluate((s) => window.__sound.render(s), solo);
    return { ...r, wav: Buffer.from(r.wav, "base64") };
  };
  const mix = await render();
  await writeFile(ctx.evidencePath("firefight.wav"), mix.wav);
  const heard = decodeWav(mix.wav);
  const all = levels(heard);
  ctx.check(
    "the firefight is heard, and never clips",
    all.rmsDb > -40 && all.peak < 0.999,
    `rms ${all.rmsDb.toFixed(1)} dBFS, peak ${all.peakDb.toFixed(2)} dBFS`,
  );

  const buses = {};
  for (const bus of ["units", "effects", "ambience"]) {
    const r = await render(bus);
    buses[bus] = levels(decodeWav(r.wav));
    if (bus === "effects") buses.effectsWav = decodeWav(r.wav);
  }
  ctx.check(
    "every bus is audible on its own",
    ["units", "effects", "ambience"].every((b) => buses[b].rmsDb > -60),
    ["units", "effects", "ambience"]
      .map((b) => `${b} rms ${buses[b].rmsDb.toFixed(1)} peak ${buses[b].peakDb.toFixed(1)}`)
      .join("; "),
  );
  ctx.check(
    "gunfire and blasts stand over the countryside bed",
    buses.effects.peakDb > buses.ambience.peakDb + 6,
    `effects peak ${buses.effects.peakDb.toFixed(1)}, ambience ${buses.ambience.peakDb.toFixed(1)} dBFS`,
  );

  // Onsets: each loud visual event has its sound's sharpest rise within 25 ms.
  const events = await page.evaluate(() => window.__sound.events());
  const { hop, rise } = onsetCurve(buses.effectsWav.mono, mix.sampleRate);
  const aligned = events.map((e) => {
    let best = -Infinity;
    let at = null;
    for (
      let i = Math.max(0, Math.floor((e.t - 0.06) / hop));
      i <= (e.t + 0.06) / hop && i < rise.length;
      i++
    )
      if (rise[i] > best) {
        best = rise[i];
        at = i * hop;
      }
    return {
      ...e,
      onset: at,
      offsetMs: at === null ? null : Math.round((at - e.t) * 1000),
      riseDb: best,
    };
  });
  ctx.check(
    "every loud visual event's sound starts with it",
    aligned.length >= 5 &&
      aligned.every((a) => a.offsetMs !== null && Math.abs(a.offsetMs) <= 25 && a.riseDb > 6),
    aligned
      .map((a) => `${a.what}@${a.t.toFixed(2)}s ${a.offsetMs}ms +${a.riseDb.toFixed(0)}dB`)
      .join("; "),
  );

  await writeFile(
    ctx.evidencePath("firefight-spectrogram.png"),
    spectrogram(heard.mono, mix.sampleRate),
  );
  // Distance: one shot heard up close and from the farthest zoom.
  const probe = await page.evaluate(() => window.__sound.distance());
  const near = bands(Float32Array.from(probe.near), probe.sampleRate);
  const far = bands(Float32Array.from(probe.far), probe.sampleRate);
  const loud = far.rms / near.rms;
  ctx.check(
    "a shot at the farthest zoom is still heard at the floor at least",
    loud >= probe.floor,
    `${probe.farDistanceM.toFixed(0)} m off: rms ${db(far.rms).toFixed(1)} vs ${db(near.rms).toFixed(1)} dBFS up close = ${(loud * 100).toFixed(0)} % (floor ${probe.floor * 100} %); peak ${((far.peak / near.peak) * 100).toFixed(0)} %`,
  );
  ctx.check(
    "and it sounds far: its high end is low-passed away",
    far.highShare < near.highShare / 4 && far.centroidHz < near.centroidHz / 2,
    `energy above 4 kHz ${(far.highShare * 100).toFixed(1)} % vs ${(near.highShare * 100).toFixed(1)} % up close; centroid ${far.centroidHz.toFixed(0)} vs ${near.centroidHz.toFixed(0)} Hz`,
  );
  ctx.check(
    "with a tail: the far shot rings on after the close one has died",
    far.tail > near.tail * 2,
    `energy after 0.5 s ${(far.tail * 100).toFixed(1)} % vs ${(near.tail * 100).toFixed(1)} % up close`,
  );
  const cost = await page.evaluate(() => window.__sound.cost());
  await ctx.writeEvidence("sound-report.json", {
    mix: all,
    buses: Object.fromEntries(["units", "effects", "ambience"].map((b) => [b, buses[b]])),
    onsets: aligned,
    render: {
      scheduleMs: mix.scheduleMs,
      renderMs: mix.renderMs,
      started: mix.started,
      dropped: mix.dropped,
    },
    battleScale: cost,
    distance: { farDistanceM: probe.farDistanceM, floor: probe.floor, near, far },
  });
  ctx.check(
    "sound at 100 a side costs the main thread little",
    cost.p95 < 2,
    `per frame p50 ${cost.p50.toFixed(3)} ms, p95 ${cost.p95.toFixed(3)} ms; offline render of 8 s ${mix.renderMs.toFixed(0)} ms`,
  );
  await auditionTour(ctx, page);
}

/** Exercise the production auditioner with native offline contexts: fresh
 *  one-shot graphs must not change reference loudness according to duration. */
async function auditionTour(ctx, page) {
  const modules = [
    "../../apps/sound-workbench/src/audition.ts",
    "../../packages/battle-audio/src/shippedSounds.ts",
    "../../packages/battle-audio/src/loudness.ts",
    "../../packages/battle-audio/src/synth.ts",
  ].map((path) => `/@fs${fileURLToPath(new URL(path, import.meta.url))}`);
  const report = await page.evaluate(async (urls) => {
    const [{ SoundAuditioner }, { gameSounds }, { maxMomentaryLoudness }, { SOUNDS, synthesize }] =
      await Promise.all(urls.map((url) => import(/* @vite-ignore */ url)));
    const Original = window.AudioContext;
    let context;
    let frames;
    // Only the device edge changes. Source decoding, normalization, audition
    // gain, safety ceiling and channel mapping all remain production code.
    window.AudioContext = class extends OfflineAudioContext {
      constructor() {
        super(2, frames, 48000);
        context = this;
      }
      async resume() {}
      async close() {}
    };
    const sources = [
      ...Object.entries(gameSounds.clips).map(([id, clip]) => ({
        id,
        kind: "clip",
        duration: clip.frames / clip.sample_rate,
      })),
      ...Object.keys(SOUNDS).map((id) => ({
        id,
        kind: "sound",
        duration: synthesize(id, 48000).channels[0].length / 48000,
      })),
    ];
    const clips = [];
    try {
      for (const { id, kind, duration } of sources) {
        frames = Math.ceil((duration + 0.2) * 48000);
        const audition = new SoundAuditioner();
        try {
          await audition.play(gameSounds, kind, id);
          const rendered = await context.startRendering();
          const channels = [rendered.getChannelData(0), rendered.getChannelData(1)];
          let peak = 0;
          for (const channel of channels)
            for (const sample of channel) peak = Math.max(peak, Math.abs(sample));
          clips.push({ id, kind, loudness: maxMomentaryLoudness(channels, 48000), peak });
        } finally {
          audition.stop();
        }
      }
      // Relative matching survives a legitimate change to the audition master gain.
      const levels = clips.map((clip) => clip.loudness).sort((a, b) => a - b);
      return { reference: levels[Math.floor(levels.length / 2)], clips };
    } finally {
      window.AudioContext = Original;
    }
  }, modules);
  await ctx.writeEvidence("audition-levels.json", report);
  const mismatches = report.clips.filter(
    (clip) =>
      !Number.isFinite(clip.loudness) ||
      Math.abs(clip.loudness - report.reference) > 0.75 ||
      clip.peak >= 0.999,
  );
  ctx.check(
    "every stored clip and original synthesis auditions at the shared reference without clipping",
    mismatches.length === 0,
    JSON.stringify({ checked: report.clips.length, reference: report.reference, mismatches }),
  );
}
