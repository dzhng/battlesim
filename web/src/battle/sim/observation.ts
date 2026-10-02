/** Decodes a side publication using the layout the simulation publishes.
 *
 * Integers that outgrow a float32's exact range (soldier ids, shot counters)
 * travel as a `<name>Lo`/`<name>Hi` pair of `limbBits`-bit limbs, both -1 when
 * absent. */

import type { MoveDirection } from "./protocol";
import type { GroundRunsPatch } from "./ground";

/** The ground grid and exact tile-local run packing. */
export interface GroundLayout {
  /** Header field holding the patch's run count. */
  count: string;
  /** Per run: tile; local start+len*tileSize²; two packed byte words. */
  fields: string[];
  tileSize: number;
  maxRecordBytes: number;
  cellM: number;
  cols: number;
  rows: number;
  /** Side names by the header's `groundSide` tag. */
  sides: string[];
}

interface Section {
  name: string;
  count: string;
  fields: string[];
}

interface Group {
  name: string;
  count: string;
  fields: string[];
  sections: Section[];
}

export interface ObservationLayout {
  header: string[];
  groups: Group[];
  groupDelivery: {
    fields: string[];
    range: string[];
    copy: string[];
    copyAlignments: number[];
    encodings: string[];
    /** Bit grammar declared by the producer; carriers are raw u32 words. */
    packed: Record<string, string>;
  };
  fog: { count: string; maxWords: number };
  ground: GroundLayout;
  /** Bits per limb of an exact integer field pair. */
  limbBits: number;
  /** This battle's round kinds (weapon rows, in name order). */
  roundKinds: string[];
  /** What a round struck: none, ground, hull, prop or soldier. */
  hitKinds: string[];
  unitKinds: string[];
  moveStates: string[];
  policies: string[];
  directions: string[];
  contactSources: string[];
  soundCategories: string[];
  soundBands: string[];
  propKinds: string[];
  engagements: string[];
  actionReasons: string[];
  targetKinds: string[];
  postures: string[];
  /** Cover tiers, weakest first: light, medium, heavy. */
  coverTiers: string[];
  /** Which way a soldier leans out round his cover: left, right. */
  leanSides: string[];
  garrisonPhases: string[];
  serviceStatuses: string[];
  /** Suppression tiers: none, suppressed, pinned. */
  suppressionTiers: SuppressionTier[];
  encounterResults: string[];
}

export type Point2 = [number, number];
export type Point3 = [number, number, number];

export interface OwnUnitView {
  id: number;
  kind: string;
  position: Point3;
  yaw: number;
  goal: Point2 | null;
  policy: string | null;
  /** Which way the current move drives (Q31); null without a move. */
  direction: MoveDirection | null;
  /** Driving backwards this tick: a reverse move or a three-point turn's
   *  reversing leg (the reverse whine's cue). */
  reversing: boolean;
  state: string;
  /** The friendly unit this one waits for. */
  blocker: number | null;
  route: Point2[];
  queue: Point2[];
  members: Point3[];
  /** Each living soldier's id, in `members` order. */
  memberIds: number[];
  /** Each living soldier's slot in his squad type (which soldier kind he is). */
  memberSlots: number[];
  /** Each living soldier's resolved place and cover (D2+), in `members` order. */
  memberOrders: MemberOrderView[];
  /** Each living soldier's lean, in `members` order: null while tucked in. */
  memberLeans: (MemberLeanView | null)[];
  /** A squad's area round its anchor, which only an order moves;
   *  null for a vehicle. */
  area: SquadAreaView | null;
  /** The bearing the unit ends its move at (D2, Q9): the ordered facing,
   *  else the way it travels at the end (a reverse move's held facing);
   *  its yaw without a move. */
  finalFacing: number;
  /** Enemy handles this unit's own sensors identify. */
  sees: number[];
  engagement: string;
  mounts: MountView[];
  /** Every mount's pose, in `mounts` order. */
  weaponPoses: WeaponPoseView[];
  /** Vehicle health (0 for infantry). */
  hp: number;
  /** Health of each living soldier, in `members` order. */
  memberHp: number[];
  /** How suppressed the squad is, by the rules' thresholds (the sim owns
   *  them; the hidden level is never published). Vehicles are "none". */
  suppression: SuppressionTier;
  /** Enough forest or shelter concealment, with no visible enemy observer spotting it or known engagement. */
  concealed: boolean;
  /** Setup progress for units that deploy in place; null for the rest. */
  deployment: DeploymentView | null;
  /** The squad's building while entering, inside or leaving it; null otherwise. */
  garrison: GarrisonView | null;
  /** A supply vehicle's remaining stock; null for other units. */
  stock: number | null;
  /** Why this unit is or is not being served by a supply vehicle. */
  service: string;
  /** Where this unit's own sight reaches at the published tick. */
  sight: SightView;
}

/** A squad's suppression tier, as the simulation names it. */
export type SuppressionTier = "none" | "suppressed" | "pinned";

/** A cover tier, as the simulation names it. */
export type CoverTier = "light" | "medium" | "heavy";

/** A soldier's resolved place (D2+): his spot while moving, his post while
 *  holding, else where he stands; the cover he has now against his squad's
 *  threat, and the cover his spot gives. */
export interface MemberOrderView {
  spot: Point2;
  coverNow: CoverTier | null;
  coverThere: CoverTier | null;
}

/** A soldier out on his lean: leaning out past his cover's edge while
 *  he fires, his body at `at`; his `members` position stays where he tucks in. */
