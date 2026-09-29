// @vitest-environment node
import { expect, test, vi } from "vitest";
import { createSimClient } from "../src/battle/sim/client";

test("a direct authority's terminal error rejects ready before its transport closes", async () => {
  const client = createSimClient({ scenario: "{", seed: 1, side: "blue", transport: "direct" });
  const onStatus = vi.fn();
  client.onStatus(onStatus);
  try {
    client.pause();
    expect(client.paused).toBe(true);
    client.resume();
    expect(client.paused).toBe(false);
    await expect(client.ready).rejects.toThrow();
    expect(onStatus).toHaveBeenLastCalledWith("failed", false);
  } finally {
    client.dispose();
  }
}, 1000);
