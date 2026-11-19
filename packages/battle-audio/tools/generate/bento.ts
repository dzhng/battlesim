// ElevenLabs through the installed `bento` CLI, which owns the login, the
// workspace and the vendor key: nothing here reads a credential. Bento's proxy
// admits text-to-speech and speech-to-text but not Voice Design, and a voice
// created through it expires within a day, so what it speaks with is a
// shared-library voice, which never expires.
import { execFile } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

/** The two calls narration makes. */
export interface SpeechClient {
  tts(voiceId: string, text: string, model: string, seed: number): Promise<Uint8Array>;
  stt(audio: Uint8Array): Promise<{ text: string }>;
}

export type Exec = (args: string[]) => Promise<{ code: number; stderr: string }>;

const runBento: Exec = (args) =>
  new Promise((resolve) => {
    execFile("bento", args, { timeout: 180_000, maxBuffer: 16 << 20 }, (error, _out, stderr) =>
      resolve({ code: error ? 1 : 0, stderr }),
    );
  });

/** A client over `bento service proxy`; `workspace` picks one other than the
 *  CLI's selected workspace. */
export function bentoSpeech(workspace?: string, exec: Exec = runBento): SpeechClient {
  /** One proxied call: `inputs` are written beside the call's output first. */
  const call = async (
    path: string,
    inputs: Record<string, Uint8Array | string>,
    extra: string[],
  ) => {
    const dir = mkdtempSync(join(tmpdir(), "narration-"));
    try {
      for (const [name, bytes] of Object.entries(inputs)) writeFileSync(join(dir, name), bytes);
      const out = join(dir, "out");
      const { code, stderr } = await exec([
        "service",
        "proxy",
        "post",
        "--service-id",
        "elevenlabs",
        "--path",
        path,
        ...(workspace ? ["--workspace", workspace] : []),
        ...extra.map((arg) => arg.replaceAll("{dir}", dir)),
        "--out",
        out,
      ]);
      const body = existsSync(out) ? new Uint8Array(readFileSync(out)) : new Uint8Array();
      if (code !== 0)
        throw new Error(
          `bento ${path}: ${stderr.trim()} ${new TextDecoder().decode(body.slice(0, 300))}`,
        );
      return body;
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  };
  return {
    tts: (voiceId, text, model, seed) =>
      call(
        `/v1/text-to-speech/${voiceId}`,
        { "body.json": JSON.stringify({ text, model_id: model, seed }) },
        ["--body-file", "{dir}/body.json"],
      ),
    stt: async (audio) => {
      const body = await call("/v1/speech-to-text", { "take.mp3": audio }, [
        "--form",
        "model_id=scribe_v1",
        "--form",
        "file=@{dir}/take.mp3",
      ]);
      return JSON.parse(new TextDecoder().decode(body)) as { text: string };
    },
  };
}
