// @vitest-environment node
import { expect, test } from "vitest";
import { publishCard, readCard, CARD_SPEC } from "@packages/scene-assets/src/cards";
import { bakeCatalog } from "@packages/scene-assets/src/bake";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader";
import { AUTHORITY, testCatalog, testSources } from "./synthetic";

test("prepared appearance and depth pixels survive publication and the ordinary appearance load", async () => {
  const catalog = testCatalog();
  catalog.appearances.crate.far_card_px = 2;
  const sources = testSources();
  const out = await bakeCatalog(catalog, async (p) => sources[p], { authority: AUTHORITY }, [
    "crate",
  ]);
  const card = {
    appearance: "crate",
    bundle: out.runtime.appearances.crate.bundle,
    state: "default",
    spec: CARD_SPEC,
    width: 512,
    height: 128,
    center: [0, 0, 1] as [number, number, number],
    radius: 2,
    albedo: new Uint8Array(512 * 128 * 4).fill(127),
    depth: new Uint8Array(512 * 128 * 2),
    normal: new Uint8Array(512 * 128 * 4).fill(63),
  };
  await publishCard(out, card);
  out.files.set("catalog.json", new TextEncoder().encode(JSON.stringify(out.runtime)));
  const library = new AppearanceLibrary(memoryFetch(out.files, "/art/"));
  const loaded = await library.load("/art/");
  expect(loaded.appearances.get("crate")!.card).toEqual({
    ...card,
    spec: { ...CARD_SPEC, pitches: CARD_SPEC.pitches.map(Math.fround) },
  });
  const hash = out.runtime.cards![card.bundle];
  out.runtime.cards![card.bundle] = "a".repeat(64);
  out.files.set("catalog.json", new TextEncoder().encode(JSON.stringify(out.runtime)));
  await expect(library.load("/art/")).rejects.toThrow(/gzip/);
  expect(library.installed).toBe(loaded);
  out.runtime.cards![card.bundle] = hash;
  out.runtime.appearances.crate.bundle = "b".repeat(64);
  out.runtime.cards!["b".repeat(64)] = hash;
  await expect(readCard(out.runtime, "crate", async (p) => out.files.get(p)!)).rejects.toThrow(
    /bundle/,
  );
});

test("malformed and stale declared cards fail atomically, while an absent card keeps mesh art", async () => {
  const catalog = testCatalog();
  catalog.appearances.crate.far_card_px = 2;
  const sources = testSources();
  const out = await bakeCatalog(catalog, async (p) => sources[p], { authority: AUTHORITY }, [
    "crate",
  ]);
  out.files.set("catalog.json", new TextEncoder().encode(JSON.stringify(out.runtime)));
  const library = new AppearanceLibrary(memoryFetch(out.files, "/art/"));
  const meshOnly = await library.load("/art/");
  expect(meshOnly.appearances.get("crate")!.card).toBeUndefined();
  const card = {
    appearance: "crate",
    bundle: out.runtime.appearances.crate.bundle,
    state: "default",
    spec: CARD_SPEC,
    width: 512,
    height: 128,
    center: [0, 0, 1] as [number, number, number],
    radius: 2,
    albedo: new Uint8Array(512 * 128 * 4),
    depth: new Uint8Array(512 * 128 * 2),
    normal: new Uint8Array(512 * 128 * 4),
  };
  for (const bad of [
    { ...card, radius: 0 },
    { ...card, depth: new Uint8Array(0) },
    { ...card, center: [NaN, 0, 0] },
    { ...card, spec: { ...CARD_SPEC, yaws: 4 } },
    { ...card, depth: new Uint8Array(512 * 128 * 2), normal: new Uint8Array(4) },
    { ...card, bundle: "b".repeat(64) },
  ])
    await expect(publishCard(out, bad as typeof card)).rejects.toThrow(/card crate/);
  await publishCard(out, card);
  const hash = out.runtime.cards![card.bundle];
  const transport = out.runtime.gzip![hash];
  const path = `${transport.hash}/card.bin`;
  const valid = out.files.get(path)!;
  out.files.set(path, valid.subarray(0, valid.length - 1));
  out.files.set("catalog.json", new TextEncoder().encode(JSON.stringify(out.runtime)));
  await expect(library.load("/art/")).rejects.toThrow(/length/);
  expect(library.installed).toBe(meshOnly);
});

