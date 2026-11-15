import type { PreviewQuery, UpBatch } from "../../../../packages/multiplayer/src/wire.ts";

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
        typeof item.order.kind === "string",
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
