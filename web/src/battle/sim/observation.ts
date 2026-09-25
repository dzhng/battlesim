/** Decodes a side publication using the layout the simulation publishes. */

export interface ObservationLayout {
  header: string[];
  own: { stride: number; fields: string[] };
  unitKinds: string[];
}

export interface OwnUnitView {
  id: number;
  kind: string;
  position: [number, number, number];
  yaw: number;
  goal: [number, number] | null;
  queued: number;
}

export interface ObservationView {
  tick: number;
  own: OwnUnitView[];
}

export function decodeObservation(layout: ObservationLayout, data: Float32Array): ObservationView {
  const header = Object.fromEntries(layout.header.map((f, i) => [f, data[i]]));
  const at = Object.fromEntries(layout.own.fields.map((f, i) => [f, i]));
  const own: OwnUnitView[] = [];
  let base = layout.header.length;
  for (let n = 0; n < header.ownCount; n++, base += layout.own.stride) {
    const f = (name: string) => data[base + at[name]];
    own.push({
      id: f("id"),
      kind: layout.unitKinds[f("kind")],
      position: [f("x"), f("y"), f("z")],
      yaw: f("yaw"),
      goal: Number.isNaN(f("goalX")) ? null : [f("goalX"), f("goalY")],
      queued: f("queued"),
    });
  }
  return { tick: header.tick, own };
}