export interface MemberLeanView {
  side: "left" | "right";
  at: Point2;
}

/** A squad's area: the disc round its anchor its soldiers fight in. */
export interface SquadAreaView {
  anchor: Point2;
  radius: number;
}

/** Sight multipliers dead ahead, abeam and astern (infantry: all 1). */
export interface SightShape {
  front: number;
  side: number;
  rear: number;
}

/**
 * The sight the simulation spotted and swept with this tick: for fog, read
 * at the published tick, never interpolated. Reach toward world bearing `b` is
 * `range * sightMultiplier(shape, b - forward)` (battle-renderer's sightOverlay).
 */
export interface SightView {
  /** The unit's eye, or one eye per facade a garrison holds. */
  eyes: Point3[];
  /** World bearing it looks along: a tank's turret, otherwise the hull. */
  forward: number;
  shape: SightShape;
  /** Ground range in the open, before shape, foliage and concealment. */
  range: number;
}

/** A squad's hold on a building: which one, the phase and its timer progress. */
export interface GarrisonView {
  /** The building's prop id in the static map. */
  building: number;
  /** entering, inside or exiting. */
  phase: string;
  /** Entering or leaving progress in [0, 1]; 1 while inside. */
  progress: number;
  /** The building's footprint on the ground: its centre and half extents in
   *  its own frame (every corner lies `hypot(half)` from the centre). */
  center: Point2;
  half: Point2;
}

/** A deploying unit's one progress value and the end state it heads to. */
export interface DeploymentView {
  /** In [0, 1]: 0 packed and free to move, 1 fully deployed. */
  progress: number;
  target: string;
}

/** A fallen soldier: own, or an enemy this side saw fall. */
export interface CorpseView {
  position: Point3;
  own: boolean;
  /** The soldier's id. */
  soldier: number;
  /** The unit type of the squad the soldier fought in, and his slot in it. */
  kind: string;
  slot: number;
  /** The squad's heading when the soldier fell. */
  yaw: number;
}

/**
 * What a weapon mount is doing, for posing its model (the renderer derives
 * the pose; the simulation never names an animation).
 */
export interface WeaponPoseView {
  /** Index into the unit kind's mount list in the rules. */
  mount: number;
  /** The visible infantry operator's stable soldier id, or null. */
  operator: number | null;
  /** World bearing: a turret's heading, or a hand weapon's last aim. */
  bearing: number;
  /** Elevation of the mount's last launched round; 0 before it first fires. */
  elevation: number;
  /** Rounds launched so far (a squad volley counts per soldier); a rise is a shot. */
  shots: number;
}

/** A round's burst this tick: own anywhere, enemy only over seen ground. */
export interface BlastView {
  point: Point3;
  radius: number;
  /** The round kind (a weapon row name). */
  kind: string;
}

/** What a mount is aimed at: a side-scoped handle or a ground point. */
export type MountTargetView =
  | { kind: "identified" | "contact"; id: number }
  | { kind: "ground"; point: Point3 };

/** One weapon mount's readiness, exactly as the simulation reports it. */
export interface MountView {
  /** Index into the unit kind's mount list in the rules. */
  mount: number;
  /** Loaded ammunition kind (index into the mount's weapons), or null. */
  loaded: number | null;
  /** Rounds left per ammunition kind; null means unlimited. */
  ammo: (number | null)[];
  /** Aim progress in [0, 1] (1 once acquired) and reload progress in [0, 1]. */
  aim: number;
  reload: number;
  target: MountTargetView | null;
  reason: string;
  /** Guiding a missile in flight (one at a time). */
  guiding: boolean;
  /** The ammunition kind being reloaded, or null. */
  reloading: number | null;
}

/** One of this side's own guided missiles and the point it steers to. */
export interface GuidedView {
  /** Stable while it flies. */
  id: number;
  position: Point3;
  point: Point3;
  /** Its launcher still guides it; once false, the point is fixed for good. */
  supported: boolean;
}

/** A visible stretch of a projectile's flight this tick. */
export interface ProjectileView {
  /** The flown path, at least two points: it bends where the round ricocheted. */
  path: Point3[];
  /** Where along `path` the round glanced off a hull, with the outward normal there. */
  ricochets: RicochetView[];
  own: boolean;
  /** The round kind (a weapon row name): an enemy tracer reveals its shooter's class. */
  kind: string;
  /** The soldier who fired it, or null for a vehicle's gun. */
  shooterMember: number | null;
  /** What the round struck at the path's end this tick: none, ground, hull, prop or soldier. */
  hit: string;
  /** Outward surface normal at the impact, or null without one. */
  impactNormal: Point3 | null;
}

/** A ricochet: the round glanced off a hull at `path[point]`. */
export interface RicochetView {
  point: number;
  normal: Point3;
}

/** A team-identified enemy: side-scoped handle and only what was observed. */
export interface IdentifiedView {
  id: number;
  kind: string;
  cost: number;
  position: Point3;
  yaw: number;
  velocity: Point2;
  /** Soldiers actually seen (infantry). */
  members: Point3[];
  /** The seen soldiers' ids, in `members` order. */
  memberIds: number[];
  /** Each seen soldier's slot in his squad type. */
  memberSlots: number[];
  /** Each seen soldier's lean, in `members` order: null while tucked in. */
  memberLeans: (MemberLeanView | null)[];
  /** Every mount's pose while identified. */
  weaponPoses: WeaponPoseView[];
  /** Driving backwards this tick (a seen vehicle's reverse whine). */
  reversing: boolean;
}

