// @vitest-environment node
import { afterEach, expect, test, vi } from "vitest";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetModules();
});

test("the browser credential and edited callsign survive a fresh page module", async () => {
  const values = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
  });
  const first = await import("../../../apps/battle-lab/src/online/identity");
  const id = first.browserId();
  expect(/^[a-f0-9]{32}$/.test(id)).toBe(true);
  expect(first.browserId() === id).toBe(true);
  first.setCallsign("SCOUT-42");
  vi.resetModules();
  const reloaded = await import("../../../apps/battle-lab/src/online/identity");
  expect(reloaded.browserId() === id).toBe(true);
  expect(reloaded.callsign()).toBe("SCOUT-42");
});

test.each(["absent", "throws", "full", "garbage"])(
  "%s storage still gives a usable identity",
  async (kind) => {
    const unavailable = () => {
      throw new Error("storage unavailable");
    };
    vi.stubGlobal(
      "localStorage",
      kind === "absent"
        ? undefined
        : {
            getItem:
              kind === "throws" ? unavailable : () => (kind === "garbage" ? "☃\u0000#" : null),
            setItem: unavailable,
          },
    );
    const identity = await import("../../../apps/battle-lab/src/online/identity");
    const id = identity.browserId();
    expect(/^[a-f0-9]{32}$/.test(id)).toBe(true);
    expect(identity.browserId() === id).toBe(true);
    const name = identity.callsign();
    expect(name.length > 0).toBe(true);
    const { cleanCallsign } = await import("../../../packages/multiplayer/src/callsign");
    expect(cleanCallsign(name)).toBe(name);
    expect(identity.callsign()).toBe(name);
    expect(identity.setCallsign("SCOUT-42")).toBe("SCOUT-42");
    expect(identity.callsign()).toBe("SCOUT-42");
    expect(identity.setCallsign("☃").length > 0).toBe(true);
  },
);

test("a throwing storage property cannot prevent identity creation", async () => {
  const original = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    get() {
      throw new Error("denied");
    },
  });
  try {
    const identity = await import("../../../apps/battle-lab/src/online/identity");
    expect(/^[a-f0-9]{32}$/.test(identity.browserId())).toBe(true);
    expect(identity.callsign().length > 0).toBe(true);
    expect(identity.setCallsign("SCOUT")).toBe("SCOUT");
  } finally {
    if (original) Object.defineProperty(globalThis, "localStorage", original);
    else Reflect.deleteProperty(globalThis, "localStorage");
  }
});
