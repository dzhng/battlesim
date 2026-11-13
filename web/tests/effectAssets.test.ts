import { afterEach, expect, test, vi } from "vitest";
import sheets from "../../assets/effect-sheets.json";
import { loadEffectAtlasLayers } from "@web/battle/render/effectAssets";

afterEach(() => vi.unstubAllGlobals());

test("an effect flipbook still an LFS pointer names the exact pull", async () => {
  const pointer = `version https://git-lfs.github.com/spec/v1\noid sha256:${"a".repeat(64)}\nsize 999\n`;
  vi.stubGlobal("fetch", async () => new Response(pointer));
  await expect(loadEffectAtlasLayers()).rejects.toThrow(
    `git lfs pull --include="assets/third-party/effects/${sheets[0]}"`,
  );
});