/** Uncertain evidence: an area, never an exact position, strength or
 *  count; only what the side learned when the evidence came. */
export interface ContactView {
  id: number;
  /** One preferred report label per enemy; other evidence remains targetable. */
  primaryLabel: boolean;
  source: string;
  center: Point2;
  radius: number;
  evidenceTick: number;
  expiresTick: number;
  /** A last sighting's unit type, as the side identified it; null for a
   *  firing report (heard, never identified). */
  kind: string | null;
  /** A firing report's weapon rows as heard (round kinds, each mount's
   *  every row: a report doesn't say which round); empty for a last sighting. */
  heard: string[];
}

/** A sound a friendly listener heard from an unseen enemy. */
export interface SoundCueView {
  listener: number;
  category: string;
  /** 0 = east, counter-clockwise in 45° steps. */
  sector: number;
  band: string;
  moving: boolean;
}

/** A side-known obstacle state, with its immutable authored association. */
export interface KnownPropView {
  id: number;
  building: number | null;
  structureOwner: number | null;
  authoredProp: number | null;
  kind: string;
  center: Point2;
  yaw: number;
  half: Point3;
  baseZ: number;
  /** The authored prop this one stands in place of (a ruin's building), or null. */
  replaces: number | null;
  /** The side saw `replaces` destroyed with nothing in its place (a crate
   *  blown away): the entry draws nothing and only removes that prop. */
  destroyed: boolean;
}

/** Ground cells this side can see, one bit each (row-major). */
export interface VisibilityView {
  cellM: number;
  nx: number;
  ny: number;
  bits: Uint32Array;
}

export interface ObservationView {
  tick: number;
  own: OwnUnitView[];
  identified: IdentifiedView[];
  contacts: ContactView[];
  audible: SoundCueView[];
  knownProps: readonly KnownPropView[];
  projectiles: ProjectileView[];
  blasts: BlastView[];
  corpses: readonly CorpseView[];
  guided: GuidedView[];
  /** The fixture's completion condition, when it has one. */
  encounter: { heldS: number; result: string } | null;
  fog: VisibilityView;
  groundPatch: GroundRunsPatch;
}

type Row = { field: (name: string) => number; sections: Record<string, number[][]> };

type GroupBaseline = {
  words: Float32Array[];
  staticViews: Pick<ObservationView, "corpses" | "knownProps">;
};

/** One ordered side-publication stream. Retained observations are immutable;
 * unchanged fog and static groups share their views, changed groups replace them. */
export class ObservationDecoder {
  private epoch = 0;
  private floor = 0;
  private side: string | null = null;
  private revision = 0;
  private groundRevision = 0;
  private fog: VisibilityView | null = null;
  private expectedSide: string | null = null;
  private groupBaseline: GroupBaseline | null = null;

  constructor(private readonly layout: ObservationLayout) {}

  /** In-flight records from the old side are returned as stale, without
   * becoming observations. The next epoch must start with a full field. */
  invalidate(side?: string) {
    this.expectedSide = side ?? null;
    this.floor = this.epoch;
    this.fog = null;
    this.groupBaseline = null;
  }

  decode(data: Float32Array): ObservationView | null {
    const layout = this.layout;
    if (data.byteLength > layout.ground.maxRecordBytes || data.length < layout.header.length)
      throw new Error("publication exceeds its admitted record bound");
    const header = Object.fromEntries(layout.header.map((f, i) => [f, data[i]]));
    const epoch = header.groundEpoch;
    if (!Number.isInteger(epoch) || epoch <= 0)
      throw new Error("publication epoch must be positive");
    if (epoch < this.epoch || epoch <= this.floor) return null;
    const side = layout.ground.sides[header.groundSide];
    if (this.expectedSide !== null && side !== this.expectedSide) return null;
    const integer = (name: string) => {
      const low = header[`${name}Lo`],
        high = header[`${name}Hi`];
      if (
        !Number.isInteger(low) ||
        !Number.isInteger(high) ||
        low < 0 ||
        high < 0 ||
        low >= 2 ** layout.limbBits ||
        high >= 2 ** layout.limbBits
      )
        throw new Error("fog revision limbs must be exact 16-bit integers");
      return low + high * 2 ** layout.limbBits;
    };
    const base = integer("fogBase");
    const revision = integer("fogRevision");
    const fresh = epoch > this.epoch;
    const groundBase = header.groundBase;
    const groundRevision = header.groundRevision;
    if (
      side === undefined ||
      !Number.isInteger(groundBase) ||
      groundBase < 0 ||
      groundBase >= 2 ** 24 ||
      !Number.isInteger(groundRevision) ||
      groundRevision < groundBase ||
      groundRevision >= 2 ** 24 ||
      (fresh
        ? header.groundFull !== 1 || groundBase !== 0
        : header.groundFull !== 0 || groundBase !== this.groundRevision || side !== this.side)
    )
      throw new Error("ground publication does not follow its side/epoch/revision baseline");
    if (fresh ? header.fogFull !== 1 || base !== 0 : side !== this.side || base !== this.revision)
      throw new Error("fog publication does not follow its side/epoch/revision baseline");
    if (!Number.isSafeInteger(revision) || revision <= base)
      throw new Error("fog publication revision must advance");
    const reconstructed = reconstructGroups(
      layout,
      data,
      fresh ? null : (this.groupBaseline?.words ?? null),
    );
    const observation = decodeFrame(
      layout,
      data,
      header,
      fresh ? null : this.fog,
      reconstructed.groups,
      reconstructed.tailOffset,
      fresh ? null : this.groupBaseline,
    );
    // Commit only after the complete record has decoded successfully.
    this.epoch = epoch;
    this.side = side;
    this.revision = revision;
    this.groundRevision = groundRevision;
    this.fog = observation.fog;
    this.groupBaseline = {
      words: reconstructed.groups,
      staticViews: { corpses: observation.corpses, knownProps: observation.knownProps },
    };
    return observation;
  }
}

