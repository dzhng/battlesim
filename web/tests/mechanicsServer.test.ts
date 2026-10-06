// @vitest-environment node
import { cp, mkdtemp, mkdir, writeFile, readFile, rm } from "node:fs/promises";
import * as filesystem from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterEach, expect, test, vi } from "vitest";
import { MechanicsStore, nativeValidator } from "../../apps/mechanics-editor/server";
import { mechanicsSourcePaths } from "../../apps/fixture-publication/publication";
import type { JsonObject } from "../../apps/mechanics-editor/src/protocol";

vi.mock("node:fs/promises", async (importOriginal) => ({
  ...(await importOriginal<typeof import("node:fs/promises")>()),
  rename: vi.fn((await importOriginal<typeof import("node:fs/promises")>()).rename),
}));

const roots: string[] = [];
afterEach(async () => {
  vi.restoreAllMocks();
  vi.mocked(filesystem.rename).mockImplementation(
    (await vi.importActual<typeof import("node:fs/promises")>("node:fs/promises")).rename,
  );
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

async function store() {
  const root = await mkdtemp(join(tmpdir(), "mechanics-editor-"));
  roots.push(root);
  await mkdir(join(root, "fixtures/units"), { recursive: true });
  await mkdir(join(root, "fixtures/props"), { recursive: true });
  await writeFile(
    join(root, "fixtures/game.json"),
    JSON.stringify({ weapons: { rifle: { damage: 35 } } }),
  );
  await writeFile(
    join(root, "fixtures/units/test.json"),
    JSON.stringify({ units: { test: { cost: 1 } } }),
  );
  await writeFile(join(root, "fixtures/catalog.json"), "{}");
  const validate = async (game: JsonObject, documents: JsonObject[]) =>
    JSON.stringify(
      {
        weapons: game.weapons,
        documents: [{ soldiers: {}, ...documents[0] }],
        units: [],
      },
      null,
      2,
    ) + "\n";
  return { root, editor: new MechanicsStore(root, validate) };
}

test("preview changes no files and save writes the reviewed JSON and matching catalog", async () => {
  const { root, editor } = await store();
  const initial = await editor.snapshot();
  const draft = {
    revision: initial.revision,
    changes: [{ section: "weapons" as const, id: "rifle", path: ["damage"], value: 40 }],
  };
  const preview = await editor.preview(draft);
  expect(
    JSON.parse(await readFile(join(root, "fixtures/game.json"), "utf8")).weapons.rifle.damage,
  ).toBe(35);
  expect(preview.files.find((file) => file.path === "fixtures/game.json")?.after).toContain("40");
  await editor.save(draft);
  expect(
    JSON.parse(await readFile(join(root, "fixtures/game.json"), "utf8")).weapons.rifle.damage,
  ).toBe(40);
  expect(
    JSON.parse(await readFile(join(root, "fixtures/catalog.json"), "utf8")).weapons.rifle.damage,
  ).toBe(40);
});

test("a change to an untouched source makes the draft stale without overwriting either file", async () => {
  const { root, editor } = await store();
  const initial = await editor.snapshot();
  const outside = '{"units":{"test":{"cost":2}}}\n';
  await writeFile(join(root, "fixtures/units/test.json"), outside);
  await expect(
    editor.save({
      revision: initial.revision,
      changes: [{ section: "weapons", id: "rifle", path: ["damage"], value: 40 }],
    }),
  ).rejects.toMatchObject({ status: 409 });
  expect(await readFile(join(root, "fixtures/units/test.json"), "utf8")).toBe(outside);
  expect(
    JSON.parse(await readFile(join(root, "fixtures/game.json"), "utf8")).weapons.rifle.damage,
  ).toBe(35);
  expect(await readFile(join(root, "fixtures/catalog.json"), "utf8")).toBe("{}");
});

test("malformed JSON edit operations return named validation errors without writes", async () => {
  const { editor } = await store();
  const initial = await editor.snapshot();
  for (const change of [
    null,
    { section: "weapons", id: "rifle", path: ["damage"], restore: "yes" },
    { section: "weapons", id: "rifle", path: ["name"], value: "Renamed" },
  ]) {
    await expect(
      editor.save(JSON.parse(JSON.stringify({ revision: initial.revision, changes: [change] }))),
    ).rejects.toMatchObject({ status: 400 });
  }
  expect(await editor.snapshot()).toEqual(initial);
});

test("a publication failure restores already replaced sources and leaves the store usable", async () => {
  const { root, editor } = await store();
  const initial = await editor.snapshot();
  const rename = (await vi.importActual<typeof import("node:fs/promises")>("node:fs/promises"))
    .rename;
  vi.spyOn(filesystem, "rename").mockImplementation(async (from, to) => {
    if (String(to) === join(root, "fixtures/catalog.json")) throw new Error("Disk write failed");
    return rename(from, to);
  });
  const draft = {
    revision: initial.revision,
    changes: [{ section: "weapons" as const, id: "rifle", path: ["damage"], value: 40 }],
  };
  await expect(editor.save(draft)).rejects.toThrow("Disk write failed");
  expect(await editor.snapshot()).toEqual(initial);
  vi.restoreAllMocks();
  vi.mocked(filesystem.rename).mockImplementation(rename);
  const saved = await editor.save(draft);
  expect((saved.catalog.weapons as JsonObject).rifle).toMatchObject({ damage: 40 });
});

test("an outside edit during publication aborts the save and survives rollback", async () => {
  const { root, editor } = await store();
  const initial = await editor.snapshot();
  const rename = (await vi.importActual<typeof import("node:fs/promises")>("node:fs/promises"))
    .rename;
  const outside = '{"units":{"test":{"cost":2}}}\n';
  vi.mocked(filesystem.rename).mockImplementation(async (from, to) => {
    await rename(from, to);
    if (String(to) === join(root, "fixtures/game.json"))
      await writeFile(join(root, "fixtures/units/test.json"), outside);
  });
  await expect(
    editor.save({
      revision: initial.revision,
      changes: [{ section: "weapons", id: "rifle", path: ["damage"], value: 40 }],
    }),
  ).rejects.toMatchObject({ status: 409 });
  expect(await readFile(join(root, "fixtures/units/test.json"), "utf8")).toBe(outside);
  expect(
    JSON.parse(await readFile(join(root, "fixtures/game.json"), "utf8")).weapons.rifle.damage,
  ).toBe(35);
  expect(await readFile(join(root, "fixtures/catalog.json"), "utf8")).toBe("{}");
});

test("the next reader rolls an interrupted publication back before exposing a generation", async () => {
  const { root, editor } = await store();
  const initial = await editor.snapshot();
  const preview = await editor.preview({
    revision: initial.revision,
    changes: [{ section: "weapons", id: "rifle", path: ["damage"], value: 40 }],
  });
  const departed = spawnSync(process.execPath, ["-e", ""]);
  expect(departed.status).toBe(0);
  await writeFile(
    join(root, "throwaway/mechanics-editor-transaction.json"),
    JSON.stringify({ pid: departed.pid, files: preview.files }),
  );
  const first = preview.files[0];
  await writeFile(join(root, first.path), first.after);
  expect(
    await new MechanicsStore(
      root,
      async (game) =>
        JSON.stringify(
          {
            weapons: game.weapons,
            documents: initial.catalog.documents,
            units: [],
          },
          null,
          2,
        ) + "\n",
    ).snapshot(),
  ).toEqual(initial);
});

async function nativeStore() {
  const project = resolve("..");
  const root = await mkdtemp(join(tmpdir(), "mechanics-native-"));
  roots.push(root);
  await mkdir(join(root, "fixtures"));
  for (const path of ["game.json", "catalog.json", "units", "props"])
    await cp(join(project, "fixtures", path), join(root, "fixtures", path), { recursive: true });
  return { root, editor: new MechanicsStore(root, nativeValidator(project)) };
}

test("a shared weapon preview names all users and invalid flight settings never write", async () => {
  const { editor } = await nativeStore();
  const initial = await editor.snapshot();
  const weapon = (initial.catalog.weapons as JsonObject).rifle as JsonObject;
  const preview = await editor.preview({
    revision: initial.revision,
    changes: [
      { section: "weapons", id: "rifle", path: ["damage"], value: Number(weapon.damage) + 1 },
    ],
  });
  expect(preview.affectedUnits.sort()).toEqual([
    "assault_squad",
    "at",
    "eastern_atgm_team_kornet",
    "eastern_rpg_team_rpg_29",
    "eastern_rpg_team_rpg_7",
    "eastern_scouts_light_patrol",
    "europe_recon_patrol_light_patrol",
    "recon",
    "rifle",
    "rifle_squad",
    "us_army_scouts_light_patrol",
    "us_atgm_team_bgm_71_tow_2a",
    "us_force_recon_recon_patrol",
    "us_marine_squad_close_quarters_infantry",
  ]);
  await expect(
    editor.save({
      revision: initial.revision,
      changes: [{ section: "weapons", id: "grenade", path: ["range_m"], value: 1 }],
    }),
  ).rejects.toThrow();
  expect(await editor.snapshot()).toEqual(initial);
}, 30000);

test("an inherited named mount edits and restores without copying sibling mounts", async () => {
  const { root, editor } = await nativeStore();
  const source = join(root, "fixtures/units/generic/tanks.json");
  const authored = JSON.parse(await readFile(source, "utf8"));
  authored.units.variant = { extends: "tank", name: "Variant tank" };
  await writeFile(source, JSON.stringify(authored));
  let snapshot = await editor.snapshot();
  snapshot = await editor.save({
    revision: snapshot.revision,
    changes: [
      { section: "units", id: "variant", path: ["mounts", "HMG", "weapons"], value: ["rifle"] },
    ],
  });
  expect(JSON.parse(await readFile(source, "utf8")).units.variant.mounts).toEqual([
    { name: "HMG", weapons: ["rifle"] },
  ]);
  snapshot = await editor.save({
    revision: snapshot.revision,
    changes: [
      { section: "units", id: "variant", path: ["mounts", "HMG", "weapons"], restore: true },
    ],
  });
  expect(JSON.parse(await readFile(source, "utf8")).units.variant).toEqual(authored.units.variant);
  expect((snapshot.catalog.documents as JsonObject[])[0].units).toMatchObject({
    variant: { mounts: [{ name: "cannon" }, { name: "HMG", weapons: ["hmg"] }] },
  });
}, 30000);

test("an upgrade-masked edit is refused instead of saving an ineffective value", async () => {
  const { root, editor } = await nativeStore();
  const source = join(root, "fixtures/units/generic/infantry.json");
  const authored = JSON.parse(await readFile(source, "utf8"));
  authored.parts = {
    fixed_cost: {
      name: "Fixed cost",
      description: "Test upgrade",
      nodes: [],
      patch: { cost: 150 },
    },
  };
  authored.units.rifle.parts = ["fixed_cost"];
  await writeFile(source, JSON.stringify(authored));
  const initial = await editor.snapshot();
  await expect(
    editor.save({
      revision: initial.revision,
      changes: [{ section: "units", id: "rifle", path: ["cost"], value: 200 }],
    }),
  ).rejects.toThrow("would not take effect");
  expect(await editor.snapshot()).toEqual(initial);
}, 30000);

test("decimal gameplay values survive native admission and publication without rounding the authored JSON", async () => {
  const { root, editor } = await nativeStore();
  const snapshot = await editor.snapshot();
  const scatter = 24.666666666666668;
  const saved = await editor.save({
    revision: snapshot.revision,
    changes: [
      { section: "weapons", id: "grenade", path: ["scatter_mrad"], value: scatter },
      {
        section: "units",
        id: "rifle",
        path: ["mobility", "foot", "offroad_kmh"],
        value: 12.3456789,
      },
    ],
  });
  expect((saved.catalog.weapons as JsonObject).grenade).toMatchObject({ scatter_mrad: scatter });
  expect(
    JSON.parse(await readFile(join(root, "fixtures/game.json"), "utf8")).weapons.grenade
      .scatter_mrad,
  ).toBe(scatter);
  expect((saved.catalog.documents as JsonObject[])[0].units).toMatchObject({
    rifle: { mobility: { foot: { offroad_kmh: 12.3456789 } } },
  });
}, 30000);

test("save publishes the Rust catalog generator’s exact canonical bytes", async () => {
  const { root, editor } = await nativeStore();
  const snapshot = await editor.snapshot();
  await editor.save({
    revision: snapshot.revision,
    changes: [{ section: "weapons", id: "rifle", path: ["damage"], value: 40 }],
  });
  const texts: string[] = [];
  for (const path of (await mechanicsSourcePaths(root)).filter((path) => path !== "fixtures/game.json"))
    texts.push(await readFile(join(root, path), "utf8"));
  const game = await readFile(join(root, "fixtures/game.json"), "utf8");
  const native = spawnSync(
    resolve(
      "..",
      process.env.CARGO_TARGET_DIR ?? "throwaway/target",
      "debug/examples/mechanics_validate",
    ),
    [],
    { input: `{"game":${game},"catalog":[${texts.join(",")}]}`, encoding: "utf8" },
  );
  expect(native.status, native.stderr).toBe(0);
  expect(await readFile(join(root, "fixtures/catalog.json"), "utf8")).toBe(native.stdout);
}, 30000);

test("editing an existing local soldier variant still lists its unit in the preview", async () => {
  const { editor } = await nativeStore();
  let snapshot = await editor.snapshot();
  snapshot = await editor.save({
    revision: snapshot.revision,
    changes: [{ section: "soldiers", id: "rifleman", unit: "rifle", path: ["hp"], value: 120 }],
  });
  const preview = await editor.preview({
    revision: snapshot.revision,
    changes: [
      { section: "soldiers", id: "rifle__rifleman", unit: "rifle", path: ["hp"], value: 140 },
    ],
  });
  expect(preview.affectedUnits).toEqual(["rifle"]);
}, 30000);

test("restoring multiple soldier overrides removes the local variant and restores inherited slots", async () => {
  const { root, editor } = await nativeStore();
  const source = join(root, "fixtures/units/generic/infantry.json");
  const authored = JSON.parse(await readFile(source, "utf8"));
  authored.units.veteran = { extends: "rifle", name: "Veteran squad" };
  await writeFile(source, JSON.stringify(authored));
  let snapshot = await editor.snapshot();
  snapshot = await editor.save({
    revision: snapshot.revision,
    changes: [
      { section: "soldiers", id: "rifleman", unit: "veteran", path: ["hp"], value: 120 },
      {
        section: "soldiers",
        id: "rifleman",
        unit: "veteran",
        path: ["mounts", "rifles", "squad"],
        value: false,
      },
    ],
  });
  const resolved = snapshot.catalog.documents as JsonObject[];
  expect((resolved[0].soldiers as JsonObject).rifleman).toMatchObject({ hp: 100 });
  expect((resolved[0].soldiers as JsonObject).veteran__rifleman).toMatchObject({ hp: 120 });
  snapshot = await editor.save({
    revision: snapshot.revision,
    changes: [
      {
        section: "soldiers",
        id: "veteran__rifleman",
        unit: "veteran",
        path: ["hp"],
        restore: true,
      },
      {
        section: "soldiers",
        id: "veteran__rifleman",
        unit: "veteran",
        path: ["mounts", "rifles", "squad"],
        restore: true,
      },
    ],
  });
  const saved = JSON.parse(await readFile(source, "utf8"));
  expect(saved.units.veteran).toEqual(authored.units.veteran);
  expect(saved.soldiers).toBeUndefined();
  expect((snapshot.catalog.documents as JsonObject[])[0].soldiers).not.toHaveProperty(
    "veteran__rifleman",
  );
}, 30000);

test("accepted soldier identities follow clone creation and cleanup across reordered slots", async () => {
  const { editor } = await nativeStore();
  const initial = await editor.snapshot();
  const creation = await editor.preview({
    revision: initial.revision,
    changes: [{ section: "soldiers", id: "rifleman", unit: "rifle", path: ["hp"], value: 120 }],
  });
  expect(creation.soldierIds).toEqual({ '["soldiers","rifleman","rifle"]': "rifle__rifleman" });
  const saved = await editor.save({
    revision: initial.revision,
    changes: [
      {
        section: "soldiers",
        id: "rifleman",
        unit: "rifle",
        path: ["mounts", "rifles", "squad"],
        value: false,
      },
      {
        section: "soldiers",
        id: "rifleman",
        unit: "rifle",
        path: ["mounts", "rifles", "special"],
        value: true,
      },
      {
        section: "soldiers",
        id: "grenadier",
        unit: "rifle",
        path: ["mounts", "grenade launcher", "special"],
        value: false,
      },
    ],
  });
  const before = (saved.catalog.documents as JsonObject[])[0];
  expect((before.soldiers as JsonObject).rifleman).toMatchObject({ mounts: [{ special: false }] });
  const reordered = ["rifle__rifleman", "rifle__grenadier"];
  const reorderedSnapshot = await editor.save({
    revision: saved.revision,
    changes: [
      { section: "units", id: "rifle", path: ["body", "squad", "slots"], value: reordered },
    ],
  });
  const preview = await editor.preview({
    revision: reorderedSnapshot.revision,
    changes: [
      {
        section: "soldiers",
        id: "rifle__rifleman",
        unit: "rifle",
        path: ["mounts"],
        restore: true,
      },
      {
        section: "soldiers",
        id: "rifle__grenadier",
        unit: "rifle",
        path: ["mounts"],
        restore: true,
      },
    ],
  });
  expect(preview.soldierIds).toEqual({
    '["soldiers","rifle__rifleman","rifle"]': "rifleman",
    '["soldiers","rifle__grenadier","rifle"]': "grenadier",
  });
  const document = (preview.catalog.documents as JsonObject[])[0];
  expect((document.units as JsonObject).rifle).toMatchObject({
    body: { squad: { slots: ["rifleman", "grenadier"] } },
  });
  expect((document.soldiers as JsonObject).rifle__rifleman).toBeUndefined();
  expect((document.soldiers as JsonObject).rifle__grenadier).toBeUndefined();
  expect((document.soldiers as JsonObject).grenadier).toMatchObject({
    mounts: [
      { name: "rifles", weapons: ["rifle"], squad: true, special: false },
      { name: "grenade launcher", weapons: ["grenade"], special: true },
    ],
  });
}, 30000);

test("a save response resolves its catalog from the same captured sources after an outside edit", async () => {
  const { root, editor } = await store();
  const initial = await editor.snapshot();
  const real = await vi.importActual<typeof import("node:fs/promises")>("node:fs/promises");
  vi.spyOn(filesystem, "rm").mockImplementation(async (path, options) => {
    await real.rm(path, options);
    if (String(path) === join(root, "throwaway/mechanics-editor-transaction.json"))
      await writeFile(join(root, "fixtures/game.json"), '{"weapons":{"rifle":{"damage":60}}}');
  });
  const saved = await editor.save({
    revision: initial.revision,
    changes: [{ section: "weapons", id: "rifle", path: ["damage"], value: 40 }],
  });
  expect(saved.documents.find((file) => file.path === "fixtures/game.json")?.value).toMatchObject({
    weapons: { rifle: { damage: 60 } },
  });
  expect(saved.catalog.weapons).toMatchObject({ rifle: { damage: 60 } });
  expect(saved).toEqual(await editor.snapshot());
});
