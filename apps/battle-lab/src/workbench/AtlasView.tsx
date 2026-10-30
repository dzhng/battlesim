import { useRef, useEffect } from "react";
import type { ImpostorAtlas } from "@packages/battle-renderer/src/models/impostor";
import type { FeedSource } from "@web/feed";
import { atlasCanvas } from "./sheet";
/** The baked atlas's channels as canvases; fed (`useFeed`), since its pixels
 *  as a changing prop would fill React's development measures. */
export function AtlasView({ atlas }: { atlas: FeedSource<ImpostorAtlas | null> }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const draw = (next: ImpostorAtlas | null) => {
      const el = ref.current;
      if (!el || !next) return;
      el.replaceChildren(
        atlasCanvas(next.width, next.height, next.albedo),
        atlasCanvas(next.width, next.height, next.normal),
      );
    };
    draw(atlas.current);
    return atlas.subscribe(draw);
  }, [atlas]);
  return <div ref={ref} className="wb-atlas" />;
}