/** Reconstruct non-map groups independently; fog and ground keep their sparse
 * payloads. No cursor is committed until the whole observation validates. */
function reconstructGroups(
  layout: ObservationLayout,
  encoded: Float32Array,
  previous: Float32Array[] | null,
): { tailOffset: number; groups: Float32Array[] } {
  let cursor = layout.header.length;
  let length = cursor;
  const groups: Float32Array[] = [];
  const integer = (value: number) => Number.isSafeInteger(value) && value >= 0;
  for (let g = 0; g < layout.groups.length; g++) {
    const metadata = Object.fromEntries(
      layout.groupDelivery.fields.map((name) => [name, encoded[cursor++]]),
    );
    const { length: size, encoding, floats: payload } = metadata;
    const mode = layout.groupDelivery.encodings[encoding];
    if (
      !integer(size) ||
      !integer(payload) ||
      !["replacement", "snapshot", "copies", "packed"].includes(mode) ||
      length + size > layout.ground.maxRecordBytes / 4 ||
      cursor + payload > encoded.length ||
      (previous === null && mode !== "snapshot" && mode !== "packed")
    )
      throw new Error("invalid observation group payload");
    const end = cursor + payload;
    let values: Float32Array;
    if (mode === "packed") {
      values = unpackGroup(layout, g, encoded, cursor, payload, size, previous?.[g] ?? null);
      cursor = end;
    } else if (mode === "snapshot") {
      if (payload !== size) throw new Error("observation group snapshot length mismatch");
      values = encoded.slice(cursor, end);
      cursor = end;
    } else if (mode === "copies") {
      const old = previous![g];
      const group = layout.groups[g];
      const stride = layout.groupDelivery.copyAlignments[g];
      if (stride !== (group.sections.length === 0 ? group.fields.length : 1) || size % stride !== 0)
        throw new Error("observation copy alignment disagrees with its group");
      values = new Float32Array(size);
      let at = 0;
      while (cursor < end) {
        const operation = Object.fromEntries(
          layout.groupDelivery.copy.map((name) => [name, encoded[cursor++]]),
        );
        const { source, length: count } = operation;
        if (
          !integer(count) ||
          count === 0 ||
          count % stride !== 0 ||
          at + count > size ||
          (source === -1
            ? cursor + count > end
            : !integer(source) || source % stride !== 0 || source + count > old.length)
        )
          throw new Error("invalid observation row copy");
        values.set(
          source === -1
            ? encoded.subarray(cursor, cursor + count)
            : old.subarray(source, source + count),
          at,
        );
        if (source === -1) cursor += count;
        at += count;
      }
      if (cursor !== end || at !== size)
        throw new Error("observation row copies do not fill their group");
    } else {
      const old = previous![g];
      if (size === old.length && payload === 0) values = old;
      else {
        values = new Float32Array(size);
        values.set(old.subarray(0, size));
      }
      let last = 0;
      while (cursor < end) {
        const range = Object.fromEntries(
          layout.groupDelivery.range.map((name) => [name, encoded[cursor++]]),
        );
        const { start, length: count } = range;
        if (
          !integer(start) ||
          !integer(count) ||
          count === 0 ||
          start < last ||
          start + count > size ||
          cursor + count > end ||
          (start > old.length && last < start)
        )
          throw new Error("invalid observation group replacement range");
        values.set(encoded.subarray(cursor, cursor + count), start);
        cursor += count;
        last = start + count;
      }
      if (cursor !== end || (size > old.length && last < size))
        throw new Error("observation group growth must supply its new words");
    }
    groups.push(values);
    length += size;
    if (length > layout.ground.maxRecordBytes / 4)
      throw new Error("reconstructed observation exceeds its admitted record bound");
  }
  length += encoded.length - cursor;
  if (length > layout.ground.maxRecordBytes / 4)
    throw new Error("reconstructed observation exceeds its admitted record bound");
  return { tailOffset: cursor, groups };
}

/** Raw-carrier reader shared by groups and ground; JS float reads can canonicalize NaNs. */
class PackedReader {
  private bit = 0;
  private readonly words: Uint32Array;
  constructor(data: Float32Array) {
    this.words = new Uint32Array(data.buffer, data.byteOffset, data.length);
  }
  read(count: number): number {
    if (this.bit + count > this.words.length * 32) throw new Error("truncated packed publication");
    const index = Math.floor(this.bit / 32),
      shift = this.bit % 32;
    let value = this.words[index] >>> shift;
    if (shift + count > 32) value |= this.words[index + 1] << (32 - shift);
    this.bit += count;
    return (value & (count === 32 ? 0xffffffff : 2 ** count - 1)) >>> 0;
  }
  integer(): number {
    let value = 0;
    for (let byte = 0; byte < 4; byte++) {
      const part = this.read(8);
      value += (part & 127) * 2 ** (byte * 7);
      if (part < 128) {
        if (byte !== 0 && part === 0) throw new Error("noncanonical packed publication integer");
        return value;
      }
    }
    throw new Error("oversized packed publication integer");
  }
  finish(): void {
    if (this.words.length * 32 - this.bit > 31)
      throw new Error("excess packed publication padding");
    while (this.bit < this.words.length * 32)
      if (this.read(1) !== 0) throw new Error("nonzero packed publication padding");
  }
}

