// @vitest-environment node
// Which appearance the battle draws for a unit on a side: a hull type's one
// model, a soldier's own member of his slot's soldier kind's set, recoloured
// per side by the tint mask its materials carry.
import { expect, test } from "vitest";
import {
  AppearanceCatalog,
  type SideFactions,
} from "@packages/scene-assets/src/appearanceCatalog.ts";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import { importScene } from "@packages/scene-assets/src/scene.ts";
import type { Catalog } from "@packages/scene-assets/src/schema.ts";
import { UnitCatalog } from "@packages/scene-assets/src/units.ts";
import { createPoseDriver } from "@apps/battle-lab/src/poseFeed";
import {
  AUTHORITY,
  GltfBuilder,
  soldierGlb,
  syntheticUnits,
  tankMounts,
  testCatalog,
  testSources,
} from "./synthetic";

async function install(catalog: Catalog = testCatalog(), wearing?: ReadonlySet<string>) {
  const sources = testSources();
  const result = await bakeCatalog(catalog, async (path) => sources[path], {
    authority: AUTHORITY,
  });
  expect(result.ok).toBe(true);
  const files = new Map<string, Uint8Array>([
    ["catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime))],
    ...result.files,
  ]);
  return new AppearanceLibrary(memoryFetch(files, "/assets/")).load("/assets/", wearing);
}

/** The synthetic units with the rifleman soldier kind wearing `set`, and a
 *  second soldier kind `gunner` in the squad's second slot wearing `gunner`. */
function units(set: string[], gunner: string[] = set): UnitCatalog {
  const view = syntheticUnits().view;
  return new UnitCatalog({
    ...view,
    soldiers: {
      rifleman: { ...view.soldiers.rifleman, appearance: set },
      gunner: { ...view.soldiers.rifleman, appearance: gunner },
    },
    units: view.units.map((t) =>
      t.id === "rifle" ? { ...t, body: { squad: { slots: ["rifleman", "gunner"] } } } : t,
    ),
  });
}

test("a hull type draws its appearance and a squad its soldiers', tinted by the side", async () => {
  const catalog = new AppearanceCatalog(await install(), syntheticUnits());
  const sides = testCatalog().sides;
  expect(catalog.resolve("rifle", "blue")).toEqual({ appearance: "rifleman", tint: sides.blue });
  expect(catalog.resolve("rifle", "red")).toEqual({ appearance: "rifleman", tint: sides.red });
  expect(catalog.resolve("tank", "red")?.appearance).toBe("tank");
  expect(catalog.resolve("supply", "red")?.appearance).toBe("truck");
  // A type the catalog lacks, or a slot the squad lacks, draws nothing.
  expect(catalog.resolve("recon", "blue")).toBeNull();
  expect(catalog.resolve("rifle", "blue", 0, 7)).toBeNull();
});

test("a soldier kind's set is picked by soldier id, so consecutive soldiers differ", async () => {
  const base = testCatalog();
  const installed = await install({
    ...base,
    appearances: {
      ...base.appearances,
      rifleman_b: { ...base.appearances.rifleman },
      rifleman_c: { ...base.appearances.rifleman },
    },
  });
  const catalog = new AppearanceCatalog(
    installed,
    units(["rifleman", "rifleman_b", "rifleman_c"], ["rifleman_c"]),
  );
  const worn = [0, 1, 2, 3, 4, 5].map((id) => catalog.resolve("rifle", "blue", id)?.appearance);
  expect(worn).toEqual([
    "rifleman",
    "rifleman_b",
    "rifleman_c",
    "rifleman",
    "rifleman_b",
    "rifleman_c",
  ]);
  // The side changes only the tint, never the member.
  expect(catalog.resolve("rifle", "red", 4)).toEqual({
    appearance: "rifleman_b",
    tint: base.sides.red,
  });
  // The second slot is a different soldier kind, with his own set.
  expect(catalog.resolve("rifle", "blue", 4, 1)?.appearance).toBe("rifleman_c");
});

test("a soldier wears his faction's look of his appearance, on whichever side it fights", async () => {
  const base = testCatalog();
  const installed = await install(
    {
      ...base,
      appearances: {
        ...base.appearances,
        rifleman: { ...base.appearances.rifleman, factions: { eastern: "rifleman_eastern" } },
        rifleman_eastern: { ...base.appearances.rifleman },
        rifleman_unworn: { ...base.appearances.rifleman },
      },
    },
    syntheticUnits().appearances,
  );
  // A page loading only what its units wear takes their faction looks too.
  expect(installed.appearances.has("rifleman_eastern")).toBe(true);
  expect(installed.appearances.has("rifleman_unworn")).toBe(false);
  const drawn = (factions?: SideFactions) =>
    new AppearanceCatalog(installed, syntheticUnits(), factions);
  const usVsEastern = drawn({ blue: "us", red: "eastern" });
  expect(usVsEastern.resolve("rifle", "blue")?.appearance).toBe("rifleman");
  expect(usVsEastern.resolve("rifle", "red")).toEqual({
    appearance: "rifleman_eastern",
    tint: base.sides.red,
  });
  // The faction's look, not the side's: an Eastern player sees his own in it.
  expect(drawn({ blue: "eastern", red: "us" }).resolve("rifle", "blue")?.appearance).toBe(
    "rifleman_eastern",
  );
  // A faction with no look of its own, and a battle of no factions, wear the appearance.
  expect(drawn({ blue: "europe", red: "us" }).resolve("rifle", "blue")?.appearance).toBe(
    "rifleman",
  );
  expect(drawn().resolve("rifle", "red")?.appearance).toBe("rifleman");
});

test("an operated weapon selects its carrier's appearance without changing other soldiers", async () => {
  const base = testCatalog();
  const installed = await install({
    ...base,
    appearances: {
      ...base.appearances,
      launcher: { ...base.appearances.rifleman },
      carried: { ...base.appearances.rifleman },
    },
  });
  const view = syntheticUnits().view;
  const armed = new UnitCatalog({
    ...view,
    units: view.units.map((t) =>
      t.id === "rifle"
        ? {
            ...t,
            mounts: [
              {
                ...tankMounts()[0],
                turret: false,
                operator_appearance: { active: ["launcher"], carried: ["carried"] },
              },
            ],
          }
        : t,
    ),
  });
  const catalog = new AppearanceCatalog(installed, armed);
  expect(catalog.resolve("rifle", "blue", 7, 0, 0, 0)?.appearance).toBe("launcher");
  expect(catalog.resolve("rifle", "blue", 7, 0, 0, null)?.appearance).toBe("carried");
  expect(catalog.resolve("rifle", "blue", 8, 0, null)?.appearance).toBe("rifleman");
  // After handoff, the model follows the new operator, not his original slot.
  expect(catalog.resolve("rifle", "blue", 7, 0, null)?.appearance).toBe("rifleman");
  expect(catalog.resolve("rifle", "blue", 8, 0, 0, 0)?.appearance).toBe("launcher");
});

test("variants share a hold family but a squad can mix rifle and launcher holds", async () => {
  const base = testCatalog();
  const catalog: Catalog = {
    ...base,
    skeletons: { ...base.skeletons, "test-rig-2": base.skeletons["test-rig"] },
    appearances: {
      ...base.appearances,
      rifleman_b: { ...base.appearances.rifleman, skeleton: "test-rig-2" },
    },
  };
  const installed = await install(catalog);
  // Within one soldier kind's set.
  expect(() => new AppearanceCatalog(installed, units(["rifleman", "rifleman_b"]))).toThrow(
    /soldier kind rifleman's appearances use skeletons test-rig and test-rig-2/,
  );
  // Across the squad's soldier kinds.
  expect(() => new AppearanceCatalog(installed, units(["rifleman"], ["rifleman_b"]))).not.toThrow();
});

test("mixed soldier holds advance using their own installed clip duration", async () => {
  const base = testCatalog();
  const installed = await install({
    ...base,
    skeletons: { ...base.skeletons, "test-rig-2": base.skeletons["test-rig"] },
    appearances: {
      ...base.appearances,
      rifleman_b: { ...base.appearances.rifleman, skeleton: "test-rig-2" },
    },
  });
  const slow = installed.skeletons.get("test-rig-2")!;
  installed.skeletons.set("test-rig-2", {
    ...slow,
    clips: slow.clips.map((c) => (c.name === "idle" ? { ...c, duration: 20 } : c)),
  });
  const catalog = units(["rifleman"], ["rifleman_b"]);
  const phase = (slot: number) => {
    const driver = createPoseDriver({ cover: { lean_hold_s: 0.1 } }, catalog, installed);
    const actor = {
      id: 1,
      kind: "rifle",
      side: "blue" as const,
      position: [0, 0, 0] as [number, number, number],
      yaw: 0,
      soldiers: [
        { id: 0, slot, activeMount: null, position: [0, 0, 0] as [number, number, number] },
      ],
      mounts: [],
      deployment: null,
      pinned: false,
    };
    driver.update({ time: 0, units: [actor], fallen: [] });
    const pose = driver.update({ time: 0.1, units: [actor], fallen: [] }).soldiers[0];
    expect(pose.clip).toBe("idle");
    return pose.phase;
  };
  const fastDuration = installed.skeletons
    .get("test-rig")!
    .clips.find((c) => c.name === "idle")!.duration;
  expect(phase(1)).toBeCloseTo((phase(0) * fastDuration) / 20, 8);
});

test("a material's glTF extras tint is its side-tint mask weight", () => {
  const b = new GltfBuilder();
  b.json.materials.push({ name: "cloth", pbrMetallicRoughness: {}, extras: { tint: 0.6 } });
  b.roots(b.node({ name: "part", mesh: b.box([0, 0, 0], [1, 1, 1]) }));
  const { scene } = importScene(b.glb(), "tinted.glb", 0);
  expect(scene!.materials.map((m) => [m.name, m.tint])).toEqual([
    ["paint", 0],
    ["cloth", 0.6],
  ]);
  expect(importScene(soldierGlb(), "x.glb", 90).scene!.materials.every((m) => m.tint === 0)).toBe(
    true,
  );
});
