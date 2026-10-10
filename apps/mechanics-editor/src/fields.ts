import type { Json, JsonObject, Section } from "./protocol";

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
export type Conversion = "percent" | "cadence" | "dimensions" | "spread";
export const CONVERSION_CAPTIONS: Record<Conversion, string> = {
  spread: "Angular scatter · milliradians",
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
  initial?: Json;
}

const field = (
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
const faces = ["front", "side", "rear", "roof"];
const weights = ["light", "medium", "heavy", "immovable"];
const mounts = (soldier: boolean): GameplayField[] => [
  field(
    "mounts",
    "Weapon mounts",
    "Weapons",
    "Physical weapons, each an id and the label its card shows. Adding a mount gives it its own firing cycle and ammunition. Inherited mounts cannot be removed; extend a common parent to omit one.",
    { kind: "mounts" },
  ),
  field(
    "mounts.*.weapons",
    "Ammunition / weapon rows",
    "Weapons",
    "Shared weapon rows available to this physical mount. Changing the assignment changes this unit; changing a linked weapon row affects all its users.",
    { kind: "strings", references: "weapons" },
  ),
  ...(soldier
    ? [
        field(
          "mounts.*.squad",
          "Default squad gun",
          "Weapons",
          "Each living carrier fires independently; these guns share the squad targeting and ammunition readout.",
          { kind: "boolean" },
        ),
        field(
          "mounts.*.special",
          "Transferable special weapon",
          "Weapons",
          "A surviving soldier can recover this weapon and its remaining rounds when its original carrier falls.",
          { kind: "boolean" },
        ),
      ]
    : [
        field(
          "mounts.*.turret",
          "Traversing mount",
          "Weapons",
          "Turns toward its target at the turret rate and fires only within the permitted bearing tolerance.",
          { kind: "boolean" },
        ),
        field(
          "mounts.*.on",
          "Carrier mount",
          "Weapons",
          "An earlier turret mount that carries this weapon. None means attached to the hull.",
          { kind: "reference", references: "mounts", optional: true },
        ),
        field(
          "mounts.*.pivot_m",
          "Pivot position",
          "Weapons",
          "Forward, left and up coordinates in the carrier frame where this mount rotates. Geometry affects flight origins and must agree with the model.",
          { kind: "vector", unit: "m", min: undefined },
        ),
        field(
          "mounts.*.muzzle_m",
          "Muzzle offset",
          "Weapons",
          "Forward, left and up from the pivot, rotated with the mount. None is a hand weapon muzzle. Model-fit checks still apply.",
          { kind: "vector", unit: "m", min: undefined, optional: true },
        ),
      ]),
];

const GAMEPLAY_FIELDS: Record<Section, GameplayField[]> = {
  units: [
    field(
      "cost",
      "Deployment cost",
      "Composition",
      "Points spent to deploy this exact unit type.",
      { integer: true, unit: "points" },
    ),
    field(
      "roles",
      "Tactical roles",
      "Composition",
      "Roles used by encounter scripts and AI to select units. Roles do not grant mechanics.",
      { kind: "strings", references: "roles" },
    ),
    field(
      "parts",
      "Upgrade parts",
      "Composition",
      "Upgrade patches applied after inheritance. A part can mask a local override; preview reports this rather than silently discarding it.",
      { kind: "strings", references: "parts", optional: true },
    ),
    field(
      "body.squad.slots",
      "Soldier slots",
      "Composition",
      "One soldier kind per member, in slot order. Repeated kinds mean multiple soldiers; their weapons come from their soldier definitions.",
      { kind: "strings", references: "soldiers" },
    ),
    field(
      "body.hull.half_extents_m",
      "Hull dimensions",
      "Body & protection",
      "Full length along heading, width and height. Stored as half-extents. Changes affect movement, collision and cover; the drawn model must still fit.",
      { kind: "vector", unit: "m", conversion: "dimensions" },
    ),
    field(
      "body.hull.eye_m",
      "Optics height",
      "Body & protection",
      "Sight height above the hull base; terrain and intervening bodies determine the resulting line of sight.",
      { unit: "m" },
    ),
    field(
      "body.hull.hp",
      "Hull health",
      "Body & protection",
      "Damage capacity of the live vehicle. Reaching zero destroys it and leaves its named wreck.",
      { unit: "HP" },
    ),
    ...faces.map((face) =>
      field(
        `body.hull.armor.${face}`,
        `${face[0].toUpperCase()}${face.slice(1)} armour`,
        "Body & protection",
        `Protection on the ${face} face, compared with a round's penetration. This is a game protection value rather than plate thickness.`,
        { unit: "protection" },
      ),
    ),
    ...faces.map((face) =>
      field(
        `body.hull.armor.ricochet.${face}`,
        `${face[0].toUpperCase()}${face.slice(1)} ricochet`,
        "Body & protection",
        `Chance that a kinetic round which fails to penetrate the ${face} face glances off. Independent of the drawn plate slope.`,
        { conversion: "percent", unit: "%", max: 1 },
      ),
    ),
    field(
      "body.hull.weight_class",
      "Body weight class",
      "Body & protection",
      "How hard the body is to shove and the cover tier it provides. A pusher must be strictly heavier; immovable bodies never move.",
      { kind: "enum", choices: weights },
    ),
    field(
      "body.hull.push_class",
      "Push strength",
      "Body & protection",
      "The vehicle can push bodies strictly lighter than this class, subject to space and collision rules.",
      { kind: "enum", choices: weights },
    ),
    field(
      "body.hull.wreck",
      "Destroyed vehicle body",
      "Body & protection",
      "Prop type left behind when this vehicle is destroyed. Its cover tier must match the live vehicle.",
      { kind: "reference", references: "props" },
    ),
    ...["foot", "tracked", "wheeled"].flatMap((mode) => [
      field(
        `mobility.${mode}.offroad_kmh`,
        "Off-road speed",
        "Movement",
        "Top speed on open ground. Surface rules interpolate between this and road speed.",
        { unit: "km/h", max: 130 },
      ),
      field(
        `mobility.${mode}.road_kmh`,
        "Road speed",
        "Movement",
        "Top speed on a full road; never slower than off-road. Acceleration and braking remain controlled by the shared drive rules.",
        { unit: "km/h", max: 130 },
      ),
      ...(mode === "foot"
        ? []
        : [
            field(
              `mobility.${mode}.turn_deg_s`,
              "Turning rate",
              "Movement",
              "Maximum hull yaw rate. Tracks can pivot in place; wheels follow their minimum turn radius.",
              { unit: "°/s" },
            ),
            field(
              `mobility.${mode}.reverse_fraction`,
              "Reverse speed",
              "Movement",
              "Reverse top speed as a percentage of the forward speed.",
              { conversion: "percent", unit: "%", max: 1 },
            ),
          ]),
      ...(mode === "wheeled"
        ? [
            field(
              `mobility.${mode}.turning_radius_m`,
              "Minimum turn radius",
              "Movement",
              "Smallest circle the wheeled vehicle can drive; also constrains speed through bends.",
              { unit: "m" },
            ),
          ]
        : []),
    ]),
    field(
      "mobility.air.cruise_kmh",
      "Cruise speed",
      "Movement",
      "An aircraft's one speed: it flies over every surface alike, easing in and out with the shared drive acceleration time.",
      { unit: "km/h", max: 320 },
    ),
    field(
      "mobility.air.turn_deg_s",
      "Turning rate",
      "Movement",
      "How fast the airframe turns, on the spot when hovering or to bring a fixed gun to bear while flying.",
      { unit: "°/s" },
    ),
    field(
      "mobility.air.climb_mps",
      "Climb rate",
      "Movement",
      "How fast it climbs over roofs and sinks to a low hover; the heights themselves are the rules' air section.",
      { unit: "m/s" },
    ),
    field(
      "sensors.ground_m",
      "Ground sight range",
      "Reconnaissance",
      "Maximum clear-ground spotting distance before directional multipliers, occlusion and concealment.",
      { unit: "m" },
    ),
    ...["front", "side", "rear"].map((face) =>
      field(
        `sensors.sight_shape.${face}`,
        `${face[0].toUpperCase()}${face.slice(1)} sight reach`,
        "Reconnaissance",
        `Directional spotting range as a percentage of ground sight range toward the ${face}.`,
        { conversion: "percent", unit: "%" },
      ),
    ),
    field(
      "sensors.on",
      "Optics carrier",
      "Reconnaissance",
      "Turret mount the optics rotate with. None makes the optics follow the hull.",
      { kind: "reference", references: "mounts", optional: true },
    ),
    field(
      "capabilities.deploy",
      "Deployment capability",
      "Service",
      "Set up before providing service, then pack before moving. Enable adds editable setup and packing durations.",
      { kind: "object", optional: true, initial: { seconds: 3, pack_seconds: null } },
    ),
    field(
      "capabilities.deploy.seconds",
      "Setup time",
      "Service",
      "Time to become fully deployed. Reversing setup preserves deterministic partial progress.",
      { unit: "s" },
    ),
    field(
      "capabilities.deploy.pack_seconds",
      "Packing time",
      "Service",
      "Time to pack before moving. None uses setup time; packing and setup share reversible progress.",
      { unit: "s", optional: true },
    ),
    field(
      "capabilities.supply",
      "Supply capability",
      "Service",
      "Enable a service stock which rearms, repairs and reinforces eligible nearby units after deployment.",
      { kind: "object", optional: true, initial: { stock: 600 } },
    ),
    field(
      "capabilities.supply.stock",
      "Service stock",
      "Service",
      "Starting stock available for ammunition, repairs and replacement soldiers; shared service rules set their costs.",
      { unit: "stock", integer: true },
    ),
    field(
      "sound.profile",
      "Heard movement category",
      "Reconnaissance",
      "Category of the sound contact heard by enemy listeners. It describes the moving body; audio playback styling remains outside this editor.",
      { kind: "enum", choices: ["infantry", "vehicle", "shot"] },
    ),
    field(
      "sound.loudness_m",
      "Audible movement range",
      "Reconnaissance",
      "Distance at which movement can be heard by enemy listeners, creating a sound contact. Playback styling is outside this editor.",
      { unit: "m" },
    ),
    ...mounts(false),
  ],
  soldiers: [
    field(
      "hp",
      "Soldier health",
      "Soldiers",
      "Damage capacity of each soldier of this kind. Editing here makes a variant and rebinds this type’s matching slots; descendants inherit them.",
      { unit: "HP" },
    ),
    ...mounts(true),
  ],
  weapons: [
    field(
      "min_range_m",
      "Minimum engagement range",
      "Flight & accuracy",
      "Closest muzzle-to-aim-point distance this weapon fires. Inside it the crew holds fire; zero allows unrestricted close shots.",
      { unit: "m" },
    ),
    field(
      "range_m",
      "Maximum engagement range",
      "Flight & accuracy",
      "Farthest distance this weapon fires. Changing range keeps the current landing spread in metres and recalculates angular scatter.",
      { unit: "m" },
    ),
    field(
      "scatter_mrad",
      "Landing spread at maximum range",
      "Flight & accuracy",
      "Baseline one-axis launch spread in metres at maximum engagement range: angular scatter × range ÷ 1000. This is not a maximum miss distance or blast radius; rounds can land farther away. Movement, suppression and cover modify observed spread. Guided missiles steer out their initial spread.",
      { conversion: "spread", unit: "m" },
    ),
    field(
      "speed_mps",
      "Launch speed",
      "Flight & accuracy",
      "Speed as the round leaves its muzzle or rail. Flight timing and gravity determine its trajectory.",
      { unit: "m/s" },
    ),
    field(
      "gravity_scale",
      "Gravity strength",
      "Flight & accuracy",
      "Share of world gravity used by unguided flight, shown as a percentage. Slow visual rounds can use reduced gravity to preserve their ballistic line.",
      { conversion: "percent", unit: "%" },
    ),
    field(
      "trajectory",
      "Preferred arc",
      "Flight & accuracy",
      "Direct prefers a low arc; indirect prefers a high ballistic arc. Bodies still determine whether the path is clear.",
      { kind: "enum", choices: ["direct", "indirect"] },
    ),
    field(
      "turn_deg_s",
      "Guidance turn rate",
      "Flight & accuracy",
      "Maximum heading correction while supported by the launcher. None is unguided; guided rounds ignore gravity and are exempt from the unguided accuracy floor.",
      { unit: "°/s", optional: true },
    ),
    field(
      "guidance",
      "Guidance",
      "Flight & accuracy",
      "When the launcher can steer its guided round: stationary keeps guiding only while the launcher holds still; on the move keeps guiding as it moves. Every guided row states it.",
      { kind: "enum", choices: ["stationary", "on_the_move"], optional: true },
    ),
    field(
      "accel_mps2",
      "Motor acceleration",
      "Flight & accuracy",
      "Acceleration along flight direction for a guided rocket. Requires a paired top speed above launch speed; none means no motor.",
      { unit: "m/s²", optional: true },
    ),
    field(
      "top_speed_mps",
      "Motor top speed",
      "Flight & accuracy",
      "Speed limit for the paired guided motor; must exceed launch speed. None means no motor.",
      { unit: "m/s", optional: true },
    ),
    field(
      "top_attack",
      "Top attack",
      "Flight & accuracy",
      "A guided missile that climbs above its target, then dives onto the roof while the launcher supports it. Requires a guidance turn rate; released missiles fly straight at their fixed point.",
      { kind: "object", optional: true, initial: { loft_m: 60, dive_deg: 40 } },
    ),
    field(
      "top_attack.loft_m",
      "Loft height",
      "Flight & accuracy",
      "Height above the target the missile climbs toward before its dive.",
      { unit: "m", min: Number.MIN_VALUE },
    ),
    field(
      "top_attack.dive_deg",
      "Dive angle",
      "Flight & accuracy",
      "The missile pitches over once its target lies this far below its horizon, so it strikes at least this steeply. A low turn rate cannot pull through a steep dive.",
      { unit: "°", min: Number.MIN_VALUE, max: 90 },
    ),
    field(
      "lifetime_s",
      "Flight lifetime",
      "Flight & accuracy",
      "Maximum time before a round expires. None uses the shared unguided lifetime bound; authored lifetimes cannot exceed that bound.",
      { unit: "s", optional: true },
    ),
    field(
      "stationary",
      "Requires stationary firing",
      "Firing cycle",
      "Aims, reloads and fires only while its carrier is stationary. Movement interrupts eligible stationary weapons.",
      { kind: "boolean" },
    ),
    field(
      "default",
      "Always available gun",
      "Firing cycle",
      "Marks the unit’s default gun, available alongside its other ammunition kinds.",
      { kind: "boolean" },
    ),
    field(
      "aim_s",
      "Aim time",
      "Firing cycle",
      "Nominal delay to acquire and fire, before movement, suppression and cover penalties.",
      { unit: "s" },
    ),
    field(
      "reload_s",
      "Reload time",
      "Firing cycle",
      "Pause to replace a magazine or belt, or after each round without a magazine. Zero refills immediately; otherwise longer than its shot interval.",
      { unit: "s" },
    ),
    field(
      "ammo",
      "Carried ammunition",
      "Firing cycle",
      "Total rounds carried by each physical mount, including loaded magazines. Unlimited is a separate state, not a large count.",
      { kind: "ammo", integer: true },
    ),
    field(
      "magazine",
      "Magazine / belt",
      "Firing cycle",
      "Enable a physical capacity and minimum interval between shots. None reloads after each round.",
      {
        kind: "object",
        optional: true,
        initial: { rounds: 30, shot_interval_s: 0.1, burst: null },
      },
    ),
    field(
      "magazine.rounds",
      "Magazine capacity",
      "Firing cycle",
      "Rounds in one magazine or belt. Requires at least two; separate mounts have separate magazines.",
      { integer: true, min: 2, unit: "rounds" },
    ),
    field(
      "magazine.shot_interval_s",
      "Cyclic firing rate",
      "Firing cycle",
      "Physical cadence within a magazine, shown as rounds per minute. Stored as seconds between shots; aim delays can make actual firing slower.",
      { conversion: "cadence", unit: "rounds/min", min: Number.MIN_VALUE },
    ),
    field(
      "magazine.burst",
      "Burst fire",
      "Firing cycle",
      "Enable groups of rapid shots with a newly sampled aim delay before each burst. None fires continuously until the magazine empties.",
      { kind: "object", optional: true, initial: { rounds: 3, aim_max_s: 1 } },
    ),
    field(
      "magazine.burst.rounds",
      "Rounds per burst",
      "Firing cycle",
      "At least two rounds and no more than the magazine capacity. A burst pause keeps its remaining magazine loaded.",
      { integer: true, min: 2, unit: "rounds" },
    ),
    field(
      "magazine.burst.aim_max_s",
      "Burst aim delay limit",
      "Firing cycle",
      "Aim delay independently sampled from zero to this limit before each burst, while physical cadence remains the lower bound.",
      { unit: "s" },
    ),
    field(
      "penetration",
      "Armour penetration",
      "Impact & suppression",
      "Compared against protection on the struck face to decide whether a round penetrates.",
      { unit: "protection" },
    ),
    field(
      "damage",
      "Direct damage",
      "Impact & suppression",
      "Health damage when a round hits an eligible target; armour and blast rules determine what is applied.",
      { unit: "HP" },
    ),
    field(
      "armor_fraction",
      "Damage through unpenetrated armour",
      "Impact & suppression",
      "Percentage of direct damage still dealt to a vehicle when this round fails to penetrate, such as a high-explosive partial effect.",
      { conversion: "percent", unit: "%", max: 1 },
    ),
    field(
      "targets",
      "Engages",
      "Impact & suppression",
      "The height bands its crew can bring it to bear on: ground, and low_air for guns that swing up at helicopters. Rows that extend another inherit it.",
      { kind: "strings" },
    ),
    field(
      "armor_piercing",
      "Armour-piercing targeting",
      "Impact & suppression",
      "Prefers identified vehicles and never fires at an unclassified contact.",
      { kind: "boolean" },
    ),
    field(
      "anti_armor",
      "Dedicated anti-armour",
      "Impact & suppression",
      "Fires only at identified vehicles, even when other targets are visible.",
      { kind: "boolean" },
    ),
    field(
      "blast_radius_m",
      "Blast radius",
      "Impact & suppression",
      "Area reached by explosion damage around impact. Independent of landing spread and near-miss suppression distance.",
      { unit: "m" },
    ),
    field(
      "structural_damage",
      "Structural damage",
      "Impact & suppression",
      "Damage to destructible bodies such as cover and wrecks; direct damage to soldiers is a separate value.",
      { unit: "HP" },
    ),
    field(
      "near_miss_suppression",
      "Near-miss suppression",
      "Impact & suppression",
      "Suppression added by a round passing close enough to a unit. Shown as percentage points of the suppression scale.",
      { conversion: "percent", unit: "%", max: 1 },
    ),
    field(
      "suppression_radius_m",
      "Near-miss radius",
      "Impact & suppression",
      "Distance from a flown path within which a passing round reports a near miss. Independent of blast radius.",
      { unit: "m" },
    ),
  ],
};

/** A single allowlist shared by the UI and dev server. Mount selectors are names. */
export function gameplayField(section: Section, path: readonly string[]): GameplayField | null {
  const descriptor = GAMEPLAY_FIELDS[section]?.find(
    (f) =>
      f.path.length === path.length &&
      f.path.every((p, i) => p === path[i] || (p === "*" && path[i].length > 0)),
  );
  return descriptor ? { ...descriptor, path: [...path] } : null;
}

export function isObject(value: Json | undefined): value is JsonObject {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

/** Read nested values; mounts use stable ids, never inherited array indices. */
export function valueAt(value: Json | undefined, path: readonly string[]): Json | undefined {
  let cursor = value;
  for (const part of path) {
    cursor = Array.isArray(cursor)
      ? cursor.find((row) => isObject(row) && row.id === part)
      : isObject(cursor)
        ? cursor[part]
        : undefined;
  }
  return cursor;
}

const numericText = (n: number) => Number(n.toPrecision(14)).toString();
export function displayFieldValue(
  f: GameplayField,
  value: Json | undefined,
  entry: JsonObject,
): string {
  if (value === undefined || value === null) return "";
  if (f.kind === "vector" && Array.isArray(value))
    return value
      .map((n) =>
        typeof n === "number" ? numericText(f.conversion === "dimensions" ? n * 2 : n) : String(n),
      )
      .join(", ");
  if (f.kind === "strings" && Array.isArray(value)) return value.join(", ");
  if (typeof value !== "number")
    return typeof value === "object" ? JSON.stringify(value, null, 2) : String(value);
  const range = typeof entry.range_m === "number" ? entry.range_m : 0;
  const display =
    f.conversion === "spread"
      ? (value * range) / 1000
      : f.conversion === "cadence"
        ? 60 / value
        : f.conversion === "percent"
          ? value * 100
          : value;
  return numericText(display);
}

export function validateGameplayValue(
  section: Section,
  path: readonly string[],
  value: Json,
): string | undefined {
  const f = gameplayField(section, path);
  if (!f) return "This path is outside gameplay editing.";
  if (value === null)
    return f.optional && f.kind !== "strings" ? undefined : "A value is required.";
  if (f.kind === "boolean") return typeof value === "boolean" ? undefined : "Choose on or off.";
  if (f.kind === "enum")
    return typeof value === "string" && f.choices?.includes(value)
      ? undefined
      : "Choose a supported value.";
  if (f.kind === "reference")
    return typeof value === "string" && value.length > 0
      ? undefined
      : "Choose an existing reference.";
  if (f.kind === "strings")
    return Array.isArray(value) && value.every((v) => typeof v === "string" && v.length > 0)
      ? undefined
      : "Use a comma-separated list of reference ids.";
  if (f.kind === "object") {
    if (!isObject(value)) return "Expected a gameplay object or none.";
    for (const [key, child] of Object.entries(value)) {
      const error = validateGameplayValue(section, [...path, key], child);
      if (error) return `${key}: ${error}`;
    }
    return undefined;
  }
  if (f.kind === "mounts") {
    if (!Array.isArray(value)) return "Expected a list of mounts, each with an id.";
    const ids = new Set<string>();
    for (const row of value) {
      if (!isObject(row) || typeof row.id !== "string" || !row.id.trim() || ids.has(row.id))
        return "Every mount needs a unique id.";
      ids.add(row.id);
      // Its label is the catalog's to judge (`contract::labels`), on preview.
      for (const [key, child] of Object.entries(row)) {
        if (key === "id" || key === "name") continue;
        const error = validateGameplayValue(section, [...path, row.id, key], child);
        if (error) return `${row.id}.${key}: ${error}`;
      }
    }
    return undefined;
  }
  const numbers = f.kind === "vector" && Array.isArray(value) ? value : [value];
  if (f.kind === "vector" && (!Array.isArray(value) || value.length !== 3))
    return "Enter three numbers: forward, left, up (or length, width, height).";
  if (f.kind === "ammo" && value === "unlimited") return undefined;
  for (const n of numbers) {
    if (typeof n !== "number" || !Number.isFinite(n)) return "Enter a finite number.";
    if (f.integer && !Number.isInteger(n)) return "Enter a whole number.";
    if (f.min !== undefined && n < f.min) return `Stored value must be at least ${f.min}.`;
    if (f.max !== undefined && n > f.max)
      return f.conversion === "percent"
        ? `Enter no more than ${f.max * 100}%.`
        : `Enter no more than ${f.max}.`;
  }
  return undefined;
}

export type ParsedField = { value: Json; error?: never } | { error: string; value?: never };
export function parseFieldValue(f: GameplayField, text: string, entry: JsonObject): ParsedField {
  const trimmed = text.trim();
  if (!trimmed)
    return f.kind === "strings"
      ? { value: [] }
      : f.optional
        ? { value: null }
        : { error: "A value is required." };
  const current = valueAt(entry, f.path);
  // Preserve the original bits when a rounded human display is submitted unchanged.
  if (current !== undefined && trimmed === displayFieldValue(f, current, entry))
    return { value: current };
  let value: Json;
  if (f.kind === "boolean")
    value = trimmed === "true" ? true : trimmed === "false" ? false : trimmed;
  else if (f.kind === "strings")
    value = trimmed
      .split(",")
      .map((v) => v.trim())
      .filter(Boolean);
  else if (
    f.kind === "enum" ||
    f.kind === "reference" ||
    (f.kind === "ammo" && trimmed === "unlimited")
  )
    value = trimmed;
  else if (f.kind === "object" || f.kind === "mounts") {
    try {
      value = JSON.parse(trimmed) as Json;
    } catch {
      return { error: "Enter valid JSON." };
    }
  } else if (f.kind === "vector") {
    const parts = trimmed.split(",");
    if (parts.some((v) => !v.trim())) return { error: "Enter all three numbers." };
    value = parts.map((v) => Number(v.trim()) / (f.conversion === "dimensions" ? 2 : 1));
  } else {
    const number = Number(trimmed);
    const range = entry.range_m;
    if (f.conversion === "spread" && (typeof range !== "number" || range <= 0))
      return { error: "Set a positive engagement range first." };
    value =
      f.conversion === "spread"
        ? (number * 1000) / Number(range)
        : f.conversion === "cadence"
          ? 60 / number
          : f.conversion === "percent"
            ? number / 100
            : number;
  }
  // The section is only needed for allowlisting; callers already have this descriptor.
  const section = (Object.keys(GAMEPLAY_FIELDS) as Section[]).find(
    (s) => gameplayField(s, f.path)?.label === f.label,
  );
  const error = section ? validateGameplayValue(section, f.path, value) : "Unknown gameplay field.";
  return error ? { error } : { value };
}

/** Expand only the entry's active body/mobility; absent optional values remain editable. */
export function entryFields(section: Section, entry: JsonObject): GameplayField[] {
  return GAMEPLAY_FIELDS[section].flatMap((f) => {
    if (f.path.includes("*")) {
      const rows = entry.mounts;
      return Array.isArray(rows)
        ? rows.flatMap((row) =>
            isObject(row) && typeof row.id === "string"
              ? [{ ...f, path: f.path.map((p) => (p === "*" ? String(row.id) : p)) }]
              : [],
          )
        : [];
    }
    if (f.path.length > 1 && valueAt(entry, f.path.slice(0, -1)) === undefined) return [];
    if (f.path.length > 1 && valueAt(entry, f.path.slice(0, -1)) === null) return [];
    if (valueAt(entry, f.path) === undefined && !f.optional) return [];
    return [f];
  });
}