function unpackGroup(
  layout: ObservationLayout,
  groupIndex: number,
  encoded: Float32Array,
  offset: number,
  length: number,
  size: number,
  old: Float32Array | null,
): Float32Array {
  const reader = new PackedReader(encoded.subarray(offset, offset + length));
  const form = reader.read(8);
  if (form > 2 || (old === null && form !== 1))
    throw new Error("packed observation group requires its baseline");
  const baseline = old === null ? null : new Uint32Array(old.buffer, old.byteOffset, old.length);
  const values = new Float32Array(size);
  const result = new Uint32Array(values.buffer);
  const literal = (at: number): void => {
    const tag = reader.read(4);
    if (tag > 9 || (form === 1 && tag >= 5))
      throw new Error("invalid packed observation literal tag");
    const count = tag < 5 ? tag : tag - 5;
    const value = count === 0 ? 0 : reader.read(count * 8);
    result[at] = tag < 5 ? value : (value ^ (baseline?.[at] ?? 0)) >>> 0;
  };
  if (form === 1) {
    for (let at = 0; at < size; at++) literal(at);
  } else if (form === 0) {
    result.set(baseline!.subarray(0, size));
    const operations = reader.integer();
    if (operations > size) throw new Error("invalid packed observation replacement count");
    let last = 0;
    for (let operation = 0; operation < operations; operation++) {
      const start = reader.integer();
      const count = reader.integer();
      if (
        count === 0 ||
        start < last ||
        start + count > size ||
        (start > baseline!.length && last < start)
      )
        throw new Error("invalid packed observation replacement range");
      for (let at = start; at < start + count; at++) literal(at);
      last = start + count;
    }
    if (size > baseline!.length && last < size)
      throw new Error("packed observation growth must supply its new words");
  } else {
    const group = layout.groups[groupIndex];
    const stride = layout.groupDelivery.copyAlignments[groupIndex];
    if (stride !== (group.sections.length === 0 ? group.fields.length : 1) || size % stride !== 0)
      throw new Error("packed observation copy alignment disagrees with its group");
    let at = 0;
    while (at < size) {
      const sourcePlusOne = reader.integer();
      const count = reader.integer();
      const source = sourcePlusOne - 1;
      if (
        count === 0 ||
        count % stride !== 0 ||
        at + count > size ||
        (sourcePlusOne !== 0 && (source % stride !== 0 || source + count > baseline!.length))
      )
        throw new Error("invalid packed observation source copy");
      if (sourcePlusOne === 0) {
        for (let end = at + count; at < end; at++) literal(at);
      } else {
        result.set(baseline!.subarray(source, source + count), at);
        at += count;
      }
    }
  }
  reader.finish();
  return values;
}

