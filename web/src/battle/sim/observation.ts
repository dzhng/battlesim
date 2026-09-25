/** Decodes a side publication using the layout the simulation publishes. */

interface Section {
  name: string;
  count: string;
  fields: string[];
}

export interface ObservationLayout {
  header: string[];
  own: { stride: number; fields: string[] };
  sections: Section[];
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
}

export interface ObservationView {
  tick: number;
  own: OwnUnitView[];
}

export function decodeObservation(layout: ObservationLayout, data: Float32Array): ObservationView {
  const header = Object.fromEntries(layout.header.map((f, i) => [f, data[i]]));
  const at = Object.fromEntries(layout.own.fields.map((f, i) => [f, i]));
  const rows: ((name: string) => number)[] = [];
  let base = layout.header.length;
  for (let n = 0; n < header.ownCount; n++, base += layout.own.stride) {
    const row = base;
    rows.push((name) => data[row + at[name]]);
  }
  // Variable sections follow the rows, in unit order, in layout section order.
  let cursor = base;
  const own = rows.map((f): OwnUnitView => {
    const sections: Record<string, number[][]> = {};
    for (const section of layout.sections) {
      const width = section.fields.length;
      const points: number[][] = [];
      for (let k = 0; k < f(section.count); k++, cursor += width) {
        points.push(Array.from(data.subarray(cursor, cursor + width)));
      }
      sections[section.name] = points;
    }
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
    };
  });
  return { tick: header.tick, own };
}
