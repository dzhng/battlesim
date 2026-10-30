export const RESPONSE_TRIGGERS = [
  "selected",
  "move",
  "attack",
  "attack_move",
  "stop",
  "weapons_free",
  "hold_fire",
  "enter_building",
  "leave_building",
  "board",
  "unload",
  "no_room_to_unload",
  "withdraw",
  "arrived",
  "refused",
] as const;
export const BARK_TRIGGERS = [
  "contact_infantry",
  "contact_vehicle",
  "contact_air",
  "opens_fire",
  "under_fire",
  "suppressed",
  "pinned",
  "hit",
  "man_down",
  "badly_damaged",
  "kill",
  "reloading",
  "out_of_ammo",
  "missile_away",
  "incoming_missile",
  "destroyed",
  "friendly_destroyed_nearby",
] as const;
export const ALL_TRIGGERS = [...RESPONSE_TRIGGERS, ...BARK_TRIGGERS] as const;
export type BarkTrigger = (typeof BARK_TRIGGERS)[number];

/** A bark's voice name from the simulation's tag: the contract names B5
 *  `b5_under_fire` (the publication layout's `shoutTriggers`),
 *  a voice line `under_fire`. The one place the two forms meet; a tag whose
 *  number and name aren't the same bark is refused. */
export function barkTrigger(tag: string): BarkTrigger {
  const match = /^b(\d+)_(.+)$/.exec(tag);
  const trigger = match ? BARK_TRIGGERS[Number(match[1]) - 1] : undefined;
  if (!match || trigger !== match[2]) throw new Error(`${tag} names no bark`);
  return trigger;
}
export type Trigger = (typeof ALL_TRIGGERS)[number];
export type Intensity = "steady" | "stressed";
export interface VoiceLineSet {
  steady: [string, string];
  stressed: [string, string];
}
export interface VoiceRole {
  lines: Record<Trigger, VoiceLineSet>;
}
export interface VoiceSpec {
  id: string;
  speaks: "responses" | "barks" | "all";
  gender: "male" | "female";
  description: string;
  voice_id: string | null;
}
/** The processing profiles a set may speak through (`assets.py` `VOICE_PROFILES`). */
export const VOICE_PROFILES = [
  "voice-dry",
  "voice-radio-light",
  "voice-earpiece",
  "voice-intercom",
  "voice-cockpit",
] as const;
export type VoiceProfile = (typeof VOICE_PROFILES)[number];
export interface VoiceSet {
  role: string;
  country: "us" | "de" | "uk" | "fr" | "se" | "it" | "ru" | "cn";
  callsign: string;
  processing: VoiceProfile;
  voices: VoiceSpec[];
}
export interface VoiceCatalog {
  roles: Record<string, VoiceRole>;
  sets: Record<string, VoiceSet>;
  assignments: Record<string, string | Record<string, string>>;
  retakes: Record<string, number>;
}
const own = (value: unknown): Record<string, unknown> => {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("voice catalog: expected object");
  return value as Record<string, unknown>;
};
const countries = new Set(["us", "de", "uk", "fr", "se", "it", "ru", "cn"]);

const triggerSet = new Set<string>(ALL_TRIGGERS);
function checkLine(text: unknown, where: string): string {
  if (typeof text !== "string" || text.trim().length === 0)
    throw new Error(`${where}: invalid line`);
  return text;
}

function validateVoiceRoles(catalog: VoiceCatalog) {
  for (const [role, raw] of Object.entries(catalog.roles)) {
    const lines = own(own(raw).lines) as Record<string, unknown>;
    for (const trigger of ALL_TRIGGERS) {
      const set = own(lines?.[trigger]);
      for (const intensity of ["steady", "stressed"] as const) {
        const takes = set[intensity];
        if (!Array.isArray(takes) || takes.length !== 2)
          throw new Error(`role ${role} ${trigger} ${intensity}: expected two takes`);
        takes.forEach((line, i) => checkLine(line, `role ${role} ${trigger} ${intensity}/${i}`));
      }
    }
    if (Object.keys(lines).some((trigger) => !triggerSet.has(trigger)))
      throw new Error(`role ${role}: unknown trigger`);
    for (const trigger of ALL_TRIGGERS) {
      if (
        trigger !== "selected" &&
        trigger !== "arrived" &&
        JSON.stringify(lines[trigger]).includes("{callsign}")
      )
        throw new Error(`role ${role} ${trigger}: callsign is only legal in R1 and R11`);
    }
  }
}

