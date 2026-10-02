// A lab's switch for the detail tier its buildings are drawn at: by distance,
// as the game draws them, or every building at one tier. The viewport's frame
// takes its building style when it is built (`LabViewport` `buildingStyle`),
// so a change of tier rebuilds the frame.
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { BuildingStyle } from "@packages/battle-renderer/src/models/buildingReferences";
import { metresPerPxAt } from "@packages/renderer-core/src/camera3d";
import { gameCamera } from "./gameCamera";
import { gameBuildingStyle } from "./gameModels";

/** `style` with every building drawn at `tier` whatever its distance, or
 *  `style` itself for null. The tier is still chosen by the style's own rule:
 *  the thresholds are put where no view, or every view, passes them. */
export function tierStyle(style: BuildingStyle, tier: number | null): BuildingStyle {
  if (tier === null) return style;
  const never = [4e9, 2e9, 1e9];
  const always = [4e-9, 2e-9, 1e-9];
  return {
    ...style,
    lod_px_per_m: [0, 1, 2].map((k) => (k < tier ? never[k] : always[k])) as [
      number,
      number,
      number,
    ],
  };
}

/** How far off a building changes from tier `boundary - 1` to `boundary`, in
 *  a viewport `heightPx` tall through a lens of `fovY`: where a metre covers
 *  the style's threshold in pixels. */
export function boundaryRange(
  style: BuildingStyle,
  boundary: 1 | 2 | 3,
  fovY: number,
  heightPx: number,
): number {
  return 1 / (metresPerPxAt(1, fovY, heightPx) * style.lod_px_per_m[boundary - 1]);
}

/** The tier a lab address asks for (`?tier=0..3`), or null: by distance. */
export function askedTier(search: string): number | null {
  const tier = new URLSearchParams(search).get("tier");
  return tier !== null && /^[0-3]$/.test(tier) ? Number(tier) : null;
}

/** How far off a building changes to tier 1, 2 and 3 in this window, metres
 *  from the eye to its chunk. */
export function tierBoundaries(): [number, number, number] {
  const heightPx = window.innerHeight * (window.devicePixelRatio || 1);
  const at = (boundary: 1 | 2 | 3) =>
    boundaryRange(gameBuildingStyle, boundary, gameCamera.lens.fovY, heightPx);
  return [at(1), at(2), at(3)];
}

export function useBuildingTier(initial: number | null): {
  tier: number | null;
  /** The style to build the frame with. */
  style: BuildingStyle;
  /** Draw every building at tier `next` (null: by distance); resolves once
   *  the frame is rebuilt with it. */
  drawAt: (next: number | null) => Promise<void>;
} {
  const [tier, setTier] = useState(initial);
  const style = useMemo(() => tierStyle(gameBuildingStyle, tier), [tier]);
  const built = useRef(style);
  const waiting = useRef<(() => void)[]>([]);
  useEffect(() => {
    if (built.current === style) return;
    built.current = style;
    void Promise.resolve(window.__lab?.rebuild?.()).then(() => {
      for (const done of waiting.current.splice(0)) done();
    });
  }, [style]);
  const now = useRef(tier);
  now.current = tier;
  const drawAt = useCallback(
    (next: number | null) =>
      new Promise<void>((resolve) => {
        if (now.current === next) return resolve();
        waiting.current.push(resolve);
        setTier(next);
      }),
    [],
  );
  return { tier, style, drawAt };
}
