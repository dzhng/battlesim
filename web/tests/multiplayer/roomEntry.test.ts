// @vitest-environment node
// The retired entry can no longer seat a browser or create a battle.
import { expect, test } from "vitest";
import { roomHttp } from "./support/roomHttp";

test("the removed dev endpoint refuses every credential, carries transport headers, and never echoes credentials", async () => {
  const server = await roomHttp();
  try {
    for (const credential of ["short", "0123456789abcdef0123456789abcdef"]) {
      const response = await fetch(`${server.address}/dev/battle`, {
        method: "POST",
        headers: { "X-Player": credential },
      });
      expect(response.status).toBe(404);
      expect(response.headers.get("Access-Control-Allow-Origin")).toBe("*");
      expect(response.headers.get("Cache-Control")).toContain("no-store");
      expect(JSON.stringify(await response.json())).not.toContain(credential);
    }
  } finally {
    await server.close();
  }
});