function validateVoiceSets(catalog: VoiceCatalog) {
  const callsigns = new Set<string>();
  const voiceIds = new Set<string>();
  for (const [name, raw] of Object.entries(catalog.sets)) {
    const set = own(raw) as unknown as VoiceSet;
    if (
      !catalog.roles[set.role] ||
      !countries.has(set.country) ||
      !(VOICE_PROFILES as readonly string[]).includes(set.processing) ||
      typeof set.callsign !== "string" ||
      !Array.isArray(set.voices) ||
      set.voices.length === 0
    )
      throw new Error(`set ${name}: invalid metadata`);
    const ids = new Set<string>();
    for (const voice of set.voices) {
      if (
        ids.has(voice.id) ||
        !voice.id ||
        !["responses", "barks", "all"].includes(voice.speaks) ||
        !["male", "female"].includes(voice.gender) ||
        typeof voice.description !== "string" ||
        (voice.voice_id !== null && typeof voice.voice_id !== "string")
      )
        throw new Error(`set ${name}: invalid voice`);
      ids.add(voice.id);
      if (callsigns.has(set.callsign)) throw new Error(`set ${name}: duplicate callsign`);
      if (voice.voice_id !== null) {
        if (voiceIds.has(voice.voice_id)) throw new Error(`set ${name}: duplicate voice_id`);
        voiceIds.add(voice.voice_id);
      }
    }
    callsigns.add(set.callsign);
  }
}

function validateVoiceAssignments(catalog: VoiceCatalog) {
  for (const [unit, assignment] of Object.entries(catalog.assignments)) {
    if (typeof assignment === "string") {
      if (!catalog.sets[assignment]) throw new Error(`assignment ${unit}: unknown set`);
    } else
      for (const [faction, set] of Object.entries(own(assignment)))
        if (!catalog.sets[String(set)])
          throw new Error(`assignment ${unit}/${faction}: unknown set`);
  }
}

// The caller may pass a roster map to enforce resolution. Deferred rows with no roles are not playable.
function validateRosterAssignments(catalog: VoiceCatalog, roster: unknown) {
  if (!roster || typeof roster !== "object") return;
  for (const [unit, raw] of Object.entries(roster as Record<string, unknown>)) {
    const row = own(raw);
    const roles = Array.isArray(row.roles) ? row.roles : [];
    if (unit.startsWith("test_") || unit.startsWith("capture_") || roles.length === 0) continue;
    const assignment = catalog.assignments[unit];
    if (!assignment) throw new Error(`playable unit ${unit}: no voice assignment`);
    const factions = own(row.roster).factions;
    for (const faction of Array.isArray(factions) ? factions : []) {
      const set = typeof assignment === "string" ? assignment : assignment[faction];
      if (!set || !catalog.sets[set])
        throw new Error(`playable unit ${unit}/${faction}: no voice set`);
    }
  }
}

export function validateVoiceCatalog(value: unknown, roster: unknown): VoiceCatalog {
  const root = own(value);
  for (const key of ["roles", "sets", "assignments", "retakes"]) own(root[key]);
  if (Object.keys(root).some((key) => !["roles", "sets", "assignments", "retakes"].includes(key)))
    throw new Error("voice catalog: unknown section");
  const catalog = value as VoiceCatalog;
  validateVoiceRoles(catalog);
  validateVoiceSets(catalog);
  validateVoiceAssignments(catalog);
  validateRosterAssignments(catalog, roster);
  return catalog;
}
/** The set a unit type speaks with when `faction` fields it: its assignment, by
 *  faction for a shared row; null for a unit with none (test and capture rows). */
