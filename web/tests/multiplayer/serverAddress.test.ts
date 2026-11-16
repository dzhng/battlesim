// @vitest-environment node
import { afterEach, expect, test, vi } from "vitest";
import { serverAddress } from "../../../apps/battle-lab/src/online/server";
// Discovery's committed data is a boundary, not a live deployment to pin in a test.
vi.mock("../../../apps/battle-lab/src/online/server.json", () => ({
  default: { address: "https://authority.invalid" },
}));
afterEach(() => vi.unstubAllEnvs());

test("production localhost pages use the local server and ignore overrides", () => {
  vi.stubEnv("DEV", false);
  for (const hostname of ["localhost", "127.0.0.1", "[::1]"])
    expect(serverAddress({ hostname, search: "?server=https://other.invalid" })).toBe(
      "http://127.0.0.1:8787",
    );
  expect(serverAddress({ hostname: "page.invalid", search: "?server=https://other.invalid" })).toBe(
    "https://authority.invalid",
  );
});

test("a development page can override discovery without changing committed data", () => {
  vi.stubEnv("DEV", true);
  expect(serverAddress({ hostname: "localhost", search: "?server=http://127.0.0.1:54321/" })).toBe(
    "http://127.0.0.1:54321",
  );
});
