import type { Documents, Field, Json, JsonObject } from "./protocol";
import { valueAtPath } from "../../fixture-publication/changes";

export function fieldValue(documents: Documents, field: Field): Json | undefined {
  return valueAtPath(documents[field.document], field.path) as Json | undefined;
}

export function editField(documents: Documents, field: Field, value: Json): Documents {
  const edited = structuredClone(documents);
  let node: Json = edited[field.document];
  for (const key of field.path.slice(0, -1))
    node = Array.isArray(node) ? node[Number(key)] : (node as JsonObject)[key];
  const key = field.path.at(-1)!;
  if (Array.isArray(node)) node[Number(key)] = value;
  else (node as JsonObject)[key] = value;
  return edited;
}

export function fieldText(value: Json | undefined, field: Field): string {
  if (value === undefined) return "—";
  if (typeof value === "number" && field.unit === "%")
    return String(Number((100 * value).toPrecision(8)));
  return typeof value === "object" ? JSON.stringify(value) : String(value);
}

/** This parses unfinished input; Rust still decides whether the value is legal. */
export function parseField(text: string, current: Json | undefined, field: Field): Json {
  if (typeof current === "string") return text;
  if (typeof current === "number") {
    if (!text.trim()) throw new Error("Enter a number");
    const value = Number(text);
    if (!Number.isFinite(value)) throw new Error("Finish the number");
    return field.unit === "%" ? value / 100 : value;
  }
  return JSON.parse(text) as Json;
}

export const humanize = (text: string): string =>
  text
    .replaceAll("_", " ")
    .replaceAll(".", " · ")
    .replace(/^\w/, (letter) => letter.toUpperCase());