test("mesh rebakes retain identical-source cards and drop changed-source cards without a GPU", async () => {
  const { retainCards } = await import("@packages/scene-assets/src/cards");
  const { catalogLoadBytes } = await import("@packages/scene-assets/src/gzip");
  const catalog = testCatalog();
  catalog.appearances.crate.far_card_px = 2;
  const sources = testSources();
  const out = await bakeCatalog(catalog, async (p) => sources[p], { authority: AUTHORITY }, [
    "crate",
  ]);
  const before = catalogLoadBytes(out.runtime);
  await publishCard(out, {
    appearance: "crate",
    bundle: out.runtime.appearances.crate.bundle,
    state: "default",
    spec: CARD_SPEC,
    width: 512,
    height: 128,
    center: [0, 0, 1],
    radius: 2,
    albedo: new Uint8Array(512 * 128 * 4),
    depth: new Uint8Array(512 * 128 * 2),
    normal: new Uint8Array(512 * 128 * 4),
  });
  const hash = out.runtime.cards![out.runtime.appearances.crate.bundle];
  expect(catalogLoadBytes(out.runtime) - before).toBe(out.runtime.gzip![hash].bytes);
  const rebaked = await bakeCatalog(catalog, async (p) => sources[p], { authority: AUTHORITY }, [
    "crate",
  ]);
  await retainCards(rebaked, out.runtime, async (p) => out.files.get(p)!);
  expect(rebaked.runtime.cards).toEqual(out.runtime.cards);
  const changed = structuredClone(rebaked.runtime);
  delete changed.cards;
  changed.appearances.crate.bundle = "b".repeat(64);
  await retainCards(
    { runtime: changed, files: new Map() },
    out.runtime,
    async (p) => out.files.get(p)!,
  );
  expect(changed.cards).toBeUndefined();
});

test("appearances sharing source bytes load the shared card under their own names", async () => {
  const catalog = testCatalog();
  catalog.appearances.crate.far_card_px = 2;
  catalog.appearances.alias = { ...catalog.appearances.crate };
  const sources = testSources();
  const out = await bakeCatalog(catalog, async (p) => sources[p], { authority: AUTHORITY }, [
    "crate",
    "alias",
  ]);
  expect(out.runtime.appearances.alias.bundle).toBe(out.runtime.appearances.crate.bundle);
  await publishCard(out, {
    appearance: "crate",
    bundle: out.runtime.appearances.crate.bundle,
    state: "default",
    spec: CARD_SPEC,
    width: 512,
    height: 128,
    center: [0, 0, 1],
    radius: 2,
    albedo: new Uint8Array(512 * 128 * 4).fill(17),
    depth: new Uint8Array(512 * 128 * 2),
    normal: new Uint8Array(512 * 128 * 4).fill(29),
  });
  out.files.set("catalog.json", new TextEncoder().encode(JSON.stringify(out.runtime)));
  const loaded = await new AppearanceLibrary(memoryFetch(out.files, "/art/")).load("/art/");
  expect(loaded.appearances.get("alias")!.card).toEqual({
    ...loaded.appearances.get("crate")!.card,
    appearance: "alias",
  });
});

