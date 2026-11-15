import { cleanCallsign } from "../../../../packages/multiplayer/src/callsign";

function identity() {
  let id: string | undefined;
  let name: string | undefined;
  const read = (key: string): string => {
    try {
      return globalThis.localStorage?.getItem(key) ?? "";
    } catch {
      return "";
    }
  };
  const write = (key: string, value: string) => {
    try {
      globalThis.localStorage?.setItem(key, value);
    } catch {
      /* This page still owns a usable identity when storage is denied or full. */
    }
  };
  const randomName = () => {
    const words = ["VIPER", "FALCON", "RAVEN", "COBRA", "WOLF", "LYNX", "HAWK", "KITE"];
    const bytes = crypto.getRandomValues(new Uint32Array(2));
    return `${words[bytes[0] % words.length]}-${String(bytes[1] % 100).padStart(2, "0")}`;
  };
  return {
    browserId(): string {
      if (id) return id;
      const stored = read("battle.browser-id");
      id = /^[a-f0-9]{32}$/.test(stored)
        ? stored
        : Array.from(crypto.getRandomValues(new Uint8Array(16)), (byte) =>
            byte.toString(16).padStart(2, "0"),
          ).join("");
      write("battle.browser-id", id);
      return id;
    },
    callsign(): string {
      if (name) return name;
      name = cleanCallsign(read("battle.callsign")) || randomName();
      write("battle.callsign", name);
      return name;
    },
    setCallsign(value: string): string {
      name = cleanCallsign(value) || randomName();
      write("battle.callsign", name);
      return name;
    },
  };
}

/** One owner per loaded page, including when persistent storage is unavailable. */
export const { browserId, callsign, setCallsign } = identity();
