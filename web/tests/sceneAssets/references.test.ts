// @vitest-environment node
// A family's reference library (`assets/references/<family>/references.json`):
// what `asset check` refuses by name, and which reference the review sheet
// sets beside each of the model's views. Images are judged by their headers,
// so each test builds just the bytes a header needs.
import { expect, test } from "vitest";
import { sha256Hex } from "@packages/scene-assets/src/glb.ts";
import {
  checkReferences,
  sheetReferences,
  type ReferenceLibrary,
} from "@packages/scene-assets/src/references.ts";

/** A PNG's signature and IHDR: all the check reads of one. */
function png(width: number, height: number): Uint8Array {
  const bytes = new Uint8Array(33);
  bytes.set([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82]);
  const view = new DataView(bytes.buffer);
  view.setUint32(16, width);
  view.setUint32(20, height);
  bytes.set([8, 2, 0, 0, 0], 24);
  return bytes;
}

/** A baseline JPEG's start, an APP0 segment and its frame header. */
function jpeg(width: number, height: number): Uint8Array {
  const app0 = [0xff, 0xe0, 0, 16, 74, 70, 73, 70, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0];
  const sof = [0xff, 0xc0, 0, 17, 8, height >> 8, height & 255, width >> 8, width & 255, 3];
  return new Uint8Array([
    0xff,
    0xd8,
    ...app0,
    ...sof,
    ...Array.from({ length: 9 }, () => 0),
    0xff,
    0xd9,
  ]);
}

const lfsPointer = (oid: string) =>
  new TextEncoder().encode(
    `version https://git-lfs.github.com/spec/v1\noid sha256:${oid}\nsize 123456\n`,
  );

type Files = Record<string, Uint8Array>;

/** A library over `files`, each entry's sha256 its file's unless given. */
async function library(files: Files, entries: Record<string, unknown>[], gaps: string[] = []) {
  const withHashes = await Promise.all(
    entries.map(async (e) => ({
      sha256: files[e.file as string] ? await sha256Hex(files[e.file as string]) : "0".repeat(64),
      ...e,
    })),
  );
  return JSON.stringify({ entries: withHashes, gaps });
}

const photo = (file: string, view: string, extra: Record<string, unknown> = {}) => ({
  file,
  variant: "tank_a",
  view,
  source: "photo",
  page: "https://commons.wikimedia.org/wiki/File:Tank_A.jpg",
  author: "Sgt. A. Photographer, US Army",
  licence: "public-domain",
  note: "",
  ...extra,
});

const generated = (file: string, view: string, extra: Record<string, unknown> = {}) => ({
  file,
  variant: "tank_a",
  view,
  source: "generated",
  licence: "ours",
  note: "layout only",
  model: "openai/gpt-image-2",
  prompt: "tank_a, three-quarter rear view",
  inputs: ["tank_a-side.jpg"],
  seed: 7,
  ...extra,
});

const check = async (text: string, files: Files) =>
  checkReferences("tanks", text, (file) => files[file] ?? null);
const codes = (findings: { code: string; severity: string }[]) =>
  findings.filter((f) => f.severity === "error").map((f) => f.code);

test("a library of licensed photos, a drawing and a labelled generated view passes", async () => {
  const files = {
    "tank_a-side.jpg": jpeg(1600, 900),
    "tank_a-top.png": png(800, 1200),
    "tank_a-side-2-generated.png": png(1536, 1024),
  };
  const text = await library(files, [
    photo("tank_a-side.jpg", "side", { licence: "CC-BY-SA-4.0" }),
    photo("tank_a-top.png", "top", { source: "drawing", licence: "CC0-1.0" }),
    generated("tank_a-side-2-generated.png", "side"),
  ]);
  expect(await check(text, files)).toEqual([]);
});

test("references.json that does not parse, or is not entries and gaps, is refused by name", async () => {
  expect(codes(await check("{ entries: [", {}))).toEqual(["references.json"]);
  expect(codes(await check(JSON.stringify([photo("a.jpg", "side")]), {}))).toEqual([
    "references.json",
  ]);
});

test("a listed file that is not there, or whose bytes are not its sha256, is refused", async () => {
  const files = { "tank_a-side.jpg": jpeg(800, 600) };
  const text = await library(files, [
    photo("tank_a-side.jpg", "side", { sha256: "a".repeat(64) }),
    photo("tank_a-front.jpg", "front"),
    photo("../tanks2/tank_a-rear.jpg", "rear"),
  ]);
  expect(codes(await check(text, { ...files, "../tanks2/tank_a-rear.jpg": jpeg(10, 10) }))).toEqual(
    ["references.sha256", "references.file", "references.file"],
  );
});

