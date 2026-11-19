/** Rings round the HUD controls a coach step points at, following each
 *  control's rectangle as the HUD lays out. A control that isn't on screen
 *  has no ring. */
import { useEffect, useState } from "react";
import { anchorElement, type HudAnchor } from "../present/hudAnchors";

interface Ring {
  anchor: HudAnchor;
  left: number;
  top: number;
  width: number;
  height: number;
}

const same = (a: Ring[], b: Ring[]) =>
  a.length === b.length &&
  a.every(
    (ring, i) =>
      ring.anchor === b[i].anchor &&
      ring.left === b[i].left &&
      ring.top === b[i].top &&
      ring.width === b[i].width &&
      ring.height === b[i].height,
  );

export function CoachHighlights({ anchors }: { anchors: readonly HudAnchor[] }) {
  const [rings, setRings] = useState<Ring[]>([]);
  const key = anchors.join(" ");
  useEffect(() => {
    if (!key) return;
    const names = key.split(" ") as HudAnchor[];
    let frame = 0;
    const follow = () => {
      const next = names.flatMap((anchor): Ring[] => {
        const box = anchorElement(anchor)?.getBoundingClientRect();
        if (!box || box.width === 0) return [];
        return [{ anchor, left: box.left, top: box.top, width: box.width, height: box.height }];
      });
      setRings((current) => (same(current, next) ? current : next));
      frame = requestAnimationFrame(follow);
    };
    frame = requestAnimationFrame(follow);
    return () => cancelAnimationFrame(frame);
  }, [key]);
  return (
    <>
      {rings
        .filter((ring) => anchors.includes(ring.anchor))
        .map((ring) => (
          <span
            key={ring.anchor}
            className="hud-coach-highlight"
            data-highlight={ring.anchor}
            aria-hidden
            style={{ left: ring.left, top: ring.top, width: ring.width, height: ring.height }}
          />
        ))}
    </>
  );
}
