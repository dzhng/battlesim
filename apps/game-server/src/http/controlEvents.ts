import {
  KEYFRAME,
  jsonRecord,
  recordSource,
} from "../../../../packages/multiplayer/src/stream/index.ts";
import type { BattleEnd } from "../battleHost/index.ts";

export type StreamPresence = { player: string; connection: string };
export type ControlEvent =
  | { kind: "snapshot"; streams: StreamPresence[]; orders: Record<string, number> }
  | { kind: "stream"; stream: StreamPresence; open: boolean }
  | { kind: "order"; player: string }
  | { kind: "ended"; end: BattleEnd };

/** A snapshot reconciles connections and order activity after a real ring gap. */
export function controlEvents() {
  const streams = new Map<string, StreamPresence>();
  const orders: Record<string, number> = {};
  let ended = false;
  const source = recordSource(
    600 * 1024,
    () => ({}),
    () => append({ kind: "snapshot", streams: [...streams.values()], orders }, KEYFRAME),
  );
  function append(event: ControlEvent, flags = 0) {
    if (!ended) source.append("state", flags, jsonRecord("state", event).payload);
  }
  return {
    source,
    stream(stream: StreamPresence, open: boolean) {
      if (open) streams.set(stream.connection, stream);
      else streams.delete(stream.connection);
      append({ kind: "stream", stream, open });
    },
    order(player: string) {
      orders[player] = (orders[player] ?? 0) + 1;
      append({ kind: "order", player });
    },
    end(end: BattleEnd) {
      // An exception may quote a credential supplied as invalid input.
      append({
        kind: "ended",
        end: end.kind === "finished" ? end : { kind: "crashed", message: "battle server failed" },
      });
      ended = true;
      source.end(end.kind);
    },
  };
}
