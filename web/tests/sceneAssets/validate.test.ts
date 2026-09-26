// @vitest-environment node
// One golden failure per finding code: each case breaks exactly one rule of a
// valid synthetic asset and must produce that code. The valid assets produce
// none, so every code below is caused by its mutation alone.
import { expect, test } from "vitest";
import { buildClips } from "@packages/scene-assets/src/build.ts";
import { importScene } from "@packages/scene-assets/src/scene.ts";
import {
  FINDING_CODES,
  type AppearanceEntry,
  type Finding,
  type FindingCode,
  type ProvenanceEntry,
  type SkeletonEntry,
} from "@packages/scene-assets/src/schema.ts";
import { validateAppearance, validateSkeleton } from "@packages/scene-assets/src/validate.ts";
import {
  AUTHORITY,
  SKELETON_ENTRY,
  SOLDIER_CLIPS,
  TOLERANCES,
  buildingGlb,
  soldierGlb,
  tankGlb,
  treeGlb,
  truckGlb,
  type SoldierOptions,
  type TankOptions,
} from "./synthetic";

const context = { authority: AUTHORITY, tolerances: TOLERANCES };
const rig = (() => {
  const { scene } = importScene(soldierGlb(), "rig.glb", 90);
  return {
    clips: buildClips(scene!, "rig.glb", "test-rig", 30, SKELETON_ENTRY.clips).built!,
    aim_reference: SKELETON_ENTRY.aim_reference,
  };
})();

async function soldier(
  options: SoldierOptions = {},
  entry: Partial<AppearanceEntry> = {},
  bytes?: Uint8Array,
) {
  const source = "soldier.glb";
  return (
    await validateAppearance(
      {
        name: "soldier",
        entry: { unit: "rifle", source, basis_yaw_deg: 90, skeleton: "test-rig", ...entry },
        files: { [source]: bytes ?? soldierGlb({ animated: false, ...options }) },
        skeleton: rig,
      },
      context,
    )
  ).findings;
}

async function tank(
  options: TankOptions = {},
  entry: Partial<AppearanceEntry> = {},
  bytes?: Uint8Array,
  provenance?: ProvenanceEntry[],
) {
  const source = "tank.glb";
  return (
    await validateAppearance(
      {
        name: "tank",
        entry: { unit: "tank", source, basis_yaw_deg: 0, ...entry },
        files: { [source]: bytes ?? tankGlb(options) },
      },
      { ...context, provenance },
    )
  ).findings;
}

async function truck(options: Parameters<typeof truckGlb>[0] = {}) {
  return (
    await validateAppearance(
      {
        name: "truck",
        entry: { unit: "supply", source: "truck.glb", basis_yaw_deg: 0 },
        files: { "truck.glb": truckGlb(options) },
      },
      context,
    )
  ).findings;
}

async function house(states: Record<string, Uint8Array>) {
  const files = Object.fromEntries(
    Object.entries(states).map(([state, bytes]) => [`${state}.glb`, bytes]),
  );
  const entry: AppearanceEntry = {
    unit: "building",
    states: Object.fromEntries(Object.keys(states).map((s) => [s, `${s}.glb`])),
    basis_yaw_deg: 0,
  };
  return (await validateAppearance({ name: "house", entry, files }, context)).findings;
}

/** A scenery appearance: `kind` names a `SCENERY_KINDS` row. */
async function scenery(kind: string | undefined, states: Record<string, Uint8Array>) {
  const files = Object.fromEntries(
    Object.entries(states).map(([state, bytes]) => [`${state}.glb`, bytes]),
  );
  const entry: AppearanceEntry = {
    unit: "scenery",
    scenery: kind,
    states: Object.fromEntries(Object.keys(states).map((s) => [s, `${s}.glb`])),
    basis_yaw_deg: 0,
  };
  return validateAppearance({ name: kind ?? "scenery", entry, files }, context);
}

async function skeleton(entry: Partial<SkeletonEntry>, bytes = soldierGlb()) {
  return (await validateSkeleton("test-rig", { ...SKELETON_ENTRY, ...entry }, bytes, {})).findings;
}