export function assignedSet(
  assignments: VoiceCatalog["assignments"],
  unit: string,
  faction: string,
): string | null {
  const assignment = assignments[unit];
  return typeof assignment === "string" ? assignment : (assignment?.[faction] ?? null);
}
/** The set each unit type `faction`'s roster fields speaks with, by type. */
export function factionVoices(
  assignments: VoiceCatalog["assignments"],
  units: Iterable<{ id: string; factions: readonly string[] }>,
  faction: string,
): Record<string, string> {
  const voices: Record<string, string> = {};
  for (const { id, factions } of units) {
    const set = factions.includes(faction) ? assignedSet(assignments, id, faction) : null;
    if (set) voices[id] = set;
  }
  return voices;
}
export function lineText(
  catalog: VoiceCatalog,
  set: string,
  voice: string,
  trigger: Trigger,
  intensity: Intensity,
  take: number,
): string {
  const selected = catalog.sets[set];
  const role = selected?.role;
  const lines = role && catalog.roles[role]?.lines[trigger];
  if (
    !selected ||
    !lines ||
    ![0, 1].includes(take) ||
    !selected.voices.some((candidate) => candidate.id === voice)
  )
    throw new Error(`line ${set}/${voice}/${trigger}/${intensity}/${take}: not found`);
  return lines[intensity][take].replaceAll("{callsign}", selected.callsign);
}

/** The triggers a voice speaks: a leader answers, a soldier shouts, a lone voice does both. */
export function voiceTriggers(voice: Pick<VoiceSpec, "speaks">): readonly Trigger[] {
  return voice.speaks === "responses"
    ? RESPONSE_TRIGGERS
    : voice.speaks === "barks"
      ? BARK_TRIGGERS
      : ALL_TRIGGERS;
}

/** One take of one line: a voice speaking a trigger at an intensity. */
export interface VoiceTake {
  set: string;
  voice: string;
  trigger: Trigger;
  intensity: Intensity;
  take: number;
}
/** The take's identity, as `retakes` and the generation lock key it. */
export function takeId({ set, voice, trigger, intensity, take }: VoiceTake): string {
  return `${set}/${voice}/${trigger}/${intensity}/${take}`;
}
/** A take's file name: its identity in the set's folder, then a content hash. */
export function takeFile(take: VoiceTake, sha256: string): string {
  return `${take.voice}-${take.trigger}-${take.intensity}-${take.take}.${sha256.slice(0, 8)}.mp3`;
}
/** The served URL of a take's processed bytes: the only builder of this path. */
export function voiceUrl(take: VoiceTake, sha256: string): string {
  return `/audio/voices/${take.set}/${takeFile(take, sha256)}`;
}

/** One playable take in a pack. */
export interface PackTake extends Omit<VoiceTake, "set"> {
  url: string;
  duration_s: number;
}
/** `pack.json`: what a set can say. A trigger without a take is silent. */
export interface VoicePack {
  set: string;
  /** Each voice and whether it answers or shouts. */
  voices: Pick<VoiceSpec, "id" | "speaks">[];
  takes: PackTake[];
}
/** A set's pack, at a fixed name so the browser revalidates it: the only builder of this path. */
export function packUrl(set: string): string {
  return `/audio/voices/${set}/pack.json`;
}
/**
 * A set's pack from the takes `served` admits: a take's served sha256 and length,
 * or null to leave it out. The generation tool admits accepted takes; the
 * workbench preview admits every take not judged bad.
 */
export function voicePack(
  catalog: VoiceCatalog,
  set: string,
  served: (take: string, which: VoiceTake) => { sha256: string; duration_s: number } | null,
): VoicePack {
  const takes: PackTake[] = [];
  for (const voice of catalog.sets[set].voices)
    for (const trigger of voiceTriggers(voice))
      for (const intensity of ["steady", "stressed"] as const)
        for (const take of [0, 1]) {
          const id: VoiceTake = { set, voice: voice.id, trigger, intensity, take };
          const file = served(takeId(id), id);
          if (file)
            takes.push({
              voice: voice.id,
              trigger,
              intensity,
              take,
              url: voiceUrl(id, file.sha256),
              duration_s: file.duration_s,
            });
        }
  return {
    set,
    voices: catalog.sets[set].voices.map(({ id, speaks }) => ({ id, speaks })),
    takes,
  };
}