test("card generation loads only its source mesh even when the old card is broken", async () => {
  const { cardSourceCatalog } = await import("@packages/scene-assets/src/cards");
  const catalog = testCatalog();
  catalog.appearances.crate.far_card_px = 2;
  const sources = testSources();
  const out = await bakeCatalog(catalog, async (p) => sources[p], { authority: AUTHORITY }, [
    "crate",
  ]);
  out.runtime.cards = { [out.runtime.appearances.crate.bundle]: "a".repeat(64) };
  const source = cardSourceCatalog(out.runtime, "crate");
  out.files.set("catalog.json", new TextEncoder().encode(JSON.stringify(source)));
  const loaded = await new AppearanceLibrary(memoryFetch(out.files, "/art/")).load("/art/");
  expect(loaded.appearances.get("crate")!.bundle.kind).toBe("static");
  expect(loaded.appearances.get("crate")!.card).toBeUndefined();
  expect(out.runtime.cards).toEqual({ [out.runtime.appearances.crate.bundle]: "a".repeat(64) });
});

test("replacement publication removes superseded card transport and remains check-consistent", async () => {
  const { retainCards } = await import("@packages/scene-assets/src/cards");
  const catalog = testCatalog();
  catalog.appearances.crate.far_card_px = 2;
  const sources = testSources();
  const out = await bakeCatalog(catalog, async (p) => sources[p], { authority: AUTHORITY }, [
    "crate",
  ]);
  const card = {
    appearance: "crate",
    bundle: out.runtime.appearances.crate.bundle,
    state: "default",
    spec: CARD_SPEC,
    width: 512,
    height: 128,
    center: [0, 0, 1] as [number, number, number],
    radius: 2,
    albedo: new Uint8Array(512 * 128 * 4),
    depth: new Uint8Array(512 * 128 * 2),
    normal: new Uint8Array(512 * 128 * 4),
  };
  await publishCard(out, card);
  const old = out.runtime.cards![card.bundle];
  const oldPath = `${out.runtime.gzip![old].hash}/card.bin`;
  card.albedo[0] = 255;
  expect(await publishCard(out, card)).toBe(oldPath);
  expect(out.files.has(oldPath)).toBe(false);
  expect(out.runtime.gzip![old]).toBeUndefined();
  const checked = await bakeCatalog(catalog, async (p) => sources[p], { authority: AUTHORITY }, [
    "crate",
  ]);
  await retainCards(checked, out.runtime, async (p) => out.files.get(p)!);
  expect(checked.runtime).toEqual(out.runtime);
  expect(checked.files).toEqual(out.files);
});

test("deferred aliases retain their own card names and appearance metadata", async () => {
  const catalog = testCatalog();
  catalog.appearances.crate.far_card_px = 2;
  catalog.appearances.alias = { ...catalog.appearances.crate };
  const sources = testSources();
  const out = await bakeCatalog(catalog, async (p) => sources[p], { authority: AUTHORITY }, [
    "crate",
    "alias",
  ]);
  await publishCard(out, {
    appearance: "crate",
    bundle: out.runtime.appearances.crate.bundle,
    state: "default",
    spec: CARD_SPEC,
    width: 512,
    height: 128,
    center: [0, 0, 1],
    radius: 2,
    albedo: new Uint8Array(512 * 128 * 4),
    depth: new Uint8Array(512 * 128 * 2),
    normal: new Uint8Array(512 * 128 * 4),
  });
  out.runtime.appearances.crate.regional_family = "region";
  out.runtime.appearances.alias.regional_family = "region";
  out.runtime.appearances.alias.far_card_px = 1;
  out.files.set("catalog.json", new TextEncoder().encode(JSON.stringify(out.runtime)));
  const library = new AppearanceLibrary(memoryFetch(out.files, "/art/"));
  await library.load("/art/");
  const loaded = await library.withAppearances(["crate", "alias"]);
  expect(loaded.appearances.get("crate")!.card!.appearance).toBe("crate");
  expect(loaded.appearances.get("alias")!.card!.appearance).toBe("alias");
  expect(loaded.appearances.get("alias")!.farCardPx).toBe(1);
});