function decodeFrame(
  layout: ObservationLayout,
  data: Float32Array,
  header: Record<string, number>,
  previous: VisibilityView | null,
  payloads: Float32Array[],
  tailOffset: number,
  baseline: GroupBaseline | null,
): ObservationView {
  let cursor = tailOffset;
  const groups: Record<string, Row[]> = {};
  const reuse = { corpses: false, knownProps: false };
  for (const [index, group] of layout.groups.entries()) {
    const payload = payloads[index];
    let offset = 0;
    if (
      !Number.isSafeInteger(header[group.count]) ||
      header[group.count] < 0 ||
      header[group.count] * group.fields.length > payload.length
    )
      throw new Error("observation group row count exceeds its payload");
    if (
      baseline !== null &&
      (group.name === "corpses" || group.name === "knownProps") &&
      group.sections.length === 0 &&
      payload === baseline.words[index]
    ) {
      if (header[group.count] * group.fields.length !== payload.length)
        throw new Error("observation group values do not match their counts");
      reuse[group.name] = true;
      continue;
    }
    const at = Object.fromEntries(group.fields.map((f, i) => [f, i]));
    const rows: Row[] = [];
    for (let n = 0; n < header[group.count]; n++, offset += group.fields.length) {
      const base = offset;
      rows.push({ field: (name) => payload[base + at[name]], sections: {} });
    }
    // Each row's variable sections follow all the rows, in row order.
    for (const row of rows) {
      for (const section of group.sections) {
        const width = section.fields.length;
        const points: number[][] = [];
        if (
          !Number.isSafeInteger(row.field(section.count)) ||
          row.field(section.count) < 0 ||
          offset + row.field(section.count) * width > payload.length
        )
          throw new Error("observation group section count exceeds its payload");
        for (let k = 0; k < row.field(section.count); k++, offset += width) {
          points.push(Array.from(payload.subarray(offset, offset + width)));
        }
        row.sections[section.name] = points;
      }
    }
    if (offset !== payload.length)
      throw new Error("observation group values do not match their counts");
    groups[group.name] = rows;
  }
  const nx = header.fogNx,
    ny = header.fogNy,
    cellM = header.fogCellM;
  const cells = nx * ny;
  const words = Math.ceil(cells / 32);
  if (
    !Number.isSafeInteger(nx) ||
    !Number.isSafeInteger(ny) ||
    nx < 0 ||
    ny < 0 ||
    words > layout.fog.maxWords ||
    (cells > 0 && !(cellM > 0))
  )
    throw new Error("fog field dimensions exceed the admitted delivery bound");
  const count = header[layout.fog.count];
  const full = header.fogFull === 1;
  if (header.fogFull !== 0 && !full) throw new Error("unknown fog payload encoding");
  if (
    !Number.isSafeInteger(count) ||
    count < 0 ||
    (full ? count !== words * 2 : count % 3 !== 0 || count > words * 2)
  )
    throw new Error("fog payload does not match its encoding");
  if (
    (!full && !previous) ||
    (previous && (previous.nx !== nx || previous.ny !== ny || previous.cellM !== cellM))
  )
    throw new Error("fog delta has no matching field baseline");
  const groundRuns = header[layout.ground.count];
  if (!Number.isSafeInteger(groundRuns) || groundRuns < 0)
    throw new Error("ground runs do not match their count");
  const groundFloats = groundRuns * layout.ground.fields.length;
  const logicalWords =
    layout.header.length +
    payloads.reduce((sum, group) => sum + group.length, 0) +
    count +
    groundFloats;
  if (cursor + count > data.length || logicalWords > layout.ground.maxRecordBytes / 4)
    throw new Error("publication length does not match its layout");
  const bits = full
    ? new Uint32Array(words)
    : count === 0
      ? previous!.bits
      : previous!.bits.slice();
  const readWord = (at: number) => {
    const low = data[at],
      high = data[at + 1];
    if (
      !Number.isInteger(low) ||
      !Number.isInteger(high) ||
      low < 0 ||
      high < 0 ||
      low >= 2 ** layout.limbBits ||
      high >= 2 ** layout.limbBits
    )
      throw new Error("fog word limbs must be exact 16-bit integers");
    return (low | (high << layout.limbBits)) >>> 0;
  };
  if (full) {
    for (let i = 0; i < words; i++) bits[i] = readWord(cursor + i * 2);
  } else {
    let last = -1;
    for (let at = cursor; at < cursor + count; at += 3) {
      const index = data[at];
      if (!Number.isInteger(index) || index <= last || index >= words)
        throw new Error("fog delta word indices must be ordered and inside the field");
      bits[index] = readWord(at + 1);
      last = index;
    }
  }
  if (cells % 32 && bits[words - 1] >>> (cells % 32))
    throw new Error("fog padding bits must be zero");
  cursor += count;
  const groundPatch = decodeGroundPatch(layout, header, data.subarray(cursor), groundRuns);

  // An exact integer from its limbs; null when absent.
  const limbs = (f: (name: string) => number, name: string): number | null => {
    const lo = f(`${name}Lo`);
    return lo < 0 ? null : lo + f(`${name}Hi`) * 2 ** layout.limbBits;
  };
  // A section's points read by field name.
  const reader = (group: string, section: string) => {
    const fields = layout.groups
      .find((g) => g.name === group)!
      .sections.find((s) => s.name === section)!.fields;
    const at = Object.fromEntries(fields.map((f, i) => [f, i]));
    return (point: number[]) => (name: string) => point[at[name]];
  };
  const ownMount = reader("own", "mounts");
  const ownOrder = reader("own", "memberOrders");
  const tier = (k: number): CoverTier | null =>
    k < 0 ? null : (layout.coverTiers[k] as CoverTier);
  const ids = (points: number[][], read: ReturnType<typeof reader>) =>
    points.map((p) => limbs(read(p), "id")!);
  const slots = (points: number[][], read: ReturnType<typeof reader>) =>
    points.map((p) => read(p)("slot"));
  const poses = (points: number[][], read: ReturnType<typeof reader>) =>
    points.map((p): WeaponPoseView => {
      const f = read(p);
      return {
        mount: f("mount"),
        bearing: f("bearing"),
        elevation: f("elevation"),
        shots: limbs(f, "shots")!,
        operator: limbs(f, "operator"),
      };
    });
  const [ownIds, ownPoses] = [reader("own", "memberIds"), reader("own", "weaponPoses")];
  const [seenIds, seenPoses] = [
    reader("identified", "memberIds"),
    reader("identified", "weaponPoses"),
  ];
  const leans = (points: number[][], read: ReturnType<typeof reader>) =>
    points.map((p): MemberLeanView | null => {
      const f = read(p);
      const side = f("side");
      return side < 0
        ? null
        : { side: layout.leanSides[side] as MemberLeanView["side"], at: [f("x"), f("y")] };
    });
  const [ownLeans, seenLeans] = [reader("own", "memberLeans"), reader("identified", "memberLeans")];
  const own = groups.own.map(({ field: f, sections }): OwnUnitView => {
    const policy = f("policy");
    const direction = f("direction");
    const blocker = f("blocker");
    const deployTarget = f("deployTarget");
    const garrisonPhase = f("garrisonPhase");
    return {
      id: f("id"),
      kind: layout.unitKinds[f("kind")],
      position: [f("x"), f("y"), f("z")],
      yaw: f("yaw"),
      goal: Number.isNaN(f("goalX")) ? null : [f("goalX"), f("goalY")],
      policy: policy < 0 ? null : layout.policies[policy],
      direction: direction < 0 ? null : (layout.directions[direction] as MoveDirection),
      reversing: f("reversing") === 1,
      state: layout.moveStates[f("state")],
      blocker: blocker < 0 ? null : blocker,
      route: sections.route as Point2[],
      queue: sections.queue as Point2[],
      members: sections.members as Point3[],
      memberIds: ids(sections.memberIds, ownIds),
      memberSlots: slots(sections.memberIds, ownIds),
      memberOrders: sections.memberOrders.map((p) => {
        const m = ownOrder(p);
        return {
          spot: [m("x"), m("y")],
          coverNow: tier(m("coverNow")),
          coverThere: tier(m("coverThere")),
        };
      }),
      memberLeans: leans(sections.memberLeans, ownLeans),
      // The area fields are NaN for a vehicle.
      area: Number.isNaN(f("areaM"))
        ? null
        : { anchor: [f("areaX"), f("areaY")], radius: f("areaM") },
      finalFacing: f("finalFacing"),
      sees: sections.sees.map((p) => p[0]),
      engagement: layout.engagements[f("engagement")],
      mounts: sections.mounts.map((m) => decodeMount(layout, ownMount(m))),
      weaponPoses: poses(sections.weaponPoses, ownPoses),
      hp: f("hp"),
      memberHp: sections.memberHp.map((p) => p[0]),
      suppression: layout.suppressionTiers[f("suppression")],
      concealed: f("concealed") === 1,
      // Both deployment fields are -1 for units that never deploy.
      deployment:
        deployTarget < 0
          ? null
          : { progress: f("deployProgress"), target: layout.postures[deployTarget] },
      // All three garrison fields are -1 without a building.
      garrison:
        garrisonPhase < 0
          ? null
          : {
              building: limbs(f, "garrisonBuilding")!,
              phase: layout.garrisonPhases[garrisonPhase],
              progress: f("garrisonProgress"),
              center: [f("garrisonX"), f("garrisonY")],
              half: [f("garrisonHalfX"), f("garrisonHalfY")],
            },
      stock: f("stock") < 0 ? null : f("stock"),
      service: layout.serviceStatuses[f("service")],
      sight: {
        eyes: sections.sightEyes as Point3[],
        forward: f("sightForward"),
        shape: { front: f("sightFront"), side: f("sightSide"), rear: f("sightRear") },
        range: f("sightRange"),
      },
    };
  });
  const identified = groups.identified.map(
    ({ field: f, sections }): IdentifiedView => ({
      id: f("id"),
      kind: layout.unitKinds[f("kind")],
      cost: f("cost"),
      position: [f("x"), f("y"), f("z")],
      yaw: f("yaw"),
      velocity: [f("vx"), f("vy")],
      members: sections.members as Point3[],
      memberIds: ids(sections.memberIds, seenIds),
      memberSlots: slots(sections.memberIds, seenIds),
      memberLeans: leans(sections.memberLeans, seenLeans),
      weaponPoses: poses(sections.weaponPoses, seenPoses),
      reversing: f("reversing") === 1,
    }),
  );
  const contacts = groups.contacts.map(
    ({ field: f }): ContactView => ({
      id: f("id"),
      source: layout.contactSources[f("source")],
      primaryLabel: f("primaryLabel") === 1,
      center: [f("x"), f("y")],
      radius: f("radius"),
      evidenceTick: f("evidenceTick"),
      expiresTick: f("expiresTick"),
      kind: f("kind") < 0 ? null : layout.unitKinds[f("kind")],
      heard: layout.roundKinds.filter((_, k) => (f("heard") >> k) & 1),
    }),
  );
  const audible = groups.audible.map(
    ({ field: f }): SoundCueView => ({
      listener: f("listener"),
      category: layout.soundCategories[f("category")],
      sector: f("sector"),
      band: layout.soundBands[f("band")],
      moving: f("moving") === 1,
    }),
  );
  const knownProps = reuse.knownProps
    ? baseline!.staticViews.knownProps
    : groups.knownProps.map(
        ({ field: f }): KnownPropView => ({
          kind: layout.propKinds[f("kind")],
          center: [f("x"), f("y")],
          yaw: f("yaw"),
          half: [f("hx"), f("hy"), f("hz")],
          baseZ: f("baseZ"),
          replaces: limbs(f, "replaces"),
          id: limbs(f, "id")!,
          building: limbs(f, "building"),
          structureOwner: limbs(f, "structureOwner"),
          authoredProp: limbs(f, "authoredProp"),
          destroyed: f("destroyed") === 1,
        }),
      );
  const bounce = reader("projectiles", "ricochets");
  const projectiles = groups.projectiles.map(({ field: f, sections }): ProjectileView => {
    const hit = layout.hitKinds[f("hit")];
    return {
      path: sections.path as Point3[],
      ricochets: sections.ricochets.map((p) => {
        const r = bounce(p);
        return { point: r("point"), normal: [r("nx"), r("ny"), r("nz")] };
      }),
      own: f("own") === 1,
      kind: layout.roundKinds[f("kind")],
      shooterMember: limbs(f, "shooter"),
      hit,
      impactNormal: hit === "none" ? null : [f("nx"), f("ny"), f("nz")],
    };
  });
  const blasts = groups.blasts.map(
    ({ field: f }): BlastView => ({
      point: [f("x"), f("y"), f("z")],
      radius: f("radius"),
      kind: layout.roundKinds[f("kind")],
    }),
  );
  const guided = groups.guided.map(
    ({ field: f }): GuidedView => ({
      id: limbs(f, "id")!,
      position: [f("x"), f("y"), f("z")],
      point: [f("px"), f("py"), f("pz")],
      supported: f("supported") === 1,
    }),
  );
  const corpses = reuse.corpses
    ? baseline!.staticViews.corpses
    : groups.corpses.map(
        ({ field: f }): CorpseView => ({
          position: [f("x"), f("y"), f("z")],
          own: f("own") === 1,
          soldier: limbs(f, "soldier")!,
          kind: layout.unitKinds[f("kind")],
          slot: f("slot"),
          yaw: f("yaw"),
        }),
      );
  return {
    tick: header.tick,
    own,
    identified,
    contacts,
    audible,
    knownProps,
    projectiles,
    blasts,
    corpses,
    guided,
    encounter:
      header.encounterResult < 0
        ? null
        : { heldS: header.encounterHeldS, result: layout.encounterResults[header.encounterResult] },
    fog: { cellM: header.fogCellM, nx: header.fogNx, ny: header.fogNy, bits },
    groundPatch,
  };
}

