// @vitest-environment node
import { expect, test } from "vitest";
import { createSimClient } from "../src/battle/sim/client";

test("a direct authority's terminal error rejects ready before its transport closes", async () => {
  const client = createSimClient({ scenario: "{", seed: 1, side: "blue", transport: "direct" });
  try {
    client.pause();
    expect(client.paused).toBe(true);
    client.resume();
    expect(client.paused).toBe(false);
    await expect(client.ready).rejects.toThrow();
    expect(client.status).toBe("failed");
  } finally {
    client.dispose();
  }
}, 1000);
