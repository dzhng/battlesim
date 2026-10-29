import type { Json } from "./protocol";

export type FieldKind =
  | "number"
  | "boolean"
  | "enum"
  | "reference"
  | "strings"
  | "vector"
  | "ammo"
  | "object"
  | "mounts";
export type Conversion = "percent" | "cadence" | "dimensions";
export const CONVERSION_CAPTIONS: Record<Conversion, string> = {
  percent: "Stored fraction · 1 = 100%",
  cadence: "Shot interval · seconds",
  dimensions: "Half dimensions · metres",
};
export interface GameplayField {
  path: string[];
  label: string;
  group: string;
  explanation: string;
  kind: FieldKind;
  unit?: string;
  conversion?: Conversion;
  choices?: readonly string[];
  references?: "weapons" | "soldiers" | "props" | "roles" | "parts" | "mounts";
  optional?: boolean;
  integer?: boolean;
  min?: number;
  max?: number;
  /** Enabled value; numbers also reset to this neutral value, omitted by native serialization. */
  initial?: Json;
}

export const field = (
  path: string,
  label: string,
  group: string,
  explanation: string,
  extra: Partial<GameplayField> = {},
): GameplayField => ({
  path: path.split("."),
  label,
  group,
  explanation,
  kind: "number",
  min: 0,
  ...extra,
});
export type ParsedField = { value: Json; error?: never } | { error: string; value?: never };