const withJson = (bytes: Uint8Array, edit: (json: Record<string, unknown>) => void): Uint8Array => {
  const dv = new DataView(bytes.buffer, bytes.byteOffset);
  const length = dv.getUint32(12, true);
  const json = JSON.parse(new TextDecoder().decode(bytes.subarray(20, 20 + length)));
  edit(json);
  const text = new TextEncoder().encode(JSON.stringify(json));
  const padded = Math.ceil(text.length / 4) * 4;
  const binChunk = bytes.subarray(20 + length);
  const out = new Uint8Array(20 + padded + binChunk.length);
  out.set(bytes.subarray(0, 12));
  const odv = new DataView(out.buffer);
  odv.setUint32(8, out.length, true);
  odv.setUint32(12, padded, true);
  odv.setUint32(16, 0x4e4f534a, true);
  out.set(text, 20);
  out.fill(0x20, 20 + text.length, 20 + padded);
  out.set(binChunk, 20 + padded);
  return out;
};

const LFS_POINTER = new TextEncoder().encode(
  "version https://git-lfs.github.com/spec/v1\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\nsize 12345\n",
);

const GOLDEN: Record<FindingCode, () => Promise<Finding[]>> = {
  "structure.unreadable": () => tank({}, {}, new TextEncoder().encode("not a glb at all")),
  "structure.lfs_pointer": () => tank({}, {}, LFS_POINTER),
  "structure.external_buffer": () =>
    tank(
      {},
      {},
      withJson(tankGlb(), (j) => ((j.buffers as { uri?: string }[])[0].uri = "tank.bin")),
    ),
  "structure.unsupported": () =>
    tank(
      {},
      {},
      withJson(tankGlb(), (j) => (j.extensionsRequired = ["KHR_draco_mesh_compression"])),
    ),
  "structure.root": () => tank({ twoRoots: true }),
  "structure.skin_count": () => tank({ skinned: true }),
  "structure.unskinned_mesh": () => soldier({ looseMesh: true }),
  "structure.weights": () => soldier({ fiveWeights: true }),
  "structure.attributes": () => soldier({ noNormals: true }),
  "structure.scale": () => soldier({ stretch: true }),
  "structure.tier_count": () => tank({ lods: "none" }),
  "structure.tier_order": () => tank({ lods: "inverted" }),
  "structure.skeleton": () => soldier({ extraJoint: true }),
  "structure.loop_flags": () =>
    skeleton({ clips: { ...SKELETON_ENTRY.clips, walk: {} as { loop: boolean } } }),
  "structure.clips": () =>
    skeleton({}, soldierGlb({ clips: SOLDIER_CLIPS.filter((c) => c !== "prone_pinned") })).then(
      (f) => f,
    ),
  "structure.states": () => house({ intact: buildingGlb(6) }),
  "structure.scenery_kind": async () =>
    (await scenery("gazebo", { default: buildingGlb(3) })).findings,
  "structure.texture": () =>
    tank(
      {},
      {},
      withJson(tankGlb(), (j) => (j.images = [{ bufferView: 0, mimeType: "image/png" }])),
    ),
  "basis.ground": () => soldier({ lift: 0.1 }),
  "basis.forward": () => tank({}, { basis_yaw_deg: 180 }),
  "basis.up": () => tank({ flip: true }),
  "fit.soldier_height": () => soldier({ top: 1.9 }),
  "fit.eye": () => soldier({ eyeY: 1.3 }),
  "fit.muzzle": () => soldier({ muzzleY: 1.0 }),
  "fit.hull_extents": () => tank({ hullHalfY: 2.2 }),
  "fit.tank_muzzle": () => tank({ muzzleX: 5.9 }),
  "fit.muzzle_arc": () => tank({ turretX: -1 }),
  "fit.canopy": async () => (await scenery("tree", { summer: treeGlb(12.5) })).findings,
  "nodes.missing": () => tank({ omit: "hmg_muzzle" }),
  "nodes.hierarchy": () => tank({ muzzleUnderTurret: true }),
  "nodes.duplicate": () => tank({ duplicateWheel: true }),
  "nodes.track_properties": () => tank({ noTrackProperties: true }),
  "nodes.deploy_motion": () => truck({ noDeployMotion: true }),
  "provenance.unlisted": () => tank({}, {}, undefined, []),
  "provenance.licence": async () => {
    const bytes = tankGlb();
    const digest = Buffer.from(
      await crypto.subtle.digest("SHA-256", bytes as Uint8Array<ArrayBuffer>),
    ).toString("hex");
    return tank({}, {}, bytes, [
      { path: "tank.glb", sha256: digest, licence: "CC-BY-NC-4.0", accepted_by: "nobody" },
    ]);
  },
};

