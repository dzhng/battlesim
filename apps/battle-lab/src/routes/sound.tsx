// The sound lab: a scripted eight-second firefight rendered offline through
// the battle's audio graph, to listen to here and to measure in the scene
// (levels per bus, clipping, onsets against the visual events). The scene
// drives `window.__sound`.
import { useEffect, useState } from "react";
import type { Bus } from "@packages/battle-audio/src/audioPresentation";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import {
  battleScaleCost,
  firefightScript,
  renderDistanceProbe,
  renderFirefight,
  visualEvents,
  type FirefightRender,
} from "../soundFirefight";
import { SoundControls } from "../SoundControls";

/** 16-bit PCM WAV bytes for interleaved stereo. */
export function encodeWav(samples: Float32Array, sampleRate: number): Uint8Array {
  const bytes = new Uint8Array(44 + samples.length * 2);
  const view = new DataView(bytes.buffer);
  const text = (at: number, s: string) =>
    [...s].forEach((c, i) => view.setUint8(at + i, c.charCodeAt(0)));
  text(0, "RIFF");
  view.setUint32(4, 36 + samples.length * 2, true);
  text(8, "WAVE");
  text(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 2, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * 4, true);
  view.setUint16(32, 4, true);
  view.setUint16(34, 16, true);
  text(36, "data");
  view.setUint32(40, samples.length * 2, true);
  for (let i = 0; i < samples.length; i++)
    view.setInt16(44 + i * 2, Math.round(Math.max(-1, Math.min(1, samples[i])) * 32767), true);
  return bytes;
}

function base64(bytes: Uint8Array): string {
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000)
    s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}

declare global {
  interface Window {
    __sound?: {
      ready: boolean;
      render: (
        solo?: Bus,
        catalog?: SoundCatalog,
      ) => Promise<Omit<FirefightRender, "samples"> & { wav: string }>;
      events: () => { t: number; what: string }[];
      cost: () => ReturnType<typeof battleScaleCost>;
      distance: () => Promise<
        Omit<Awaited<ReturnType<typeof renderDistanceProbe>>, "near" | "far"> & {
          near: number[];
          far: number[];
        }
      >;
    };
  }
}

export default function Sound() {
  const [url, setUrl] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    window.__sound = {
      ready: true,
      async render(solo, catalog) {
        const r = await renderFirefight(solo, catalog);
        const { samples, ...rest } = r;
        return { ...rest, wav: base64(encodeWav(samples, r.sampleRate)) };
      },
      events: () => visualEvents(firefightScript()),
      cost: battleScaleCost,
      async distance() {
        const r = await renderDistanceProbe();
        return { ...r, near: Array.from(r.near), far: Array.from(r.far) };
      },
    };
    return () => void delete window.__sound;
  }, []);
  const render = async () => {
    setBusy(true);
    const r = await renderFirefight();
    const blob = new Blob([encodeWav(r.samples, r.sampleRate) as Uint8Array<ArrayBuffer>], {
      type: "audio/wav",
    });
    setUrl((old) => {
      if (old) URL.revokeObjectURL(old);
      return URL.createObjectURL(blob);
    });
    setBusy(false);
  };
  return (
    <main style={{ padding: 24 }}>
      <p>
        <a href="/labs">Labs</a>
      </p>
      <h1>Sound</h1>
      <p>
        A scripted firefight, eight seconds, heard from behind the blue hedge: rifles, a tank
        driving and traversing, red's AP, HE and HMG, a ricochet, a guided missile, a burning wreck,
        the countryside, and two unseen enemies heard only as cues.
      </p>
      <SoundControls />
      <p>
        <button onClick={render} disabled={busy}>
          {busy ? "Rendering…" : "Render firefight"}
        </button>
      </p>
      {url && <audio controls src={url} data-testid="firefight-audio" />}
    </main>
  );
}
