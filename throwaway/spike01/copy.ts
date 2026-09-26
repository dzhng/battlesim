// Copy the foundation's pruned closure out of ~/dev/game (read-only) into
// throwaway/spike01/port/, keeping relative layout so imports resolve unchanged.
import { mkdirSync, copyFileSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { createHash } from "node:crypto";
const GAME = "/Users/david/dev/game";
const OUT = resolve(import.meta.dir, "port");
export const FILES = [
  "packages/battle-renderer/src/world/frame.ts",
  "packages/battle-renderer/src/world/camera.ts",
  "packages/battle-renderer/src/world/environment.ts",
  "packages/battle-renderer/src/world/sky.ts",
  "packages/battle-renderer/src/world/pmrem.ts",
  "packages/battle-renderer/src/world/pmremSampling.ts",
  "packages/battle-renderer/src/world/shadow.ts",
  "packages/battle-renderer/src/world/post.ts",
  "packages/battle-renderer/src/world/textureUpload.ts",
  "packages/battle-renderer/src/shaders/physicalSky.ts",
  "packages/battle-renderer/src/shaders/pmrem.ts",
  "packages/battle-renderer/src/shaders/standardPbr.ts",
  "packages/battle-renderer/src/shaders/dfgLut.ts",
  "packages/battle-renderer/src/shaders/aerial.ts",
  "packages/battle-renderer/src/shaders/environment.ts",
  "packages/battle-renderer/src/shaders/shadow.ts",
  "packages/battle-renderer/src/shaders/post.ts",
  "packages/battle-renderer/src/shadowData.ts",
  "packages/battle-renderer/src/frameCamera.ts",
  "packages/battle-renderer/src/gpuAdmission.ts",
  "packages/battle-renderer/src/gpuScope.ts",
  "packages/battle-renderer/src/worldDepth.ts",
  "packages/game-renderer/src/battle/shadowPolicy.ts",
  "packages/game-renderer/src/battle/cascadePolicy.ts",
  "packages/game-renderer/src/environment/environment.ts",
  "packages/game-renderer/src/environment/physicalEnvironment.ts",
  "packages/game-renderer/src/environment/skyParameters.ts",
  "packages/game-renderer/src/environment/aerialParameters.ts",
  "packages/game-renderer/src/environment/postParameters.ts",
  "packages/renderer-core/src/camera3d.ts",
  "packages/renderer-core/src/mat4.ts",
  "packages/renderer-core/src/cameraUniform.ts",
  "packages/renderer-core/src/math.ts",
  "packages/renderer-core/src/depthContract.ts",
  "packages/renderer-core/src/pipelineContracts.ts",
];
if (import.meta.main) {
  for (const f of FILES) {
    const dst = resolve(OUT, f);
    mkdirSync(dirname(dst), { recursive: true });
    copyFileSync(resolve(GAME, f), dst);
    const sha = createHash("sha256").update(readFileSync(resolve(GAME, f))).digest("hex");
    console.log(sha.slice(0, 12), f);
  }
}