test("a licence we may not redistribute, or ours on a real photo, is refused", async () => {
  const files = { "a-side.jpg": jpeg(800, 600), "a-front.jpg": jpeg(800, 600) };
  const text = await library(files, [
    photo("a-side.jpg", "side", { licence: "CC-BY-NC-4.0" }),
    photo("a-front.jpg", "front", { licence: "ours" }),
  ]);
  expect(codes(await check(text, files))).toEqual(["references.licence", "references.licence"]);
});

test("an image whose long edge is over 1600 px is refused, JPEG or PNG", async () => {
  const files = {
    "a-side.jpg": jpeg(1601, 900),
    "a-top.png": png(900, 1601),
    "a-rear.jpg": jpeg(1600, 1600),
    "a-front.gif": new TextEncoder().encode("GIF89a"),
  };
  const text = await library(files, [
    photo("a-side.jpg", "side"),
    photo("a-top.png", "top"),
    photo("a-rear.jpg", "rear"),
    photo("a-front.gif", "front"),
  ]);
  expect(codes(await check(text, files))).toEqual([
    "references.image",
    "references.image",
    "references.image",
  ]);
});

test("a generated view must carry model, prompt, inputs and seed, be named so, and come from real references", async () => {
  const files = {
    "tank_a-side.jpg": jpeg(800, 600),
    "tank_a-side-2-generated.png": png(1536, 1024),
    "tank_a-side-3.png": png(1536, 1024),
    "tank_a-side-4-generated.png": png(1536, 1024),
  };
  const text = await library(files, [
    photo("tank_a-side.jpg", "side"),
    generated("tank_a-side-2-generated.png", "side", { model: "", seed: undefined }),
    generated("tank_a-side-3.png", "side"),
    generated("tank_a-side-4-generated.png", "side", {
      inputs: ["tank_a-side-2-generated.png", "tank_a-rear.jpg"],
    }),
  ]);
  const findings = (await check(text, files)).filter((f) => f.severity === "error");
  expect(findings.map((f) => f.code)).toEqual([
    "references.generated",
    "references.generated",
    "references.generated",
    "references.generated",
  ]);
  expect(findings[0].message).toContain("lacks model, seed");
  expect(findings[1].message).toContain("tank_a-side-3.png");
  expect(findings[2].message).toContain("tank_a-side-2-generated.png");
  expect(findings[3].message).toContain("tank_a-rear.jpg");
});

test("a view whose only sources are generated is refused; another variant's photo does not cover it", async () => {
  const files = {
    "tank_b-three_quarter_rear.jpg": jpeg(800, 600),
    "tank_a-side.jpg": jpeg(800, 600),
    "tank_a-three_quarter_rear-generated.png": png(1536, 1024),
  };
  const text = await library(files, [
    photo("tank_b-three_quarter_rear.jpg", "three_quarter_rear", { variant: "tank_b" }),
    photo("tank_a-side.jpg", "side"),
    generated("tank_a-three_quarter_rear-generated.png", "three_quarter_rear"),
  ]);
  const findings = (await check(text, files)).filter((f) => f.severity === "error");
  expect(findings.map((f) => f.code)).toEqual(["references.view"]);
  expect(findings[0].message).toContain("tank_a three_quarter_rear");
});

test("an unpulled LFS file is judged by its pointer's oid and named as unpulled, not misjudged", async () => {
  const content = jpeg(800, 600);
  const oid = await sha256Hex(content);
  const files = { "a-side.jpg": lfsPointer(oid), "a-rear.jpg": lfsPointer("b".repeat(64)) };
  const text = JSON.stringify({
    entries: [
      { ...photo("a-side.jpg", "side"), sha256: oid },
      { ...photo("a-rear.jpg", "rear"), sha256: oid },
    ],
    gaps: [],
  });
  const findings = await check(text, files);
  expect(codes(findings)).toEqual(["references.sha256"]);
  expect(findings.filter((f) => f.severity === "warning").map((f) => f.code)).toEqual([
    "references.unpulled",
    "references.unpulled",
  ]);
});

test("the sheet sets a photo before a generated view beside each view, and marks the rest missing", () => {
  const lib: ReferenceLibrary = {
    entries: [
      generated("tank_a-side-generated.png", "side") as never,
      photo("tank_a-side.jpg", "side") as never,
      generated("tank_a-top-generated.png", "top") as never,
      photo("tank_b-front.jpg", "front", { variant: "tank_b" }) as never,
    ],
    gaps: [],
  };
  const rows = sheetReferences(lib, "tank_a");
  const byView = Object.fromEntries(rows.map((r) => [r.view, [r.entry?.file ?? null, r.count]]));
  expect(byView).toEqual({
    three_quarter_front: [null, 0],
    side: ["tank_a-side.jpg", 2],
    three_quarter_rear: [null, 0],
    front: [null, 0],
    rear: [null, 0],
    top: ["tank_a-top-generated.png", 1],
    detail: [null, 0],
  });
});
