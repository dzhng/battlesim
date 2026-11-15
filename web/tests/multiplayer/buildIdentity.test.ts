// @vitest-environment node
import { createHash } from "node:crypto";
import { afterEach, expect, test, vi } from "vitest";
import { buildIdentity, simFingerprint } from "../../../apps/battle-lab/src/buildIdentity";

afterEach(() => vi.unstubAllGlobals());
test("the page sends a full hash of served bytes while the menu keeps its short fingerprint", async () => {
  const bytes = new Uint8Array([0, 97, 115, 109, 1, 2, 3]);
  vi.stubGlobal("fetch", async () => new Response(bytes));
  const identity = await buildIdentity();
  const digest = createHash("sha256").update(bytes).digest("hex");
  expect(identity.sim).toBe(digest);
  expect(identity.commit).toMatch(/^[a-f0-9]{40}$/);
  expect(await simFingerprint()).toBe(digest.slice(0, 8));
});

test("a missing simulation cannot produce an identity; the menu keeps its failed-load marker", async () => {
  vi.stubGlobal("fetch", async () => new Response("missing", { status: 404 }));
  await expect(buildIdentity()).rejects.toThrow("simulation build could not be checked");
  expect(await simFingerprint()).toBe("unknown");
});