test("the valid synthetic assets produce no findings at all", async () => {
  const all = [
    ...(await soldier()),
    ...(await tank()),
    ...(await truck()),
    ...(await house({ intact: buildingGlb(6), ruin: buildingGlb(2) })),
    ...(await scenery("wall", { default: buildingGlb(1.2) })).findings,
    ...(await scenery("tree", { summer: treeGlb() })).findings,
    ...(await scenery("hedgerow", { summer: treeGlb(3) })).findings,
    ...(await skeleton({})),
  ];
  expect(all).toEqual([]);
});

test("every finding code has a golden failure", () => {
  expect(Object.keys(GOLDEN).sort()).toEqual([...FINDING_CODES].sort());
});

for (const code of FINDING_CODES)
  test(`golden failure: ${code}`, async () => {
    const findings = await GOLDEN[code]();
    const hit = findings.find((f) => f.code === code);
    expect(hit, JSON.stringify(findings, null, 1)).toBeDefined();
    expect(hit!.fix.length).toBeGreaterThan(0);
    expect(hit!.severity).toBe(code === "structure.texture" ? "warning" : "error");
  });

test("a tree must stand inside the simulation's canopy; a hedgerow need not", async () => {
  // The crown top is measured against the lowest canopy the fixture's forests have.
  const codes = async (kind: string, height: number) =>
    (await scenery(kind, { summer: treeGlb(height) })).findings.map((f) => f.code);
  expect(await codes("tree", 12.5)).toEqual(["fit.canopy"]);
  expect(await codes("tree", 11.9)).toEqual([]);
  expect(await codes("hedgerow", 14)).toEqual([]);
});

test("an LFS pointer's finding prints the exact pull command", async () => {
  const [finding] = await tank({}, {}, LFS_POINTER);
  expect(finding.fix).toBe('run: git lfs pull --include="tank.glb"');
});

test("the tank muzzle is measured against the fixture's rule, not a built-in reach", async () => {
  const realistic = tankGlb({ muzzleX: 5.9 });
  expect((await tank({}, {}, realistic)).map((f) => f.code)).toContain("fit.tank_muzzle");
  const retuned = await validateAppearance(
    {
      name: "tank",
      entry: { unit: "tank", source: "t.glb", basis_yaw_deg: 0 },
      files: { "t.glb": realistic },
    },
    { tolerances: TOLERANCES, authority: { ...AUTHORITY, tank_muzzle_local_m: [5.9, 0, 2] } },
  );
  expect(retuned.findings).toEqual([]);
});

test("a supply truck facing backwards and missing a mast stage is caught", async () => {
  expect((await truck({ reversed: true })).map((f) => f.code)).toContain("basis.forward");
  const missing = await truck({ omit: "deploy_mast_3" });
  expect(missing.map((f) => f.message).join("\n")).toContain('no "deploy_mast_3" node');
  const noPad = await truck({ omit: "deploy_leg_RL_pad" });
  expect(noPad.map((f) => f.message).join("\n")).toContain('no "deploy_leg_RL_pad" node');
});

test("a deployed pad that stops short of the ground is caught, by name", async () => {
  const short = await truck({ jackDrop: 0.05 });
  const found = short.filter((f) => f.code === "nodes.deploy_motion");
  expect(found.map((f) => f.message.match(/"(deploy_leg_\w+)"/)?.[1]).sort()).toEqual([
    "deploy_leg_FL_pad",
    "deploy_leg_FR_pad",
    "deploy_leg_RL_pad",
    "deploy_leg_RR_pad",
  ]);
});

test("a scenery kind needs its own states, not a building's, and builds a static bundle", async () => {
  const wall = await scenery("wall", { default: buildingGlb(1.2) });
  expect(wall.bundle?.kind).toBe("static");
  const missing = await scenery("wall", { intact: buildingGlb(1.2) });
  expect(missing.findings.map((f) => f.message).join("\n")).toContain('no "default" state');
});

test("catalog tolerances, not the rules, admit art that sits outside the default", async () => {
  expect(
    (await tank({ hullHalfY: 2.2 }, { tolerances: { hull_extent_m: 0.5 } })).map((f) => f.code),
  ).toEqual([]);
});
