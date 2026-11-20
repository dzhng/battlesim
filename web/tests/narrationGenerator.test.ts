// @vitest-environment node
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import type { NarrationPack, VoiceCatalog } from "@packages/battle-audio/src/voices/voiceCatalog";
import { narrationPackUrl } from "@packages/battle-audio/src/voices/voiceCatalog";
import { bentoSpeech, type Exec } from "@packages/battle-audio/tools/generate/bento";
import { LOCK, readLock, recordVerdicts } from "@packages/battle-audio/tools/generate/lock";
import { generateNarration } from "@packages/battle-audio/tools/generate/narration";
import type { Processor, VoiceProfiles } from "@packages/battle-audio/tools/generate/voices";

const catalog = (lines: Record<string, string>) =>
  ({
    roles: {},
    sets: {},
    assignments: {},
    retakes: {},
    narration: {
      speakers: {
        hq: {
          voice_id: "library-voice",
          model: "m",
          seed: 7,
          processing: "voice-earpiece",
          description: "",
        },
      },
      lines: Object.fromEntries(
        Object.entries(lines).map(([id, text]) => [id, { speaker: "hq", text }]),
      ),
    },
  }) as unknown as VoiceCatalog;
const profiles: VoiceProfiles = { encoder: {}, profiles: { "voice-earpiece": { bed: null } } };
const processor: Processor = async ({ profile, input, output }) => {
  writeFileSync(output, `${profile}:${readFileSync(input)}`);
  return { duration_s: 2 };
};

/** `bento` at the process edge: text-to-speech speaks the body's text, speech
 *  to text echoes the take back, and every call is recorded. */
function fakeBento(fail?: string) {
  const calls: string[][] = [];
  const exec: Exec = async (args) => {
    calls.push(args);
    const arg = (flag: string) => args[args.indexOf(flag) + 1];
    if (fail) return { code: 1, stderr: fail };
    const path = arg("--path");
    if (path.startsWith("/v1/text-to-speech/")) {
      const { text } = JSON.parse(readFileSync(arg("--body-file"), "utf8"));
      writeFileSync(arg("--out"), `speech:${text}`);
    } else {
      const file = args.find((a) => a.startsWith("file=@"))!.slice("file=@".length);
      writeFileSync(
        arg("--out"),
        JSON.stringify({ text: String(readFileSync(file)).slice("speech:".length) }),
      );
    }
    return { code: 0, stderr: "" };
  };
  const paths = () => calls.map((c) => c[c.indexOf("--path") + 1]);
  return { exec, calls, tts: () => paths().filter((p) => p.includes("text-to-speech")).length };
}

const roots: string[] = [];
afterEach(() => roots.splice(0).forEach((root) => rmSync(root, { recursive: true, force: true })));
function workspace() {
  const root = mkdtempSync(join(tmpdir(), "narration-"));
  roots.push(root);
  return root;
}
const run = (root: string, lines: Record<string, string>, exec: Exec | null, imports?: string) =>
  generateNarration({
    root,
    catalog: catalog(lines),
    client: exec ? bentoSpeech("armata", exec) : null,
    profiles,
    processor,
    imports,
    log: () => {},
  });
const pack = (root: string) =>
  JSON.parse(
    readFileSync(join(root, "assets/runtime", narrationPackUrl("hq")), "utf8"),
  ) as NarrationPack;
/** Judge every take as heard, the way the sound workbench does. */
function judge(root: string, value: "ok" | "bad", ...lines: string[]) {
  const takes = readLock(join(root, LOCK)).takes;
  recordVerdicts(
    join(root, LOCK),
    value,
    lines.map((l) => `narration/hq/${l}@${takes[`narration/hq/${l}`].served_sha256}`),
    () => {},
  );
}

describe("narration generation", () => {
  it("speaks each line once through bento in the named workspace, with the speaker's voice", async () => {
    const root = workspace();
    const bento = fakeBento();
    const lines = { hello: "Lantern on the net.", bye: "Lantern out." };
    expect(await run(root, lines, bento.exec)).toEqual({ bought: 2, flagged: 0 });
    for (const call of bento.calls)
      expect(call.slice(call.indexOf("--workspace"), call.indexOf("--workspace") + 2)).toEqual([
        "--workspace",
        "armata",
      ]);
    expect(
      bento.calls.find((c) => c.join(" ").includes("text-to-speech/library-voice")),
    ).toBeDefined();
    expect(await run(root, lines, bento.exec)).toEqual({ bought: 0, flagged: 0 });
    expect(bento.tts()).toBe(2);
  });

  it("serves only takes judged ok on the bytes the reviewer heard", async () => {
    const root = workspace();
    const lines = { hello: "Lantern on the net.", bye: "Lantern out." };
    await run(root, lines, fakeBento().exec);
    expect(pack(root).lines).toEqual([]);
    judge(root, "ok", "hello");
    judge(root, "bad", "bye");
    await run(root, lines, null);
    expect(pack(root).lines.map((l) => l.line)).toEqual(["hello"]);
    // A rewritten line is a new recording: bought again, and unjudged until heard.
    const bento = fakeBento();
    await run(root, { ...lines, hello: "Lantern, on the net now." }, bento.exec);
    expect(bento.tts()).toBe(1);
    expect(pack(root).lines).toEqual([]);
  });

  it("imports a review take of the line instead of buying one", async () => {
    const root = workspace();
    const imports = join(root, "review");
    mkdirSync(imports);
    writeFileSync(join(imports, "hello.mp3"), "speech:Lantern on the net.");
    const bento = fakeBento();
    expect(await run(root, { hello: "Lantern on the net." }, bento.exec, imports)).toEqual({
      bought: 0,
      flagged: 0,
    });
    expect(bento.tts()).toBe(0);
  });

  it("reports what bento said when a call fails", async () => {
    const root = workspace();
    await expect(
      run(root, { hello: "Lantern on the net." }, fakeBento("not logged in").exec),
    ).rejects.toThrow(/not logged in/);
  });
});
