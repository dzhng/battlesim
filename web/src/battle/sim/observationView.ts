/** One side's observation as the Rust reader decodes it
 * (`crates/sim/src/publication/reader.rs`), and the publication layout it
 * reads. Integers that outgrow a float32's exact range (soldier ids, shot
 * counters) arrive whole; an absent one is null. */

import type { MoveDirection } from "./protocol";

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
  /** What a round struck: none, ground, hull, prop, soldier or metal_prop. */
  hitKinds: string[];
  unitKinds: string[];
  moveStates: string[];
  policies: string[];
  directions: string[];
  contactSources: string[];
  /** Height bands a contact's cause can occupy: ground, low_air. */
  layers: string[];
  /** What an unseen enemy's sound was: infantry, vehicle, shot or shout. */
  soundCategories: string[];
  soundBands: string[];
  /** The barks B1–B17, in order (`b1_contact_infantry`, …): the contract's names. */
  shoutTriggers: string[];
  /** How a bark is said: steady, stressed. */
  intensities: string[];
  propKinds: string[];
  engagements: string[];
  actionReasons: string[];
  targetKinds: string[];
  postures: string[];
  /** Cover tiers, weakest first: light, medium, heavy. */
  coverTiers: string[];
  /** Which way a soldier leans out round his cover: left, right. */
  leanSides: string[];
  hostKinds: string[];
  holdPhases: string[];
  orderErrors: string[];
  serviceStatuses: string[];
  /** Suppression tiers: none, suppressed, pinned. */
  suppressionTiers: SuppressionTier[];
  encounterResults: string[];
  objectiveIds: string[];
}

export type Point2 = [number, number];
export type Point3 = [number, number, number];

