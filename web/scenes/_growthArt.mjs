// Fixture-derived work for the actual installed generations. Runtime files,
// rather than a roster or today's counts, define the art arriving at this gate.
import { readFile } from "node:fs/promises";
import { gunzipSync } from "node:zlib";
import { joinTextures } from "../../packages/scene-assets/src/codec.ts";

export async function growthArt(page, catalog, initialNames, names) {
  const identity = (name) => {
    const entry = catalog.appearances[name];
    if (!entry) throw new Error(`growth appearance ${name} is absent from the runtime catalog`);
    return {
      bundle: entry.bundle,
      skeleton: entry.skeleton ? catalog.skeletons[entry.skeleton] : null,
    };
  };
  const key = (id) => JSON.stringify([id.bundle, id.skeleton]);
  const generation = (names) =>
    new Map(
      names
        .filter((name) => catalog.appearances[name])
        .map((name) => {
          const id = identity(name);
          return [key(id), id];
        }),
    );
  for (const name of names.filter((name) => !initialNames.includes(name))) identity(name);
  const before = generation(initialNames);
  const after = generation(names);
  const added = new Map([...after].filter(([id]) => !before.has(id)));
  const newFiles = new Set(
    [...added.values()].flatMap((id) => [id.bundle, ...(id.skeleton ? [id.skeleton] : [])]),
  );
  const read = async (hash) =>
    gunzipSync(
      await readFile(
        new URL(`../../assets/runtime/${catalog.gzip[hash].hash}/bundle.bin`, import.meta.url),
      ),
    );
  const headers = new Map();
  const skeletons = {};
  // Release each large mesh payload after reading its small header.
  for (const [id, content] of new Map([...before, ...after])) {
    const bytes = await read(content.bundle);
    headers.set(id, JSON.parse(bytes.subarray(16, 16 + bytes.readUInt32LE(8)).toString()));
    if (content.skeleton && !skeletons[content.skeleton]) {
      const clips = joinTextures(await read(content.skeleton), []);
      skeletons[content.skeleton] = JSON.parse(
        JSON.stringify(clips, (_, value) =>
          ArrayBuffer.isView(value) ? Array.from(value) : value,
        ),
      );
    }
  }
  // Sample through the existing CPU pose owner. Compare the resulting f32
  // affine palettes, not pose-reference labels: two clips can pose identically.
  const palettes = await page.evaluate(
    async ({ entries, skeletons, poseUrl, trsUrl }) => {
      const { sampleClip, worldTransforms } = await import(poseUrl);
      const { mul } = await import(trsUrl);
      return Object.fromEntries(
        entries
          .filter(([, header]) => header.content.kind === "skinned")
          .map(([id, header, skeleton]) => {
            const body = header.content;
            const clips = skeletons[skeleton];
            return [
              id,
              [body.far_pose, ...body.corpse_poses].map((pose) => {
                const clip = clips.clips.find((clip) => clip.name === pose.clip);
                if (!clip) throw new Error(`${pose.clip}: growth pose clip missing`);
                const worlds = worldTransforms(
                  body.joints.map((joint) => joint.parent),
                  sampleClip(clips, clip, body.joints, pose.phase),
                );
                const palette = worlds.flatMap((world, i) => {
                  const matrix = mul(world, body.joints[i].inverse_bind);
                  return [0, 1, 2].flatMap((row) =>
                    [0, 4, 8, 12].map((column) => matrix[column + row]),
                  );
                });
                // Preserve signed zero just as the Rust key's f32 bytes do.
                return Array.from(new Uint32Array(new Float32Array(palette).buffer));
              }),
            ];
          }),
      );
    },
    {
      entries: [...headers].map(([id, header]) => [id, header, JSON.parse(id)[1]]),
      skeletons,
      poseUrl: `/@fs${new URL("../../packages/scene-assets/src/pose.ts", import.meta.url).pathname}`,
      trsUrl: `/@fs${new URL("../../packages/scene-assets/src/trs.ts", import.meta.url).pathname}`,
    },
  );
  const textureIds = (generation) =>
    new Set(
      [...generation.keys()].flatMap((id) => {
        const body = headers.get(id).content;
        return body.materials.flatMap((material) =>
          Object.values(material.textures ?? {}).map((index) => body.textures[index].id),
        );
      }),
    );
  // Square full mip chains: the edge determines all three CardKey shape fields.
  const shapes = (generation) =>
    ["rgba8unorm-srgb", "rgba8unorm"].map((format) => {
      const resident = textureIds(generation);
      return Math.max(
        1,
        ...[...generation.keys()].flatMap((id) =>
          headers
            .get(id)
            .content.textures.filter(
              (texture) => texture.format === format && resident.has(texture.id),
            )
            .map((texture) => texture.width),
        ),
      );
    });
  const facts = (generation) => {
    const rest = new Set();
    const cards = new Set();
    let vertexBytes = 0;
    // VERTEX_BYTES in crates/renderer-core/src/models/geometry.rs: shared GPU ABI.
    const PACKED_VERTEX_BYTES = 48;
    const meshVertices = (header, node) => {
      if (!node || typeof node !== "object") return 0;
      if (node.positions?.$view !== undefined) return header.views[node.positions.$view][2] / 3;
      return Object.values(node).reduce((sum, child) => sum + meshVertices(header, child), 0);
    };
    const shape = shapes(generation);
    for (const id of generation.keys()) {
      const header = headers.get(id);
      const body = header.content;
      let poses;
      // Skinned art packs its living mesh once. Rest facts and cards follow
      // the rendered palette, shared by rows whose sampled poses agree,
      // independently of whether those rows draw living bodies or corpses.
      if (body.kind === "skinned") {
        poses = new Set(palettes[id].map((palette) => JSON.stringify(palette)));
        for (const pose of poses) cards.add(JSON.stringify([id, pose, shape]));
      } else if (body.kind === "static") {
        poses = new Set(
          body.states.filter((state) => state.tiers.length).map((state) => state.name),
        );
      } else {
        poses = new Set(["articulated"]);
      }
      for (const pose of poses) rest.add(JSON.stringify([id, pose]));
      if (added.has(id)) vertexBytes += meshVertices(header, body) * PACKED_VERTEX_BYTES;
    }
    return { rest, cards, vertexBytes };
  };
  const old = facts(before);
  const next = facts(after);
  const difference = (next, old) => [...next].filter((key) => !old.has(key)).length;
  return {
    added: [...added.values()],
    newContents: added.size,
    newFiles: newFiles.size,
    vertexBytes: next.vertexBytes,
    restPoses: difference(next.rest, old.rest),
    textureLayers: difference(textureIds(after), textureIds(before)),
    cards: difference(next.cards, old.cards),
  };
}
