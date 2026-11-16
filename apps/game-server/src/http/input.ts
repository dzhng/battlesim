import type {
  PreviewQuery,
  RoomAction,
  UpBatch,
} from "../../../../packages/multiplayer/src/wire.ts";

/** Keep a whole army selectable while bounding unit-id arrays copied into the battle's history. */
export const MAX_ORDER_UNITS = 4096;
/** Bound each order retained by the battle, including oversized strings and extra JSON fields. */
export const MAX_ORDER_BYTES = 32 * 1024;

const object = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === "object" && !Array.isArray(value);
const natural = (value: unknown): value is number =>
  Number.isSafeInteger(value) && (value as number) >= 0;

/** Validate the envelope here; the simulation owns the meaning of an order.
 * Claimed side/player/seat fields are deliberately never read. */
export function isBatch(value: unknown): value is UpBatch {
  return (
    object(value) &&
    natural(value.seq) &&
    value.seq > 0 &&
    Array.isArray(value.items) &&
    value.items.every(
      (item) =>
        object(item) &&
        natural(item.id) &&
        typeof item.queued === "boolean" &&
        object(item.order) &&
        typeof item.order.kind === "string" &&
        [item.order.units, item.order.squads].reduce<number>(
          (count, ids) => count + (Array.isArray(ids) ? ids.length : 0),
          0,
        ) <= MAX_ORDER_UNITS &&
        Buffer.byteLength(JSON.stringify(item.order), "utf8") <= MAX_ORDER_BYTES,
    )
  );
}
export function isPreview(value: unknown): value is PreviewQuery {
  if (!object(value)) return false;
  if (value.kind === "purchase")
    return (
      typeof value.variant === "string" &&
      Array.isArray(value.destination) &&
      value.destination.length === 2 &&
      value.destination.every((n) => typeof n === "number" && Number.isFinite(n))
    );
  if (value.kind === "move")
    return (
      object(value.move) &&
      Array.isArray(value.move.units) &&
      value.move.units.every(natural) &&
      Array.isArray(value.move.goal) &&
      value.move.goal.length === 2 &&
      value.move.goal.every((n) => typeof n === "number" && Number.isFinite(n))
    );
  return (
    value.kind === "building" &&
    object(value.building) &&
    Array.isArray(value.building.units) &&
    value.building.units.every(natural) &&
    natural(value.building.building)
  );
}

/** Validate action envelopes only; the room model validates settings and permissions. */
export function isRoomAction(value: unknown): value is RoomAction {
  if (!object(value)) return false;
  switch (value.kind) {
    case "sit":
    case "remove_seat":
      return typeof value.seat === "string";
    case "ready":
      return typeof value.ready === "boolean";
    case "leave":
    case "start":
      return true;
    case "add_seat":
      return value.side === "blue" || value.side === "red";
    case "set_seat":
      return typeof value.seat === "string" && (value.to === "open" || value.to === "ai");
    case "settings":
      return object(value.patch);
    default:
      return false;
  }
}