export interface OwnUnitView {
  withdrawing: boolean;
  fuelS: number | null;
  afterburner: boolean;
  inDogfight: boolean;
  flares: { charges: number; cooldown: number | null } | null;
  /** Defensive charges and running cooldown progress; private to the owning side. */
  protection: { charges: number; cooldown: number | null } | null;
  id: number;
  owner?: number | null;
  kind: string;
  position: Point3;
  yaw: number;
  goal: Point2 | null;
  /** The current movement order lets its passengers out at `goal`. */
  goalUnloads: boolean;
  policy: string | null;
  /** Which way the current move drives (Q31); null without a move. */
  direction: MoveDirection | null;
  /** Driving backwards this tick: a reverse move or a three-point turn's
   *  reversing leg (the reverse beeper's cue). */
  reversing: boolean;
  state: string;
  /** The friendly unit this one waits for. */
  blocker: number | null;
  route: Point2[];
  queue: Point2[];
  /** Whether each `queue` order lets its passengers out at its destination. */
  queueUnloads: boolean[];
  members: Point3[];
  /** Each living soldier's id, in `members` order. */
  memberIds: number[];
  /** Each living soldier's slot in his squad type (which soldier kind he is). */
  memberSlots: number[];
  /** Each published soldier's selected mount, or null, in member order. */
  memberActiveMounts: (number | null)[];
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
  /** It trails smoke, by the rule an enemy's is published (`IdentifiedView`). */
  smoking: boolean;
  /** Health of each living soldier, in `members` order. */
  memberHp: number[];
  /** How suppressed the squad is, by the rules' thresholds (the sim owns
   *  them; the hidden level is never published). Vehicles are "none". */
  suppression: SuppressionTier;
  /** Enough samples on concealing ground with net concealment below 1, and no visible enemy observer spotting it or known engagement. */
  concealed: boolean;
  /** Setup progress for units that deploy in place; null for the rest. */
  deployment: DeploymentView | null;
  /** The squad's host (a building or a carrier) while entering, inside or
   *  leaving it; null otherwise. */
  hold: HoldView | null;
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

/** A unit's host while entering, inside or leaving it. */
export type HoldHostView =
  | { kind: "building"; id: number; center: Point2; half: Point2 }
  | { kind: "carrier"; id: number };

/** Whether a carrier holds `u` (boarding, aboard or getting out): it is
 *  not on the field, and the player can't select or order it (D10). */
export const aboardCarrier = (u: Pick<OwnUnitView, "hold">) => u.hold?.host.kind === "carrier";

/** Whether a building holds `u` (entering, garrisoned or leaving). */
export const inBuilding = (u: Pick<OwnUnitView, "hold">) => u.hold?.host.kind === "building";

/** The carrier holding `u`, or null when none does. */
export const carrierOf = (u: Pick<OwnUnitView, "hold">) =>
  u.hold?.host.kind === "carrier" ? u.hold.host.id : null;

/** The own squads a carrier holds (boarding, aboard or getting out), in
 *  the side's own order: the one owner of who rides in what, which the
 *  Seats row counts, the readouts stack under the carrier's card and
 *  Unload reaches. */
export function passengersOf<U extends Pick<OwnUnitView, "hold">>(
  own: readonly U[],
  carrier: number,
): readonly U[] {
  return own.filter((u) => carrierOf(u) === carrier);
}

export interface HoldView {
  host: HoldHostView;
  /** entering, inside or exiting. */
  phase: string;
  /** Entering or leaving progress in [0, 1]; 1 while inside. */
  progress: number;
}

export interface RefusalView {
  unit: number;
  reason: string;
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

/** A toppled body (a felled tree) this side knocked down or saw where it
 * stood: blocks nothing, gives no cover. Presentation draws its fall. */
export interface FallenBodyView {
  /** The body's prop id (a trunk's id in the props export). */
  prop: number;
  /** Where its foot stood. */
  at: [number, number];
  /** Horizontal unit direction it fell toward. */
  toward: [number, number];
  /** The tick it fell, however late this side learned of it. */
  tick: number;
}

/** A downed airframe still falling, seen by a side that saw it go down. Its
 *  wreck takes over where it lands. */
export interface FallingAirframeView {
  /** The id the side knew it by: an own unit's id, else its identified handle. */
  id: number;
  own: boolean;
  /** Its unit type, so it is drawn as itself. */
  kind: string;
  /** Its foot. */
  position: Point3;
  yaw: number;
  /** Nose up, radians: negative as its nose drops into the fall. */
  pitch: number;
  /** Right side down, radians: it leans into its spin. */
  roll: number;
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
  /** Index of the last launched row in the mount’s weapon list; 0 before firing. */
  round: number;
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

/** One of this side's own guided missiles and its commanded point. */
export interface GuidedView {
  /** Stable while it flies. */
  id: number;
  position: Point3;
  /** Its commanded point; a top-attack missile climbs above it before
   *  diving onto it, so this is not where it is heading now. */
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
  /** What the round struck at the path's end this tick: none, ground, hull, prop, soldier or metal_prop. */
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
  /** Each published soldier's selected mount, or null, in member order. */
  memberActiveMounts: (number | null)[];
  /** Each seen soldier's lean, in `members` order: null while tucked in. */
  memberLeans: (MemberLeanView | null)[];
  /** Every mount's pose while identified. */
  weaponPoses: WeaponPoseView[];
  /** Driving backwards this tick (a seen vehicle's reverse beeper). */
  reversing: boolean;
  /** It trails smoke: a coarse sign of damage (an aircraft below half its
   *  health), never its health. */
  smoking: boolean;
}

/** Uncertain evidence: an area, never an exact position, strength or
 *  count; only what the side learned when the evidence came. */
export interface ContactView {
  id: number;
  source: string;
  /** The area's centre, at its cause's height when the evidence came. */
  center: Point3;
  /** The height band its cause occupied (`layers`): an air contact takes
   *  no area fire. */
  layer: string;
  /** The area hangs in the air at `center`'s height: its cause flew more
   *  than the low hover over the ground (D33). Every other area lies on the
   *  ground, a low-hovering helicopter's too. */
  aloft: boolean;
  radius: number;
  evidenceTick: number;
  expiresTick: number;
  /** The type this side previously identified; null when never identified. */
  kind: string | null;
  /** A firing report's weapon rows as heard (round kinds, each mount's
   *  every row: a report doesn't say which round); empty for a last sighting. */
  heard: string[];
  /** Quantized bearing from the receiver that found it, present for radar. */
  bearingDeg?: number | null;
  /** Quantized range from the receiver that found it, present for radar. */
  rangeM?: number | null;
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

/** A bark this side hears: its own unit's, or an identified enemy's. An
 *  unseen enemy's shout is never one; it reaches the side only as an anonymous
 *  `shout` sound cue. */
export interface ShoutView {
  /** True for this side's own unit, false for an identified enemy. */
  own: boolean;
  /** The own unit's id, or the identified enemy's handle. */
  id: number;
  kind: string;
  /** Where it was said; a unit that has just died still speaks its last line. */
  position: Point3;
  /** One of the layout's `shoutTriggers`. */
  trigger: string;
  /** One of the layout's `intensities`. */
  intensity: string;
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
  /** A wreck's unit type: drawn as that unit's own wreck. Null for any other prop. */
  wreckOf: string | null;
}

/** Ground cells this side can see, one bit each (row-major). */
export interface VisibilityView {
  cellM: number;
  nx: number;
  ny: number;
  bits: Uint32Array;
}

export interface ObjectiveView {
  id: string;
  center: Point2;
  radiusM: number;
  owner: "blue" | "red" | null;
  capturing: "blue" | "red" | null;
  captureProgress: number;
  contested: boolean;
}

export interface SkirmishView {
  phase: "preparation" | "active" | "finished";
  players: PlayerView[];
  preparationRemainingS: number;
  scores: [number, number];
  result: "blue" | "red" | "draw" | null;
  objectives: ObjectiveView[];
  report: MatchReportView | null;
}

export interface PlayerView {
  index: number;
  id: string;
  controller: "human" | "active_ai" | "passive_ai";
  ready: boolean;
  credits: number;
  unitCount: number;
  cap: number;
  pending: PendingPurchaseView[];
}

export interface PendingPurchaseView {
  owner?: number;
  id: number;
  kind: string;
  destination: Point2;
  confirmedTick: number;
  blocked: boolean;
  /** The tick a waiting aircraft reaches the map; 0 when it doesn't wait. */
  arrivalTick: number;
}

export interface MatchReportView {
  totals: { spawned: number; spent: number; killed: number; refunded: number }[];
  events: MatchEventView[];
}

export type MatchEventView =
  | {
      kind: "spawn";
      tick: number;
      side: "blue" | "red";
      unitId: number;
      unitKind: string;
      owner?: number | null;
      amount: number;
    }
  | {
      kind: "refund";
      tick: number;
      side: "blue" | "red";
      unitId: number;
      unitKind: string;
      owner?: number | null;
      amount: number;
    }
  | {
      kind: "kill";
      tick: number;
      side: "blue" | "red";
      unitId: number;
      unitKind: string;
      owner?: number | null;
      victimCost: number;
      killer: {
        side: "blue" | "red";
        unitId: number;
        unitKind: string;
        owner?: number | null;
        cost: number;
      } | null;
    };

export interface ObservationView {
  tick: number;
  own: OwnUnitView[];
  identified: IdentifiedView[];
  contacts: ContactView[];
  audible: SoundCueView[];
  refusals: RefusalView[];
  knownProps: readonly KnownPropView[];
  projectiles: ProjectileView[];
  blasts: BlastView[];
  airTransits: AirTransitView[];
  corpses: readonly CorpseView[];
  fallenBodies: readonly FallenBodyView[];
  /** Downed airframes still falling that this side saw go down. */
  crashes: FallingAirframeView[];
  /** Barks spoken this tick that this side hears. */
  shouts: ShoutView[];
  guided: GuidedView[];
  /** The fixture's completion condition, when it has one. */
  encounter: { heldS: number; result: string } | null;
  skirmish: SkirmishView | null;
  fog: VisibilityView;
  /** Which patch of the side's learned ground this publication carried
   *  (the presentation applies its runs: `LearnedGround`). */
  groundPatch: {
    epoch: number;
    side: string;
    baseRevision: number;
    revision: number;
    full: boolean;
  };
  /** The bodies shoved this tick that this side sees, where they stand
   *  (ahead of `knownProps`, which re-learns a shoved body only now and then). */
  propDisplayPoses: { propId: number; center: Point2; yaw: number; baseZ: number }[];
  /** Where hulls grind the metal bodies shoved this tick that this side sees:
   *  halfway up the lower of hull and body, with the pusher's travel. */
  grinds: { point: Point3; heading: Point2 }[];
}

export interface AirTransitView {
  kind: string;
  point: Point3;
  heading: number;
  speedMps: number;
  edgeTick: number;
  dir: "in" | "out";
}
