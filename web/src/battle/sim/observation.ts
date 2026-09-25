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

  const own = groups.own.map(({ field: f, sections }): OwnUnitView => {
    const policy = f("policy");
    const blocker = f("blocker");
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
  return {
    tick: header.tick,
    own,
    identified,
    fog: { cellM: header.fogCellM, nx: header.fogNx, ny: header.fogNy, bits },
  };
}

export function fogVisible(fog: VisibilityView, x: number, y: number): boolean {
  const i = Math.floor(x / fog.cellM),
    j = Math.floor(y / fog.cellM);
  if (i < 0 || j < 0 || i >= fog.nx || j >= fog.ny) return false;
  const k = j * fog.nx + i;
  return (fog.bits[k >> 5] & (1 << (k & 31))) !== 0;
}
