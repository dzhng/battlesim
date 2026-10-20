/** Triangle-list work issued to draw/drawIndexed, before clipping or discard.
 * Instances count per submission, not unique objects across surfaces/passes. */
export interface SubmittedDraws {
  triangles: number;
  instances: number;
  draws: number;
}
export const submittedDraws = (): SubmittedDraws => ({ triangles: 0, instances: 0, draws: 0 });

export function recordDraw(
  counter: SubmittedDraws | undefined,
  vertices: number,
  instances: number,
) {
  if (!counter) return;
  counter.triangles += Math.floor(vertices / 3) * instances;
  counter.instances += instances;
  counter.draws++;
}

/** Deliberately partial frame accounting: effects, grass, sky, paint, proxies,
 * x-ray and overlays are excluded. Water and backdrop ground are separate;
 * scenery includes its backdrop population. The skirt shares a mesh with debug
 * traversal boxes. Models include appearance props and cards. */
export function submittedPass() {
  return {
    terrain: submittedDraws(),
    skirtAndDebugProps: submittedDraws(),
    water: submittedDraws(),
    backdrop: submittedDraws(),
    buildings: submittedDraws(),
    scenery: submittedDraws(),
    modelsIncludingProps: submittedDraws(),
  };
}
export type SubmittedPass = ReturnType<typeof submittedPass>;
export interface SubmittedFrame {
  depth: SubmittedPass;
  color: SubmittedPass;
  highDepth: SubmittedPass;
  highColor: SubmittedPass;
  cascades: SubmittedPass[];
}

/** Counters are retained across frames; snapshots allocate only on inspection. */
export function createSubmittedFrame() {
  const depth = submittedPass(),
    color = submittedPass();
  const highDepth = submittedPass(),
    highColor = submittedPass();
  const cascades: SubmittedPass[] = [];
  let cascadeCount = 0;
  const clear = (pass: SubmittedPass) => {
    for (const key in pass) {
      const counter = pass[key as keyof SubmittedPass];
      counter.triangles = counter.instances = counter.draws = 0;
    }
  };
  return {
    depth,
    color,
    highDepth,
    highColor,
    reset() {
      clear(depth);
      clear(color);
      clear(highDepth);
      clear(highColor);
      for (const pass of cascades) clear(pass);
      cascadeCount = 0;
    },
    cascade(index: number) {
      cascadeCount = Math.max(cascadeCount, index + 1);
      while (cascades.length <= index) cascades.push(submittedPass());
      return cascades[index];
    },
    snapshot(): SubmittedFrame {
      return structuredClone({
        depth,
        color,
        highDepth,
        highColor,
        cascades: cascades.slice(0, cascadeCount),
      });
    },
  };
}