function decodeGroundPatch(
  layout: ObservationLayout,
  header: Record<string, number>,
  data: Float32Array,
  n: number,
): GroundRunsPatch {
  const { fields, tileSize, cols, rows } = layout.ground;
  if (
    fields.length !== 4 ||
    tileSize !== 16 ||
    !Number.isSafeInteger(cols) ||
    !Number.isSafeInteger(rows) ||
    cols <= 0 ||
    rows <= 0 ||
    cols * rows > 2 ** 32
  )
    throw new Error("ground grid does not match its admitted tile codec");
  const at = Object.fromEntries(fields.map((f, i) => [f, i]));
  if (n * fields.length > layout.ground.maxRecordBytes / 4 || n * 29 > data.length * 32)
    throw new Error("ground runs do not match their count");
  // Count and minimum 29 bits/run admission precede the only run allocation.
  const reader = new PackedReader(data);
  const runs = new Float32Array(n * fields.length);
  let tile = 0;
  for (let row = 0; row < runs.length; row += fields.length) {
    tile += reader.integer();
    const start = reader.read(8),
      len = reader.read(8) + 1;
    const mask = reader.read(5);
    let a = 0,
      b = 0;
    for (let mark = 0; mark < 5; mark++) {
      if ((mask & (1 << mark)) === 0) continue;
      const value = reader.read(8);
      if (value === 0) throw new Error("ground mark presence must encode a nonzero byte");
      if (mark < 2) a += value * 2 ** (mark * 8);
      else b += value * 2 ** ((mark - 2) * 8);
    }
    if (tile >= 2 ** 24) throw new Error("ground tile exceeds its exact address range");
    runs[row] = tile;
    runs[row + 1] = start + len * 256;
    runs[row + 2] = a;
    runs[row + 3] = b;
  }
  reader.finish();
  const tilesX = Math.ceil(cols / tileSize),
    tilesY = Math.ceil(rows / tileSize);
  let prior = -1,
    end = 0;
  for (let row = 0; row < runs.length; row += fields.length) {
    const tile = runs[row + at.tile],
      span = runs[row + at.span];
    const a = runs[row + at.craterScorch],
      b = runs[row + at.tracksTrampledCleared];
    const start = span % 256,
      len = Math.floor(span / 256);
    if (
      ![tile, span, a, b].every(Number.isInteger) ||
      tile < 0 ||
      tile >= tilesX * tilesY ||
      tile >= 2 ** 24 ||
      len <= 0 ||
      start + len > 256 ||
      a < 0 ||
      a > 65535 ||
      b < 0 ||
      b > 16777215 ||
      tile < prior ||
      (tile === prior && start < end)
    )
      throw new Error("ground run is unordered, overlapping or outside its exact encoding");
    const x = (tile % tilesX) * tileSize,
      y = Math.floor(tile / tilesX) * tileSize;
    if (
      y + Math.floor((start + len - 1) / tileSize) >= rows ||
      x + (start % tileSize) >= cols ||
      (x + tileSize > cols &&
        (Math.floor(start / tileSize) !== Math.floor((start + len - 1) / tileSize) ||
          x + ((start + len - 1) % tileSize) >= cols))
    )
      throw new Error("ground run crosses unused cells in a partial edge tile");
    prior = tile;
    end = start + len;
  }
  return {
    epoch: header.groundEpoch,
    side: layout.ground.sides[header.groundSide],
    baseRevision: header.groundBase,
    revision: header.groundRevision,
    full: header.groundFull === 1,
    // Credits return the input buffer immediately after applyRuns. A retained
    // observation must therefore own its exact compact payload.
    runs,
  };
}

function decodeMount(layout: ObservationLayout, f: (name: string) => number): MountView {
  const loaded = f("loaded");
  // Ammo per kind: -1 unlimited, -2 no such kind on this mount.
  const ammo = [f("ammo0"), f("ammo1")].slice(0, f("kinds")).map((n) => (n === -1 ? null : n));
  const kind = layout.targetKinds[f("targetKind")];
  const target: MountTargetView | null =
    kind === "none"
      ? null
      : kind === "ground"
        ? { kind, point: [f("targetX"), f("targetY"), f("targetZ")] }
        : { kind: kind as "identified" | "contact", id: f("targetId") };
  return {
    mount: f("mount"),
    loaded: loaded < 0 ? null : loaded,
    ammo,
    aim: f("aim"),
    reload: f("reload"),
    target,
    reason: layout.actionReasons[f("reason")],
    guiding: f("guiding") === 1,
    reloading: f("reloadKind") < 0 ? null : f("reloadKind"),
  };
}
