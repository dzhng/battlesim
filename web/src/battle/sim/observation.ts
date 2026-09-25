/** Decodes a side publication using the layout the simulation publishes. */

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
  fog: { bitsPerFloat: number; count: string };
  unitKinds: string[];
  moveStates: string[];
  policies: string[];
  contactSources: string[];
  soundCategories: string[];
  soundBands: string[];
  propKinds: string[];
  engagements: string[];
  actionReasons: string[];
  targetKinds: string[];
  postures: string[];
  garrisonPhases: string[];
  serviceStatuses: string[];
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
  state: string;
  /** The friendly unit this one waits for. */
  blocker: number | null;
  route: Point2[];
  queue: Point2[];
  members: Point3[];
  /** Enemy handles this unit's own sensors identify. */
  sees: number[];
  engagement: string;
  mounts: MountView[];
  /** Vehicle health (0 for infantry). */
  hp: number;
  /** Health of each living soldier, in `members` order. */
  memberHp: number[];
  /** Infantry suppression in [0, 1]. */
  suppression: number;
  /** Setup progress for units that deploy in place; null for the rest. */
  deployment: DeploymentView | null;
  /** The squad's building while entering, inside or leaving it; null otherwise. */
  garrison: GarrisonView | null;
  /** A supply vehicle's remaining stock; null for other units. */
  stock: number | null;
  /** Why this unit is or is not being served by a supply vehicle. */
  service: string;
}

/** A squad's hold on a building: which one, the phase and its timer progress. */
export interface GarrisonView {
  /** The building's prop id in the static map. */
  building: number;
  /** entering, waiting_for_room, inside or exiting. */
  phase: string;
  /** Entering or leaving progress in [0, 1]; 1 while inside. */
  progress: number;
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
  from: Point3;
  to: Point3;
  own: boolean;
  /** The round struck something at `to` this tick. */
  impact: boolean;
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
}

/** Uncertain evidence: an area, never a class or exact position. */
export interface ContactView {
  id: number;
  source: string;
  center: Point2;
  radius: number;
  evidenceTick: number;
  expiresTick: number;
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

/** An obstacle added after setup that this side knows about. */
export interface KnownPropView {
  kind: string;
  center: Point2;
  yaw: number;
  half: Point3;
  baseZ: number;
  /** The authored prop this one stands in place of (a ruin's building), or null. */
  replaces: number | null;
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
  knownProps: KnownPropView[];
  projectiles: ProjectileView[];
  corpses: CorpseView[];
  guided: GuidedView[];
  fog: VisibilityView;
}

type Row = { field: (name: string) => number; sections: Record<string, number[][]> };

export function decodeObservation(layout: ObservationLayout, data: Float32Array): ObservationView {
  const header = Object.fromEntries(layout.header.map((f, i) => [f, data[i]]));
  let cursor = layout.header.length;
  const groups: Record<string, Row[]> = {};
  for (const group of layout.groups) {
    const at = Object.fromEntries(group.fields.map((f, i) => [f, i]));
    const rows: Row[] = [];
    for (let n = 0; n < header[group.count]; n++, cursor += group.fields.length) {
      const base = cursor;
      rows.push({ field: (name) => data[base + at[name]], sections: {} });
    }
    // Each row's variable sections follow all the rows, in row order.
    for (const row of rows) {
      for (const section of group.sections) {
        const width = section.fields.length;
        const points: number[][] = [];
        for (let k = 0; k < row.field(section.count); k++, cursor += width) {
          points.push(Array.from(data.subarray(cursor, cursor + width)));
        }
        row.sections[section.name] = points;
      }
    }
    groups[group.name] = rows;
  }
  const cells = header.fogNx * header.fogNy;
  const bits = new Uint32Array(Math.ceil(cells / 32));
  const per = layout.fog.bitsPerFloat;
  for (let w = 0; w < header[layout.fog.count]; w++) {
    const word = data[cursor + w];
    for (let b = 0; b < per; b++) {
      const k = w * per + b;
      if (k < cells && word & (1 << b)) bits[k >> 5] |= 1 << (k & 31);
    }
  }

  const mountFields = layout.groups
    .find((g) => g.name === "own")!
    .sections.find((s) => s.name === "mounts")!.fields;
  const mountAt = Object.fromEntries(mountFields.map((f, i) => [f, i]));
  const own = groups.own.map(({ field: f, sections }): OwnUnitView => {
    const policy = f("policy");
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
      state: layout.moveStates[f("state")],
      blocker: blocker < 0 ? null : blocker,
      route: sections.route as Point2[],
      queue: sections.queue as Point2[],
      members: sections.members as Point3[],
      sees: sections.sees.map((p) => p[0]),
      engagement: layout.engagements[f("engagement")],
      mounts: sections.mounts.map((m) => decodeMount(layout, mountAt, m)),
      hp: f("hp"),
      memberHp: sections.memberHp.map((p) => p[0]),
      suppression: f("suppression"),
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
              building: f("garrisonBuilding"),
              phase: layout.garrisonPhases[garrisonPhase],
              progress: f("garrisonProgress"),
            },
      stock: f("stock") < 0 ? null : f("stock"),
      service: layout.serviceStatuses[f("service")],
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
    }),
  );
  const contacts = groups.contacts.map(
    ({ field: f }): ContactView => ({
      id: f("id"),
      source: layout.contactSources[f("source")],
      center: [f("x"), f("y")],
      radius: f("radius"),
      evidenceTick: f("evidenceTick"),
      expiresTick: f("expiresTick"),
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
  const knownProps = groups.knownProps.map(
    ({ field: f }): KnownPropView => ({
      kind: layout.propKinds[f("kind")],
      center: [f("x"), f("y")],
      yaw: f("yaw"),
      half: [f("hx"), f("hy"), f("hz")],
      baseZ: f("baseZ"),
      replaces: f("replaces") < 0 ? null : f("replaces"),
    }),
  );
  const projectiles = groups.projectiles.map(
    ({ field: f }): ProjectileView => ({
      from: [f("x0"), f("y0"), f("z0")],
      to: [f("x1"), f("y1"), f("z1")],
      own: f("own") === 1,
      impact: f("impact") === 1,
    }),
  );
  const guided = groups.guided.map(
    ({ field: f }): GuidedView => ({
      id: f("id"),
      position: [f("x"), f("y"), f("z")],
      point: [f("px"), f("py"), f("pz")],
      supported: f("supported") === 1,
    }),
  );
  const corpses = groups.corpses.map(
    ({ field: f }): CorpseView => ({ position: [f("x"), f("y"), f("z")], own: f("own") === 1 }),
  );
  return {
    tick: header.tick,
    own,
    identified,
    contacts,
    audible,
    knownProps,
    projectiles,
    corpses,
    guided,
    fog: { cellM: header.fogCellM, nx: header.fogNx, ny: header.fogNy, bits },
  };
}

function decodeMount(
  layout: ObservationLayout,
  at: Record<string, number>,
  row: number[],
): MountView {
  const f = (name: string) => row[at[name]];
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
  };
}

export function fogVisible(fog: VisibilityView, x: number, y: number): boolean {
  const i = Math.floor(x / fog.cellM),
    j = Math.floor(y / fog.cellM);
  if (i < 0 || j < 0 || i >= fog.nx || j >= fog.ny) return false;
  const k = j * fog.nx + i;
  return (fog.bits[k >> 5] & (1 << (k & 31))) !== 0;
}
