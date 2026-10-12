import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

/** Duet's ElevenLabs service proxy: the only place its address appears. Undocumented by Duet. */
export const BASE = "https://ctl.duet.so/v1/services/elevenlabs";
/** Raw masters keep the proxy's full-rate MP3; processing owns the served encoding. */
const RAW_FORMAT = "mp3_44100_128";
const TIMEOUT_MS = 180_000;

/** A refused or failed request. Its status says whether trying again can help. */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly body: string,
    path: string,
  ) {
    super(`ElevenLabs ${path}: ${status || "network error"} ${body.slice(0, 300)}`);
  }
  /** A bad key or an exhausted quota stops the whole run: nothing after it can succeed. */
  get fatal(): boolean {
    return this.status === 401 || this.status === 402 || /quota_exceeded/.test(this.body);
  }
  get retryable(): boolean {
    return !this.fatal && (this.status === 0 || this.status === 429 || this.status >= 500);
  }
}

export interface ClientOptions {
  fetch?: typeof fetch;
  base?: string;
  workspace: string;
  token: string;
}
export interface DesignedVoice {
  generated_voice_id: string;
  audio_base_64: string;
  duration_secs: number;
}

/** ElevenLabs through the proxy. The workspace must travel as a query parameter. */
export class ElevenLabsClient {
  private readonly fetch: typeof fetch;
  private readonly base: string;
  private readonly workspace: string;
  private readonly token: string;
  constructor(options: ClientOptions) {
    this.fetch = options.fetch ?? fetch;
    this.base = options.base ?? BASE;
    this.workspace = options.workspace;
    this.token = options.token;
  }

  private async post(path: string, body: unknown, query: Record<string, string> = {}) {
    const params = new URLSearchParams({ ...query, workspace: this.workspace });
    const form = body instanceof FormData;
    let response: Response;
    try {
      response = await this.fetch(`${this.base}${path}?${params}`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${this.token}`,
          ...(form ? {} : { "content-type": "application/json" }),
        },
        body: form ? body : JSON.stringify(body),
        signal: AbortSignal.timeout(TIMEOUT_MS),
      });
    } catch (error) {
      throw new ApiError(0, String(error), path);
    }
    if (!response.ok) throw new ApiError(response.status, await response.text(), path);
    return response;
  }
  private async bytes(path: string, body: unknown, query: Record<string, string> = {}) {
    return new Uint8Array(await (await this.post(path, body, query)).arrayBuffer());
  }

  /** Text-to-speech: v3 reads the line's audio tags. */
  tts(voiceId: string, text: string, settings: object, model = "eleven_v3") {
    return this.bytes(
      `/v1/text-to-speech/${voiceId}`,
      { text, model_id: model, voice_settings: settings },
      { output_format: RAW_FORMAT },
    );
  }
  async stt(audio: Uint8Array): Promise<{ text: string }> {
    const form = new FormData();
    form.append("model_id", "scribe_v1");
    form.append("language_code", "en");
    form.append("file", new Blob([audio as BlobPart], { type: "audio/mpeg" }), "take.mp3");
    return (await this.post("/v1/speech-to-text", form)).json();
  }
  /** Sound effects; a bed asks for a seamless loop. */
  sound(prompt: string, durationS: number, loop: boolean, model = "eleven_text_to_sound_v2") {
    return this.bytes(
      "/v1/sound-generation",
      { text: prompt, duration_seconds: durationS, loop, model_id: model },
      { output_format: RAW_FORMAT },
    );
  }
  music(prompt: string, lengthMs: number, model = "music_v1") {
    return this.bytes(
      "/v1/music",
      { prompt, music_length_ms: lengthMs, model_id: model },
      { output_format: RAW_FORMAT },
    );
  }
  /** Voice Design: candidate previews speaking `text`, not yet saved. */
  async designVoice(description: string, text: string, model = "eleven_ttv_v3") {
    const response = await this.post(
      "/v1/text-to-voice/design",
      { voice_description: description, text, model_id: model },
      { output_format: RAW_FORMAT },
    );
    return ((await response.json()) as { previews: DesignedVoice[] }).previews;
  }
  /** Saves a designed candidate permanently and returns its voice_id. */
  async saveVoice(
    name: string,
    description: string,
    generatedVoiceId: string,
    labels: Record<string, string> = {},
  ) {
    const response = await this.post("/v1/text-to-voice", {
      voice_name: name,
      voice_description: description,
      generated_voice_id: generatedVoiceId,
      labels,
    });
    return ((await response.json()) as { voice_id: string }).voice_id;
  }
}

/** The client for this machine's Duet login. The token is read here and never printed. */
export function duetClient(path = join(homedir(), ".duet/config.json")): ElevenLabsClient {
  const config = JSON.parse(readFileSync(path, "utf8")) as {
    apiKey?: string;
    defaultWorkspace?: string;
  };
  if (!config.apiKey || !config.defaultWorkspace)
    throw new Error(`${path}: no Duet login (apiKey and defaultWorkspace)`);
  return new ElevenLabsClient({ workspace: config.defaultWorkspace, token: config.apiKey });
}
